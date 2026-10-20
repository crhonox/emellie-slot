use omega_store::{move_config, read_app_state};

const FILES: [&str; 5] = [
    "omega-zero.state",
    "selected_core.ini",
    "video_mode.ini",
    "cart_shell.ini",
    "theme.txt",
];

#[test]
fn the_players_files_move_from_system_to_config() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("System")).unwrap();
    for f in FILES {
        std::fs::write(d.path().join("System").join(f), f).unwrap();
    }
    std::fs::write(d.path().join("System/omega-zero"), "binary").unwrap();

    move_config(d.path()).unwrap();

    for f in FILES {
        // the state goes on under Emellie Slot's name
        let kept = if f == "omega-zero.state" {
            "emellie-slot.state"
        } else {
            f
        };
        assert_eq!(
            std::fs::read_to_string(d.path().join("Config").join(kept)).unwrap(),
            f
        );
        assert!(
            !d.path().join("System").join(f).exists(),
            "{f} was left behind"
        );
    }
    assert!(
        d.path().join("System/omega-zero").exists(),
        "a shipped file moved"
    );
}

#[test]
fn a_file_already_in_config_is_not_overwritten() {
    let d = tempfile::tempdir().unwrap();
    for dir in ["System", "Config"] {
        std::fs::create_dir_all(d.path().join(dir)).unwrap();
    }
    std::fs::write(d.path().join("System/selected_core.ini"), "old").unwrap();
    std::fs::write(d.path().join("Config/selected_core.ini"), "new").unwrap();

    move_config(d.path()).unwrap();

    assert_eq!(
        std::fs::read_to_string(d.path().join("Config/selected_core.ini")).unwrap(),
        "new"
    );
}

#[test]
fn settings_written_before_the_move_are_still_read_after_it() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("System")).unwrap();
    std::fs::write(
        d.path().join("System/omega-zero.state"),
        "cart=Emerald\nbrightness=3\nblue_light=1\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=-240\n",
    )
    .unwrap();

    move_config(d.path()).unwrap();

    let s = read_app_state(d.path());
    assert_eq!(
        (s.cart.as_deref(), s.volume, s.utc_offset_min),
        (Some("Emerald"), 40, -240)
    );
}

#[test]
fn omega_zeros_state_goes_on_under_emellie_slots_name() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    std::fs::write(
        d.path().join("Config/omega-zero.state"),
        "cart=Emerald\nbrightness=3\nblue_light=1\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=-240\n",
    )
    .unwrap();
    move_config(d.path()).unwrap();
    assert!(!d.path().join("Config/omega-zero.state").exists());
    assert_eq!(read_app_state(d.path()).cart.as_deref(), Some("Emerald"));
}

#[test]
fn emellie_slots_own_state_is_not_written_over_by_an_old_one() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    std::fs::write(d.path().join("Config/omega-zero.state"), "cart=Old\nbrightness=3\nblue_light=1\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=-240\n").unwrap();
    std::fs::write(d.path().join("Config/emellie-slot.state"), "cart=New\nbrightness=3\nblue_light=1\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=-240\n").unwrap();
    move_config(d.path()).unwrap();
    assert_eq!(read_app_state(d.path()).cart.as_deref(), Some("New"));
}
