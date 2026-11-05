use crate::common;

use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use omega_input::{Action, Btn, Millis};
use omega_store::{write_app_state, AppState, Platform};
use omega_ui::SettingsRow;
use omega_zero::app::{App, Phase};
use omega_zero::label_sync::Http;
use omega_zero::screens::update::{update_view, ACTION_ROW};
use omega_zero::update::{
    install, parse_release, parse_sha, sha256_file, unpacked_system, Fail, Release, Sha256, Tools,
    UpdateJob, UpdateJobs, UpdateNews, UpdateState, Updates, Version, DIR, READY,
};
use omega_zero::wifi::{Status, WifiJob, WifiJobs, WifiNews};
use tempfile::TempDir;

fn v(major: u32, minor: u32, patch: u32) -> Version {
    Version {
        major,
        minor,
        patch,
    }
}

#[test]
fn a_tag_is_es_and_three_numbers() {
    assert_eq!(Version::parse("es-1.2.3"), Some(v(1, 2, 3)));
    assert_eq!(Version::parse("1.2.3"), Some(v(1, 2, 3)));
    assert_eq!(Version::parse("es-1.2"), None);
    assert_eq!(Version::parse("es-1.2.3.4"), None);
    assert_eq!(
        Version::parse("v1.2.0"),
        None,
        "upstream's tags are not ours"
    );
    assert_eq!(v(1, 2, 3).tag(), "es-1.2.3");
    assert!(v(1, 10, 0) > v(1, 9, 9), "the numbers compare as numbers");
    assert!(v(2, 0, 0) > v(1, 99, 99));
    assert_eq!(
        Version::parse(&Version::current().tag()),
        Some(Version::current())
    );
}

const REPLY: &str = r#"{
  "tag_name": "es-1.4.0",
  "name": "es-1.4.0",
  "draft": false,
  "prerelease": false,
  "body": "\r\n## Lid wakes the SP\r\nMore below.",
  "assets": [
    {"name": "emellie-slot-es-1.4.0-h700.zip.sha256", "size": 99,
     "browser_download_url": "https://example.com/z.sha256"},
    {"name": "emellie-slot-es-1.4.0-h700.zip", "size": 123456789,
     "browser_download_url": "https://example.com/z.zip"}
  ]
}"#;

#[test]
fn githubs_latest_release_is_read_for_its_zip_its_sha_and_its_first_line() {
    let r = parse_release(REPLY).expect("a release");
    assert_eq!(r.version, v(1, 4, 0));
    assert_eq!(r.notes, "Lid wakes the SP");
    assert_eq!(r.zip_url, "https://example.com/z.zip");
    assert_eq!(r.zip_size, 123_456_789);
    assert_eq!(r.sha_url, "https://example.com/z.sha256");
}

#[test]
fn a_release_without_our_tag_or_its_files_is_not_one() {
    assert_eq!(
        parse_release(&REPLY.replace("es-1.4.0\",\n  \"name", "v1.4.0\",\n  \"name")),
        None
    );
    assert_eq!(
        parse_release(&REPLY.replace("-h700.zip\"", "-other.zip\"")),
        None
    );
    assert_eq!(parse_release(&REPLY.replace(".sha256", ".md5")), None);
    assert_eq!(parse_release("not json"), None);
}

#[test]
fn sha256_matches_the_standards_vectors() {
    let hex = |bytes: &[u8]| {
        let mut s = Sha256::default();
        s.update(bytes);
        s.hex()
    };
    assert_eq!(
        hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    // a million a's, fed in pieces that straddle the blocks
    let mut s = Sha256::default();
    for _ in 0..1000 {
        s.update(&[b'a'; 1000]);
    }
    assert_eq!(
        s.hex(),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn a_sha256_file_names_its_hash_first() {
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(
        parse_sha(&format!("{hash}  emellie.zip\n")).as_deref(),
        Some(hash)
    );
    assert_eq!(parse_sha(&hash.to_uppercase()).as_deref(), Some(hash));
    assert_eq!(parse_sha("abc  emellie.zip"), None);
    assert_eq!(parse_sha(""), None);
}

/// GitHub's files as the test has them: the SHA-256 file, or nothing.
struct Files {
    sha: Option<String>,
}

impl Http for Files {
    fn get(&mut self, _url: &str) -> Option<(u16, Vec<u8>)> {
        self.sha.clone().map(|s| (200, s.into_bytes()))
    }
}

/// curl as a copy of a file on disk, 7-Zip as a copy of a tree: the zip is any bytes, and what
/// it unpacks to is `tree`.
struct Copies {
    zip: std::path::PathBuf,
    tree: std::path::PathBuf,
    unpacks: bool,
}

impl Tools for Copies {
    fn download(&mut self, _url: &str, to: &Path) -> std::io::Result<Child> {
        Command::new("cp").arg(&self.zip).arg(to).spawn()
    }

    fn unpack(&mut self, _zip: &Path, into: &Path) -> bool {
        self.unpacks
            && Command::new("cp")
                .arg("-R")
                .arg(&self.tree)
                .arg(into)
                .status()
                .is_ok_and(|s| s.success())
    }
}

struct Install {
    d: TempDir,
    release: Release,
    tools: Copies,
    sha: String,
}

fn install_rig() -> Install {
    let d = tempfile::tempdir().unwrap();
    let zip = d.path().join("served.zip");
    std::fs::write(&zip, b"pretend this is a zip").unwrap();
    let sha = sha256_file(&zip).unwrap();
    // the tree as release.yml zips it: one folder named after the tag, System/ in it
    let tree = d.path().join("tree");
    std::fs::create_dir_all(tree.join("emellie-slot-es-1.4.0/System")).unwrap();
    std::fs::write(
        tree.join("emellie-slot-es-1.4.0/System/emellie-slot"),
        b"new",
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join("card/System")).unwrap();
    std::fs::write(d.path().join("card/System/emellie-slot"), b"old").unwrap();
    let release = parse_release(REPLY).unwrap();
    Install {
        tools: Copies {
            zip,
            tree,
            unpacks: true,
        },
        d,
        release,
        sha,
    }
}

impl Install {
    fn run(&mut self, sha: Option<String>) -> (Result<(), Fail>, Vec<UpdateNews>) {
        let mut said = Vec::new();
        let result = install(
            &self.d.path().join("card"),
            &self.release,
            &mut Files { sha },
            &mut self.tools,
            &AtomicBool::new(false),
            &mut |n| said.push(n),
        );
        (result, said)
    }

    fn card(&self, path: &str) -> std::path::PathBuf {
        self.d.path().join("card").join(path)
    }
}

#[test]
fn an_install_leaves_the_new_system_beside_the_old_and_says_it_is_ready() {
    let mut r = install_rig();
    let sha = format!("{}  emellie-slot-es-1.4.0-h700.zip\n", r.sha);
    let (result, said) = r.run(Some(sha));
    assert_eq!(result, Ok(()));
    assert_eq!(
        std::fs::read(r.card(".update/System/emellie-slot")).unwrap(),
        b"new"
    );
    assert_eq!(
        std::fs::read(r.card("System/emellie-slot")).unwrap(),
        b"old",
        "the System in use was touched"
    );
    assert_eq!(
        std::fs::read_to_string(r.card(&format!("{DIR}/{READY}"))).unwrap(),
        "es-1.4.0\n"
    );
    assert!(
        !r.card(".update/update.zip").exists(),
        "the zip was kept after unpacking"
    );
    assert!(
        !r.card(".update/unpacked").exists(),
        "the unpacked tree was kept"
    );
    assert!(said.contains(&UpdateNews::Verifying));
    assert!(said.contains(&UpdateNews::Unpacking));
    assert!(matches!(said.first(), Some(UpdateNews::Progress(..))));
}

#[test]
fn a_zip_that_is_not_the_one_its_sha256_names_is_thrown_away() {
    let mut r = install_rig();
    let wrong = "0".repeat(64);
    let (result, _) = r.run(Some(wrong));
    assert_eq!(result, Err(Fail::Corrupt));
    assert!(
        !r.card(DIR).exists(),
        "a damaged download was left on the card"
    );
}

#[test]
fn no_sha256_no_install() {
    let mut r = install_rig();
    assert_eq!(r.run(None).0, Err(Fail::Unreachable));
    assert!(!r.card(".update/ready").exists());
}

#[test]
fn a_zip_that_will_not_unpack_or_holds_no_emellie_slot_leaves_nothing_ready() {
    let mut r = install_rig();
    r.tools.unpacks = false;
    let sha = r.sha.clone();
    assert_eq!(r.run(Some(sha.clone())).0, Err(Fail::Unpack));
    assert!(!r.card(DIR).exists());

    let mut r = install_rig();
    std::fs::remove_file(
        r.tools
            .tree
            .join("emellie-slot-es-1.4.0/System/emellie-slot"),
    )
    .unwrap();
    let sha = r.sha.clone();
    assert_eq!(r.run(Some(sha)).0, Err(Fail::Unpack));
    assert!(!r.card(DIR).exists());
}

#[test]
fn the_system_is_found_in_the_trees_one_folder_or_at_its_top() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(unpacked_system(d.path()), None);
    std::fs::create_dir_all(d.path().join("System")).unwrap();
    std::fs::write(d.path().join("System/emellie-slot"), b"").unwrap();
    assert_eq!(unpacked_system(d.path()), Some(d.path().join("System")));
}

#[derive(Default)]
struct GitHubState {
    asked: Vec<UpdateJob>,
    news: Vec<UpdateNews>,
}

/// GitHub and the downloader as the test says, remembering what they were asked.
#[derive(Clone, Default)]
struct GitHub(Arc<Mutex<GitHubState>>);

impl GitHub {
    fn say(&self, news: UpdateNews) {
        self.0.lock().unwrap().news.push(news);
    }

    fn asked(&self) -> Vec<UpdateJob> {
        self.0.lock().unwrap().asked.clone()
    }

    fn checks(&self) -> usize {
        self.asked()
            .iter()
            .filter(|j| **j == UpdateJob::Check)
            .count()
    }
}

impl UpdateJobs for GitHub {
    fn ask(&mut self, job: UpdateJob) {
        self.0.lock().unwrap().asked.push(job);
    }

    fn news(&mut self) -> Vec<UpdateNews> {
        std::mem::take(&mut self.0.lock().unwrap().news)
    }
}

fn release(version: Version) -> Release {
    Release {
        version,
        notes: "What is new".into(),
        zip_url: "https://example.com/z.zip".into(),
        zip_size: 100 * 1024 * 1024,
        sha_url: "https://example.com/z.sha256".into(),
    }
}

#[test]
fn only_a_newer_release_is_offered_and_its_news_told_once() {
    let gh = GitHub::default();
    let mut u = Updates::with_installed(Box::new(gh.clone()), v(1, 0, 0));
    u.check();
    assert_eq!(u.state(), &UpdateState::Checking);
    gh.say(UpdateNews::Checked(release(v(1, 0, 0))));
    u.tick();
    assert_eq!(u.state(), &UpdateState::UpToDate);
    assert_eq!(u.take_news(), None);

    u.check();
    gh.say(UpdateNews::Checked(release(v(1, 1, 0))));
    u.tick();
    assert_eq!(u.available().map(|r| r.version), Some(v(1, 1, 0)));
    assert_eq!(u.take_news(), Some(v(1, 1, 0)));
    assert_eq!(u.take_news(), None, "the news was told twice");
}

#[test]
fn an_install_goes_through_its_steps_and_a_failed_one_can_be_tried_again() {
    let gh = GitHub::default();
    let mut u = Updates::with_installed(Box::new(gh.clone()), v(1, 0, 0));
    u.check();
    gh.say(UpdateNews::Checked(release(v(1, 1, 0))));
    u.tick();
    u.install();
    assert!(u.installing());
    assert!(matches!(gh.asked().last(), Some(UpdateJob::Install(r)) if r.version == v(1, 1, 0)));
    gh.say(UpdateNews::Progress(10, 100));
    u.tick();
    assert!(matches!(
        u.state(),
        UpdateState::Downloading {
            got: 10,
            total: 100,
            ..
        }
    ));
    gh.say(UpdateNews::Failed(Fail::Download));
    u.tick();
    assert!(matches!(
        u.state(),
        UpdateState::Failed {
            fail: Fail::Download,
            release: Some(_)
        }
    ));
    assert!(!u.installing());

    u.install();
    gh.say(UpdateNews::Verifying);
    gh.say(UpdateNews::Unpacking);
    gh.say(UpdateNews::Ready(v(1, 1, 0)));
    u.tick();
    assert!(u.ready());
    u.check();
    assert_eq!(
        gh.checks(),
        1,
        "a check was made over an update waiting to go in"
    );
}

/// A radio that is on and joins when the test says.
#[derive(Clone, Default)]
struct Radio(Arc<Mutex<Vec<WifiNews>>>);

impl WifiJobs for Radio {
    fn present(&self) -> bool {
        true
    }

    fn ask(&mut self, job: WifiJob) {
        if job == WifiJob::Up {
            self.0.lock().unwrap().push(WifiNews::Started(true));
        }
    }

    fn news(&mut self) -> Vec<WifiNews> {
        std::mem::take(&mut self.0.lock().unwrap())
    }
}

struct Rig {
    _d: TempDir,
    a: App,
    gh: GitHub,
    radio: Radio,
    now: Millis,
}

impl Rig {
    /// A card with three GBA carts booted, the one named in `seated` in the handheld when the
    /// handheld went off.
    fn booted(seated: Option<&str>) -> Rig {
        let d = tmp_root_with_carts(&["Emerald", "Fusion", "Zero"]);
        write_app_state(
            d.path(),
            &AppState {
                clock_set: true,
                wifi: true,
                cart: seated.map(str::to_string),
                cart_platform: seated.map(|_| Platform::Gba),
                ..AppState::default()
            },
        )
        .unwrap();
        let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
        let (gh, radio) = (GitHub::default(), Radio::default());
        a.set_wifi_jobs(Box::new(radio.clone()));
        a.set_update_jobs(Box::new(gh.clone()), v(1, 0, 0));
        let mut r = Rig {
            _d: d,
            a,
            gh,
            radio,
            now: 1_000,
        };
        r.tick(16);
        r
    }

    fn press(&mut self, btn: Btn) {
        self.a.apply(Action::GbaDown(btn));
        self.a.apply(Action::GbaUp(btn));
    }

    fn tick(&mut self, ms: Millis) {
        self.now += ms;
        self.a.tick_ms(self.now);
    }

    fn join_wifi(&mut self) {
        self.radio.0.lock().unwrap().push(WifiNews::Status(Status {
            enabled: true,
            state: "COMPLETED".into(),
            ssid: Some(b"Home".to_vec()),
            ip: Some("192.168.1.20".into()),
            rssi: Some(-50),
            mac: None,
            hostname: "rg35xxsp".into(),
            ntp: true,
            ntp_synced: true,
        }));
        self.tick(16);
        assert!(self.a.wifi().status().joined(), "Wi-Fi did not join");
    }

    fn open_update(&mut self) {
        self.press(Btn::Start);
        assert!(self.a.settings_row().is_some(), "START opened no Settings");
        while self.a.settings_row() != Some(SettingsRow::About) {
            self.press(Btn::Down);
            assert_ne!(
                self.a.settings_row(),
                Some(SettingsRow::FastForward),
                "the list went round without reaching About"
            );
        }
        self.press(Btn::A);
        assert!(
            matches!(self.a.phase(), Phase::About),
            "A on About did not open it"
        );
        self.press(Btn::A);
        assert!(
            matches!(self.a.phase(), Phase::Update),
            "A on About did not open Update"
        );
    }
}

#[test]
fn a_boot_to_the_shelf_looks_for_an_update_once_a_network_is_joined() {
    let mut r = Rig::booted(None);
    assert!(r.a.update_check_due());
    r.tick(16);
    assert_eq!(r.gh.checks(), 0, "it looked before there was a network");
    r.join_wifi();
    r.tick(16);
    assert_eq!(r.gh.checks(), 1);
    r.tick(5_000);
    assert_eq!(r.gh.checks(), 1, "it looked again");
}

#[test]
fn a_boot_that_puts_a_game_straight_back_in_does_not_look() {
    let mut r = Rig::booted(Some("Fusion"));
    assert!(
        matches!(r.a.phase(), Phase::Inserting { .. } | Phase::Playing { .. }),
        "the game was not put back in: {:?}",
        r.a.phase()
    );
    assert!(!r.a.update_check_due());
    r.join_wifi();
    r.tick(5_000);
    assert_eq!(
        r.gh.checks(),
        0,
        "a boot straight into a game looked for updates"
    );
}

#[test]
fn a_newer_release_found_at_boot_is_named_on_settings_about_row() {
    let mut r = Rig::booted(None);
    r.join_wifi();
    r.tick(16);
    assert_eq!(r.a.settings_text(SettingsRow::About), None);
    r.gh.say(UpdateNews::Checked(release(v(1, 1, 0))));
    r.tick(16);
    assert_eq!(
        r.a.settings_text(SettingsRow::About).as_deref(),
        Some("ES-1.1.0")
    );
}

#[test]
fn a_on_the_update_screen_installs_what_was_found_and_b_cannot_leave_mid_install() {
    let mut r = Rig::booted(None);
    r.join_wifi();
    r.tick(16);
    r.gh.say(UpdateNews::Checked(release(v(1, 1, 0))));
    r.tick(16);
    r.open_update();
    let view = r.a.update_view().expect("the panel shows");
    assert_eq!(view.lit, Some(ACTION_ROW));
    assert!(view.status[0].contains("es-1.1.0"), "{:?}", view.status);
    r.press(Btn::A);
    assert!(matches!(r.gh.asked().last(), Some(UpdateJob::Install(_))));
    r.gh.say(UpdateNews::Progress(50 * 1024 * 1024, 100 * 1024 * 1024));
    r.tick(16);
    assert_eq!(
        r.a.update_view().unwrap().status,
        ["Downloading 50%", "50 of 100 MB"]
    );
    r.press(Btn::B);
    assert!(
        matches!(r.a.phase(), Phase::Update),
        "B left an install half done"
    );
    r.gh.say(UpdateNews::Verifying);
    r.gh.say(UpdateNews::Unpacking);
    r.gh.say(UpdateNews::Ready(v(1, 1, 0)));
    r.tick(16);
    assert!(
        r.a.update_ready(),
        "a ready update does not have omega-zero leave"
    );
}

#[test]
fn opening_the_update_screen_after_a_look_that_failed_looks_again() {
    let mut r = Rig::booted(None);
    r.join_wifi();
    r.tick(16);
    assert_eq!(r.gh.checks(), 1);
    r.gh.say(UpdateNews::Failed(Fail::Unreachable));
    r.tick(16);
    r.open_update();
    assert_eq!(
        r.gh.checks(),
        2,
        "the screen opened on a failed look without looking again"
    );
    assert_eq!(r.a.updates().state(), &UpdateState::Checking);
}

#[test]
fn without_a_network_there_is_nothing_to_press() {
    let gh = GitHub::default();
    let u = Updates::with_installed(Box::new(gh), v(1, 0, 0));
    let view = update_view(&u, false, true);
    assert_eq!(view.lit, None);
    assert!(view.rows[ACTION_ROW].dim);
    assert_eq!(view.status[0], "Join a Wi-Fi network");
}

#[test]
fn a_low_battery_off_the_charger_holds_an_install_back() {
    let gh = GitHub::default();
    let mut u = Updates::with_installed(Box::new(gh.clone()), v(1, 0, 0));
    u.check();
    gh.say(UpdateNews::Checked(release(v(1, 1, 0))));
    u.tick();
    let view = update_view(&u, true, false);
    assert_eq!(
        view.lit, None,
        "an install could be started on a low battery"
    );
    assert!(view.status[1].contains("Charge"), "{:?}", view.status);
}

#[test]
fn b_on_update_goes_back_to_about() {
    let mut r = Rig::booted(None);
    r.open_update();
    r.press(Btn::B);
    assert!(matches!(r.a.phase(), Phase::About), "{:?}", r.a.phase());
}
