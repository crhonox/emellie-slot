use std::fs;
use std::path::Path;

use omega_input::{Btn, RawEvent};
use omega_zero::input::evdev::{decode, has_bit, pick_devices, to_raw, EVENT_BYTES};

fn packet(kind: u16, code: u16, value: i32) -> [u8; EVENT_BYTES] {
    let mut b = [0x11u8; EVENT_BYTES];
    b[16] = kind as u8;
    b[17] = (kind >> 8) as u8;
    b[18] = code as u8;
    b[19] = (code >> 8) as u8;
    b[20..24].copy_from_slice(&value.to_le_bytes());
    b
}

const EV_KEY: u16 = 0x01;
const EV_SW: u16 = 0x05;
const EV_SYN: u16 = 0x00;
const CODE_R1: u16 = 0x135;
const KEY_VOLUMEUP: u16 = 115;
const SW_LID: u16 = 0x00;

#[test]
fn a_key_press_decodes_from_the_evdev_byte_layout() {
    let down = decode(&packet(EV_KEY, CODE_R1, 1)).and_then(to_raw);
    assert_eq!(down, Some(RawEvent::Down(Btn::R1)));
    let up = decode(&packet(EV_KEY, CODE_R1, 0)).and_then(to_raw);
    assert_eq!(up, Some(RawEvent::Up(Btn::R1)));
    let vol = decode(&packet(EV_KEY, KEY_VOLUMEUP, 1)).and_then(to_raw);
    assert_eq!(vol, Some(RawEvent::Down(Btn::VolUp)));
}

#[test]
fn autorepeat_is_not_a_press() {
    assert_eq!(decode(&packet(EV_KEY, CODE_R1, 2)).and_then(to_raw), None);
}

#[test]
fn the_lid_arrives_as_a_switch_rather_than_a_key() {
    let shut = decode(&packet(EV_SW, SW_LID, 1)).and_then(to_raw);
    assert_eq!(shut, Some(RawEvent::Down(Btn::Lid)));
    let open = decode(&packet(EV_SW, SW_LID, 0)).and_then(to_raw);
    assert_eq!(open, Some(RawEvent::Up(Btn::Lid)));
    assert_ne!(
        decode(&packet(EV_KEY, SW_LID, 1)).and_then(to_raw),
        Some(RawEvent::Down(Btn::Lid))
    );
}

#[test]
fn frame_markers_and_unmapped_codes_are_dropped() {
    assert_eq!(decode(&packet(EV_SYN, 0, 0)).and_then(to_raw), None);
    assert_eq!(decode(&packet(EV_KEY, 0x2ff, 1)).and_then(to_raw), None);
}

#[test]
fn a_short_read_decodes_to_nothing_rather_than_panicking() {
    assert!(decode(&[0u8; EVENT_BYTES - 1]).is_none());
    assert!(decode(&[]).is_none());
}

#[test]
fn a_capability_bitmask_is_indexed_from_the_last_word() {
    assert!(has_bit("10 0", 68));
    assert!(!has_bit("10 0", 4));
    assert!(has_bit("8", 3));
    assert!(!has_bit("", 0));
    assert!(!has_bit("0 0 0", 300));
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn input_tree() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let (dev, sys) = (d.path().join("dev/input"), d.path().join("sys/class/input"));
    for node in ["event0", "event1", "event2", "mice"] {
        write(&dev.join(node), "");
    }
    write(&sys.join("event0/device/name"), "axp2202-pek\n");
    write(&sys.join("event0/device/capabilities/key"), "0\n");
    write(&sys.join("event1/device/name"), "gpio-keys\n");
    write(
        &sys.join("event1/device/capabilities/key"),
        "1000000000000 0 0 0 0\n",
    );
    write(&sys.join("event2/device/name"), "gpio-keys-lid\n");
    write(&sys.join("event2/device/capabilities/key"), "0\n");
    write(&sys.join("event2/device/capabilities/sw"), "1\n");
    d
}

#[test]
fn devices_are_picked_by_capability_and_never_by_position() {
    let d = input_tree();
    let picked = pick_devices(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    );
    let names: Vec<_> = picked
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["event1", "event2"]);
}

#[test]
fn an_input_node_with_no_sysfs_entry_is_skipped_rather_than_opened() {
    let d = tempfile::tempdir().unwrap();
    write(&d.path().join("dev/input/event0"), "");
    let picked = pick_devices(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    );
    assert!(picked.is_empty(), "{picked:?}");
}

#[test]
fn the_survey_reports_the_nodes_that_were_not_opened_too() {
    let d = input_tree();
    let lines = omega_zero::input::trace::survey(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    );
    let line = |node: &str| {
        lines
            .iter()
            .find(|l| l.contains(node))
            .unwrap_or_else(|| panic!("{node} was left out of the survey: {lines:?}"))
            .clone()
    };
    assert!(line("event0").contains("skipped"), "{}", line("event0"));
    assert!(
        line("event0").contains("axp2202-pek"),
        "the survey did not name the node: {}",
        line("event0")
    );
    assert!(line("event1").contains("opened"), "{}", line("event1"));
    assert!(line("event2").contains("opened"), "{}", line("event2"));
}

#[test]
fn an_unmapped_code_is_traced_by_number_rather_than_dropped() {
    let ev = |code| omega_zero::input::evdev::Ev {
        kind: EV_KEY,
        code,
        value: 1,
    };
    let mapped = omega_zero::input::trace::event_line("event1", 12, ev(CODE_R1));
    assert!(mapped.contains("R1"), "{mapped}");
    let unmapped = omega_zero::input::trace::event_line("event1", 13, ev(0x2ff));
    assert!(unmapped.contains("0x2ff"), "{unmapped}");
    assert!(
        unmapped.contains("unmapped"),
        "an unknown code has to say so, or it looks like nothing arrived: {unmapped}"
    );
}

#[test]
fn a_trace_writes_the_survey_to_the_card() {
    let d = input_tree();
    let card = tempfile::tempdir().unwrap();
    let input = omega_zero::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
        card.path(),
        true,
    );
    drop(input);
    let log = std::fs::read_to_string(card.path().join("input-trace.log"))
        .expect("no trace file reached the card");
    assert!(log.contains("event0 skipped"), "{log}");
    assert!(log.contains("event1 opened"), "{log}");
    assert!(log.contains("gpio-keys-lid"), "{log}");
}

#[test]
fn no_trace_file_is_written_unless_one_was_asked_for() {
    let d = input_tree();
    let card = tempfile::tempdir().unwrap();
    let input = omega_zero::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
        card.path(),
        false,
    );
    drop(input);
    assert!(!card.path().join("input-trace.log").exists());
}

#[test]
fn the_pad_is_polled_as_fast_as_the_kernel_tick_allows() {
    let d = input_tree();
    let sys = d.path().join("sys/class/input");
    write(&sys.join("event1/device/poll"), "20\n");
    write(&sys.join("event0/device/poll"), "20\n");
    let card = tempfile::tempdir().unwrap();
    let input = omega_zero::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &sys,
        card.path(),
        false,
    );
    drop(input);
    let poll = |node: &str| std::fs::read_to_string(sys.join(node).join("device/poll")).unwrap();
    assert_eq!(
        poll("event1").trim(),
        "10",
        "the pad is still polled at 20 ms"
    );
    assert_eq!(
        poll("event0").trim(),
        "20",
        "a node omega-zero does not read was changed"
    );
}

#[test]
fn a_pad_already_polled_faster_is_left_alone() {
    let d = input_tree();
    let sys = d.path().join("sys/class/input");
    write(&sys.join("event1/device/poll"), "5\n");
    let card = tempfile::tempdir().unwrap();
    drop(omega_zero::input::DeviceInput::open_in(
        &d.path().join("dev/input"),
        &sys,
        card.path(),
        false,
    ));
    let poll = std::fs::read_to_string(sys.join("event1/device/poll")).unwrap();
    assert_eq!(poll.trim(), "5");
}

#[test]
fn the_pad_maps_by_the_codes_the_device_sends() {
    let expected = [
        (0x130, Btn::A),
        (0x131, Btn::B),
        (0x132, Btn::Y),
        (0x133, Btn::X),
        (0x134, Btn::L1),
        (0x135, Btn::R1),
        (0x136, Btn::Select),
        (0x137, Btn::Start),
        (0x138, Btn::Menu),
        (0x13a, Btn::L2),
        (0x13b, Btn::R2),
        (115, Btn::VolUp),
        (114, Btn::VolDown),
        (116, Btn::Power),
    ];
    for (code, want) in expected {
        assert_eq!(
            omega_zero::input::evdev::code_to_btn(code),
            Some(want),
            "code {code:#x}"
        );
    }
}

#[test]
fn the_menu_buttons_second_code_is_swallowed() {
    assert_eq!(omega_zero::input::evdev::code_to_btn(0x162), None);
}

#[test]
fn the_d_pad_arrives_as_a_hat_rather_than_as_keys() {
    let mut hat = omega_zero::input::evdev::Hat::default();
    let ev = |code, value| omega_zero::input::evdev::Ev {
        kind: omega_zero::input::evdev::EV_ABS,
        code,
        value,
    };
    assert_eq!(hat.feed(ev(0x11, -1)), vec![RawEvent::Down(Btn::Up)]);
    assert_eq!(hat.feed(ev(0x11, 0)), vec![RawEvent::Up(Btn::Up)]);
    assert_eq!(hat.feed(ev(0x11, 1)), vec![RawEvent::Down(Btn::Down)]);
    assert_eq!(hat.feed(ev(0x11, 0)), vec![RawEvent::Up(Btn::Down)]);
    assert_eq!(hat.feed(ev(0x10, -1)), vec![RawEvent::Down(Btn::Left)]);
    assert_eq!(hat.feed(ev(0x10, 0)), vec![RawEvent::Up(Btn::Left)]);
    assert_eq!(hat.feed(ev(0x10, 1)), vec![RawEvent::Down(Btn::Right)]);
    assert_eq!(hat.feed(ev(0x10, 0)), vec![RawEvent::Up(Btn::Right)]);
}

#[test]
fn a_hat_swung_end_to_end_releases_before_it_presses() {
    let mut hat = omega_zero::input::evdev::Hat::default();
    let ev = |code, value| omega_zero::input::evdev::Ev {
        kind: omega_zero::input::evdev::EV_ABS,
        code,
        value,
    };
    assert_eq!(hat.feed(ev(0x10, -1)), vec![RawEvent::Down(Btn::Left)]);
    assert_eq!(
        hat.feed(ev(0x10, 1)),
        vec![RawEvent::Up(Btn::Left), RawEvent::Down(Btn::Right)]
    );
    assert!(hat.feed(ev(0x10, 1)).is_empty());
}

#[test]
fn the_node_carrying_only_the_power_key_is_opened() {
    let d = tempfile::tempdir().unwrap();
    let (dev, sys) = (d.path().join("dev/input"), d.path().join("sys/class/input"));
    write(&dev.join("event0"), "");
    write(&sys.join("event0/device/name"), "axp2202-pek\n");
    write(
        &sys.join("event0/device/capabilities/key"),
        "10000000000000 0\n",
    );
    assert_eq!(
        pick_devices(&dev, &sys).len(),
        1,
        "the power node was skipped"
    );
}

#[test]
fn a_hat_axis_is_traced_as_the_axis_it_is() {
    let line = omega_zero::input::trace::event_line(
        "event1",
        9,
        omega_zero::input::evdev::Ev {
            kind: omega_zero::input::evdev::EV_ABS,
            code: 0x11,
            value: -1,
        },
    );
    assert!(line.contains("hat0y"), "{line}");
    assert!(!line.contains("ignored"), "{line}");
}

const EV_ABS: u16 = 0x03;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_HAT0X: u16 = 0x10;

fn axis(code: u16, value: i32) -> omega_zero::input::evdev::Ev {
    decode(&packet(EV_ABS, code, value)).expect("an axis event")
}

fn stick_pad() -> omega_zero::input::evdev::Hat {
    omega_zero::input::evdev::Hat::with_stick(Some((0, 4000)), Some((0, 4000)))
}

#[test]
fn the_left_stick_pushed_well_over_works_the_d_pad() {
    let mut hat = stick_pad();
    assert!(
        hat.feed(axis(ABS_X, 2600)).is_empty(),
        "a nudge pressed a direction"
    );
    assert_eq!(
        hat.feed(axis(ABS_X, 3600)),
        vec![RawEvent::Down(Btn::Right)]
    );
    assert_eq!(hat.feed(axis(ABS_X, 2000)), vec![RawEvent::Up(Btn::Right)]);
    assert_eq!(hat.feed(axis(ABS_Y, 200)), vec![RawEvent::Down(Btn::Up)]);
    assert_eq!(
        hat.feed(axis(ABS_Y, 3900)),
        vec![RawEvent::Up(Btn::Up), RawEvent::Down(Btn::Down)]
    );
}

#[test]
fn a_stick_resting_near_the_line_does_not_flicker_a_direction() {
    let mut hat = stick_pad();
    assert_eq!(
        hat.feed(axis(ABS_X, 3100)),
        vec![RawEvent::Down(Btn::Right)]
    );
    // back under the line it crossed, but not past the one that lets go
    assert!(hat.feed(axis(ABS_X, 2900)).is_empty());
    assert!(hat.feed(axis(ABS_X, 3050)).is_empty());
    assert_eq!(hat.feed(axis(ABS_X, 2500)), vec![RawEvent::Up(Btn::Right)]);
}

#[test]
fn the_d_pad_and_the_stick_together_are_one_d_pad() {
    let mut hat = stick_pad();
    assert_eq!(
        hat.feed(axis(ABS_HAT0X, 1)),
        vec![RawEvent::Down(Btn::Right)]
    );
    // the stick to the right as well: still one Right held
    assert!(hat.feed(axis(ABS_X, 3900)).is_empty());
    assert!(
        hat.feed(axis(ABS_HAT0X, 0)).is_empty(),
        "the stick still holds Right"
    );
    assert_eq!(hat.feed(axis(ABS_X, 2000)), vec![RawEvent::Up(Btn::Right)]);
    // the hat wins where they disagree
    hat.feed(axis(ABS_X, 100));
    assert_eq!(
        hat.feed(axis(ABS_HAT0X, 1)),
        vec![RawEvent::Up(Btn::Left), RawEvent::Down(Btn::Right)]
    );
}

#[test]
fn a_pad_with_no_stick_takes_no_direction_from_its_axes() {
    let mut hat = omega_zero::input::evdev::Hat::default();
    assert!(hat.feed(axis(ABS_X, 4000)).is_empty());
    assert_eq!(
        hat.feed(axis(ABS_HAT0X, -1)),
        vec![RawEvent::Down(Btn::Left)]
    );
}

#[test]
fn a_game_reading_the_left_stick_as_a_stick_has_it_to_itself() {
    let mut hat = stick_pad();
    assert_eq!(
        hat.feed_as(axis(ABS_X, 3600), true),
        vec![RawEvent::Down(Btn::Right)]
    );
    // the game switches to analog: the stick lets the d-pad go, and holds it no more
    assert_eq!(
        hat.feed_as(axis(ABS_X, 3700), false),
        vec![RawEvent::Up(Btn::Right)]
    );
    assert!(hat.feed_as(axis(ABS_Y, 0), false).is_empty());
    // the hat is the d-pad's still
    assert_eq!(
        hat.feed_as(axis(ABS_HAT0X, -1), false),
        vec![RawEvent::Down(Btn::Left)]
    );
}

#[test]
fn a_stick_is_read_from_its_middle_a_resting_one_centred() {
    use omega_zero::input::sticks::scaled;
    assert_eq!(scaled(2000, (0, 4000)), 0);
    assert_eq!(
        scaled(2100, (0, 4000)),
        0,
        "a stick resting off its middle drifted"
    );
    assert_eq!(scaled(4000, (0, 4000)), 32767);
    assert_eq!(scaled(0, (0, 4000)), -32767);
    assert_eq!(scaled(-5, (0, 4000)), -32767, "past its reach is its reach");
    let half = scaled(3000, (0, 4000));
    assert!((14000..18000).contains(&half), "{half}");
    assert_eq!(scaled(10, (10, 10)), 0);
}

#[test]
fn the_right_stick_is_rx_and_ry_or_else_z_and_rz() {
    use omega_zero::input::sticks::{Sticks, ABS_RX, ABS_RZ, ABS_X, ABS_Y, ABS_Z};
    let both = Sticks::of(|c| [ABS_X, ABS_Y, ABS_RX, 4].contains(&c).then_some((0, 4000)));
    let zed = Sticks::of(|c| {
        [ABS_X, ABS_Y, ABS_Z, ABS_RZ]
            .contains(&c)
            .then_some((0, 4000))
    });
    assert!(both.any() && zed.any());
    assert_ne!(both, zed);
    assert!(!Sticks::of(|_| None).any(), "a pad with no sticks has none");
}
