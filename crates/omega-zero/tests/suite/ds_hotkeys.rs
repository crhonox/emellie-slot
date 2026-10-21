use omega_zero::ds_hotkeys::{reset, with_hotkeys_of};

const SHIPPED: &str = "[video]\nlayout = vertical\n[padhotkeys]\nmodifier = guide\npause = mod\n[hotkeys]\nquit = none\n[cheevos]\nenabled = false\n";

#[test]
fn only_the_hotkey_sections_come_back_from_the_shipped_settings() {
    let own = "[video]\nlayout = single\n[padhotkeys]\nmodifier = none\npause = none\n[pad]\na = b\n[hotkeys]\nquit = Escape\n";
    let out = with_hotkeys_of(own, SHIPPED);
    assert!(
        out.contains("layout = single"),
        "the player's layout went: {out}"
    );
    assert!(
        out.contains("[pad]\na = b\n"),
        "the player's buttons went: {out}"
    );
    assert!(
        out.contains("[padhotkeys]\nmodifier = guide\npause = mod\n"),
        "{out}"
    );
    assert!(out.contains("[hotkeys]\nquit = none\n"), "{out}");
    assert!(
        !out.contains("modifier = none") && !out.contains("Escape"),
        "{out}"
    );
    assert!(
        !out.contains("layout = vertical"),
        "a section past the hotkeys came too: {out}"
    );
    assert!(
        !out.contains("[cheevos]"),
        "a section past the hotkeys came too: {out}"
    );
}

#[test]
fn settings_with_no_hotkeys_are_given_the_shipped_ones() {
    let out = with_hotkeys_of("[video]\nlayout = single", SHIPPED);
    assert!(
        out.starts_with("[video]\nlayout = single\n[padhotkeys]\n"),
        "{out}"
    );
}

#[test]
fn every_pads_settings_on_the_card_are_reset_and_nothing_else() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    std::fs::create_dir_all(root.join("System/dsperate")).unwrap();
    std::fs::create_dir_all(root.join("Config/dsperate")).unwrap();
    std::fs::write(root.join("System/dsperate/nds.ini"), SHIPPED).unwrap();
    let broken = "[padhotkeys]\npause = none\n";
    for f in ["nds-nostick.ini", "nds-2stick.ini", "Castlevania.ini"] {
        std::fs::write(root.join("Config/dsperate").join(f), broken).unwrap();
    }
    assert_eq!(reset(root).unwrap(), 2);
    for f in ["nds-nostick.ini", "nds-2stick.ini"] {
        let got = std::fs::read_to_string(root.join("Config/dsperate").join(f)).unwrap();
        assert!(got.contains("pause = mod"), "{f}: {got}");
    }
    let game = std::fs::read_to_string(root.join("Config/dsperate/Castlevania.ini")).unwrap();
    assert_eq!(game, broken, "a game's own settings were touched");
}

#[test]
fn a_card_with_no_settings_yet_has_nothing_to_reset() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("System/dsperate")).unwrap();
    std::fs::write(d.path().join("System/dsperate/nds.ini"), SHIPPED).unwrap();
    assert_eq!(reset(d.path()).unwrap(), 0);
}
