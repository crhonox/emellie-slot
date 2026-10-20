use crate::common;

use common::{boot, tmp_root_with_carts};
use omega_input::{Action, Btn};
use omega_store::{read_app_state, Platform};
use omega_zero::app::{App, Phase};

/// Two DS games, so that the boot leaves the shelf up rather than put the only cart in.
fn ds_card() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&[]);
    for (stem, head) in [
        ("Platinum", "POKEMON PL\0\0CPUE01"),
        ("Zelda", "ZELDA PH\0\0\0\0AZEE01"),
    ] {
        let mut rom = head.as_bytes().to_vec();
        rom.resize(0x200, 0);
        std::fs::write(d.path().join(format!("Games/NDS/{stem}.nds")), rom).unwrap();
    }
    d
}

fn settle(a: &mut App) {
    for _ in 0..180 {
        a.update(1.0 / 60.0);
    }
}

#[test]
fn a_ds_cart_goes_in_and_waits_to_be_handed_over_rather_than_played() {
    let d = ds_card();
    let mut a = boot(d.path());
    a.apply(Action::Insert);
    settle(&mut a);
    assert!(
        matches!(a.phase(), Phase::Inserting { .. }),
        "the DS cart did not stay in to be handed over: {:?}",
        a.phase()
    );
    let h = a.handoff().expect("no handoff once the cart was in");
    assert_eq!(h.platform, Platform::Nds);
    assert_eq!(h.stem, "Platinum");
    assert_eq!(h.rom, d.path().join("Games/NDS/Platinum.nds"));
    assert!(!h.clean, "a tap started the game afresh");
}

#[test]
fn the_handoff_is_not_there_before_the_cart_is_all_the_way_in() {
    let d = ds_card();
    let mut a = boot(d.path());
    a.apply(Action::Insert);
    a.update(1.0 / 60.0);
    assert!(a.handoff().is_none(), "handed over before the cart went in");
}

#[test]
fn handing_over_writes_the_game_for_launch_frontend_and_notes_it_as_in_the_handheld() {
    let d = ds_card();
    let mut a = boot(d.path());
    a.apply(Action::Insert);
    settle(&mut a);
    assert!(a.hand_over(), "the handoff was not written");
    let text = std::fs::read_to_string(d.path().join("System/.handoff")).unwrap();
    assert!(text.contains("platform=NDS\n"), "{text}");
    assert!(text.contains("stem=Platinum\n"), "{text}");
    let rom = d.path().join("Games/NDS/Platinum.nds");
    assert!(text.contains(&format!("rom={}\n", rom.display())), "{text}");
    assert!(text.contains("clean=0\n"), "{text}");
    let state = read_app_state(d.path());
    assert_eq!(state.cart.as_deref(), Some("Platinum"));
    assert_eq!(state.cart_platform, Some(Platform::Nds));
}

#[test]
fn a_held_a_hands_the_ds_game_over_to_start_afresh() {
    let d = ds_card();
    let mut a = boot(d.path());
    a.apply_at(Action::GbaDown(Btn::A), 0);
    a.tick_ms(500);
    assert!(a.starting_clean(), "holding A still resumed");
    settle(&mut a);
    assert!(
        a.handoff().is_some_and(|h| h.clean),
        "the clean start was lost"
    );
}

#[test]
fn a_ds_game_switched_off_in_goes_straight_back_to_its_emulator_at_boot() {
    let d = ds_card();
    let mut a = boot(d.path());
    a.apply(Action::Insert);
    settle(&mut a);
    assert!(a.hand_over());
    let mut again = boot(d.path());
    again.update(1.0 / 60.0);
    assert!(
        again.handoff().is_some_and(|h| h.stem == "Platinum"),
        "the game the handheld went down in was not handed over again: {:?}",
        again.phase()
    );
}
