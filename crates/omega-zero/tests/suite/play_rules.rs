//! Each system's games are played by its own rules on the emulator thread: on the handhelds X
//! is B and Y is A, on the Game Boys L1 is A and R1 is B too, and no system keeps rewind
//! snapshots.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use omega_retro::{AvInfo, ButtonMask, CoreError, MockCore, RetroCore};
use omega_store::Platform;
use omega_zero::audio::{AudioSink, StubSink};
use omega_zero::emu::{CoreState, EmuHandle, Speed};

/// A mock that writes down what each frame was played with and counts the states taken of it.
struct Recorder {
    inner: MockCore,
    inputs: Arc<Mutex<Vec<u16>>>,
    states: Arc<AtomicUsize>,
}

impl RetroCore for Recorder {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError> {
        self.inner.load(rom)
    }
    fn run_frame(&mut self, input: ButtonMask) {
        self.inputs.lock().unwrap().push(input.0);
        self.inner.run_frame(input);
    }
    fn video_xrgb8888(&self) -> &[u8] {
        self.inner.video_xrgb8888()
    }
    fn video_size(&self) -> (u32, u32) {
        self.inner.video_size()
    }
    fn take_audio(&mut self) -> Vec<i16> {
        self.inner.take_audio()
    }
    fn serialize(&mut self) -> Result<Vec<u8>, CoreError> {
        self.states.fetch_add(1, Ordering::Relaxed);
        self.inner.serialize()
    }
    fn unserialize(&mut self, data: &[u8]) -> Result<(), CoreError> {
        self.inner.unserialize(data)
    }
    fn save_ram(&self) -> Option<Vec<u8>> {
        self.inner.save_ram()
    }
    fn load_save_ram(&mut self, data: &[u8]) -> Result<(), CoreError> {
        self.inner.load_save_ram(data)
    }
    fn av_info(&self) -> AvInfo {
        self.inner.av_info()
    }
}

struct Played {
    emu: EmuHandle,
    inputs: Arc<Mutex<Vec<u16>>>,
    states: Arc<AtomicUsize>,
}

fn play(platform: Platform, held: u16) -> Played {
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let states = Arc::new(AtomicUsize::new(0));
    let core = Box::new(Recorder {
        inner: MockCore::new(),
        inputs: inputs.clone(),
        states: states.clone(),
    });
    let emu = EmuHandle::spawn_for(
        platform,
        core,
        PathBuf::from("mock"),
        StubSink::draining().ring(),
        None,
        None,
        None,
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while emu.state() == CoreState::Loading {
        assert!(Instant::now() < deadline, "the core never settled");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(emu.state(), CoreState::Ready);
    emu.set_input(ButtonMask(held));
    emu.set_speed(Speed::Normal);
    Played {
        emu,
        inputs,
        states,
    }
}

/// The inputs of the first `n` frames, waiting for them to run.
fn first_frames(p: &Played, n: usize) -> Vec<u16> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let seen = p.inputs.lock().unwrap().clone();
        if seen.len() >= n {
            return seen[..n].to_vec();
        }
        assert!(Instant::now() < deadline, "only {} frames ran", seen.len());
        std::thread::sleep(Duration::from_millis(5));
    }
}

const XY: u16 = ButtonMask::X | ButtonMask::Y;

#[test]
fn on_a_handheld_held_x_is_b_and_y_is_a() {
    for platform in [Platform::Gba, Platform::Gb, Platform::Gbc] {
        for (held, lands) in [
            (ButtonMask::X, ButtonMask::B),
            (ButtonMask::Y, ButtonMask::A),
        ] {
            let p = play(platform, held);
            let frames = first_frames(&p, 12);
            assert!(
                frames.iter().all(|&m| m == lands),
                "{platform:?}: {held:#x} was not played as {lands:#x}: {frames:?}"
            );
        }
    }
}

#[test]
fn on_a_game_boy_l1_is_a_and_r1_is_b_and_on_the_gba_they_are_themselves() {
    for platform in [Platform::Gb, Platform::Gbc] {
        for (held, lands) in [
            (ButtonMask::L, ButtonMask::A),
            (ButtonMask::R, ButtonMask::B),
        ] {
            let p = play(platform, held);
            let frames = first_frames(&p, 12);
            assert!(
                frames.iter().all(|&m| m == lands),
                "{platform:?}: {held:#x} was not played as {lands:#x}: {frames:?}"
            );
        }
    }
    let lr = ButtonMask::L | ButtonMask::R;
    let p = play(Platform::Gba, lr);
    let frames = first_frames(&p, 12);
    assert!(frames.iter().all(|&m| m == lr), "GBA: {frames:?}");
}

#[test]
fn on_a_console_x_and_y_reach_the_game_as_themselves() {
    for platform in [Platform::Snes, Platform::Ps1] {
        let p = play(platform, XY | ButtonMask::L2 | ButtonMask::R2);
        let frames = first_frames(&p, 12);
        assert!(
            frames
                .iter()
                .all(|&m| m == XY | ButtonMask::L2 | ButtonMask::R2),
            "{platform:?}: the game was not played with X, Y, L2 and R2 as held: {frames:?}"
        );
    }
}

#[test]
fn no_system_keeps_rewind_snapshots() {
    for platform in Platform::ALL.into_iter().filter(|p| !p.external()) {
        let p = play(platform, 0);
        first_frames(&p, 30);
        assert_eq!(
            p.states.load(Ordering::Relaxed),
            0,
            "{platform:?}: a state was taken for rewind"
        );
        p.emu.set_rewinding(true);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(p.emu.rewind_fill(), 0, "{platform:?}");
    }
}
