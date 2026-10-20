use std::ffi::{c_char, CString};
use std::path::Path;

use omega_store::Platform;

/// RetroAchievements' numbers for the consoles omega-zero plays itself (rc_consoles.h).
pub const SNES: u32 = 3;
pub const GB: u32 = 4;
pub const GBA: u32 = 5;
pub const GBC: u32 = 6;
pub const PS1: u32 = 12;

const CGB_FLAG: usize = 0x143;

/// The console a game of `platform` counts as, `rom` its first bytes at least: a Game Boy cart
/// that knows the Color is a Color game, wherever it is shelved. None for the DS, whose
/// emulator keeps its own achievements.
pub fn console_for(platform: Platform, rom: &[u8]) -> Option<u32> {
    match platform {
        Platform::Gba => Some(GBA),
        Platform::Gb | Platform::Gbc => Some(match rom.get(CGB_FLAG) {
            Some(f) if f & 0x80 != 0 => GBC,
            _ => GB,
        }),
        Platform::Snes => Some(SNES),
        Platform::Ps1 => Some(PS1),
        Platform::Nds => None,
    }
}

/// The console and RetroAchievements hash of the game at `rom`. A cart is hashed whole from
/// memory, a SNES dump's copier header left out by rcheevos; a PlayStation game is hashed off
/// its disc (a .cue with its .bin, or the first disc of an .m3u), which rcheevos reads itself.
pub fn identify(platform: Platform, rom: &Path) -> Option<(u32, String)> {
    if platform == Platform::Ps1 {
        return Some((PS1, hash_file(PS1, rom)?));
    }
    let bytes = match std::fs::read(rom) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("omega-zero: cheevos: {}: {e}", rom.display());
            return None;
        }
    };
    let console = console_for(platform, &bytes)?;
    Some((console, hash(console, &bytes)?))
}

pub fn hash(console: u32, rom: &[u8]) -> Option<String> {
    let mut out = [0 as c_char; 33];
    let ok = unsafe {
        crate::ffi::rc_hash_generate_from_buffer(out.as_mut_ptr(), console, rom.as_ptr(), rom.len())
    };
    (ok != 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}

pub fn hash_file(console: u32, path: &Path) -> Option<String> {
    let path = CString::new(path.to_str()?).ok()?;
    let mut out = [0 as c_char; 33];
    let ok = unsafe {
        crate::ffi::omega_hash_init_cdreader();
        crate::ffi::rc_hash_generate_from_file(out.as_mut_ptr(), console, path.as_ptr())
    };
    (ok != 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}
