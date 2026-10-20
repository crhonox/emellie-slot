use omega_retro::ButtonMask;

const LIKE_GBA: [(u16, u16); 2] = [
    (ButtonMask::X, ButtonMask::B),
    (ButtonMask::Y, ButtonMask::A),
];

fn remapped(mask: u16, pairs: &[(u16, u16)]) -> u16 {
    ButtonMask(mask).remap(pairs).0
}

#[test]
fn x_is_b_and_y_is_a() {
    assert_eq!(remapped(ButtonMask::X, &LIKE_GBA), ButtonMask::B);
    assert_eq!(remapped(ButtonMask::Y, &LIKE_GBA), ButtonMask::A);
}

#[test]
fn a_held_itself_stays_down() {
    let both = ButtonMask::A | ButtonMask::Y;
    assert_eq!(remapped(both, &LIKE_GBA), ButtonMask::A);
    assert_eq!(remapped(ButtonMask::A, &LIKE_GBA), ButtonMask::A);
}

#[test]
fn two_buttons_can_land_on_one() {
    let pairs = [
        (ButtonMask::Y, ButtonMask::A),
        (ButtonMask::L, ButtonMask::A),
        (ButtonMask::R, ButtonMask::B),
    ];
    assert_eq!(remapped(ButtonMask::L, &pairs), ButtonMask::A);
    assert_eq!(remapped(ButtonMask::R, &pairs), ButtonMask::B);
    assert_eq!(
        remapped(ButtonMask::L | ButtonMask::Y, &pairs),
        ButtonMask::A
    );
}

#[test]
fn other_buttons_pass_through_and_the_moved_ones_never_reach_the_core() {
    let held = ButtonMask::X | ButtonMask::Y | ButtonMask::RIGHT;
    let m = remapped(held, &LIKE_GBA);
    assert_ne!(m & ButtonMask::RIGHT, 0);
    assert_eq!(m & (ButtonMask::X | ButtonMask::Y), 0);
}

#[test]
fn no_pairs_change_nothing() {
    let held = ButtonMask::X | ButtonMask::L | ButtonMask::START;
    assert_eq!(remapped(held, &[]), held);
}
