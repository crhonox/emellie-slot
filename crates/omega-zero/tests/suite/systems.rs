use crate::common;
use common::{boot, tmp_root_with_carts};
use omega_input::{Action, Btn};
use omega_store::{write_app_state, AppState, Platform};
use omega_ui::SettingsRow;
use omega_zero::app::{App, Phase};
use omega_zero::spruce::{self, cell, decode, set_for, step};

fn tap(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

/// The cell picked, while the grid is shown.
fn grid_at(a: &App) -> Option<usize> {
    match a.phase() {
        Phase::Systems { at } => Some(*at),
        _ => None,
    }
}

fn on_shelf(a: &App) -> bool {
    matches!(a.phase(), Phase::Shelf)
}

fn settle(a: &mut App) {
    for _ in 0..40 {
        a.update(1.0 / 60.0);
    }
}

/// A card with two GBA games, a Game Boy one and a SNES one, its clock set.
fn card() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    let snes = d.path().join("Games/SNES/Super Metroid.sfc");
    std::fs::create_dir_all(snes.parent().unwrap()).unwrap();
    std::fs::write(&snes, vec![0u8; 0x8000]).unwrap();
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
fn a_card_with_games_on_several_systems_opens_on_their_grid() {
    let d = card();
    let mut a = boot(d.path());
    assert!(
        matches!(a.phase(), Phase::Home { at: 3 }),
        "{:?}",
        a.phase()
    );
    common::into_grid(&mut a);
    assert_eq!(grid_at(&a), Some(0));
    assert_eq!(a.systems(), [Platform::Gba, Platform::Gb, Platform::Snes]);

    // one system: there is nothing to choose, and the shelf it is
    let one = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let a = boot(one.path());
    assert!(on_shelf(&a), "{:?}", a.phase());
    assert!(!a.uses_grid());
}

#[test]
fn a_opens_the_picked_systems_shelf_and_b_comes_back_to_it() {
    let d = card();
    let mut a = boot(d.path());
    common::into_grid(&mut a);
    tap(&mut a, Btn::Right);
    assert_eq!(grid_at(&a), Some(1));
    tap(&mut a, Btn::A);
    settle(&mut a);
    assert!(on_shelf(&a), "{:?}", a.phase());
    assert_eq!(a.selected_stem(), Some("Tetris"));

    // the shoulders still go round the systems from the shelf
    tap(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Super Metroid"));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(
        grid_at(&a),
        Some(2),
        "B did not come back to the system the shelf was on"
    );
}

#[test]
fn the_grid_goes_round_across_and_stops_at_its_ends_down() {
    let d = card();
    let mut a = boot(d.path());
    common::into_grid(&mut a);
    tap(&mut a, Btn::Left);
    assert_eq!(grid_at(&a), Some(2), "left from the first goes round");
    tap(&mut a, Btn::Right);
    assert_eq!(grid_at(&a), Some(0));
    tap(&mut a, Btn::Up);
    assert_eq!(grid_at(&a), Some(0));
    tap(&mut a, Btn::Down);
    assert_eq!(grid_at(&a), Some(0), "there is no second row");

    assert_eq!(step(1, 7, 4, 0, 1), 5);
    assert_eq!(step(5, 7, 4, 0, -1), 1);
    assert_eq!(
        step(3, 7, 4, 0, 1),
        6,
        "down onto a short last row takes its last cell"
    );
    assert_eq!(step(6, 7, 4, 1, 0), 0);
    assert_eq!(step(0, 7, 4, -1, 0), 6);
}

#[test]
fn settings_comes_back_to_where_it_was_opened() {
    let d = card();
    let mut a = boot(d.path());
    common::into_grid(&mut a);
    tap(&mut a, Btn::Start);
    settle(&mut a);
    assert!(matches!(a.phase(), Phase::Settings { .. }));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(grid_at(&a), Some(0));

    tap(&mut a, Btn::A);
    settle(&mut a);
    tap(&mut a, Btn::Start);
    settle(&mut a);
    assert!(matches!(
        a.phase(),
        Phase::Settings { row } if *row == SettingsRow::ALL[0]
    ));
    tap(&mut a, Btn::B);
    settle(&mut a);
    assert!(
        on_shelf(&a),
        "Settings opened on a shelf left it for the grid"
    );
}

#[test]
fn a_boot_opens_the_grid_on_the_system_last_in_view() {
    let d = card();
    let mut a = boot(d.path());
    common::into_grid(&mut a);
    tap(&mut a, Btn::Right);
    tap(&mut a, Btn::Right);
    tap(&mut a, Btn::A);
    settle(&mut a);
    tap(&mut a, Btn::B);
    settle(&mut a);
    let mut again = boot(d.path());
    common::into_grid(&mut again);
    assert_eq!(grid_at(&again), Some(2));
}

#[test]
fn every_panels_pictures_decode_at_spruces_sizes() {
    for ((w, h), icon, frame, cols, rows) in [
        ((640, 480), (120, 130), (154, 170), 4, 2),
        ((720, 480), (120, 130), (154, 170), 4, 2),
        ((720, 720), (134, 146), (173, 190), 4, 3),
    ] {
        let set = set_for(w, h);
        assert_eq!((set.cols, set.rows), (cols, rows), "{w}x{h}");
        let f = decode(set.frame).expect("frame");
        assert_eq!((f.w, f.h), frame);
        for p in Platform::ALL {
            let (plain, lit) = set.icon(p);
            for png in [plain, lit] {
                let i = decode(png).unwrap_or_else(|| panic!("{p:?} at {w}x{h}"));
                assert_eq!((i.w, i.h), icon, "{p:?} at {w}x{h}");
                assert_eq!(i.rgba.len(), (i.w * i.h * 4) as usize);
            }
        }
    }
}

#[test]
fn names_are_written_in_the_themes_font() {
    let name = spruce::text("GBA", 18.0, spruce::TEXT_LIT);
    assert!(name.w > 20 && name.h >= 18, "{}x{}", name.w, name.h);
    assert!(
        name.rgba.chunks_exact(4).any(|p| p[3] > 200),
        "nothing was written"
    );
    let wider = spruce::text("PlayStation", 18.0, spruce::TEXT);
    assert!(wider.w > name.w);
}

#[test]
fn cells_stand_as_pyui_lays_its_grid() {
    // 640 wide, between a top at 36 and a bottom at 450, two rows of four
    let first = cell(0, 4, 2, 640.0, 36.0, 450.0, 24.0);
    assert_eq!(first.x, 10.0 + 155.0 / 2.0);
    assert_eq!(first.frame_y, 36.0 + 207.0 / 2.0);
    assert_eq!(first.icon_y, first.frame_y - 12.0);
    let below = cell(4, 4, 2, 640.0, 36.0, 450.0, 24.0);
    assert_eq!(below.x, first.x);
    assert_eq!(below.frame_y, first.frame_y + 207.0);
    let last = cell(3, 4, 2, 640.0, 36.0, 450.0, 24.0);
    assert_eq!(last.x, 640.0 - 10.0 - 155.0 / 2.0);
}

fn write_png(path: &std::path::Path, w: u32, h: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().unwrap();
    writer
        .write_image_data(&vec![200u8; (w * h * 4) as usize])
        .unwrap();
}

#[test]
fn with_no_theme_on_the_card_the_menu_is_spruces_over_omegas_own_background() {
    use omega_zero::menu_theme::MenuTheme;
    let t = MenuTheme::load(None, 640, 480);
    assert!(
        t.background.is_none(),
        "a background hides omega-zero's own"
    );
    assert_eq!(t.frame.as_ref().map(|f| (f.w, f.h)), Some((154, 170)));
    assert_eq!((t.grid_cols, t.grid_rows), (4, 2));
    assert!(t.names);
    assert_eq!(t.icons.len(), Platform::ALL.len());
    assert_eq!(t.entries.len(), 5);
    let square = MenuTheme::load(None, 720, 720);
    assert_eq!((square.grid_cols, square.grid_rows), (4, 3));
    assert_eq!(square.icons[0].1.w, 134);
}

#[test]
fn a_theme_on_the_card_overrides_what_it_has_its_panels_folder_first() {
    use omega_zero::menu_theme::{colour, MenuTheme};
    let d = tempfile::tempdir().unwrap();
    let menu = d.path().join("Menu");
    std::fs::create_dir_all(menu.join("720x720")).unwrap();
    std::fs::write(
        menu.join("menu.ini"),
        "grid_columns = 3\ntext_colour = #102030\nnames = no\nframe = none\n",
    )
    .unwrap();
    std::fs::write(menu.join("720x720/menu.ini"), "grid_columns = 5\n").unwrap();
    write_png(&menu.join("background.png"), 640, 480);
    write_png(&menu.join("icons/gba.png"), 50, 60);
    write_png(&menu.join("720x720/icons/gba.png"), 70, 80);
    std::fs::write(menu.join("icons/gb.png"), b"not a png").unwrap();

    let t = MenuTheme::load(Some(d.path()), 640, 480);
    assert_eq!(t.grid_cols, 3);
    assert_eq!(t.grid_rows, 2, "a setting left out is the default");
    assert_eq!(t.text, [0x10, 0x20, 0x30]);
    assert!(!t.names);
    assert!(t.frame.is_none());
    assert_eq!(t.background.as_ref().map(|b| (b.w, b.h)), Some((640, 480)));
    let icon = |t: &MenuTheme, p| t.icons.iter().find(|(q, _, _)| *q == p).unwrap().clone();
    let (_, gba, gba_lit) = icon(&t, Platform::Gba);
    assert_eq!((gba.w, gba.h), (50, 60));
    assert_eq!(
        (gba_lit.w, gba_lit.h),
        (120, 130),
        "the lit one left out is the default"
    );
    assert_eq!(
        icon(&t, Platform::Gb).1.w,
        120,
        "an unreadable PNG falls back"
    );

    let square = MenuTheme::load(Some(d.path()), 720, 720);
    assert_eq!(square.grid_cols, 5, "the panel's own menu.ini wins");
    assert_eq!(icon(&square, Platform::Gba).1.w, 70);
    assert!(!square.names, "the rest of Menu/menu.ini still counts");

    assert_eq!(colour("#FBF1C7"), Some([0xfb, 0xf1, 0xc7]));
    assert_eq!(colour("7c6f64"), Some([0x7c, 0x6f, 0x64]));
    assert_eq!(colour("#12"), None);
}

#[test]
fn the_template_copied_to_menu_draws_the_menu_as_it_comes() {
    use omega_zero::menu_theme::MenuTheme;
    let template =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../card/System/menu-template");
    let d = tempfile::tempdir().unwrap();
    let copy = |from: &std::path::Path| {
        for entry in walk(from) {
            let to = d
                .path()
                .join("Menu")
                .join(entry.strip_prefix(from).unwrap());
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(&entry, &to).unwrap();
        }
    };
    copy(&template);
    for (w, h) in [(640, 480), (720, 480), (720, 720)] {
        let (ours, built_in) = (
            MenuTheme::load(Some(d.path()), w, h),
            MenuTheme::load(None, w, h),
        );
        assert!(
            ours.background.is_none(),
            "{w}x{h}: the template sets a background"
        );
        assert_eq!(
            (ours.grid_cols, ours.grid_rows),
            (built_in.grid_cols, built_in.grid_rows)
        );
        assert_eq!(ours.text, built_in.text);
        for ((p, a, b), (_, c, e)) in ours.icons.iter().zip(&built_in.icons) {
            assert_eq!((a, b), (c, e), "{p:?} at {w}x{h}");
        }
        for ((n, a, b), (_, c, e)) in ours.entries.iter().zip(&built_in.entries) {
            assert_eq!((a, b), (c, e), "{n:?} at {w}x{h}");
        }
        assert_eq!(ours.frame, built_in.frame);
    }
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}
