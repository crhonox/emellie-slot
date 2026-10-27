use omega_store::{
    core_for, core_for_platform, read_selected_cores, write_selected_core, Core, Platform,
};
use tempfile::tempdir;

fn root_with(ini: Option<&str>) -> tempfile::TempDir {
    let d = tempdir().unwrap();
    std::fs::create_dir(d.path().join("Config")).unwrap();
    if let Some(text) = ini {
        std::fs::write(d.path().join("Config/selected_core.ini"), text).unwrap();
    }
    d
}

#[test]
fn absent_file_means_everything_defaults() {
    let d = root_with(None);
    assert!(read_selected_cores(d.path()).is_empty());
    assert_eq!(core_for(d.path(), "Emerald"), Core::Armgba);
}

#[test]
fn a_listed_stem_gets_its_core() {
    let d = root_with(Some("Pokemon - Emerald Version (USA, Europe) = mgba\n"));
    assert_eq!(
        core_for(d.path(), "Pokemon - Emerald Version (USA, Europe)"),
        Core::Mgba
    );
}

#[test]
fn an_unlisted_stem_defaults_to_armgba() {
    let d = root_with(Some("Emerald = mgba\n"));
    assert_eq!(core_for(d.path(), "Metroid Fusion"), Core::Armgba);
}

#[test]
fn malformed_lines_are_ignored_rather_than_fatal() {
    let d = root_with(Some(concat!(
        "\n",
        "# a comment\n",
        "; another comment\n",
        "[cores]\n",
        "no equals sign here\n",
        "Emerald = notacore\n",
        "  Spaced Out   =   mgba  \n",
        "= mgba\n",
        "Trailing =\n",
    )));
    let map = read_selected_cores(d.path());
    assert_eq!(
        map.get("Spaced Out"),
        Some(&Core::Mgba),
        "whitespace not trimmed"
    );
    assert_eq!(
        map.get("Emerald"),
        None,
        "an unknown core name must not be stored"
    );
    assert_eq!(core_for(d.path(), "Emerald"), Core::Armgba);
    assert_eq!(map.len(), 1, "only the one good line should survive");
}

#[test]
fn a_later_duplicate_wins() {
    let d = root_with(Some("Emerald = armgba\nEmerald = mgba\n"));
    assert_eq!(core_for(d.path(), "Emerald"), Core::Mgba);
}

#[test]
fn the_last_line_for_a_cart_wins_even_when_it_is_the_typo() {
    let d = root_with(Some("Emerald = mgba\nEmerald = notacore\n"));
    assert_eq!(
        core_for(d.path(), "Emerald"),
        Core::Armgba,
        "a line the later one superseded is still in force"
    );
    assert_eq!(
        read_selected_cores(d.path()).get("Emerald"),
        None,
        "the map and core_for read the same two lines differently"
    );

    let d = root_with(Some("Emerald = notacore\nEmerald = mgba\n"));
    assert_eq!(core_for(d.path(), "Emerald"), Core::Mgba);
    assert_eq!(
        read_selected_cores(d.path()).get("Emerald"),
        Some(&Core::Mgba)
    );
}

#[test]
fn core_names_round_trip() {
    for c in Core::ALL {
        assert_eq!(Core::parse(c.as_str()), Some(c));
    }
    assert_eq!(
        Core::parse("MGBA"),
        Some(Core::Mgba),
        "case is not the user's problem"
    );
    assert_eq!(Core::parse("nonsense"), None);
    assert_eq!(Core::default(), Core::Armgba);
}

#[test]
fn writing_a_core_creates_the_file_when_absent() {
    let d = root_with(None);
    omega_store::write_selected_core(d.path(), "Emerald", Core::Mgba).unwrap();
    assert_eq!(core_for(d.path(), "Emerald"), Core::Mgba);
}

#[test]
fn writing_a_core_replaces_that_carts_line_and_leaves_the_rest_alone() {
    let d = root_with(Some(concat!(
        "# my notes\n",
        "\n",
        "Emerald = armgba\n",
        "Metroid Fusion = mgba\n",
    )));
    omega_store::write_selected_core(d.path(), "Emerald", Core::Mgba).unwrap();

    let text = std::fs::read_to_string(d.path().join("Config/selected_core.ini")).unwrap();
    assert!(
        text.contains("# my notes"),
        "a hand-written comment was destroyed"
    );
    assert!(
        text.contains("Metroid Fusion = mgba"),
        "another cart's entry was lost"
    );
    assert_eq!(core_for(d.path(), "Emerald"), Core::Mgba);
    assert_eq!(core_for(d.path(), "Metroid Fusion"), Core::Mgba);
    assert_eq!(
        text.matches("Emerald").count(),
        1,
        "the old line was left behind"
    );
}

#[test]
fn writing_a_core_appends_a_cart_the_file_has_never_seen() {
    let d = root_with(Some("Emerald = mgba\n"));
    omega_store::write_selected_core(d.path(), "Drill Dozer", Core::Mgba).unwrap();
    assert_eq!(core_for(d.path(), "Emerald"), Core::Mgba);
    assert_eq!(core_for(d.path(), "Drill Dozer"), Core::Mgba);
}

#[test]
fn writing_the_default_still_records_it() {
    let d = root_with(Some("Emerald = mgba\n"));
    omega_store::write_selected_core(d.path(), "Emerald", Core::Armgba).unwrap();
    assert_eq!(core_for(d.path(), "Emerald"), Core::Armgba);
    let text = std::fs::read_to_string(d.path().join("Config/selected_core.ini")).unwrap();
    assert!(text.contains("Emerald = armgba"));
}

#[test]
fn the_game_boys_default_to_gambatte_and_the_gba_to_armgba() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        core_for_platform(d.path(), "Tetris", Platform::Gb),
        Core::Gambatte
    );
    assert_eq!(
        core_for_platform(d.path(), "Tetris Chromatic", Platform::Gbc),
        Core::Gambatte
    );
    assert_eq!(
        core_for_platform(d.path(), "Emerald", Platform::Gba),
        Core::Armgba
    );
}

#[test]
fn a_cart_can_ask_for_a_non_default_core_by_hand() {
    let d = tempfile::tempdir().unwrap();
    write_selected_core(d.path(), "Emerald", Core::Mgba).unwrap();
    assert_ne!(Core::default_for(Platform::Gba), Core::Mgba);
    assert_eq!(
        core_for_platform(d.path(), "Emerald", Platform::Gba),
        Core::Mgba
    );
}

#[test]
fn a_line_naming_a_core_the_platform_cannot_run_is_dropped() {
    let d = tempfile::tempdir().unwrap();
    for core in [Core::Mgba, Core::Snes9x] {
        write_selected_core(d.path(), "Tetris", core).unwrap();
        for platform in [Platform::Gb, Platform::Gbc] {
            assert_eq!(
                core_for_platform(d.path(), "Tetris", platform),
                Core::Gambatte,
                "{core:?} was kept for a {platform:?} game"
            );
        }
    }
    write_selected_core(d.path(), "Tetris", Core::Armgba).unwrap();
    assert_eq!(
        core_for_platform(d.path(), "Tetris", Platform::Gb),
        Core::Armgba,
        "a Game Boy game set to armGBA by hand lost it"
    );
}

#[test]
fn the_picker_board_has_exactly_two_sockets() {
    assert_eq!(Core::for_platform(Platform::Gba).len(), 2);
}

#[test]
fn a_cart_left_on_gpsp_runs_on_armgba_which_took_its_place() {
    let d = root_with(Some("Emerald = gpsp\n"));
    assert_eq!(core_for(d.path(), "Emerald"), Core::Armgba);
    assert_eq!(
        core_for_platform(d.path(), "Emerald", Platform::Gba),
        Core::Armgba
    );
}
