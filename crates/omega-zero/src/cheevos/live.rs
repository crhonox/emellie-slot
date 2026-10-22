use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use omega_cheevos::{
    config, game, memory, Cheevos, Credentials, Http, Login, Notice, Offline, Secret,
};
use omega_gfx::{Compositor, Draw, TexId};
use omega_retro::RetroCore;
use omega_store::Platform;

use super::boot;
use super::overlay::{self, Face, Queue};

const LOGIN_BACKOFF: [u64; 4] = [15, 30, 60, 300];
const LOAD_RETRY: Duration = Duration::from_secs(5);
const SEND_RETRY: [u64; 4] = [5, 30, 60, 300];
const IDLE_AFTER: Duration = Duration::from_millis(250);
const IDLE_EVERY: Duration = Duration::from_secs(1);

/// Where the account stands, as Settings shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Turned off in Settings, or no client at all.
    #[default]
    Off,
    /// On, but no username and password saved.
    NoAccount,
    /// Waiting for Wi-Fi to sign in.
    Waiting,
    SigningIn,
    SignedIn(String),
    /// The server turned the account down: the password needs typing again.
    Rejected(String),
}

struct Live {
    cheevos: &'static Cheevos,
    root: PathBuf,
    faces: Mutex<Queue>,
    tickets: AtomicU64,
    current: AtomicU64,
    booting: AtomicU64,
    switch: Mutex<()>,
    status: Mutex<Status>,
    /// The account the sign-in thread is to sign in with next, and how many times Settings
    /// changed it, so a sign-in made for an account since replaced is let go.
    wanted: Mutex<(Option<Credentials>, u64)>,
    changed: Condvar,
}

static LIVE: OnceLock<Live> = OnceLock::new();

/// Whether Wi-Fi has a network joined, as the app saw it on its last frame.
static ONLINE: AtomicBool = AtomicBool::new(false);

struct Seated {
    ticket: u64,
    console: Arc<AtomicU32>,
    boot_left: u32,
    last_frame: Option<Instant>,
    last_idle: Option<Instant>,
}

struct Shown {
    tex: Option<TexId>,
    since: Option<Instant>,
}

thread_local! {
    static SEATED: RefCell<Option<Seated>> = const { RefCell::new(None) };
    static SHOWN: RefCell<Shown> = const { RefCell::new(Shown { tex: None, since: None }) };
}

/// Starts RetroAchievements for the card at `root`: the client, and a sign-in with the account
/// in Config/retroachievements.txt once Wi-Fi is up. Settings can sign another in later.
pub fn start(root: &Path) {
    if LIVE.get().is_some() || std::env::var_os("OMEGA_NO_CHEEVOS").is_some() {
        return;
    }
    let host = omega_store::ini::value(root, omega_cheevos::CHEEVOS_FILE, "host");
    let offline = Offline::new(root, online);
    let cheevos = Cheevos::start_with(host.as_deref(), |ua| Box::new(Http::new(ua)), Some(offline));
    let live = Live {
        cheevos,
        root: root.to_path_buf(),
        faces: Mutex::new(Queue::default()),
        tickets: AtomicU64::new(0),
        current: AtomicU64::new(0),
        booting: AtomicU64::new(0),
        switch: Mutex::new(()),
        status: Mutex::new(Status::Off),
        wanted: Mutex::new((None, 0)),
        changed: Condvar::new(),
    };
    if LIVE.set(live).is_err() {
        return;
    }
    account_changed(root);
    spawn("omega-cheevos-login", sign_in);
    spawn("omega-cheevos-load", retry_loads);
    spawn("omega-cheevos-send", send_queued);
    spawn("omega-cheevos-face", draw_faces);
}

/// Whether Wi-Fi is up: the frontend says so each frame.
pub fn set_online(online: bool) {
    ONLINE.store(online, Ordering::Release);
}

fn online() -> bool {
    ONLINE.load(Ordering::Acquire)
}

pub fn status() -> Status {
    LIVE.get()
        .map(|l| l.status.lock().unwrap().clone())
        .unwrap_or_default()
}

/// Settings saved, changed or removed the account, or turned RetroAchievements on or off: the
/// one signed in is signed out and the new one, if any, signed in.
pub fn account_changed(root: &Path) {
    let Some(live) = LIVE.get() else { return };
    live.cheevos.logout();
    let credentials = Credentials::read(root);
    let account = config::account(root);
    *live.status.lock().unwrap() = match (&credentials, account.enabled) {
        (_, false) => Status::Off,
        (None, true) => Status::NoAccount,
        (Some(_), true) => Status::Waiting,
    };
    let mut wanted = live.wanted.lock().unwrap();
    wanted.0 = credentials;
    wanted.1 += 1;
    live.changed.notify_all();
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    let thread = std::thread::Builder::new()
        .name(name.into())
        .stack_size(omega_cheevos::THREAD_STACK);
    if let Err(e) = thread.spawn(f) {
        eprintln!("omega-zero: cheevos: {e}");
    }
}

fn set_status(live: &Live, round: u64, status: Status) {
    // a sign-in Settings has since replaced says nothing
    if live.wanted.lock().unwrap().1 == round {
        *live.status.lock().unwrap() = status;
    }
}

fn sign_in() {
    let Some(live) = LIVE.get() else { return };
    let mut done = 0u64;
    loop {
        let (credentials, round) = {
            let wanted = live.wanted.lock().unwrap();
            let wanted = live.changed.wait_while(wanted, |w| w.1 == done).unwrap();
            (wanted.0.clone(), wanted.1)
        };
        done = round;
        let Some(credentials) = credentials else {
            continue;
        };
        let current = || live.wanted.lock().unwrap().1 == round;
        let mut failures = 0;
        while current() {
            // a token the server took before signs in from the card, unlocks kept to send later
            let offline = live.cheevos.signs_in_offline(&credentials);
            while !online() && !offline && current() {
                std::thread::sleep(Duration::from_secs(1));
            }
            if !current() {
                break;
            }
            set_status(live, round, Status::SigningIn);
            let (tx, rx) = channel();
            live.cheevos.login(&credentials, move |l| {
                let _ = tx.send(l);
            });
            match rx.recv() {
                Ok(Login::Ok { user, token }) => {
                    if !current() {
                        live.cheevos.logout();
                        break;
                    }
                    if matches!(credentials.secret, Secret::Password(_)) {
                        if let Err(e) = config::remember_token(&live.root, &token) {
                            eprintln!("omega-zero: cheevos: saving token: {e}");
                        }
                    }
                    set_status(live, round, Status::SignedIn(user.clone()));
                    live.cheevos.push(if online() {
                        Notice::LoggedIn { user }
                    } else {
                        Notice::Offline
                    });
                    break;
                }
                Ok(Login::Rejected {
                    reason,
                    needs_password,
                }) => {
                    eprintln!("omega-zero: cheevos: login rejected: {reason}");
                    set_status(live, round, Status::Rejected(reason.clone()));
                    live.cheevos.push(Notice::LoginFailed {
                        reason,
                        needs_password,
                    });
                    break;
                }
                Ok(Login::Unreachable) | Err(_) => {
                    if failures == 0 {
                        live.cheevos.push(Notice::Offline);
                    }
                    set_status(live, round, Status::Waiting);
                    let wait = LOGIN_BACKOFF[failures.min(LOGIN_BACKOFF.len() - 1)];
                    failures += 1;
                    let deadline = Instant::now() + Duration::from_secs(wait);
                    while Instant::now() < deadline && current() {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        }
    }
}

fn retry_loads() {
    let Some(live) = LIVE.get() else { return };
    loop {
        if online() && live.cheevos.logged_in() && live.cheevos.wants_load() {
            live.cheevos.retry_load();
        }
        std::thread::sleep(LOAD_RETRY);
    }
}

/// Sends the unlocks earned offline once Wi-Fi is back, waiting longer each time the server
/// can't be reached.
fn send_queued() {
    let Some(live) = LIVE.get() else { return };
    let mut failures = 0;
    loop {
        let wait = if failures == 0 {
            LOAD_RETRY.as_secs()
        } else {
            SEND_RETRY[failures.min(SEND_RETRY.len() - 1)]
        };
        std::thread::sleep(Duration::from_secs(wait));
        if !(online() && live.cheevos.logged_in() && live.cheevos.queued()) {
            failures = 0;
            continue;
        }
        let sent = live.cheevos.flush();
        if sent.sent > 0 {
            eprintln!(
                "omega-zero: cheevos: sent {} unlocks earned offline",
                sent.sent
            );
            live.cheevos.push(Notice::Sent { count: sent.sent });
        }
        failures = if sent.left > 0 { failures + 1 } else { 0 };
    }
}

fn draw_faces() {
    let Some(live) = LIVE.get() else { return };
    loop {
        let Some(notice) = live.cheevos.next_notice(Duration::from_secs(60)) else {
            continue;
        };
        let badge = notice
            .badge_url()
            .and_then(|url| live.cheevos.fetch(url))
            .and_then(|png| omega_cheevos::badge::decode(&png));
        let face = overlay::compose(&notice, badge.as_deref());
        live.faces.lock().unwrap().push(face);
    }
}

/// A game seated for RetroAchievements, let go when dropped.
pub struct Guard {
    ticket: Option<u64>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(ticket) = self.ticket else { return };
        SEATED.with_borrow_mut(|s| *s = None);
        if let Some(live) = LIVE.get() {
            live.booted(ticket);
            let _switch = live.switch.lock().unwrap();
            if live.current.load(Ordering::Acquire) == ticket {
                live.current.store(0, Ordering::Release);
                live.cheevos.unload_game();
            }
        }
        memory::clear();
    }
}

impl Live {
    fn booted(&self, ticket: u64) {
        let _ = self
            .booting
            .compare_exchange(ticket, 0, Ordering::AcqRel, Ordering::Acquire);
    }
}

/// The game at `rom` of `platform` has just been loaded on this, the emulator, thread.
pub fn loaded(platform: Platform, rom: &Path, resumed: bool) -> Guard {
    let Some(live) = LIVE.get() else {
        return Guard { ticket: None };
    };
    if game::console_for(platform, &[]).is_none() {
        return Guard { ticket: None };
    }
    let ticket = live.tickets.fetch_add(1, Ordering::AcqRel) + 1;
    {
        let _switch = live.switch.lock().unwrap();
        live.current.store(ticket, Ordering::Release);
    }
    let boot_left = boot::frames(&live.root, platform, resumed);
    if boot_left > 0 {
        live.booting.store(ticket, Ordering::Release);
    }
    let console = Arc::new(AtomicU32::new(0));
    SEATED.with_borrow_mut(|s| {
        *s = Some(Seated {
            ticket,
            console: console.clone(),
            boot_left,
            last_frame: None,
            last_idle: None,
        })
    });
    let rom = rom.to_path_buf();
    spawn("omega-cheevos-hash", move || {
        identify(live, ticket, platform, &rom, &console)
    });
    Guard {
        ticket: Some(ticket),
    }
}

fn identify(live: &Live, ticket: u64, platform: Platform, rom: &Path, console: &AtomicU32) {
    let Some((id, hash)) = game::identify(platform, rom) else {
        eprintln!("omega-zero: cheevos: no hash for {}", rom.display());
        return;
    };
    let _switch = live.switch.lock().unwrap();
    if live.current.load(Ordering::Acquire) != ticket {
        return;
    }
    console.store(id, Ordering::Release);
    live.cheevos.load_game(&hash);
}

fn ready(live: &Live, seated: &Seated, core: &dyn RetroCore) -> bool {
    if live.current.load(Ordering::Acquire) != seated.ticket {
        return false;
    }
    let console = live
        .cheevos
        .game_console()
        .unwrap_or_else(|| seated.console.load(Ordering::Acquire));
    console != 0 && memory::refresh(core.memory_map(), core.core_memory(), console)
}

/// The core has run a frame forward.
pub fn frame(core: &dyn RetroCore) {
    let Some(live) = LIVE.get() else { return };
    SEATED.with_borrow_mut(|s| {
        let Some(seated) = s else { return };
        if seated.boot_left > 0 {
            seated.boot_left -= 1;
            if seated.boot_left == 0 {
                live.booted(seated.ticket);
            }
        }
        if ready(live, seated, core) {
            live.cheevos.do_frame();
            seated.last_frame = Some(Instant::now());
        }
    });
}

/// The emulator thread went round without running a frame: paused, or waiting.
pub fn idle(core: &dyn RetroCore) {
    let Some(live) = LIVE.get() else { return };
    SEATED.with_borrow_mut(|s| {
        let Some(seated) = s else { return };
        let now = Instant::now();
        let running = seated
            .last_frame
            .is_some_and(|t| now.duration_since(t) < IDLE_AFTER);
        let recently = seated
            .last_idle
            .is_some_and(|t| now.duration_since(t) < IDLE_EVERY);
        if running || recently || !ready(live, seated, core) {
            return;
        }
        live.cheevos.idle();
        seated.last_idle = Some(now);
    });
}

/// The game jumped, a state loaded or rewound: what each achievement was waiting for starts
/// over, so none is earned across the jump.
pub fn reset() {
    let Some(live) = LIVE.get() else { return };
    let current = live.current.load(Ordering::Acquire);
    let seated = SEATED.with_borrow(|s| s.as_ref().is_some_and(|s| s.ticket == current));
    if seated && memory::ready() {
        live.cheevos.reset();
    }
}

/// Draws the banner up, if one is showing, over the rest of the frame.
pub fn overlay(compositor: &mut Compositor, draws: &mut Vec<Draw>) {
    let Some(live) = LIVE.get() else { return };
    SHOWN.with_borrow_mut(|shown| {
        let now = Instant::now();
        let mut alpha = shown
            .since
            .and_then(|t| overlay::alpha(now.duration_since(t)));
        if alpha.is_none() {
            shown.since = None;
            if live.booting.load(Ordering::Acquire) != 0 {
                return;
            }
            let Some(face) = live.faces.lock().unwrap().pop() else {
                return;
            };
            show(compositor, shown, &face);
            shown.since = Some(now);
            alpha = overlay::alpha(Duration::ZERO);
        }
        let (Some(tex), Some(alpha)) = (shown.tex, alpha) else {
            return;
        };
        let (x, y, w, h) = overlay::rect();
        draws.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha,
        });
    });
}

fn show(compositor: &mut Compositor, shown: &mut Shown, face: &Face) {
    let (w, h) = (overlay::FACE_W, overlay::FACE_H);
    match shown.tex {
        Some(id) => compositor.update_texture(id, w, h, &face.rgba),
        None => shown.tex = Some(compositor.create_texture(w, h, &face.rgba)),
    }
}
