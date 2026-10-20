use omega_store::Platform;

#[test]
fn every_platform_has_a_directory() {
    let names: Vec<&str> = Platform::ALL.iter().map(|p| p.dir_name()).collect();
    assert_eq!(names, vec!["GBA", "GB", "GBC", "SNES", "PS1", "NDS"]);
}

#[test]
fn every_platform_is_a_shelf_of_its_own() {
    assert_eq!(
        Platform::ALL,
        [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Snes,
            Platform::Ps1,
            Platform::Nds
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
}

#[test]
fn only_the_ds_is_handed_over_to_an_emulator_of_its_own() {
    for p in Platform::ALL {
        assert_eq!(p.external(), p == Platform::Nds, "{p:?}");
        assert_eq!(
            omega_store::Core::default_for(p).external(),
            p.external(),
            "{p:?}'s core"
        );
    }
}
