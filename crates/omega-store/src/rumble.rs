//! Whether a cartridge has a rumble motor of its own, read from the ROM's header: only those
//! games rumble, as on the original handhelds.

use std::path::Path;

use crate::Platform;

/// GBA game codes whose cartridge carries a motor, in every region: Drill Dozer and
/// WarioWare: Twisted!, the only two.
const GBA_RUMBLE_CODES: [&str; 6] = ["V49E", "V49J", "V49P", "RZWE", "RZWJ", "RZWP"];

/// The Game Boy header's cartridge type byte.
const GB_CART_TYPE_OFF: usize = 0x147;

/// MBC5 + RUMBLE, + RAM, + RAM + BATTERY.
const GB_RUMBLE_TYPES: [u8; 3] = [0x1c, 0x1d, 0x1e];

pub fn has_rumble(rom: &Path, platform: Platform) -> bool {
    match platform {
        Platform::Gba => crate::gba::header_code(rom).is_some_and(|code| gba_code_rumbles(&code)),
        Platform::Gb | Platform::Gbc => gb_cart_type(rom).is_some_and(gb_type_rumbles),
        _ => false,
    }
}

pub fn gba_code_rumbles(code: &str) -> bool {
    GBA_RUMBLE_CODES.contains(&code)
}

pub fn gb_type_rumbles(cart_type: u8) -> bool {
    GB_RUMBLE_TYPES.contains(&cart_type)
}

fn gb_cart_type(rom: &Path) -> Option<u8> {
    let mut buf = [0u8; crate::gb::HEADER_LEN];
    use std::io::Read;
    std::fs::File::open(rom).ok()?.read_exact(&mut buf).ok()?;
    Some(buf[GB_CART_TYPE_OFF])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_carts_with_a_motor_rumble() {
        let d = tempfile::tempdir().unwrap();
        let gba = |code: &[u8]| {
            let mut rom = vec![0u8; 0x200];
            rom[0xac..0xb0].copy_from_slice(code);
            let p = d
                .path()
                .join(format!("{}.gba", String::from_utf8_lossy(code)));
            std::fs::write(&p, rom).unwrap();
            p
        };
        assert!(has_rumble(&gba(b"V49E"), Platform::Gba), "Drill Dozer");
        assert!(
            has_rumble(&gba(b"RZWP"), Platform::Gba),
            "WarioWare: Twisted!"
        );
        assert!(!has_rumble(&gba(b"BPEE"), Platform::Gba), "Emerald");

        let gb = |kind: u8| {
            let mut rom = vec![0u8; 0x8000];
            rom[0x147] = kind;
            let p = d.path().join(format!("{kind:02x}.gbc"));
            std::fs::write(&p, rom).unwrap();
            p
        };
        assert!(has_rumble(&gb(0x1e), Platform::Gbc), "Pokemon Pinball");
        assert!(has_rumble(&gb(0x1c), Platform::Gb));
        assert!(
            !has_rumble(&gb(0x1b), Platform::Gbc),
            "MBC5 + RAM + battery, no motor"
        );
        assert!(
            !has_rumble(&gb(0x1e), Platform::Snes),
            "only handheld carts rumble"
        );
        assert!(!has_rumble(&d.path().join("missing.gb"), Platform::Gb));
    }
}
