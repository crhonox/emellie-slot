//! The start notice: a boot that finds a game noted as starting tells, in the opening's
//! terminal, that the game's last start took omega-zero down, on which core, and that it was
//! left on the shelf. It comes up on the opening's black in place of the shelf, ignores presses
//! until its prompt is up, and then goes on a press of A, B or START.

use crate::common;

use common::{app_booting_at, tmp_root_with_carts, write_gb_cart, CLOCK_IS_SET};
use omega_input::{Action, Btn};
use omega_store::{write_app_state, write_selected_core, AppState, Core, Platform};
use omega_ui::Draw;
use omega_zero::app::{App, Phase};
use omega_zero::audio::Sfx;
use omega_zero::opening::{Facts, CUT_OUT_MS, LEAD_MS, OUT_MS as OPENING_OUT_MS, SCRIPT_MS};
use omega_zero::start_notice::{Fell, BLACK_MS, OUT_MS, PROMPT_MS, REVEAL_MS};
use tempfile::TempDir;

/// A frame at 60 fps, in seconds.
const FRAME: f32 = 1.0 / 60.0;
const FRAME_MS: f32 = FRAME * 1000.0;

const MMX: &str = "Mega Man X (U) (V1.1) [!]";

/// A card with three GBA carts and Mega Man X, which Supafaust was chosen for.
fn card() -> TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion", "Minish Cap"]);
    snes_cart(&d, MMX);
    write_selected_core(d.path(), MMX, Core::Supafaust).unwrap();
    d
}

fn snes_cart(d: &TempDir, stem: &str) {
    let rom = d.path().join(format!("Games/SNES/{stem}.sfc"));
    std::fs::write(rom, vec![0u8; 0x8000]).unwrap();
}

/// The card's state: `cart` seated, if any, and its clock set when `clock_set`.
fn seated(d: &TempDir, cart: Option<(&str, Platform)>, clock_set: bool) {
    let state = AppState {
        cart: cart.map(|(stem, _)| stem.to_string()),
        cart_platform: cart.map(|(_, p)| p),
        clock_set,
        ..AppState::default()
    };
    write_app_state(d.path(), &state).unwrap();
}

/// Notes `key` as the game starting when omega-zero stopped.
fn noted(d: &TempDir, key: &str) {
    std::fs::write(d.path().join("Config/.starting"), key).unwrap();
}

/// A card whose last start, Mega Man X's on Supafaust, took omega-zero down.
fn fallen() -> TempDir {
    let d = card();
    seated(&d, Some((MMX, Platform::Snes)), true);
    noted(&d, &format!("SNES/{MMX}"));
    d
}

/// Boots on `d`, the opening playing as the handheld plays it.
fn booted(d: &TempDir) -> App {
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    a.start_opening(&Facts::default());
    a
}

/// Runs `a` on `ms`, a frame at a time, gathering the sounds.
fn run_for(a: &mut App, ms: f32) -> Vec<Sfx> {
    let mut heard = Vec::new();
    for _ in 0..(ms / FRAME_MS).round() as u32 {
        a.update(FRAME);
        heard.extend(a.take_sfx());
    }
    heard
}

/// Runs `a` until the notice's prompt is up, and says how long that took.
fn run_to_prompt(a: &mut App) -> f32 {
    let mut ms = 0.0;
    while !a.start_notice_prompted() {
        a.update(FRAME);
        ms += FRAME_MS;
        assert!(ms < 30_000.0, "the prompt never came up");
    }
    ms
}

fn line(a: &App, row: usize) -> Option<String> {
    a.start_notice_lines()
        .into_iter()
        .find(|(r, _)| *r == row)
        .map(|(_, t)| t)
}

/// Whether `a` draws black over the whole screen first, so nothing under it shows.
fn black(a: &App) -> bool {
    let mut out = Vec::new();
    a.draw(&mut out);
    matches!(
        out.first(),
        Some(Draw::Rect {
            colour: [0.0, 0.0, 0.0, 1.0],
            ..
        })
    )
}

fn press(a: &mut App, b: Btn) {
    a.apply(Action::GbaDown(b));
    a.apply(Action::GbaUp(b));
}

#[test]
fn the_notice_comes_up_on_the_openings_black_in_place_of_the_shelf() {
    let d = fallen();
    let mut a = booted(&d);
    assert!(a.start_notice_due());
    assert!(!a.start_notice_shown(), "it came up over the opening");
    let mut ms = 0.0;
    while !a.start_notice_prompted() {
        a.update(FRAME);
        assert!(black(&a), "the shelf showed {ms:.0} ms in");
        ms += FRAME_MS;
        assert!(ms < 30_000.0, "the prompt never came up");
    }
    assert!(!a.opening_shown());
    // a frame each to start the opening and the notice, from their first frames
    let handover = LEAD_MS + SCRIPT_MS + OPENING_OUT_MS;
    assert!(
        (ms - (handover + PROMPT_MS)).abs() <= 3.0 * FRAME_MS,
        "the prompt came up {ms:.0} ms in"
    );
    assert_eq!(a.start_notice_levels(), Some((1.0, 1.0)));
    let lines: Vec<(usize, String)> = a.start_notice_lines();
    let want: Vec<(usize, String)> = [
        (0, "EMELLIE SLOT HANDHELD SYSTEM"),
        (1, "Model RG35XX SP"),
        (3, "!! LAST START FAILED"),
        (5, "game : Mega Man X"),
        (6, "sys  : SNES"),
        (7, "core : Supafaust"),
        (9, "The game took Emellie Slot down"),
        (10, "as it started. It was left on"),
        (11, "the shelf, not resumed."),
        (13, "X on the shelf picks another"),
        (14, "core for it."),
        (16, ">press A_"),
    ]
    .into_iter()
    .map(|(r, t)| (r, t.to_string()))
    .collect();
    assert_eq!(lines, want);
    assert_eq!(
        a.start_notice_names(),
        Some(&Fell {
            platform: Platform::Snes,
            stem: MMX.into(),
            core: Core::Supafaust,
        })
    );
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    assert_eq!(a.selected_stem(), Some(MMX), "the shelf is not on the game");
}

#[test]
fn without_the_note_the_opening_lets_the_shelf_in() {
    let d = card();
    seated(&d, None, true);
    let mut a = booted(&d);
    assert!(!a.start_notice_due());
    run_for(&mut a, LEAD_MS + SCRIPT_MS + OPENING_OUT_MS + 1_000.0);
    assert!(!a.opening_shown());
    assert!(!a.start_notice_shown());
    assert!(!black(&a), "the shelf did not come in");
}

#[test]
fn lines_come_up_in_turn_and_the_cursor_blinks() {
    let d = fallen();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    a.update(FRAME);
    run_for(&mut a, BLACK_MS + 100.0);
    let rows = |a: &App| -> Vec<usize> { a.start_notice_lines().iter().map(|l| l.0).collect() };
    assert_eq!(rows(&a), [0, 1], "the header first");
    run_for(&mut a, 400.0);
    assert_eq!(rows(&a), [0, 1, 3], "then the warning");
    run_for(&mut a, 650.0);
    assert_eq!(
        rows(&a),
        [0, 1, 3, 5, 6, 7],
        "then the game, its system and core"
    );
    run_to_prompt(&mut a);
    assert_eq!(line(&a, 16).as_deref(), Some(">press A_"));
    run_for(&mut a, 600.0);
    assert_eq!(
        line(&a, 16).as_deref(),
        Some(">press A"),
        "the cursor never blinks"
    );
    run_for(&mut a, 500.0);
    assert_eq!(line(&a, 16).as_deref(), Some(">press A_"));
    assert!(
        a.start_notice_lines()
            .iter()
            .all(|(_, t)| t.chars().count() <= 32),
        "wider than the screen"
    );
}

#[test]
fn it_is_heard_at_its_warning_and_its_prompt() {
    let d = fallen();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    let heard = run_for(&mut a, BLACK_MS + PROMPT_MS + 200.0);
    assert_eq!(heard, [Sfx::Reading, Sfx::Key]);
}

#[test]
fn a_press_that_cuts_the_opening_short_still_leaves_the_notice() {
    let d = fallen();
    let mut a = booted(&d);
    run_for(&mut a, LEAD_MS + 3_000.0);
    press(&mut a, Btn::A);
    assert!(a.opening_shown(), "the press did not cut the opening short");
    run_for(&mut a, CUT_OUT_MS + 2.0 * FRAME_MS);
    assert!(!a.opening_shown());
    assert!(a.start_notice_shown(), "the cut let the shelf in");
    assert!(black(&a));
    assert_eq!(a.selected_stem(), Some(MMX));
}

#[test]
fn presses_before_the_prompt_go_nowhere() {
    let d = fallen();
    let mut a = booted(&d);
    run_for(&mut a, LEAD_MS + 1_000.0);
    press(&mut a, Btn::A);
    // mashing A, and every other button, all through the notice's lines
    let mut mashed = 0;
    while !a.start_notice_prompted() {
        for b in [Btn::A, Btn::B, Btn::Start, Btn::Right, Btn::X, Btn::L1] {
            press(&mut a, b);
        }
        a.apply(Action::Insert);
        a.apply(Action::SelectTap);
        run_for(&mut a, 100.0);
        mashed += 1;
        assert!(mashed < 100, "the prompt never came up");
    }
    assert!(mashed > 10, "the notice was only up {mashed} presses");
    assert_eq!(
        a.start_notice_levels(),
        Some((1.0, 1.0)),
        "a press took it away"
    );
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    assert_eq!(a.selected_stem(), Some(MMX), "the shelf moved under it");
}

#[test]
fn a_after_the_prompt_fades_it_out_to_the_shelf_on_the_game_and_starts_nothing() {
    let d = fallen();
    let mut a = booted(&d);
    run_to_prompt(&mut a);
    a.apply(Action::GbaDown(Btn::A));
    run_for(&mut a, OUT_MS / 2.0);
    let (veil, ink) = a.start_notice_levels().unwrap();
    assert_eq!(veil, 1.0);
    assert!(ink > 0.0 && ink < 0.8, "the terminal fades out: {ink}");
    a.apply(Action::GbaUp(Btn::A));
    run_for(&mut a, OUT_MS / 2.0 + REVEAL_MS / 2.0);
    let (veil, ink) = a.start_notice_levels().unwrap();
    assert_eq!(ink, 0.0);
    assert!(veil > 0.0 && veil < 0.8, "the black lifts: {veil}");
    run_for(&mut a, REVEAL_MS / 2.0 + 50.0);
    assert!(!a.start_notice_due(), "still shown after it was put away");
    run_for(&mut a, 1_000.0);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "the press started the game: {:?}",
        a.phase()
    );
    assert_eq!(a.selected_stem(), Some(MMX));
    assert!(!black(&a));
}

#[test]
fn b_and_start_put_it_away_and_the_other_buttons_do_not() {
    let d = fallen();
    let mut a = booted(&d);
    run_to_prompt(&mut a);
    for b in [Btn::X, Btn::Y, Btn::Right, Btn::Up, Btn::L1, Btn::Select] {
        press(&mut a, b);
    }
    run_for(&mut a, 100.0);
    assert_eq!(
        a.start_notice_levels(),
        Some((1.0, 1.0)),
        "another button took it"
    );
    press(&mut a, Btn::B);
    run_for(&mut a, OUT_MS + REVEAL_MS + 50.0);
    assert!(!a.start_notice_due());

    let d = fallen();
    let mut a = booted(&d);
    run_to_prompt(&mut a);
    press(&mut a, Btn::Start);
    run_for(&mut a, OUT_MS + REVEAL_MS + 50.0);
    assert!(!a.start_notice_due());
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "START went on to Settings: {:?}",
        a.phase()
    );
}

#[test]
fn a_press_with_the_shelf_showing_through_ends_it_and_is_the_shelfs() {
    let d = fallen();
    let mut a = booted(&d);
    run_to_prompt(&mut a);
    press(&mut a, Btn::A);
    run_for(&mut a, OUT_MS + REVEAL_MS / 3.0);
    assert!(a.start_notice_shown());
    a.apply(Action::GbaDown(Btn::Start));
    assert!(!a.start_notice_due());
    assert!(
        matches!(a.phase(), Phase::Settings { .. }),
        "START did not open Settings: {:?}",
        a.phase()
    );
}

#[test]
fn power_and_the_lid_end_it_at_once_and_brightness_works_through_it() {
    let d = fallen();
    let mut a = booted(&d);
    run_for(&mut a, LEAD_MS + 1_000.0);
    press(&mut a, Btn::A);
    run_for(&mut a, CUT_OUT_MS + 1_000.0);
    assert!(a.start_notice_shown());
    a.apply(Action::BrightnessUp);
    a.apply(Action::VolumeDown);
    a.apply(Action::LidOpen);
    assert_eq!(
        a.start_notice_levels(),
        Some((1.0, 1.0)),
        "a level key took it"
    );
    a.apply(Action::PowerPress);
    assert!(!a.start_notice_due(), "POWER waited for the notice");
}

#[test]
fn with_no_opening_it_comes_up_over_the_shelf_as_it_is_first_seen() {
    let d = fallen();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(!a.start_notice_shown());
    a.update(FRAME);
    assert!(a.start_notice_shown(), "the shelf came up without it");
    run_for(&mut a, BLACK_MS / 2.0);
    let (veil, ink) = a.start_notice_levels().unwrap();
    assert!(veil > 0.0 && veil < 1.0, "the shelf fades to black: {veil}");
    assert_eq!(ink, 0.0, "the lines came before the black");
    run_for(&mut a, BLACK_MS / 2.0 + FRAME_MS);
    assert!(black(&a));
    let ms = FRAME_MS + BLACK_MS + FRAME_MS + run_to_prompt(&mut a);
    assert!(
        (ms - (BLACK_MS + PROMPT_MS)).abs() <= 2.0 * FRAME_MS,
        "the prompt came up {ms:.0} ms after the shelf"
    );
}

#[test]
fn a_boot_to_the_clock_shows_it_once_the_clock_is_set() {
    let d = card();
    seated(&d, Some((MMX, Platform::Snes)), false);
    noted(&d, &format!("SNES/{MMX}"));
    let mut a = booted(&d);
    assert!(matches!(a.phase(), Phase::SetClock { .. }));
    assert!(!a.opening_shown());
    run_for(&mut a, 500.0);
    assert!(a.start_notice_due());
    assert!(!a.start_notice_shown(), "it came up over the clock");
    a.confirm_clock();
    a.update(FRAME);
    assert!(a.start_notice_shown());
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem(), Some(MMX));
}

#[test]
fn a_game_boy_game_is_told_it_can_have_its_other_core() {
    let d = card();
    write_gb_cart(&d, "Tetris", "TETRIS");
    seated(&d, Some(("Tetris", Platform::Gb)), true);
    noted(&d, "GB/Tetris");
    let mut a = booted(&d);
    run_to_prompt(&mut a);
    assert_eq!(line(&a, 5).as_deref(), Some("game : Tetris"));
    assert_eq!(line(&a, 6).as_deref(), Some("sys  : GB"));
    assert_eq!(line(&a, 7).as_deref(), Some("core : Gambatte"));
    assert_eq!(
        line(&a, 13).as_deref(),
        Some("X on the shelf picks another")
    );
    assert_eq!(line(&a, 14).as_deref(), Some("core for it."));
}

#[test]
fn a_long_name_is_set_on_two_lines_under_itself() {
    let d = card();
    let zelda = "The Legend of Zelda - A Link to the Past (USA)";
    snes_cart(&d, zelda);
    seated(&d, Some((zelda, Platform::Snes)), true);
    noted(&d, &format!("SNES/{zelda}"));
    let mut a = booted(&d);
    run_to_prompt(&mut a);
    let lines = a.start_notice_lines();
    let rows: Vec<usize> = lines.iter().map(|l| l.0).collect();
    assert_eq!(rows, [0, 1, 3, 5, 6, 7, 8, 10, 11, 12, 14, 15, 17]);
    assert_eq!(line(&a, 5).as_deref(), Some("game : The Legend of Zelda A"));
    assert_eq!(line(&a, 6).as_deref(), Some("       Link to the Past"));
    assert_eq!(line(&a, 8).as_deref(), Some("core : Snes9x"));
    assert!(lines.iter().all(|(_, t)| t.chars().count() <= 32));
}

#[test]
fn the_note_names_the_cart_left_on_the_shelf_though_none_was_seated() {
    let d = card();
    seated(&d, None, true);
    noted(&d, "GBA/Fusion");
    let a = booted(&d);
    assert!(a.start_notice_due());
    assert_eq!(a.selected_stem(), Some("Fusion"));
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn a_note_naming_a_game_no_longer_on_the_card_shows_nothing() {
    let d = card();
    seated(&d, Some(("Emerald", Platform::Gba)), true);
    noted(&d, "SNES/Gone");
    let a = booted(&d);
    assert!(!a.start_notice_due());
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "the seated game was resumed after a crash: {:?}",
        a.phase()
    );
}
