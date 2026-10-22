use std::path::{Path, PathBuf};

use omega_cheevos::config::Account;
use omega_input::{Action, Btn};
use omega_store::{Cart, Platform};
use omega_zero::app::screenshot_path;
use omega_zero::cheevos::{CacheFail, CacheTally, Caching, Status};
use omega_zero::cheevos_screen::{cache_lines, CheevosScreen, Outcome, Sel};
use omega_zero::label_sync::{cart_wants, wants};
use omega_zero::scope::{choices, step, Scope};

fn cart(platform: Platform, stem: &str, label: Option<PathBuf>) -> Cart {
    Cart {
        platform,
        stem: stem.into(),
        rom: PathBuf::from(format!("/card/Games/{}/{stem}.rom", platform.dir_name())),
        label,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

#[test]
fn a_fetch_goes_through_all_one_shelf_or_the_picked_cart() {
    let gba = [
        cart(Platform::Gba, "A", None),
        cart(Platform::Gba, "B", None),
    ];
    let ports = [cart(Platform::Ports, "Doom", None)];
    let empty: [Cart; 0] = [];
    let shelves = [
        (Platform::Gba, &gba[..]),
        (Platform::Snes, &empty[..]),
        (Platform::Ports, &ports[..]),
    ];
    let all = choices(shelves, Some(&gba[1]), |p| p != Platform::Ports);
    assert_eq!(all, [Scope::All, Scope::Folder(Platform::Gba), Scope::Game]);
    // a picked cart the fetch does not take is no choice
    let none = choices(shelves, Some(&ports[0]), |p| p != Platform::Ports);
    assert_eq!(none, [Scope::All, Scope::Folder(Platform::Gba)]);

    assert_eq!(step(&all, Scope::All, 1), Scope::Folder(Platform::Gba));
    assert_eq!(step(&all, Scope::All, -1), Scope::Game);
    assert_eq!(step(&all, Scope::Game, 1), Scope::All);
    assert_eq!(Scope::Folder(Platform::Gbc).name(), "GBC");
    assert_eq!(Scope::Game.name(), "This game");

    assert!(Scope::Folder(Platform::Gba).holds(&gba[0], None));
    assert!(!Scope::Folder(Platform::Snes).holds(&gba[0], None));
    assert!(Scope::Game.holds(&gba[1], Some(&gba[1])));
    assert!(!Scope::Game.holds(&gba[0], Some(&gba[1])));
}

#[test]
fn one_game_fetches_its_label_again_where_a_shelf_skips_it() {
    let labelled = cart(Platform::Gba, "Metroid", Some(PathBuf::from("/x.png")));
    assert!(wants(None, [&labelled]).is_empty());
    let again = cart_wants(None, &labelled, true);
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].stem, "Metroid");
    assert!(cart_wants(None, &cart(Platform::Ports, "Doom", None), true).is_empty());
}

fn press(s: &mut CheevosScreen, btn: Btn, caching: bool) -> Outcome {
    let scopes = [Scope::All, Scope::Folder(Platform::Gba), Scope::Game];
    s.input(
        &Account::default(),
        Action::GbaDown(btn),
        Some(Path::new("/nowhere")),
        &scopes,
        caching,
    )
}

#[test]
fn offline_cache_picks_its_games_and_starts_or_stops() {
    let mut s = CheevosScreen::default();
    press(&mut s, Btn::Up, false);
    assert_eq!(s.sel, Sel::Cache, "up from the top wraps to Offline Cache");
    press(&mut s, Btn::Right, false);
    assert_eq!(s.scope, Scope::Folder(Platform::Gba));
    assert_eq!(
        press(&mut s, Btn::A, false),
        Outcome::Cache(Scope::Folder(Platform::Gba))
    );
    // while caching, left and right leave the choice alone and A stops it
    press(&mut s, Btn::Right, true);
    assert_eq!(s.scope, Scope::Folder(Platform::Gba));
    assert_eq!(press(&mut s, Btn::A, true), Outcome::StopCache);

    let working = Caching::Working {
        done: 2,
        total: 9,
        title: "Golden Sun".into(),
    };
    let view = s.view(&Account::default(), &Status::Off, true, &working, None);
    assert_eq!(view.lit, Some(2));
    assert_eq!(view.status[0], "Caching 3 of 9: Golden Sun");
}

#[test]
fn offline_cache_says_how_it_went() {
    let done = Caching::Done(CacheTally {
        cached: 3,
        unknown: 1,
    });
    assert_eq!(
        cache_lines(&done, Scope::All, None),
        [
            "Done: 3 games cached.",
            "1 game is not on RetroAchievements."
        ]
    );
    assert_eq!(
        cache_lines(&Caching::Failed(CacheFail::SignedOut), Scope::All, None)[0],
        "Sign in first, with Wi-Fi on,"
    );
    assert_eq!(
        cache_lines(&Caching::Idle, Scope::Game, Some("Golden Sun"))[0],
        "Keeps Golden Sun's achievements"
    );
}

#[test]
fn a_screenshot_goes_in_its_systems_folder_and_never_over_another() {
    let root = tempfile::tempdir().unwrap();
    // 2024-01-02 03:04:05
    let at = 1_704_164_645;
    let first = screenshot_path(root.path(), Platform::Gbc, "Tetris DX (World)", at);
    assert_eq!(
        first,
        root.path()
            .join("Screenshots/GBC/Tetris DX (World)_2024-01-02_03-04-05.png")
    );
    std::fs::create_dir_all(first.parent().unwrap()).unwrap();
    std::fs::write(&first, b"png").unwrap();
    let second = screenshot_path(root.path(), Platform::Gbc, "Tetris DX (World)", at);
    assert_eq!(
        second.file_name().unwrap(),
        "Tetris DX (World)_2024-01-02_03-04-05_2.png"
    );
}
