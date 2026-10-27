use crate::common;

use omega_store::{Core, Platform};

fn dylib_for(core: Core) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor")
        .join(omega_zero::core::dylib_name(core))
}

fn armgba() -> Option<omega_retro::LibretroCore> {
    let path = dylib_for(Core::Armgba);
    if !path.exists() {
        eprintln!("no armGBA dylib on this host, skipping");
        return None;
    }
    Some(omega_retro::LibretroCore::open(&path).expect("open armgba"))
}

#[test]
fn each_core_resolves_to_its_own_dylib() {
    assert_ne!(
        omega_zero::core::dylib_name(Core::Mgba),
        omega_zero::core::dylib_name(Core::Armgba)
    );
    assert!(omega_zero::core::dylib_name(Core::Armgba).starts_with("armgba_libretro"));
    assert!(dylib_for(Core::Armgba)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("armgba_libretro"));
}

#[test]
fn armgba_is_always_in_its_separate_link_mode_on_the_port_it_is_handed() {
    let _g = common::core_lock();
    let Some(mut core) = armgba() else {
        return;
    };
    for port in ["cable", "wireless"] {
        omega_zero::core::apply_core_options(&mut core, Core::Armgba, port, false, false);
        assert_eq!(core.option("armgba_link_mode").as_deref(), Some("separate"));
        assert_eq!(
            core.option("armgba_link_port").as_deref(),
            Some(port),
            "armGBA was not put on the port the link screen asked for"
        );
    }
}

#[test]
fn armgba_boots_through_the_bios_when_the_card_carries_one() {
    let _g = common::core_lock();
    let Some(mut core) = armgba() else {
        return;
    };
    omega_zero::core::apply_core_options(&mut core, Core::Armgba, "cable", true, false);
    assert_eq!(core.option("armgba_bios").as_deref(), Some("nintendo"));
    assert_eq!(
        core.option("armgba_skip_bios").as_deref(),
        Some("disabled"),
        "a real BIOS on the card and armGBA still told to skip it"
    );
}

#[test]
fn armgba_runs_its_own_bios_straight_into_the_game_when_the_card_has_none() {
    let _g = common::core_lock();
    let Some(mut core) = armgba() else {
        return;
    };
    omega_zero::core::apply_core_options(&mut core, Core::Armgba, "cable", false, false);
    assert_eq!(core.option("armgba_bios").as_deref(), Some("open-source"));
    assert_eq!(core.option("armgba_skip_bios").as_deref(), Some("enabled"));
}

#[test]
fn armgba_never_restarts_on_a_held_combo() {
    let _g = common::core_lock();
    let Some(mut core) = armgba() else {
        return;
    };
    omega_zero::core::apply_core_options(&mut core, Core::Armgba, "cable", false, false);
    assert_eq!(core.option("armgba_restart_buttons").as_deref(), Some("off"));
}

#[test]
fn mgba_is_given_its_own_frameskip_and_none_of_armgbas_options() {
    let path = dylib_for(Core::Mgba);
    if !path.exists() {
        eprintln!("no mGBA dylib on this host, skipping");
        return;
    }
    let _g = common::core_lock();
    let mut core = omega_retro::LibretroCore::open(&path).expect("open mgba");
    omega_zero::core::apply_core_options(&mut core, Core::Mgba, "wireless", true, false);
    for key in ["armgba_link_port", "armgba_link_mode", "armgba_skip_bios"] {
        assert_eq!(core.option(key), None, "{key} reached mGBA");
    }
    assert_eq!(
        core.option("mgba_frameskip").as_deref(),
        Some("auto"),
        "mGBA was never put on auto frameskip, so nothing can tell it which frame to draw"
    );
}

#[test]
fn both_cores_are_told_about_colour_correction_in_their_own_words() {
    for (which, key, on, off) in [
        (Core::Mgba, "mgba_color_correction", "Auto", "OFF"),
        (Core::Armgba, "armgba_color_correction", "enabled", "disabled"),
    ] {
        let path = dylib_for(which);
        if !path.exists() {
            eprintln!("no {} dylib on this host, skipping", which.as_str());
            continue;
        }
        let _g = common::core_lock();
        let mut core = omega_retro::LibretroCore::open(&path).expect("open the core");
        for (colour, want) in [(true, on), (false, off)] {
            omega_zero::core::apply_core_options(&mut core, which, "cable", false, colour);
            assert_eq!(
                core.option(key).as_deref(),
                Some(want),
                "{} was not told {want} for colour correction {colour}",
                which.as_str()
            );
        }
        let theirs = match which {
            Core::Mgba => "armgba_color_correction",
            Core::Armgba => "mgba_color_correction",
            other => unreachable!("{other:?} has no colour correction"),
        };
        assert_eq!(
            core.option(theirs),
            None,
            "{} was handed the other core's key",
            which.as_str()
        );
    }
}

fn root_with_bios_and_logo_cart() -> Option<(tempfile::TempDir, std::path::PathBuf)> {
    let (bios, rom_bytes) = (common::real_bios()?, common::logo_rom()?);
    let d = common::tmp_root_with_carts(&[]);
    std::fs::copy(bios, d.path().join("BIOS").join("gba_bios.bin")).expect("copy bios");
    let rom = d.path().join("Games").join("Logo.gba");
    std::fs::write(&rom, rom_bytes).expect("write logo rom");
    Some((d, rom))
}

fn splash_plays(root: &std::path::Path, rom: &std::path::Path) -> bool {
    use omega_retro::ButtonMask;
    let mut core =
        omega_zero::core::open_core_for(root, Core::Armgba, "cable", false, &[dylib_for(Core::Armgba)]);
    core.load(rom).expect("armGBA refused the logo rom");
    (0..150).any(|_| {
        core.run_frame(ButtonMask::default());
        common::mostly_lit(core.video_xrgb8888())
    })
}

#[test]
fn a_cart_boots_through_a_real_bios_and_the_splash_reaches_the_screen() {
    if !dylib_for(Core::Armgba).exists() {
        eprintln!("no armGBA dylib on this host, skipping");
        return;
    }
    let Some((d, rom)) = root_with_bios_and_logo_cart() else {
        eprintln!("no real BIOS or no cart to lift a logo from on this host, skipping");
        return;
    };
    let _g = common::core_lock();
    assert!(
        splash_plays(d.path(), &rom),
        "the cart went straight to the game with a real BIOS sitting in the content root"
    );
}

#[test]
fn a_cart_goes_straight_to_the_game_when_the_card_has_no_bios() {
    if !dylib_for(Core::Armgba).exists() {
        eprintln!("no armGBA dylib on this host, skipping");
        return;
    }
    let Some((d, rom)) = root_with_bios_and_logo_cart() else {
        eprintln!("no real BIOS or no cart to lift a logo from on this host, skipping");
        return;
    };
    let _g = common::core_lock();
    std::fs::remove_file(d.path().join("BIOS").join("gba_bios.bin")).unwrap();
    assert!(
        !splash_plays(d.path(), &rom),
        "a splash played with no BIOS on the card, so this test cannot tell the two apart"
    );
}

fn a_published_frame_is_the_splash(
    root: &std::path::Path,
    rom: &std::path::Path,
    resume: Option<Vec<u8>>,
) -> bool {
    use omega_zero::audio::{AudioSink, StubSink};
    use omega_zero::emu::{CoreState, EmuHandle, Speed};
    use std::time::{Duration, Instant};

    let mut sink = StubSink::new();
    sink.open(32_768).expect("the stub refused to open");
    let drain = sink.clone();
    std::thread::spawn(move || loop {
        drain.device_drain();
        std::thread::sleep(Duration::from_millis(2));
    });

    let emu = EmuHandle::spawn(
        omega_zero::core::open_core_for(root, Core::Armgba, "cable", false, &[dylib_for(Core::Armgba)]),
        rom.to_path_buf(),
        sink.ring(),
        None,
        resume,
    );
    emu.set_speed(Speed::Normal);
    let deadline = Instant::now() + Duration::from_secs(10);
    while emu.state() == CoreState::Loading {
        assert!(Instant::now() < deadline, "the core never finished loading");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(emu.state(), CoreState::Ready, "the core refused the cart");

    let watch = Instant::now() + Duration::from_secs(3);
    while Instant::now() < watch {
        if emu.latest_frame().is_some_and(|f| common::mostly_lit(&f)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    false
}

#[test]
fn the_splash_plays_on_a_fresh_start_and_never_over_a_resume() {
    if !dylib_for(Core::Armgba).exists() {
        eprintln!("no armGBA dylib on this host, skipping");
        return;
    }
    let Some((d, rom)) = root_with_bios_and_logo_cart() else {
        eprintln!("no real BIOS or no cart to lift a logo from on this host, skipping");
        return;
    };
    let _g = common::core_lock();

    let state = {
        use omega_retro::ButtonMask;
        let mut core = omega_zero::core::open_core_for(
            d.path(),
            Core::Armgba,
            "cable",
            false,
            &[dylib_for(Core::Armgba)],
        );
        core.load(&rom).expect("armGBA refused the logo rom");
        for _ in 0..480 {
            core.run_frame(ButtonMask::default());
        }
        assert!(
            !common::mostly_lit(core.video_xrgb8888()),
            "the state standing in for a resume is itself a frame of the boot splash, so the \
             half below would fail no matter what the resume did"
        );
        core.serialize().expect("armGBA gave up no state")
    };

    assert!(
        a_published_frame_is_the_splash(d.path(), &rom, None),
        "no splash on a fresh start, so this test cannot see one and proves nothing below"
    );
    assert!(
        !a_published_frame_is_the_splash(d.path(), &rom, Some(state)),
        "the BIOS splash played over a cart the player was resuming"
    );
}

#[test]
fn an_armgba_carts_resume_is_read_from_its_own_core_directory_through_the_session() {
    use omega_input::{Btn, RawEvent};
    use omega_store::{StateRing, SELECTED_CORE_FILE};
    use omega_zero::app::Phase;
    use omega_zero::persist;
    use omega_zero::session::Session;
    use std::time::{Duration, Instant};

    let d = common::tmp_root_with_carts(&["Emerald", "Fusion"]);
    std::fs::write(d.path().join(SELECTED_CORE_FILE), "Emerald = armgba\n").unwrap();

    StateRing::new(d.path(), Platform::Gba, Core::Armgba, "Emerald")
        .write_resume(&700_000u64.to_le_bytes())
        .unwrap();
    StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
        .write_resume(&1u64.to_le_bytes())
        .unwrap();

    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut now = 32;
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    s.app_mut().tick_ms(60_000);
    s.app_mut().settle_saves();

    let state = persist::read_resume(d.path(), Platform::Gba, Core::Armgba, "Emerald")
        .expect("nothing resumed");
    let n = u64::from_le_bytes(state.try_into().expect("mock state is 8 bytes"));
    assert!(
        n >= 700_000,
        "the session resumed the wrong core's state (or none): counter is {n}"
    );
}

#[test]
fn an_armgba_cart_runs_the_dylib_planted_under_its_own_name_through_the_session() {
    use omega_input::{Btn, RawEvent};
    use omega_store::SELECTED_CORE_FILE;
    use omega_zero::app::Phase;
    use omega_zero::persist;
    use omega_zero::session::Session;
    use std::time::{Duration, Instant};

    let Some(mgba) = common::vendored_core() else {
        eprintln!("no host-openable dylib on this machine, skipping");
        return;
    };
    let _g = common::core_lock();

    let d = common::tmp_root_with_real_carts(&["Emerald"]);
    std::fs::write(d.path().join(SELECTED_CORE_FILE), "Emerald = armgba\n").unwrap();
    let planted = d
        .path()
        .join("System")
        .join(omega_zero::core::dylib_name(Core::Armgba));
    std::fs::copy(&mgba, &planted).expect("plant a dylib under armGBA's name");

    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut now = 32;
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    s.app_mut().tick_ms(60_000);
    s.app_mut().settle_saves();

    let state = persist::read_resume(d.path(), Platform::Gba, Core::Armgba, "Emerald")
        .expect("nothing resumed");
    assert!(
        state.len() > 100_000,
        "the session ran the mock, not the dylib the ini named: {} bytes",
        state.len()
    );
}

#[test]
fn changing_the_ini_mid_session_does_not_move_a_seated_carts_autosave() {
    use omega_input::{Btn, RawEvent};
    use omega_store::{StateRing, SELECTED_CORE_FILE};
    use omega_zero::app::Phase;
    use omega_zero::session::Session;
    use std::time::{Duration, Instant};

    let d = common::tmp_root_with_carts(&["Emerald"]);
    std::fs::write(d.path().join(SELECTED_CORE_FILE), "Emerald = armgba\n").unwrap();

    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut now = 32;
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    std::fs::remove_file(d.path().join(SELECTED_CORE_FILE)).unwrap();

    s.app_mut().tick_ms(60_000);
    s.app_mut().settle_saves();

    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Armgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some(),
        "the autosave did not land under the seated core's own directory"
    );
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_none(),
        "the autosave followed the ini's new (absent) reading instead of the core the \
         session actually spawned"
    );
}

#[test]
fn changing_the_ini_mid_session_does_not_move_a_manual_save_state() {
    use omega_input::{Action, Btn, RawEvent};
    use omega_store::{StateRing, SELECTED_CORE_FILE};
    use omega_zero::app::Phase;
    use omega_zero::session::Session;
    use std::time::{Duration, Instant};

    let d = common::tmp_root_with_carts(&["Emerald"]);
    std::fs::write(d.path().join(SELECTED_CORE_FILE), "Emerald = armgba\n").unwrap();

    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut now = 32;
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    std::fs::remove_file(d.path().join(SELECTED_CORE_FILE)).unwrap();

    s.app_mut().apply(Action::SaveState);

    assert!(
        !StateRing::new(d.path(), Platform::Gba, Core::Armgba, "Emerald")
            .list()
            .unwrap()
            .is_empty(),
        "the manual save did not land under the seated core's own directory"
    );
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .list()
            .unwrap()
            .is_empty(),
        "the manual save followed the ini's new (absent) reading instead of the core the \
         session actually spawned"
    );
}

#[test]
fn changing_the_ini_mid_session_does_not_move_an_ejected_carts_resume() {
    use omega_input::{Action, Btn, RawEvent};
    use omega_store::{StateRing, SELECTED_CORE_FILE};
    use omega_zero::app::Phase;
    use omega_zero::session::Session;
    use std::time::{Duration, Instant};

    let d = common::tmp_root_with_carts(&["Emerald", "Fusion"]);
    std::fs::write(d.path().join(SELECTED_CORE_FILE), "Emerald = armgba\n").unwrap();

    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut now = 32;
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    std::fs::remove_file(d.path().join(SELECTED_CORE_FILE)).unwrap();

    s.app_mut().apply(Action::Eject);

    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Armgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some(),
        "the ejected cart's resume did not land under the seated core's own directory"
    );
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_none(),
        "the ejected cart's resume followed the ini's new (absent) reading instead of the \
         core the session actually spawned"
    );
}

#[test]
fn open_core_reaches_an_armgba_named_dylib_under_the_content_roots_system_directory() {
    use omega_retro::ButtonMask;

    let Some(mgba) = common::vendored_core() else {
        eprintln!("no host-openable dylib on this machine, skipping");
        return;
    };
    let _g = common::core_lock();
    let d = common::tmp_root_with_real_carts(&["Probe"]);
    let planted = d
        .path()
        .join("System")
        .join(omega_zero::core::dylib_name(Core::Armgba));
    std::fs::copy(&mgba, &planted).expect("plant a dylib under armGBA's name");

    let mut core = omega_zero::core::open_core(d.path(), Core::Armgba, "cable", false, None).core;
    core.load(&d.path().join("Games/GBA/Probe.gba"))
        .expect("the planted core refused the test rom");
    core.run_frame(ButtonMask::default());
    assert!(
        core.serialize().expect("core gave up no state").len() > 100_000,
        "open_core fell back to the mock instead of the dylib planted at root/System"
    );
}

#[test]
fn a_game_boy_carts_mgba_line_is_dropped_and_it_runs_on_the_platform_default() {
    let default = Core::default_for(Platform::Gb);
    use omega_input::{Btn, RawEvent};
    use omega_store::{StateRing, SELECTED_CORE_FILE};
    use omega_zero::app::Phase;
    use omega_zero::persist;
    use omega_zero::session::Session;
    use std::time::{Duration, Instant};

    let d = common::tmp_root_with_gb_carts(&["Tetris", "Zzz"]);
    std::fs::write(d.path().join(SELECTED_CORE_FILE), "Tetris = mgba\n").unwrap();
    StateRing::new(d.path(), Platform::Gb, default, "Tetris")
        .write_resume(&700_000u64.to_le_bytes())
        .unwrap();

    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.feed([RawEvent::Up(Btn::A)], 32);

    let mut now = 32;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(1));
    }

    let counter = |core| {
        persist::read_resume(d.path(), Platform::Gb, core, "Tetris")
            .map(|b| u64::from_le_bytes(b.try_into().expect("the mock's state is 8 bytes")))
    };

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if counter(default).is_some_and(|n| n > 700_000) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the ini's `Tetris = mgba` was honoured for a Game Boy cart: States/GB/{} still \
             reads {:?} and States/GB/mgba reads {:?}",
            default.as_str(),
            counter(default),
            counter(Core::Mgba)
        );
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(1));
        s.app_mut().flush_resume();
    }
    assert_eq!(
        counter(Core::Mgba),
        None,
        "a Game Boy cart's state was filed under States/GB/mgba"
    );
}
