//! VOL+ and VOL- while a port plays, `omega-zero --volume-keys CARD`: `System/ports.sh` runs it
//! beside the port and stops it after. A port plays straight to the sound card, past omega-zero's
//! own volume, so the keys step the card's `lineout volume` instead, as spruceOS does, from the
//! level omega-zero was at and in its steps; each step is kept in the card's state, so omega-zero
//! comes back at it. ports.sh gives the control back the level it had.

use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;

use omega_input::Btn;
use omega_store::{read_app_state, write_app_state, AppState, VOLUME_MAX};

use crate::app::VOLUME_STEP;
use crate::input::evdev::{code_to_btn, decode, pick_devices, EVENT_BYTES, EV_KEY};

/// The sound card's own volume on the H700 handhelds.
pub const CONTROL: &str = "lineout volume";

const GPIO: &str = "/sys/kernel/debug/gpio";

/// The level after `key`, a volume key, in omega-zero's steps.
pub fn stepped(level: u8, key: Btn) -> u8 {
    match key {
        Btn::VolUp => level.saturating_add(VOLUME_STEP).min(VOLUME_MAX),
        Btn::VolDown => level.saturating_sub(VOLUME_STEP),
        _ => level,
    }
}

/// The level `state` plays at, through the headphones or the speaker: none when muted.
pub fn level_of(state: &AppState, headphones: bool) -> u8 {
    match headphones {
        true if state.muted_hp => 0,
        true => state.volume_hp,
        false if state.muted => 0,
        false => state.volume,
    }
}

/// `state` at `level` through the headphones or the speaker, unmuted, as a key leaves it.
pub fn set_level(state: &mut AppState, headphones: bool, level: u8) {
    if headphones {
        state.volume_hp = level;
        state.muted_hp = false;
    } else {
        state.volume = level;
        state.muted = false;
    }
}

fn headphones() -> bool {
    std::fs::read_to_string(GPIO).is_ok_and(|g| omega_power::headphones_in(&g))
}

fn apply(level: u8) {
    let set = Command::new("amixer")
        .args(["-q", "sset", CONTROL, &format!("{level}%")])
        .status();
    if !set.is_ok_and(|s| s.success()) {
        eprintln!("omega-zero: volume keys: amixer could not set {CONTROL} to {level}%");
    }
}

pub fn run(root: &Path) {
    let mut state = read_app_state(root);
    let mut level = level_of(&state, headphones());
    apply(level);
    let (keys, pressed) = mpsc::channel();
    for node in pick_devices(Path::new("/dev/input"), Path::new("/sys/class/input")) {
        let keys = keys.clone();
        std::thread::spawn(move || {
            let Ok(mut file) = std::fs::File::open(&node) else {
                return;
            };
            let mut packet = [0u8; EVENT_BYTES];
            while file.read_exact(&mut packet).is_ok() {
                // a press, and the repeats of one held
                let Some(ev) = decode(&packet).filter(|ev| ev.kind == EV_KEY && ev.value != 0)
                else {
                    continue;
                };
                if let Some(key @ (Btn::VolUp | Btn::VolDown)) = code_to_btn(ev.code) {
                    if keys.send(key).is_err() {
                        return;
                    }
                }
            }
        });
    }
    drop(keys);
    for key in pressed {
        let phones = headphones();
        level = stepped(level_of(&state, phones), key);
        apply(level);
        set_level(&mut state, phones, level);
        if let Err(e) = write_app_state(root, &state) {
            eprintln!("omega-zero: volume keys: state: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_step_as_omega_zero_does_within_its_range() {
        assert_eq!(stepped(60, Btn::VolUp), 65);
        assert_eq!(stepped(60, Btn::VolDown), 55);
        assert_eq!(stepped(98, Btn::VolUp), VOLUME_MAX);
        assert_eq!(stepped(3, Btn::VolDown), 0);
        assert_eq!(stepped(60, Btn::A), 60);
    }

    #[test]
    fn the_level_is_the_speakers_or_the_headphones_and_none_muted() {
        let mut state = AppState {
            volume: 40,
            volume_hp: 20,
            ..AppState::default()
        };
        assert_eq!(level_of(&state, false), 40);
        assert_eq!(level_of(&state, true), 20);
        state.muted = true;
        assert_eq!(level_of(&state, false), 0);
        assert_eq!(level_of(&state, true), 20);
    }

    #[test]
    fn a_key_unmutes_what_it_sets_and_leaves_the_other() {
        let mut state = AppState {
            volume: 40,
            volume_hp: 20,
            muted: true,
            muted_hp: true,
            ..AppState::default()
        };
        set_level(&mut state, false, 5);
        assert_eq!((state.volume, state.muted), (5, false));
        assert_eq!((state.volume_hp, state.muted_hp), (20, true));
    }
}
