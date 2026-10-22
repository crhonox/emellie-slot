use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use omega_cheevos::offline::param;
use omega_cheevos::{
    game, memory, Cheevos, Credentials, Login, Notice, Offline, Request, Response, Secret,
    Transport,
};
use omega_retro::MemoryDescriptor;

const HASH: &str = "0123456789abcdef0123456789abcdef";

const LOGIN: &str = r#"{"Success":true,"User":"Player","Token":"ApiToken","Score":10,"SoftcoreScore":5,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#;
const SETS: &str = r#"{"Success":true,"GameId":1234,"Title":"Sample Game","ConsoleId":5,"ImageIconUrl":"http://server/Images/112233.png","RichPresenceGameId":1234,"RichPresencePatch":"","Sets":[{"AchievementSetId":1111,"GameId":1234,"Title":null,"Type":"core","ImageIconUrl":"http://server/Images/112233.png","Achievements":[{"ID":5501,"Title":"First Step","Description":"Set the flag","Flags":3,"Points":5,"MemAddr":"0xH0010=1","Author":"A","BadgeName":"00234","BadgeURL":"http://server/Badge/00234.png","Created":1,"Modified":1}],"Leaderboards":[]}]}"#;
const SESSION: &str =
    r#"{"Success":true,"HardcoreUnlocks":[],"Unlocks":[],"ServerNow":1700000000}"#;
const AWARD: &str = r#"{"Success":true,"Score":15,"SoftcoreScore":10,"AchievementID":5501,"AchievementsRemaining":0}"#;
const OK: &str = r#"{"Success":true}"#;
const PNG: &[u8] = b"\x89PNG badge";

#[derive(Clone, Default)]
struct Fake {
    sent: Arc<Mutex<Vec<Request>>>,
    fetched: Arc<Mutex<Vec<String>>>,
    online: Arc<AtomicBool>,
}

impl Fake {
    fn sent_with(&self, needle: &str) -> Vec<String> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .filter_map(|r| r.post.clone())
            .filter(|p| p.contains(needle))
            .collect()
    }
}

impl Transport for Fake {
    fn send(&self, request: &Request) -> Response {
        self.sent.lock().unwrap().push(request.clone());
        if !self.online.load(Ordering::Acquire) {
            return Response::unreachable();
        }
        let post = request.post.as_deref().unwrap_or_default();
        let body = match param(post, "r").as_deref() {
            Some("login2") => LOGIN,
            Some("achievementsets") => SETS,
            Some("startsession") => SESSION,
            Some("awardachievement") => AWARD,
            _ => OK,
        };
        Response {
            status: 200,
            body: body.as_bytes().to_vec(),
        }
    }

    fn fetch(&self, url: &str) -> Option<Vec<u8>> {
        self.fetched.lock().unwrap().push(url.to_string());
        self.online.load(Ordering::Acquire).then(|| PNG.to_vec())
    }
}

fn start(fake: &Fake, dir: &std::path::Path) -> &'static Cheevos {
    let f = fake.clone();
    let online = fake.online.clone();
    let offline = Offline::at(dir.to_path_buf(), move || online.load(Ordering::Acquire));
    Cheevos::start_with(None, move |_| Box::new(f), Some(offline))
}

fn login(c: &Cheevos, secret: Secret) -> Login {
    let credentials = Credentials {
        username: "Player".into(),
        secret,
        host: None,
    };
    let (tx, rx) = channel();
    c.login(&credentials, move |l| tx.send(l).unwrap());
    rx.recv_timeout(Duration::from_secs(5))
        .expect("login answered")
}

fn iwram(ram: &mut [u8]) -> Vec<MemoryDescriptor> {
    vec![MemoryDescriptor {
        ptr: ram.as_mut_ptr() as usize,
        start: 0x3000000,
        len: ram.len(),
        ..Default::default()
    }]
}

fn pump_until(c: &Cheevos, want: impl Fn(&Notice) -> bool) -> Notice {
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end {
        c.do_frame();
        if let Some(n) = c.next_notice(Duration::from_millis(10)) {
            if want(&n) {
                return n;
            }
        }
    }
    panic!("no matching notice");
}

fn wait_for(what: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !what() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(what());
}

#[test]
fn a_game_played_online_once_plays_offline_and_its_unlocks_are_sent_later() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Fake::default();
    fake.online.store(true, Ordering::Release);

    // online: the account, the game, its session and its badge are kept on the card
    let c = start(&fake, dir.path());
    assert!(matches!(
        login(c, Secret::Password("hunter2".into())),
        Login::Ok { .. }
    ));
    let mut ram = vec![0u8; 0x8000];
    memory::refresh(
        &iwram(&mut ram),
        omega_retro::CoreMemory::default(),
        game::GBA,
    );
    c.load_game(HASH);
    pump_until(c, |n| matches!(n, Notice::GameLoaded { .. }));
    c.unload_game();
    memory::clear();
    wait_for(|| dir.path().join("badges/00234.png").exists());

    // offline: a password can't be checked, the token the server gave can
    fake.online.store(false, Ordering::Release);
    let c = start(&fake, dir.path());
    assert_eq!(
        login(c, Secret::Password("hunter2".into())),
        Login::Unreachable
    );
    let token = Credentials {
        username: "Player".into(),
        secret: Secret::Token("ApiToken".into()),
        host: None,
    };
    assert!(c.signs_in_offline(&token));
    assert!(!c.signs_in_offline(&Credentials {
        secret: Secret::Token("Stale".into()),
        ..token.clone()
    }));
    assert!(matches!(
        login(c, Secret::Token("ApiToken".into())),
        Login::Ok { .. }
    ));

    let mut ram = vec![0u8; 0x8000];
    memory::refresh(
        &iwram(&mut ram),
        omega_retro::CoreMemory::default(),
        game::GBA,
    );
    c.load_game(HASH);
    let loaded = pump_until(c, |n| matches!(n, Notice::GameLoaded { .. }));
    assert!(matches!(
        loaded,
        Notice::GameLoaded {
            unlocked: 0,
            total: 1,
            ..
        }
    ));
    for _ in 0..3 {
        c.do_frame();
    }
    ram[0x10] = 1;
    pump_until(c, Notice::is_unlock);
    // the only achievement: answered as the last, so the game shows completed
    pump_until(c, |n| matches!(n, Notice::Mastered { .. }));
    assert_eq!(
        c.fetch("http://server/Badge/00234.png").as_deref(),
        Some(PNG)
    );
    wait_for(|| c.queued());
    c.unload_game();
    memory::clear();

    // still offline, the unlock kept counts as earned
    let c = start(&fake, dir.path());
    assert!(matches!(
        login(c, Secret::Token("ApiToken".into())),
        Login::Ok { .. }
    ));
    let mut ram = vec![0u8; 0x8000];
    memory::refresh(
        &iwram(&mut ram),
        omega_retro::CoreMemory::default(),
        game::GBA,
    );
    c.load_game(HASH);
    let loaded = pump_until(c, |n| matches!(n, Notice::GameLoaded { .. }));
    assert!(matches!(
        loaded,
        Notice::GameLoaded {
            unlocked: 1,
            total: 1,
            ..
        }
    ));
    c.unload_game();
    memory::clear();

    // back online: sent once, signed for how long ago it was earned
    std::thread::sleep(Duration::from_millis(1100));
    fake.online.store(true, Ordering::Release);
    let sent = c.flush();
    assert_eq!((sent.sent, sent.left), (1, 0));
    assert!(!c.queued());
    let awards = fake.sent_with("r=awardachievement");
    let last = awards.last().unwrap();
    assert_eq!(param(last, "a").as_deref(), Some("5501"));
    assert_eq!(param(last, "t").as_deref(), Some("ApiToken"));
    assert_eq!(param(last, "h").as_deref(), Some("0"));
    assert!(
        param(last, "o").is_some_and(|o| o.parse::<u32>().unwrap() >= 1),
        "{last}"
    );
    assert_eq!(c.flush().sent, 0);
}
