use crate::common;
use common::{boot, tmp_root_with_carts};
use omega_input::{Action, Btn};
use omega_store::lists::{
    add_to_collection, collection, collection_name, collections, favorites, push_recent, read_list,
    recents, remove_from_collection, toggle_favorite, RECENTS_FILE, RECENTS_MAX,
};
use omega_store::{read_app_state, write_app_state, AppState, GameRef, Platform};
use omega_ui::Toast;
use omega_zero::app::{App, Phase};

fn tap(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn gba(stem: &str) -> GameRef {
    GameRef::new(Platform::Gba, stem)
}

#[test]
fn favorites_go_on_and_off_and_keep_their_order() {
    let d = tempfile::tempdir().unwrap();
    assert!(toggle_favorite(d.path(), &gba("Fusion")).unwrap());
    assert!(toggle_favorite(d.path(), &GameRef::new(Platform::Gb, "Tetris")).unwrap());
    assert!(toggle_favorite(d.path(), &gba("Emerald")).unwrap());
    assert!(!toggle_favorite(d.path(), &gba("Fusion")).unwrap());
    assert_eq!(
        favorites(d.path()),
        [GameRef::new(Platform::Gb, "Tetris"), gba("Emerald")]
    );
    let text = std::fs::read_to_string(d.path().join("Config/favorites.txt")).unwrap();
    assert_eq!(text, "GB\tTetris\nGBA\tEmerald\n");
}

#[test]
fn recents_put_the_newest_first_once_and_keep_twenty() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..25 {
        push_recent(d.path(), &gba(&format!("Game {i}"))).unwrap();
    }
    push_recent(d.path(), &gba("Game 10")).unwrap();
    let list = recents(d.path());
    assert_eq!(list.len(), RECENTS_MAX);
    assert_eq!(list[0], gba("Game 10"));
    assert_eq!(list[1], gba("Game 24"));
    assert_eq!(list.iter().filter(|g| **g == gba("Game 10")).count(), 1);
}

#[test]
fn a_list_passes_over_lines_it_cannot_read() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join(RECENTS_FILE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "GBA\tFusion\nN64\tMario\nnonsense\nsnes\tSuper Metroid\r\nGBA\tFusion\n",
    )
    .unwrap();
    assert_eq!(
        read_list(&path),
        [gba("Fusion"), GameRef::new(Platform::Snes, "Super Metroid")]
    );
}

#[test]
fn collections_are_named_files_that_go_when_emptied() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        collection_name("  ../RPGs: best*  ").as_deref(),
        Some("RPGs best")
    );
    assert_eq!(collection_name(" / "), None);
    let kept = add_to_collection(d.path(), "RPGs", &gba("Golden Sun")).unwrap();
    assert_eq!(kept, "RPGs");
    add_to_collection(d.path(), "RPGs", &gba("Golden Sun")).unwrap();
    add_to_collection(d.path(), "arcade", &gba("Pac-Man")).unwrap();
    assert_eq!(collections(d.path()), ["arcade", "RPGs"]);
    assert_eq!(collection(d.path(), "RPGs"), [gba("Golden Sun")]);
    remove_from_collection(d.path(), "arcade", &gba("Pac-Man")).unwrap();
    assert_eq!(collections(d.path()), ["RPGs"]);
    assert!(add_to_collection(d.path(), "//", &gba("x")).is_err());
}

#[test]
fn the_state_keeps_the_system_in_view_and_each_shelfs_pick() {
    let d = tmp_root_with_carts(&[]);
    let state = AppState {
        clock_set: true,
        system: Some(Platform::Gb),
        picks: vec![
            (Platform::Gba, "Fusion".into()),
            (Platform::Gb, "Tetris DX".into()),
        ],
        ..Default::default()
    };
    write_app_state(d.path(), &state).unwrap();
    assert_eq!(read_app_state(d.path()), state);
}

/// A card with two GBA games and a Game Boy one, its clock set.
fn card() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    write_app_state(
        d.path(),
        &AppState {
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    d
}

#[test]
fn y_on_the_shelf_makes_the_picked_game_a_favorite_and_says_so() {
    let d = card();
    let mut a = boot(d.path());
    common::onto_shelf(&mut a);
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    tap(&mut a, Btn::Y);
    assert!(a.is_favorite(Platform::Gba, "Fusion"));
    assert_eq!(a.toast(), Some(Toast::FavoriteAdded));
    assert_eq!(favorites(d.path()), [gba("Fusion")]);
    tap(&mut a, Btn::Y);
    assert!(!a.is_favorite(Platform::Gba, "Fusion"));
    assert_eq!(a.toast(), Some(Toast::FavoriteRemoved));
    assert!(favorites(d.path()).is_empty());
}

#[test]
fn a_game_put_in_goes_first_in_recents() {
    let d = card();
    let mut a = boot(d.path());
    common::onto_shelf(&mut a);
    a.apply(Action::GbaDown(Btn::R1));
    assert_eq!(a.selected_stem(), Some("Tetris"));
    a.apply(Action::Insert);
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    assert_eq!(recents(d.path()), [GameRef::new(Platform::Gb, "Tetris")]);
}

#[test]
fn a_boot_comes_back_to_the_system_and_games_last_picked() {
    let d = card();
    let mut a = boot(d.path());
    common::onto_shelf(&mut a);
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    tap(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    let s = read_app_state(d.path());
    assert_eq!(s.system, Some(Platform::Gb));
    assert!(s.picks.contains(&(Platform::Gba, "Fusion".into())));

    let mut again = boot(d.path());
    common::onto_shelf(&mut again);
    assert_eq!(again.selected_stem(), Some("Tetris"));
    tap(&mut again, Btn::L1);
    assert_eq!(
        again.selected_stem(),
        Some("Fusion"),
        "the GBA shelf forgot its game"
    );
}
