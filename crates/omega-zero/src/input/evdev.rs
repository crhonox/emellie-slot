use std::path::{Path, PathBuf};

use omega_input::{Btn, RawEvent};

pub const EVENT_BYTES: usize = 24;

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_SW: u16 = 0x05;
pub const SW_LID: u16 = 0x00;

pub const EV_ABS: u16 = 0x03;
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_HAT0X: u16 = 0x10;
pub const ABS_HAT0Y: u16 = 0x11;

/// How far over a stick is pushed, as a share of its reach either side of its middle, before it
/// holds a direction of the d-pad, and how far back it has to come to let it go: apart, so a
/// stick resting near the line does not flicker it.
pub const STICK_ON: f32 = 0.5;
pub const STICK_OFF: f32 = 0.35;

/// The d-pad, from its hat and from the left stick, which works it as well: a direction is held
/// while either holds it, the hat's over the stick's when they disagree.
#[derive(Default)]
pub struct Hat {
    x: i32,
    y: i32,
    stick_x: i32,
    stick_y: i32,
    /// The left stick's reach on each axis, the kernel's minimum and maximum, where it has one.
    reach_x: Option<(i32, i32)>,
    reach_y: Option<(i32, i32)>,
}

impl Hat {
    /// A d-pad whose left stick reaches `x` and `y` (minimum, maximum) on its two axes.
    pub fn with_stick(x: Option<(i32, i32)>, y: Option<(i32, i32)>) -> Self {
        Hat {
            reach_x: x.filter(|(lo, hi)| hi > lo),
            reach_y: y.filter(|(lo, hi)| hi > lo),
            ..Hat::default()
        }
    }

    pub fn feed(&mut self, ev: Ev) -> Vec<RawEvent> {
        self.feed_as(ev, super::sticks::left_is_dpad())
    }

    /// As `feed`, the left stick working the d-pad only while `stick_is_dpad`.
    pub fn feed_as(&mut self, ev: Ev, stick_is_dpad: bool) -> Vec<RawEvent> {
        let (horizontal, before) = match ev.code {
            ABS_HAT0X | ABS_X => (true, self.held(true)),
            ABS_HAT0Y | ABS_Y => (false, self.held(false)),
            _ => return Vec::new(),
        };
        match ev.code {
            ABS_HAT0X => self.x = ev.value.signum(),
            ABS_HAT0Y => self.y = ev.value.signum(),
            // a game reading the left stick as a stick has it to itself: it holds no direction
            ABS_X if !stick_is_dpad => self.stick_x = 0,
            ABS_Y if !stick_is_dpad => self.stick_y = 0,
            ABS_X => match self.reach_x {
                Some(reach) => self.stick_x = stick(self.stick_x, ev.value, reach),
                None => return Vec::new(),
            },
            _ => match self.reach_y {
                Some(reach) => self.stick_y = stick(self.stick_y, ev.value, reach),
                None => return Vec::new(),
            },
        }
        let now = self.held(horizontal);
        if now == before {
            return Vec::new();
        }
        let ends = match horizontal {
            true => [Btn::Left, Btn::Right],
            false => [Btn::Up, Btn::Down],
        };
        let btn = |v: i32| ends[usize::from(v > 0)];
        let mut out = Vec::new();
        if before != 0 {
            out.push(RawEvent::Up(btn(before)));
        }
        if now != 0 {
            out.push(RawEvent::Down(btn(now)));
        }
        out
    }

    /// The direction held across (or down): the hat's, else the stick's.
    fn held(&self, horizontal: bool) -> i32 {
        let (hat, stick) = match horizontal {
            true => (self.x, self.stick_x),
            false => (self.y, self.stick_y),
        };
        if hat != 0 {
            hat
        } else {
            stick
        }
    }
}

/// The direction a stick axis reaching `reach` holds at `value`, having held `was`.
fn stick(was: i32, value: i32, (lo, hi): (i32, i32)) -> i32 {
    let mid = (lo as f32 + hi as f32) / 2.0;
    let pushed = (value as f32 - mid) / ((hi - lo) as f32 / 2.0);
    let line = if was == 0 { STICK_ON } else { STICK_OFF };
    if pushed.abs() < line {
        0
    } else if was != 0 && was.signum() != pushed.signum() as i32 && pushed.abs() < STICK_ON {
        0
    } else {
        pushed.signum() as i32
    }
}

/// The reach of axis `code` on the evdev node open as `fd`, (minimum, maximum), as the kernel
/// gives it (EVIOCGABS); None where the node has no such axis.
pub fn abs_reach(fd: i32, code: u16) -> Option<(i32, i32)> {
    extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    // _IOR('E', 0x40 + code, struct input_absinfo): value, minimum, maximum, fuzz, flat, resolution
    let request = (2u64 << 30) | (24u64 << 16) | ((b'E' as u64) << 8) | (0x40 + code as u64);
    let mut info = [0i32; 6];
    let done = unsafe { ioctl(fd, request, info.as_mut_ptr()) };
    (done == 0 && info[2] > info[1]).then_some((info[1], info[2]))
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Ev {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

pub fn decode(bytes: &[u8]) -> Option<Ev> {
    let packet: &[u8; EVENT_BYTES] = bytes.get(..EVENT_BYTES)?.try_into().ok()?;
    Some(Ev {
        kind: u16::from_le_bytes([packet[16], packet[17]]),
        code: u16::from_le_bytes([packet[18], packet[19]]),
        value: i32::from_le_bytes([packet[20], packet[21], packet[22], packet[23]]),
    })
}

pub fn to_raw(ev: Ev) -> Option<RawEvent> {
    let btn = match ev.kind {
        EV_KEY => code_to_btn(ev.code)?,
        EV_SW if ev.code == SW_LID => Btn::Lid,
        _ => return None,
    };
    match ev.value {
        0 => Some(RawEvent::Up(btn)),
        1 => Some(RawEvent::Down(btn)),
        _ => None,
    }
}

pub fn code_to_btn(code: u16) -> Option<Btn> {
    Some(match code {
        0x130 => Btn::A,
        0x131 => Btn::B,
        0x132 => Btn::Y,
        0x133 => Btn::X,
        0x134 => Btn::L1,
        0x135 => Btn::R1,
        0x136 => Btn::Select,
        0x137 => Btn::Start,
        0x138 => Btn::Menu,
        0x13a => Btn::L2,
        0x13b => Btn::R2,
        115 => Btn::VolUp,
        114 => Btn::VolDown,
        116 => Btn::Power,
        _ => return None,
    })
}

const WANTED: [u16; 15] = [
    0x130, 0x131, 0x132, 0x133, 0x134, 0x135, 0x136, 0x137, 0x138, 0x13a, 0x13b, 0x162, 115, 114,
    116,
];

pub use omega_power::has_bit;

pub fn pick_devices(dev: &Path, sys: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dev) else {
        return Vec::new();
    };
    let mut nodes: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        })
        .collect();
    nodes.sort();
    nodes.retain(|node| {
        let Some(name) = node.file_name() else {
            return false;
        };
        wanted_node(&sys.join(name).join("device"))
    });
    nodes
}

fn wanted_node(device: &Path) -> bool {
    let cap = |file: &str| std::fs::read_to_string(device.join("capabilities").join(file));
    let keys = cap("key").unwrap_or_default();
    let lid = cap("sw").is_ok_and(|sw| has_bit(&sw, SW_LID));
    lid || WANTED.iter().any(|bit| has_bit(&keys, *bit))
}

pub fn device_name(sys: &Path, node: &Path) -> String {
    node.file_name()
        .and_then(|n| std::fs::read_to_string(sys.join(n).join("device/name")).ok())
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| node.display().to_string())
}
