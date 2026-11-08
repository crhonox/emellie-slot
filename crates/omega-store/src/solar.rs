//! Whether a cartridge has a solar sensor, read from the ROM's header: the Boktai games, which
//! want sunlight on the cartridge to play.

use std::path::Path;

use crate::Platform;

/// GBA game codes whose cartridge carries a solar sensor, in every region (the codes mGBA and
/// armGBA flag): Boktai, Boktai 2 and Shin Bokura no Taiyou.
const GBA_SOLAR_CODES: [&str; 7] = ["U3IE", "U3IJ", "U3IP", "U32E", "U32J", "U32P", "U33J"];

pub fn has_solar_sensor(rom: &Path, platform: Platform) -> bool {
    platform == Platform::Gba
        && crate::gba::header_code(rom).is_some_and(|code| gba_code_has_sun(&code))
}

pub fn gba_code_has_sun(code: &str) -> bool {
    GBA_SOLAR_CODES.contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_boktai_carts_have_a_sensor() {
        let d = tempfile::tempdir().unwrap();
        let gba = |code: &[u8]| {
            let mut rom = vec![0u8; 0x200];
            rom[0xac..0xb0].copy_from_slice(code);
            let p = d.path().join(format!("{}.gba", String::from_utf8_lossy(code)));
            std::fs::write(&p, rom).unwrap();
            p
        };
        assert!(has_solar_sensor(&gba(b"U3IE"), Platform::Gba), "Boktai");
        assert!(has_solar_sensor(&gba(b"U32P"), Platform::Gba), "Boktai 2");
        assert!(has_solar_sensor(&gba(b"U33J"), Platform::Gba), "Boktai 3");
        assert!(!has_solar_sensor(&gba(b"BPEE"), Platform::Gba), "Emerald");
        assert!(!has_solar_sensor(&gba(b"U3IE"), Platform::Gbc));
    }
}
