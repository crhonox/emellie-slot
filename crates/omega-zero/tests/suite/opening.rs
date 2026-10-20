//! The opening: Mega Man X5's boot screen told about this handheld, played over the shelf by a
//! boot that lands on it. The logo fades to black, the terminal fades in, types for ten
//! seconds, fades out, and the shelf fades in. A press cuts it short and goes no further.

use crate::common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use omega_gfx::{out_h, out_w};
use omega_input::{Action, Btn};
use omega_power::parse_baseos_release;
use omega_store::{write_app_state, AppState};
use omega_ui::{Draw, TexId};
use omega_zero::app::{App, Phase};
use omega_zero::audio::Sfx;
use omega_zero::opening::{
    Facts, CUT_OUT_MS, CUT_REVEAL_MS, IN_MS, LEAD_MS, OUT_MS, REVEAL_MS, SCRIPT_MS,
};
use tempfile::TempDir;

/// A frame at 60 fps, in seconds.
const FRAME: f32 = 1.0 / 60.0;

const MEMINFO: &str = "MemTotal:        1003916 kB\nMemFree:          812340 kB\nMemAvailable:     893120 kB\nBuffers:            1024 kB\n";

fn booted_on_the_shelf() -> (TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let state = AppState {
        clock_set: true,
        ..AppState::default()
    };
    write_app_state(d.path(), &state).unwrap();
    let (a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(matches!(a.phase(), Phase::Shelf));
    (d, a)
}

fn opened() -> (TempDir, App) {
    let (d, mut a) = booted_on_the_shelf();
    a.start_opening(&Facts::default());
    assert!(a.opening_shown());
    (d, a)
}

const FRAME_MS: f32 = FRAME * 1000.0;

/// Runs `a` on `ms`, a frame at a time, gathering the sounds.
fn run_for(a: &mut App, ms: f32) -> Vec<Sfx> {
    let mut heard = Vec::new();
    for _ in 0..(ms / FRAME_MS).round() as u32 {
        a.update(FRAME);
        heard.extend(a.take_sfx());
    }
    heard
}

/// Runs an opening not yet drawn on to `ms` from its first frame.
fn run_to(a: &mut App, ms: f32) -> Vec<Sfx> {
    run_for(a, ms + FRAME_MS)
}

fn line(a: &App, row: usize) -> Option<String> {
    a.opening_lines()
        .into_iter()
        .find(|(r, _)| *r == row)
        .map(|(_, t)| t)
}

#[test]
fn the_facts_are_the_handhelds_own() {
    let release = parse_baseos_release("BASEOS_TARGET=rg35xxsp\nBASEOS_MODEL=\"RG35XX SP\"\n");
    let f = Facts::from_parts(Some(release), Some(MEMINFO));
    assert_eq!(
        f,
        Facts {
            model: "RG35XX SP".into(),
            mem_mb: 1024,
            avail_mb: 872,
            battery_mah: Some(3300),
        },
        "the kernel's 980 MB are the 1 GB fitted"
    );
}

#[test]
fn a_battery_nobody_checked_goes_untold_and_memory_rounds_up_to_its_chips() {
    let release = parse_baseos_release("BASEOS_TARGET=rg40xxh\n");
    let f = Facts::from_parts(
        Some(release),
        Some("MemTotal: 2012345 kB\nMemAvailable: 1700000 kB\n"),
    );
    assert_eq!(
        f.model, "RG40XXH",
        "the target names it when no model is given"
    );
    assert_eq!(f.battery_mah, None);
    assert_eq!((f.mem_mb, f.avail_mb), (2048, 1660));
}

#[test]
fn a_computer_standing_in_tells_the_rg35xx_sps_facts() {
    assert_eq!(Facts::from_parts(None, None), Facts::default());
    let f = Facts::default();
    assert_eq!(f.model, "RG35XX SP");
    assert_eq!((f.mem_mb, f.battery_mah), (1024, Some(3300)));
}

#[test]
fn the_logo_fades_to_black_before_the_terminal_comes_in() {
    let (_d, mut a) = opened();
    a.update(FRAME);
    assert_eq!(
        a.opening_levels(),
        Some((1.0, 0.0)),
        "the first frame is black"
    );
    assert!(a.opening_lines().is_empty());
    run_for(&mut a, LEAD_MS - 20.0);
    assert_eq!(
        a.opening_levels(),
        Some((1.0, 0.0)),
        "the logo's fade is over black"
    );
    run_for(&mut a, 20.0 + IN_MS / 2.0);
    let (veil, ink) = a.opening_levels().unwrap();
    assert_eq!(veil, 1.0);
    assert!(ink > 0.2 && ink < 1.0, "the terminal fades in: {ink}");
}

#[test]
fn the_terminal_tells_the_handheld_as_x5s_told_its_system() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + 9_500.0);
    assert_eq!(a.opening_levels(), Some((1.0, 1.0)));
    let lines = a.opening_lines();
    let rows: Vec<usize> = lines.iter().map(|(r, _)| *r).collect();
    assert_eq!(
        rows,
        [0, 1, 3, 4, 5, 7, 8, 10, 11, 12, 14, 15, 17, 19, 20, 21],
        "a blank line between X5's groups"
    );
    let text = |row| line(&a, row).unwrap();
    assert_eq!(text(0), "EMELLIE SLOT HANDHELD SYSTEM");
    assert_eq!(text(1), "Model RG35XX SP");
    assert_eq!(text(3), "Copyright (c) 2026 OAURAMOS");
    assert_eq!(text(7), "real  mem = 1024 MB");
    assert_eq!(text(8), "avail mem =  872 MB");
    assert_eq!(text(10), "cpu     : H700 1.5GHz x4");
    assert_eq!(text(11), "gpu     : Mali-G31 MP2");
    assert!(text(12).starts_with("battery : 3300mAh"), "{}", text(12));
    assert_eq!(text(14), "login: root");
    assert_eq!(text(15), "code : ****");
    assert_eq!(text(17), ">omega -boot -a");
    assert_eq!(text(19), "reading \"GBA.BIOS\"");
    assert_eq!(text(20), "reading \"CARTS\"");
    assert_eq!(text(21), "reading \"OMEGA\"");
    assert!(
        lines.iter().all(|(_, t)| t.chars().count() <= 32),
        "wider than the screen"
    );
}

#[test]
fn the_battery_line_tells_the_charge_as_it_stands() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + 3_400.0);
    let percent = a.battery().map(|b| b.percent).expect("no battery read");
    assert_eq!(
        line(&a, 12).unwrap(),
        format!("battery : 3300mAh  {percent}%")
    );
}

#[test]
fn memory_counts_up_and_keys_come_one_at_a_time() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + 900.0);
    let early = line(&a, 7).expect("no count by 900 ms");
    assert_ne!(early, "real  mem = 1024 MB", "the count did not run up");
    assert!(early.starts_with("real  mem ="));
    let mut typed = Vec::new();
    for _ in 0..240 {
        a.update(FRAME);
        if let Some(t) = line(&a, 14) {
            if typed.last() != Some(&t) {
                typed.push(t);
            }
        }
    }
    assert_eq!(
        typed,
        [
            "login: ",
            "login: r",
            "login: ro",
            "login: roo",
            "login: root"
        ]
    );
}

#[test]
fn the_last_file_is_read_with_time_to_stand_before_the_fade_out() {
    let (_d, mut a) = opened();
    let mut came = None;
    let mut at = 0.0;
    a.update(FRAME);
    while came.is_none() && at < SCRIPT_MS + LEAD_MS {
        a.update(FRAME);
        at += FRAME_MS;
        if line(&a, 21).is_some() {
            came = Some(at - LEAD_MS);
        }
    }
    let came = came.expect("OMEGA was never read");
    assert!(
        (8_500.0..=SCRIPT_MS - 600.0).contains(&came),
        "OMEGA is read {came:.0} ms in"
    );
}

#[test]
fn every_key_and_file_is_heard_in_turn() {
    let (_d, mut a) = opened();
    let heard = run_to(&mut a, LEAD_MS + SCRIPT_MS);
    let keys = |n| vec![Sfx::Key; n];
    let mut want = vec![Sfx::Blip];
    want.extend(keys(4)); // root
    want.extend(keys(4)); // ****
    want.extend(keys(12)); // omega -boot -a, its spaces silent
    want.extend([Sfx::Reading, Sfx::Reading, Sfx::Omega]);
    assert_eq!(heard, want);
}

#[test]
fn it_fades_out_to_black_then_the_shelf_fades_in_and_it_is_over() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + SCRIPT_MS + OUT_MS / 2.0);
    let (veil, ink) = a.opening_levels().unwrap();
    assert_eq!(veil, 1.0, "the shelf shows before the terminal is gone");
    assert!(ink > 0.0 && ink < 0.6, "the terminal fades out: {ink}");
    run_for(&mut a, OUT_MS / 2.0 + REVEAL_MS / 2.0);
    let (veil, ink) = a.opening_levels().unwrap();
    assert_eq!(ink, 0.0);
    assert!(veil > 0.0 && veil < 0.6, "the black lifts: {veil}");
    assert!(a.opening_lines().is_empty());
    run_for(&mut a, REVEAL_MS / 2.0 + 50.0);
    assert!(!a.opening_shown(), "still playing after it has ended");
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn a_slow_frame_slows_the_opening_rather_than_skipping_part_of_it() {
    let (_d, mut a) = opened();
    a.update(FRAME);
    a.update(3.0);
    a.update(3.0);
    let (veil, ink) = a.opening_levels().unwrap();
    assert_eq!(
        (veil, ink),
        (1.0, 0.0),
        "two slow frames ran through the logo's fade"
    );
}

#[test]
fn a_press_cuts_it_short_and_goes_no_further() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + 4_000.0);
    let before = a.selected_stem().map(str::to_string);
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::ShelfRight);
    a.apply(Action::Insert);
    a.apply(Action::GbaUp(Btn::Right));
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    assert_eq!(a.selected_stem().map(str::to_string), before);
    let lines = a.opening_lines();
    let heard = run_for(&mut a, CUT_OUT_MS + 20.0);
    assert!(heard.is_empty(), "keys typed on after the cut: {heard:?}");
    assert_eq!(a.opening_lines(), Vec::new(), "the terminal is still lit");
    assert!(!lines.is_empty());
    run_for(&mut a, CUT_REVEAL_MS + 40.0);
    assert!(
        !a.opening_shown(),
        "the shelf is not in half a second after the press"
    );
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem().map(str::to_string), before);
}

#[test]
fn a_press_with_the_shelf_showing_through_ends_it_and_is_the_shelfs() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + SCRIPT_MS + OUT_MS + 100.0);
    assert!(a.opening_shown());
    a.apply(Action::GbaDown(Btn::Start));
    assert!(!a.opening_shown());
    assert!(
        matches!(a.phase(), Phase::Settings { .. }),
        "START did not open Settings: {:?}",
        a.phase()
    );
}

#[test]
fn power_and_the_lid_end_it_at_once_and_brightness_works_through_it() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + 2_000.0);
    a.apply(Action::BrightnessUp);
    a.apply(Action::VolumeDown);
    a.apply(Action::LidOpen);
    assert_eq!(
        a.opening_levels(),
        Some((1.0, 1.0)),
        "a level key cut it short"
    );
    a.apply(Action::PowerPress);
    assert!(!a.opening_shown(), "POWER waited for the opening");
}

#[test]
fn nothing_under_the_black_is_drawn_and_the_lines_stand_on_the_terminals_grid() {
    let (_d, mut a) = opened();
    run_to(&mut a, LEAD_MS + 1_000.0);
    a.set_opening_face(0, TexId::from_raw(910), 469, 20);
    a.set_opening_face(3, TexId::from_raw(913), 487, 20);
    let mut out = Vec::new();
    a.draw(&mut out);
    let (w, h) = (out_w() as f32, out_h() as f32);
    assert_eq!(
        out[0],
        Draw::Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            colour: [0.0, 0.0, 0.0, 1.0]
        }
    );
    let x = (((out_w() - 576) / 2) & !1) as f32;
    let y = (((out_h() - 396) / 2) & !1) as f32;
    assert_eq!(
        out[1..],
        [
            Draw::Tex {
                x,
                y: y - 2.0,
                w: 469.0,
                h: 20.0,
                tex: TexId::from_raw(910),
                alpha: 1.0
            },
            Draw::Tex {
                x,
                y: y + 3.0 * 18.0 - 2.0,
                w: 487.0,
                h: 20.0,
                tex: TexId::from_raw(913),
                alpha: 1.0
            },
        ],
        "the shelf is drawn under the black"
    );
    if (out_w(), out_h()) == (640, 480) {
        assert_eq!((x, y), (32.0, 42.0), "X5's margins");
    }
}

#[test]
fn the_logo_fades_over_the_black_on_top_of_it() {
    let (_d, mut a) = booted_on_the_shelf();
    let logo = TexId::from_raw(900);
    a.set_boot_logo(logo, out_w(), out_h());
    a.start_opening(&Facts::default());
    a.update(FRAME);
    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(matches!(
        out.first(),
        Some(Draw::Rect {
            colour: [0.0, 0.0, 0.0, 1.0],
            ..
        })
    ));
    assert!(
        matches!(out.last(), Some(Draw::Tex { tex, alpha, .. }) if *tex == logo && *alpha == 1.0),
        "the logo is not whole on top: {out:?}"
    );
}

#[test]
fn only_a_boot_onto_the_shelf_plays_it() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_app_state(d.path(), &AppState::default()).unwrap();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(matches!(a.phase(), Phase::SetClock { .. }));
    a.start_opening(&Facts::default());
    assert!(!a.opening_shown(), "the clock's first boot played it");
    let d = tmp_root_with_carts(&["Emerald"]);
    write_app_state(
        d.path(),
        &AppState {
            clock_set: true,
            cart: Some("Emerald".into()),
            ..AppState::default()
        },
    )
    .unwrap();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    a.start_opening(&Facts::default());
    assert!(!a.opening_shown(), "a boot back into a game played it");
}
