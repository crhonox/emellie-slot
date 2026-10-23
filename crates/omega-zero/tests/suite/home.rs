use crate::common;
use common::{boot, tmp_root_with_carts};
use omega_input::{Action, Btn};
use omega_store::lists::{add_to_collection, collection, favorites};
use omega_store::{read_app_state, write_app_state, AppState, GameRef, Platform};
use omega_ui::Toast;
use omega_zero::app::{App, Phase};
use omega_zero::screens::home::{Entry, List};

fn tap(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn settle(a: &mut App) {
    for _ in 0..40 {
        a.update(1.0 / 60.0);
    }
}

fn home_at(a: &App) -> Option<usize> {
    match a.phase() {
        Phase::Home { at } => Some(*at),
        _ => None,
    }
}

/// Lights `entry` on the main menu and opens it.
fn open(a: &mut App, entry: Entry) {
    let at = home_at(a).expect("not on the main menu");
    for _ in 0..(entry.index() + Entry::ALL.len() - at) % Entry::ALL.len() {
        tap(a, Btn::Right);
    }
    assert_eq!(home_at(a), Some(entry.index()));
    tap(a, Btn::A);
    settle(a);
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
fn the_main_menu_lists_what_there_is_and_comes_back_lit_where_it_was_left() {
    let d = card();
    let mut a = boot(d.path());
    assert_eq!(home_at(&a), Some(Entry::Games.index()));
    tap(&mut a, Btn::Right);
    assert_eq!(home_at(&a), Some(Entry::Settings.index()));
    tap(&mut a, Btn::Right);
    assert_eq!(
        home_at(&a),
        Some(Entry::Recents.index()),
        "right from the last goes round"
    );
    open(&mut a, Entry::Games);
    assert!(matches!(a.phase(), Phase::Systems { .. }));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(
        home_at(&a),
        Some(Entry::Games.index()),
        "B on the grid is the main menu"
    );
    assert_eq!(read_app_state(d.path()).home_at, Entry::Games.index() as u8);
}

#[test]
fn an_empty_list_says_so_and_opens_nothing() {
    let d = card();
    let mut a = boot(d.path());
    open(&mut a, Entry::Recents);
    assert_eq!(home_at(&a), Some(Entry::Recents.index()));
    assert_eq!(a.toast(), Some(Toast::NothingHere));
    tap(&mut a, Btn::Right);
    tap(&mut a, Btn::Right);
    tap(&mut a, Btn::A);
    assert_eq!(home_at(&a), Some(Entry::Collections.index()));
    assert_eq!(a.toast(), Some(Toast::NoCollections));
}

#[test]
fn favorites_opens_a_shelf_of_any_system_that_b_leaves_for_the_menu() {
    let d = card();
    let mut a = boot(d.path());
    common::onto_shelf(&mut a);
    tap(&mut a, Btn::Right);
    tap(&mut a, Btn::Y);
    tap(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    tap(&mut a, Btn::Y);
    assert_eq!(
        favorites(d.path()),
        [
            GameRef::new(Platform::Gba, "Fusion"),
            GameRef::new(Platform::Gb, "Tetris")
        ]
    );
    tap(&mut a, Btn::B);
    settle(&mut a);
    tap(&mut a, Btn::B);
    settle(&mut a);
    open(&mut a, Entry::Favorites);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.list_in_view(), Some(&List::Favorites));
    assert_eq!(a.shelf_platform_name(), Some("FAVORITES"));
    assert_eq!(a.selected_stem(), Some("Fusion"));
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    // the shoulders do not leave a list for a system's shelf
    tap(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(home_at(&a), Some(Entry::Favorites.index()));
}

#[test]
fn a_game_put_in_from_recents_is_the_right_systems() {
    let d = card();
    let mut a = boot(d.path());
    common::onto_shelf(&mut a);
    tap(&mut a, Btn::R1);
    a.apply(Action::Insert);
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    drop(a);
    // the game is ejected: a boot puts nothing back
    let mut s = read_app_state(d.path());
    s.cart = None;
    s.cart_platform = None;
    write_app_state(d.path(), &s).unwrap();

    let mut a = boot(d.path());
    open(&mut a, Entry::Recents);
    assert_eq!(a.list_in_view(), Some(&List::Recents));
    assert_eq!(a.selected_stem(), Some("Tetris"));
    a.apply(Action::Insert);
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    assert_eq!(a.seated_cart().map(|c| c.platform), Some(Platform::Gb));
}

#[test]
fn menu_on_a_shelf_puts_the_game_on_a_collection_and_collections_opens_it() {
    let d = card();
    add_to_collection(d.path(), "Puzzle", &GameRef::new(Platform::Gba, "Emerald")).unwrap();
    let mut a = boot(d.path());
    common::onto_shelf(&mut a);
    tap(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    a.apply(Action::QuickMenu);
    settle(&mut a);
    assert!(matches!(a.phase(), Phase::Collect(_)));
    let view = a.collect_view().expect("the panel");
    assert_eq!(view.rows.len(), 3, "Favorite, New Collection and Puzzle");
    tap(&mut a, Btn::Down);
    tap(&mut a, Btn::Down);
    tap(&mut a, Btn::A);
    assert_eq!(a.toast(), Some(Toast::CollectionAdded));
    assert!(collection(d.path(), "Puzzle").contains(&GameRef::new(Platform::Gb, "Tetris")));
    // A again takes it off
    tap(&mut a, Btn::A);
    assert_eq!(a.toast(), Some(Toast::CollectionRemoved));
    tap(&mut a, Btn::A);
    // A on New Collection puts the keyboard up, which B with nothing typed puts away
    tap(&mut a, Btn::Up);
    tap(&mut a, Btn::A);
    settle(&mut a);
    assert!(
        a.typing().is_some(),
        "no keyboard for the new collection's name"
    );
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert!(a.typing().is_none());
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert!(matches!(a.phase(), Phase::Shelf));

    tap(&mut a, Btn::B);
    settle(&mut a);
    tap(&mut a, Btn::B);
    settle(&mut a);
    open(&mut a, Entry::Collections);
    assert!(matches!(a.phase(), Phase::Collections { at: 0 }));
    tap(&mut a, Btn::A);
    settle(&mut a);
    assert_eq!(a.list_in_view(), Some(&List::Collection("Puzzle".into())));
    assert_eq!(a.selected_stem(), Some("Emerald"));
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert!(
        matches!(a.phase(), Phase::Collections { at: 0 }),
        "B on a collection's shelf went to {:?}",
        a.phase()
    );
}

#[test]
fn settings_opened_from_the_main_menu_comes_back_to_it() {
    let d = card();
    let mut a = boot(d.path());
    open(&mut a, Entry::Settings);
    assert!(matches!(a.phase(), Phase::Settings { .. }));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(home_at(&a), Some(Entry::Settings.index()));
}

#[test]
fn only_the_shelf_on_screen_wants_faces() {
    use omega_ui::TexId;
    use omega_zero::shelf_faces::ShelfKey;
    let d = card();
    let mut a = boot(d.path());
    assert_eq!(a.shelf_in_use(), None, "the main menu draws no carts");
    common::into_grid(&mut a);
    assert_eq!(a.shelf_in_use(), None, "the grid draws no carts");
    tap(&mut a, Btn::A);
    let gba = ShelfKey::System(Platform::Gba);
    assert_eq!(a.shelf_in_use(), Some(gba), "the shelf sliding in has none");
    assert!(!a.has_faces(gba));
    let carts = a.carts_on(gba);
    assert_eq!(carts.len(), 2);
    a.set_shelf_faces(gba, vec![TexId::from_raw(7), TexId::from_raw(8)]);
    assert!(a.has_faces(gba));
    assert_eq!(
        a.faces_of_cart(Platform::Gba, "Fusion"),
        [TexId::from_raw(8)]
    );
    settle(&mut a);
    tap(&mut a, Btn::B);
    assert_eq!(
        a.shelf_in_use(),
        Some(gba),
        "the shelf sliding away still shows"
    );
    settle(&mut a);
    assert_eq!(a.shelf_in_use(), None);
    assert_eq!(
        a.take_shelf_faces(gba),
        [TexId::from_raw(7), TexId::from_raw(8)]
    );
    assert!(!a.has_faces(gba));
    assert!(a.faces_of_cart(Platform::Gba, "Fusion").is_empty());

    // each list opened is a shelf of its own
    tap(&mut a, Btn::B);
    settle(&mut a);
    tap(&mut a, Btn::Up);
    toggle_favorite_on(&d, GameRef::new(Platform::Gba, "Emerald"));
    open(&mut a, Entry::Favorites);
    let first = a.shelf_in_use().expect("the list shows");
    assert!(matches!(first, ShelfKey::List(_)));
    tap(&mut a, Btn::B);
    settle(&mut a);
    open(&mut a, Entry::Favorites);
    assert_ne!(a.shelf_in_use(), Some(first));
}

fn toggle_favorite_on(d: &tempfile::TempDir, game: GameRef) {
    omega_store::lists::toggle_favorite(d.path(), &game).unwrap();
}
