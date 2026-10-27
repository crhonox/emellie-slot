use std::path::Path;

use omega_store::Platform;

use crate::root;

/// How long a game booting through a real BIOS shows its logo for: no banner covers it.
pub const FRAMES: u32 = 300;

const GB_BOOT_ROMS: [&str; 3] = ["gb_bios.bin", "gbc_bios.bin", "sgb_bios.bin"];

pub fn frames(root: &Path, platform: Platform, resumed: bool) -> u32 {
    if resumed || !boot_rom(root, platform) {
        0
    } else {
        FRAMES
    }
}

fn boot_rom(root: &Path, platform: Platform) -> bool {
    match platform {
        Platform::Gba => root::has_real_bios(root),
        Platform::Gb | Platform::Gbc => {
            let dir = root::bios_dir(root);
            GB_BOOT_ROMS.iter().any(|f| dir.join(f).is_file())
        }
        Platform::Nes
        | Platform::Snes
        | Platform::Md
        | Platform::Ps1
        | Platform::Nds
        | Platform::Pico8
        | Platform::Ports => false,
    }
}
