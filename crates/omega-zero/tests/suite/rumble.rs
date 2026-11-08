use crate::common;

use std::time::{Duration, Instant};

use common::{
    session_with_platform, tmp_root_with_carts, tmp_root_with_real_carts, write_real_cart_as,
};
use omega_input::{Btn, Millis, RawEvent, MENU_HOLD_MS, POWER_HOLD_MS};
use omega_zero::app::Phase;
use omega_zero::session::Session;

const FRAME_MS: Millis = 16;
const DT: f32 = 1.0 / 60.0;

const STRONG: u32 = 0;

/// Two carts whose header says Drill Dozer: a cartridge with a motor of its own.
fn rumble_carts() -> tempfile::TempDir {
    let d = tmp_root_with_real_carts(&["Advance Wars", "Emerald"]);
    for stem in ["Advance Wars", "Emerald"] {
        write_real_cart_as(&d, stem, "DRILL DOZER", "V49E");
    }
    d
}

#[test]
fn what_a_rumble_cart_asks_for_reaches_the_motor() {
    let d = rumble_carts();
    let (mut s, motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);
    s.core_rumble()
        .expect("a seated cart has a core")
        .set(0, STRONG, u16::MAX);
    step(&mut s, &mut now);
    assert_eq!(motor.last(), u16::MAX, "the core asked and nothing moved");

    let pressed = now;
    event(&mut s, RawEvent::Down(Btn::Menu), &mut now);
    while now < pressed + MENU_HOLD_MS + FRAME_MS {
        step(&mut s, &mut now);
    }
    assert_eq!(motor.last(), 0, "the motor outlived the cart");
    step(&mut s, &mut now);
    assert_eq!(motor.last(), 0, "the next frame turned it back on");
}

#[test]
fn a_game_without_a_rumble_cart_never_moves_the_motor() {
    let d = tmp_root_with_real_carts(&["Advance Wars", "Emerald"]);
    let (mut s, motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);
    let core = s.core_rumble().expect("a seated cart has a core");
    core.set(0, STRONG, u16::MAX);
    assert_eq!(
        core.strength(),
        u16::MAX,
        "the core is not asking, so this test proves nothing"
    );
    for _ in 0..5 {
        step(&mut s, &mut now);
    }
    assert_eq!(motor.last(), 0, "a cart with no motor rumbled");
}

#[test]
fn ejecting_stops_the_motor() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut s, motor) = session_with_platform(d.path());
    s.rumble(u16::MAX);
    assert_ne!(motor.last(), 0);
    s.feed([RawEvent::Down(Btn::Menu)], 0);
    s.feed([], MENU_HOLD_MS + 1);
    assert_eq!(
        motor.last(),
        0,
        "the motor kept running after the cart came out"
    );
}

#[test]
fn dozing_stops_the_motor() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut s, motor) = session_with_platform(d.path());
    s.rumble(u16::MAX);
    s.feed([RawEvent::Down(Btn::Lid)], 0);
    assert_eq!(motor.last(), 0);
}

#[test]
fn the_power_button_stops_the_motor() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut s, motor) = session_with_platform(d.path());
    s.rumble(u16::MAX);
    s.feed([RawEvent::Down(Btn::Power)], 0);
    assert_eq!(motor.last(), 0);
}

fn step(s: &mut Session, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([], *now);
    s.update(DT);
}

fn event(s: &mut Session, ev: RawEvent, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([ev], *now);
    s.update(DT);
}

fn play(s: &mut Session, now: &mut Millis) {
    event(s, RawEvent::Down(Btn::A), now);
    event(s, RawEvent::Up(Btn::A), now);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        step(s, now);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn the_power_menu_takes_the_motor_down() {
    let d = rumble_carts();
    let (mut s, motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);
    s.core_rumble()
        .expect("a seated cart has a core")
        .set(0, STRONG, u16::MAX);
    step(&mut s, &mut now);
    assert_eq!(
        motor.last(),
        u16::MAX,
        "the motor should be running, or this test proves nothing"
    );

    let pressed = now;
    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    while now < pressed + POWER_HOLD_MS + FRAME_MS {
        step(&mut s, &mut now);
    }
    assert_eq!(s.app().power_menu(), Some(0), "the menu never opened");
    assert_eq!(
        motor.last(),
        0,
        "the cart kept buzzing under the power menu"
    );
}
