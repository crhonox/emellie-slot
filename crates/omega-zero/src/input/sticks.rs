//! The sticks, as the core reads them: where each is pushed, kept by the threads reading the
//! pads for the emulator's to hand the core every frame. The left stick also works the d-pad
//! (evdev.rs `Hat`), but for a game reading the sticks as sticks, a PlayStation game that has
//! switched its DualShock to analog: then it is the game's stick alone, and the d-pad the hat's.

use std::sync::atomic::{AtomicBool, AtomicI16, Ordering};

use super::evdev::Ev;

pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RX: u16 = 0x03;
pub const ABS_RY: u16 = 0x04;
pub const ABS_RZ: u16 = 0x05;

/// How far from its middle a stick may rest and still read as centred, as a share of its reach.
const DEAD: f32 = 0.08;

static PUSHED: [AtomicI16; 4] = [
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
];
static LEFT_IS_DPAD: AtomicBool = AtomicBool::new(true);

/// Where the sticks are pushed: left across and down, then right across and down, each
/// -32767 to 32767.
pub fn sticks() -> [i16; 4] {
    [0, 1, 2, 3].map(|i| PUSHED[i].load(Ordering::Relaxed))
}

/// Whether the left stick works the d-pad: but while the game reads it as a stick.
pub fn left_is_dpad() -> bool {
    LEFT_IS_DPAD.load(Ordering::Relaxed)
}

pub fn set_left_is_dpad(on: bool) {
    LEFT_IS_DPAD.store(on, Ordering::Relaxed);
}

/// While the emulator runs a game, the left stick is the d-pad's or the game's as the game
/// reads it; once the game is put away, the d-pad's again.
pub struct DpadBack;

impl Drop for DpadBack {
    fn drop(&mut self) {
        set_left_is_dpad(true);
        for p in &PUSHED {
            p.store(0, Ordering::Relaxed);
        }
    }
}

/// `value` of an axis reaching `(lo, hi)` as libretro has a stick read, -32767 to 32767 from
/// its middle, a stick resting near the middle read as centred.
pub fn scaled(value: i32, (lo, hi): (i32, i32)) -> i16 {
    if hi <= lo {
        return 0;
    }
    let mid = (lo as f32 + hi as f32) / 2.0;
    let pushed = ((value as f32 - mid) / ((hi - lo) as f32 / 2.0)).clamp(-1.0, 1.0);
    if pushed.abs() < DEAD {
        return 0;
    }
    // the dead middle taken out, so a stick starts moving from nothing as it leaves it
    let out = (pushed.abs() - DEAD) / (1.0 - DEAD) * pushed.signum();
    (out * 32767.0).round() as i16
}

/// The axes of one pad that are sticks, and their reach: the left stick on X and Y, the right
/// on RX and RY, or on Z and RZ where the pad has no RX. A pad with no X, the H700 Anbernics'
/// (ANBERNIC-keys: Z, RX, RY and RZ), has its left stick on Z and RX and its right on RY and
/// RZ, the axes in the order SDL counts them and spruceOS's mapping reads them (a0 to a3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sticks {
    axes: [Option<(u16, (i32, i32))>; 4],
}

impl Sticks {
    /// The sticks of a pad whose axis `code` reaches what `reach` says, where it has one.
    pub fn of(reach: impl Fn(u16) -> Option<(i32, i32)>) -> Sticks {
        let reach = |c| reach(c).filter(|(lo, hi)| hi > lo);
        let codes = match (reach(ABS_X), reach(ABS_RX)) {
            (None, Some(_)) => [ABS_Z, ABS_RX, ABS_RY, ABS_RZ],
            (_, Some(_)) => [ABS_X, ABS_Y, ABS_RX, ABS_RY],
            (_, None) => [ABS_X, ABS_Y, ABS_Z, ABS_RZ],
        };
        Sticks {
            axes: codes.map(|c| reach(c).map(|r| (c, r))),
        }
    }

    pub fn any(&self) -> bool {
        self.axes.iter().any(Option::is_some)
    }

    /// The left stick's axes, across and down, each its code and reach, where the pad has both.
    pub fn left(&self) -> Option<[(u16, (i32, i32)); 2]> {
        Some([self.axes[0]?, self.axes[1]?])
    }

    /// Keeps where `ev` pushes a stick, if it moves one.
    pub fn feed(&self, ev: Ev) {
        for (i, axis) in self.axes.iter().enumerate() {
            if let Some((code, reach)) = axis {
                if *code == ev.code {
                    PUSHED[i].store(scaled(ev.value, *reach), Ordering::Relaxed);
                }
            }
        }
    }
}
