use crate::common;

use common::{
    app_playing_in, app_playing_with_volume, boot, tmp_root_with_carts, tmp_root_with_real_carts,
    write_real_cart_as,
};
use omega_input::Action;
use omega_store::{read_app_state, Core, SUNLIGHT_DEFAULT, SUNLIGHT_MAX};
use omega_ui::{GaugeIcon, Toast};
use omega_zero::app::Phase;

#[test]
fn levels_clamp_and_persist() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    for _ in 0..20 {
        a.apply(Action::BrightnessUp);
    }
    assert_eq!(read_app_state(d.path()).brightness, 9);
    for _ in 0..20 {
        a.apply(Action::BrightnessDown);
    }
    assert_eq!(read_app_state(d.path()).brightness, 0);
}

#[test]
fn volume_moves_five_per_press() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::VolumeUp);
    assert_eq!(read_app_state(d.path()).volume, 65);
    for _ in 0..20 {
        a.apply(Action::VolumeDown);
    }
    assert_eq!(read_app_state(d.path()).volume, 0);
}

#[test]
fn levels_work_on_the_shelf_not_only_in_game() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    assert!(matches!(a.phase(), Phase::Shelf));
    a.apply(Action::BrightnessDown);
    assert_eq!(read_app_state(d.path()).brightness, 4);
}

/// A Boktai cart: its header's game code is the one mGBA and armGBA give a solar sensor.
fn boktai() -> tempfile::TempDir {
    let d = tmp_root_with_real_carts(&["Boktai"]);
    write_real_cart_as(&d, "Boktai", "BOKTAI", "U3IE");
    d
}

#[test]
fn select_and_right_or_left_change_a_solar_carts_sunlight() {
    let d = boktai();
    let mut a = app_playing_in(d.path(), "Boktai");
    assert!(a.plays_solar_game());
    assert_eq!(a.sunlight(), SUNLIGHT_DEFAULT);
    a.apply(Action::SunlightUp);
    assert_eq!(read_app_state(d.path()).sunlight, SUNLIGHT_DEFAULT + 1);
    assert_eq!(a.take_sunlight(), Some(SUNLIGHT_DEFAULT + 1), "the core was never told");
    let gauge = a.hud().gauge().expect("the sunlight shows no gauge");
    assert_eq!(gauge.icon, GaugeIcon::Sun);
    for _ in 0..20 {
        a.apply(Action::SunlightUp);
    }
    assert_eq!(a.sunlight(), SUNLIGHT_MAX);
    for _ in 0..20 {
        a.apply(Action::SunlightDown);
    }
    assert_eq!(read_app_state(d.path()).sunlight, 0);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
}

#[test]
fn the_sunlight_keys_do_nothing_but_on_a_solar_cart() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::SunlightUp);
    assert_eq!(a.sunlight(), SUNLIGHT_DEFAULT);
    assert_eq!(a.take_sunlight(), None);
    assert!(a.hud().gauge().is_none(), "a game with no sensor showed the sunlight");

    let d = boktai();
    let mut a = boot(d.path());
    a.apply(Action::SunlightUp);
    assert_eq!(a.sunlight(), SUNLIGHT_DEFAULT, "the shelf changed the sunlight");
}

#[test]
fn under_gpsp_a_solar_cart_says_which_core_has_the_sensor() {
    let d = boktai();
    let mut a = app_playing_in(d.path(), "Boktai");
    a.set_core(Core::Gpsp);
    a.apply(Action::SunlightUp);
    assert_eq!(a.sunlight(), SUNLIGHT_DEFAULT);
    assert_eq!(a.hud().last_toast(), Some(Toast::NeedsSunCore));
}

#[test]
fn mgba_and_armgba_take_the_sunlight_as_their_option_and_gpsp_has_none() {
    use omega_zero::core::solar_option;
    assert_eq!(
        solar_option(Core::Mgba, 7),
        Some(("mgba_solar_sensor_level", "7".to_string()))
    );
    assert_eq!(
        solar_option(Core::Armgba, 10),
        Some(("armgba_solar_sensor", "10".to_string()))
    );
    assert_eq!(solar_option(Core::Gpsp, 5), None);
}

#[test]
fn adjusting_a_level_in_game_leaves_the_game_running() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::VolumeDown);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    assert_eq!(read_app_state(d.path()).volume, 55);
}

#[test]
fn muting_leaves_the_level_alone_even_at_the_ends() {
    for start in [0u8, 5, 60, 100] {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = app_playing_with_volume(d.path(), start);
        a.apply_at(Action::VolumeUp, 0);
        a.apply_at(Action::VolumeDown, 40);
        a.apply_at(Action::MuteToggle, 60);
        assert!(a.muted(), "not muted from {start}");
        a.apply_at(Action::MuteToggle, 2_000);
        assert_eq!(a.volume(), start, "unmuted to the wrong level from {start}");
    }
}

#[test]
fn muted_is_silent_but_remembers() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    assert_eq!(a.output_volume(), 0, "muted but still audible");
    assert_eq!(a.volume(), 70, "muting forgot the level");
    assert_eq!(a.hud_icon(), GaugeIcon::Muted);
}

#[test]
fn adjusting_the_volume_unmutes() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    a.apply_at(Action::VolumeUp, 1_000);
    assert!(!a.muted(), "still muted after turning it up");
    assert_eq!(a.volume(), 75);
}

#[test]
fn unmuting_with_the_chord_does_not_mute_again() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    a.apply_at(Action::VolumeDown, 1_000);
    a.apply_at(Action::VolumeUp, 1_060);
    a.apply_at(Action::MuteToggle, 1_060);
    assert!(!a.muted(), "the unmute chord muted it again");
    assert_eq!(a.volume(), 70);
}

#[test]
fn the_mute_survives_a_reboot() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    drop(a);
    let b = boot(d.path());
    assert!(b.muted());
    assert_eq!(b.volume(), 70);
}
