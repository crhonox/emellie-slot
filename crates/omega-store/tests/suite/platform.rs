use omega_store::Platform;

#[test]
fn every_platform_has_a_directory() {
    let names: Vec<&str> = Platform::ALL.iter().map(|p| p.dir_name()).collect();
    assert_eq!(
        names,
        vec!["GBA", "GB", "GBC", "FC", "SNES", "MD", "PS1", "NDS", "PICO8", "PORTS"]
    );
}

#[test]
fn every_platform_is_a_shelf_of_its_own() {
    assert_eq!(
        Platform::ALL,
        [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Nes,
            Platform::Snes,
            Platform::Md,
            Platform::Ps1,
            Platform::Nds,
            Platform::Pico8,
            Platform::Ports
        ]
    );
    for (i, p) in Platform::ALL.iter().enumerate() {
        assert!(
            !Platform::ALL[..i].contains(p),
            "{p:?} is named twice in the ring, so two shelves would hold one platform"
        );
    }
}

#[test]
fn each_platform_takes_only_its_own_extensions() {
    assert!(Platform::Gba.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(!Platform::Gba.accepts(std::path::Path::new("Tetris.gb")));
    assert!(Platform::Gb.accepts(std::path::Path::new("Tetris.gb")));
    assert!(Platform::Gb.accepts(std::path::Path::new("Tetris.gbc")));
    assert!(!Platform::Gb.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(Platform::Gba.accepts(std::path::Path::new("Shrek.GBA")));
    assert!(Platform::Nds.accepts(std::path::Path::new("Pokemon Platinum.nds")));
    assert!(!Platform::Nds.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(!Platform::Gba.accepts(std::path::Path::new("Pokemon Platinum.nds")));
    assert!(Platform::Ports.accepts(std::path::Path::new("Cave Story.sh")));
    assert!(!Platform::Ports.accepts(std::path::Path::new("cavestory")));
    assert!(Platform::Nes.accepts(std::path::Path::new("Super Mario Bros. (World).nes")));
    assert!(Platform::Nes.accepts(std::path::Path::new("Zelda no Densetsu (Japan).fds")));
    assert!(Platform::Md.accepts(std::path::Path::new("Sonic The Hedgehog (USA, Europe).md")));
    assert!(Platform::Md.accepts(std::path::Path::new("Gunstar Heroes (USA).gen")));
    assert!(!Platform::Md.accepts(std::path::Path::new("Super Mario Bros. (World).nes")));
    assert!(Platform::Pico8.accepts(std::path::Path::new("celeste.p8.png")));
    assert!(Platform::Pico8.accepts(std::path::Path::new("celeste.p8")));
    assert!(Platform::Pico8.accepts(std::path::Path::new("Splore.splore")));
}

#[test]
fn only_the_ds_pico_8_and_ports_are_handed_over_to_programs_of_their_own() {
    for p in Platform::ALL {
        assert_eq!(
            p.external(),
            matches!(p, Platform::Nds | Platform::Pico8 | Platform::Ports),
            "{p:?}"
        );
        assert_eq!(
            omega_store::Core::default_for(p).external(),
            p.external(),
            "{p:?}'s core"
        );
    }
}
