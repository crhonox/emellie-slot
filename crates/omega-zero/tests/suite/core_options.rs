use crate::common;

use omega_retro::LibretroCore;
use omega_store::Core;
use omega_zero::link_kind::{armgba_port, LinkKind};

fn dylib_for(core: Core) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor")
        .join(omega_zero::core::dylib_name(core))
}

fn every_serial_mode() -> Vec<&'static str> {
    [LinkKind::Cable, LinkKind::Wireless]
        .map(armgba_port)
        .to_vec()
}

fn every_option_is_one_the_core_has(core: &LibretroCore, what: &str) {
    let declared = core.declared_options();
    assert!(
        !declared.is_empty(),
        "{what}: the core declared no options at all, so this test would pass on anything"
    );
    let set = core.options();
    assert!(!set.is_empty(), "{what}: no options were set");
    for (key, value) in set {
        let Some(values) = declared.get(&key) else {
            let mut known: Vec<_> = declared.keys().cloned().collect();
            known.sort();
            panic!("{what}: the core declares no option {key:?}. It declares: {known:?}");
        };
        assert!(
            values.contains(&value),
            "{what}: the core declares {key:?} as {values:?}, and omega-zero set it to {value:?}"
        );
    }
}

#[test]
fn every_option_omega_zero_sets_is_one_the_core_declares() {
    let _g = common::core_lock();
    let mut ran = 0;
    for which in Core::ALL {
        let path = dylib_for(which);
        if !path.exists() {
            eprintln!("no {} dylib on this host, skipping", which.as_str());
            continue;
        }
        for serial in every_serial_mode() {
            for bios in [false, true] {
                for colour in [false, true] {
                    let mut core = LibretroCore::open(&path).expect("open core");
                    omega_zero::core::apply_core_options(&mut core, which, serial, bios, colour);
                    every_option_is_one_the_core_has(
                        &core,
                        &format!(
                            "{} serial={serial} bios={bios} colour={colour}",
                            which.as_str()
                        ),
                    );
                    ran += 1;
                }
            }
        }
    }
    if ran == 0 {
        eprintln!("no cores on this host, nothing was checked");
    }
}

#[test]
fn both_cores_declare_a_colour_correction_option() {
    let _g = common::core_lock();
    for (which, key) in [
        (Core::Mgba, "mgba_color_correction"),
        (Core::Armgba, "armgba_color_correction"),
    ] {
        let path = dylib_for(which);
        if !path.exists() {
            eprintln!("no {} dylib on this host, skipping", which.as_str());
            continue;
        }
        let core = LibretroCore::open(&path).expect("open core");
        assert!(
            core.declared_options().contains_key(key),
            "{} declares no {key}",
            which.as_str()
        );
    }
}

#[test]
fn a_cable_session_leaves_the_bios_to_the_card() {
    let _g = common::core_lock();
    let path = dylib_for(Core::Mgba);
    if !path.exists() {
        eprintln!("no mgba dylib on this host, skipping");
        return;
    }
    let mut core = LibretroCore::open(&path).expect("open core");
    omega_zero::core::apply_link_options(&mut core, Core::Mgba, 1);
    let set: std::collections::HashMap<String, String> = core.options().into_iter().collect();
    assert_eq!(
        set.get("mgba_use_bios").map(String::as_str),
        None,
        "link mode overrode the card's BIOS"
    );
    assert_eq!(set.get("mgba_link").map(String::as_str), Some("on"));
    assert_eq!(
        set.get("mgba_link_player").map(String::as_str),
        Some("1"),
        "the port this device drives did not reach the core"
    );
    every_option_is_one_the_core_has(&core, "mgba cable session");
}

#[test]
fn a_cable_session_sets_nothing_on_armgba() {
    let _g = common::core_lock();
    let path = dylib_for(Core::Armgba);
    if !path.exists() {
        eprintln!("no armgba dylib on this host, skipping");
        return;
    }
    let mut core = LibretroCore::open(&path).expect("open core");
    omega_zero::core::apply_link_options(&mut core, Core::Armgba, 0);
    assert!(
        core.options().is_empty(),
        "mGBA's cable session reached into armGBA, whose link is its own"
    );
}
