use std::fmt;
use std::path::Path;

use crate::link::Link;
use crate::rumble::Rumble;

/// The GBA's screen. A picture smaller than it, a Game Boy's, reaches the frontend centred in
/// one on black, as the handheld layouts expect; every SNES and PlayStation mode is bigger in
/// both directions and arrives at its own size.
pub const GBA_W: u32 = 240;
pub const GBA_H: u32 = 160;

/// The biggest picture the host takes from a core, room for the SNES's 512x478 and the
/// PlayStation's 640x512; anything past it is cut off.
pub const MAX_FRAME_W: u32 = 1024;
pub const MAX_FRAME_H: u32 = 576;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct ButtonMask(pub u16);

impl ButtonMask {
    pub const B: u16 = 1 << 0;
    pub const Y: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const UP: u16 = 1 << 4;
    pub const DOWN: u16 = 1 << 5;
    pub const LEFT: u16 = 1 << 6;
    pub const RIGHT: u16 = 1 << 7;
    pub const A: u16 = 1 << 8;
    pub const X: u16 = 1 << 9;
    pub const L: u16 = 1 << 10;
    pub const R: u16 = 1 << 11;
    pub const L2: u16 = 1 << 12;
    pub const R2: u16 = 1 << 13;
    pub const L3: u16 = 1 << 14;
    pub const R3: u16 = 1 << 15;

    /// The buttons as a game sees them once each `(from, to)` pair moves a held `from` onto
    /// `to`: `from` never reaches the core, and a `to` held itself stays down.
    pub fn remap(self, pairs: &[(u16, u16)]) -> ButtonMask {
        let mut mask = self.0;
        for &(from, _) in pairs {
            mask &= !from;
        }
        for &(from, to) in pairs {
            if self.0 & from != 0 {
                mask |= to;
            }
        }
        ButtonMask(mask)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AvInfo {
    pub fps: f64,
    pub sample_rate: f64,
}

#[derive(Debug)]
pub enum CoreError {
    Io(std::io::Error),
    Load(String),
    Unsupported(String),
    State(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io(e) => write!(f, "io: {e}"),
            CoreError::Load(m) => write!(f, "load: {m}"),
            CoreError::Unsupported(m) => write!(f, "unsupported: {m}"),
            CoreError::State(m) => write!(f, "state: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io(e)
    }
}

pub trait RetroCore: Send {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError>;
    fn run_frame(&mut self, input: ButtonMask);
    fn run_frame_linked(&mut self, p1: ButtonMask, _p2: ButtonMask) {
        self.run_frame(p1);
    }
    fn set_option(&mut self, _key: &str, _value: &str) {}
    fn set_frame_skip(&mut self, _skip: bool) {}
    /// The last picture the core drew, XRGB8888 in rows `video_size().0` pixels long.
    fn video_xrgb8888(&self) -> &[u8];
    /// How big that picture is, width by height. A core that never says draws the GBA's screen.
    fn video_size(&self) -> (u32, u32) {
        (GBA_W, GBA_H)
    }
    fn take_audio(&mut self) -> Vec<i16>;
    fn serialize(&mut self) -> Result<Vec<u8>, CoreError>;
    fn unserialize(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn save_ram(&self) -> Option<Vec<u8>>;
    fn load_save_ram(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn av_info(&self) -> AvInfo;
    fn rumble(&self) -> Rumble {
        Rumble::default()
    }
    /// The memory map the core declared, for RetroAchievements to read the game's RAM through.
    fn memory_map(&self) -> &[crate::memory::MemoryDescriptor] {
        &[]
    }
    /// The system and save RAM `retro_get_memory_data` hands out, for a core with no map.
    fn core_memory(&self) -> crate::memory::CoreMemory {
        crate::memory::CoreMemory::default()
    }
    fn net(&self) -> Link {
        Link::default()
    }
    fn start_link(&mut self, _client_id: u16) {}
    fn pump_link(&mut self) {}
    fn stop_link(&mut self) {}
    /// How many discs the game in the core is on, through libretro's disk control.
    fn disc_count(&self) -> usize {
        1
    }
    /// Which of them is in the drive, counting from 0.
    fn disc_index(&self) -> usize {
        0
    }
    /// Opens the drive, puts disc `index` in and closes it, as a player swapping discs would;
    /// false if the core has no such disc or no disk control.
    fn set_disc(&mut self, _index: usize) -> bool {
        false
    }
    /// The file disc `index` is read from, as the core names it, when it can say.
    fn disc_path(&self, _index: usize) -> Option<String> {
        None
    }
    /// Asks for disc `index`, read from `path`, to be the one in the drive when the game loads,
    /// as it was when the game was last put away; called before `load`. A core that checks finds
    /// `path` at that index or starts from the first disc.
    fn set_initial_disc(&mut self, _index: usize, _path: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(Vec<ButtonMask>);

    impl RetroCore for Recorder {
        fn load(&mut self, _rom: &Path) -> Result<(), CoreError> {
            Ok(())
        }
        fn run_frame(&mut self, input: ButtonMask) {
            self.0.push(input);
        }
        fn video_xrgb8888(&self) -> &[u8] {
            &[]
        }
        fn take_audio(&mut self) -> Vec<i16> {
            Vec::new()
        }
        fn serialize(&mut self) -> Result<Vec<u8>, CoreError> {
            Ok(Vec::new())
        }
        fn unserialize(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn save_ram(&self) -> Option<Vec<u8>> {
            None
        }
        fn load_save_ram(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn av_info(&self) -> AvInfo {
            AvInfo {
                fps: 60.0,
                sample_rate: 48_000.0,
            }
        }
    }

    #[test]
    fn a_core_without_link_mode_runs_player_1_alone() {
        let mut core = Recorder::default();
        core.run_frame_linked(ButtonMask(ButtonMask::A), ButtonMask(ButtonMask::B));
        assert_eq!(core.0, vec![ButtonMask(ButtonMask::A)]);
    }
}
