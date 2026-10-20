use std::path::Path;

use omega_cheevos::game::{console_for, hash, identify, GB, GBA, GBC, PS1, SNES};
use omega_store::Platform;

fn gb_rom(cgb: u8) -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    rom[0x143] = cgb;
    rom
}

#[test]
fn each_system_counts_as_its_retroachievements_console() {
    assert_eq!(console_for(Platform::Gba, &[]), Some(GBA));
    assert_eq!(console_for(Platform::Snes, &[]), Some(SNES));
    assert_eq!(console_for(Platform::Ps1, &[]), Some(PS1));
}

#[test]
fn the_ds_keeps_its_own_achievements() {
    assert_eq!(console_for(Platform::Nds, &[]), None);
    assert_eq!(identify(Platform::Nds, Path::new("x.nds")), None);
}

#[test]
fn the_cgb_flag_picks_game_boy_color_on_either_shelf() {
    for p in [Platform::Gb, Platform::Gbc] {
        assert_eq!(console_for(p, &gb_rom(0x00)), Some(GB));
        assert_eq!(console_for(p, &gb_rom(0x80)), Some(GBC));
        assert_eq!(console_for(p, &gb_rom(0xc0)), Some(GBC));
        assert_eq!(console_for(p, &[]), Some(GB));
    }
}

#[test]
fn hashes_the_whole_rom_with_md5() {
    let rom: Vec<u8> = (0..0x8000u32).map(|i| (i * 7 + 3) as u8).collect();
    for console in [GBA, GB, GBC] {
        assert_eq!(
            hash(console, &rom).as_deref(),
            Some("3db4f65017c15021ae233458aa7e0702")
        );
    }
}

#[test]
fn a_snes_dump_hashes_the_same_with_its_copier_header() {
    let rom: Vec<u8> = (0..0x8000u32).map(|i| (i * 13 + 1) as u8).collect();
    let mut headed = vec![0u8; 512];
    headed.extend_from_slice(&rom);
    assert_eq!(hash(SNES, &rom), hash(SNES, &headed));
}

#[test]
fn a_cart_is_identified_from_its_file() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("x.gbc");
    std::fs::write(&path, gb_rom(0x80)).unwrap();
    let (console, h) = identify(Platform::Gbc, &path).unwrap();
    assert_eq!(console, GBC);
    assert_eq!(Some(h), hash(GBC, &gb_rom(0x80)));
    assert_eq!(identify(Platform::Gba, &d.path().join("gone.gba")), None);
}

#[test]
fn a_disc_that_is_not_there_has_no_hash() {
    assert_eq!(
        identify(Platform::Ps1, Path::new("/nowhere/game.cue")),
        None
    );
}
