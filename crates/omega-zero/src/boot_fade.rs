//! The boot logo fading into the shelf. The bootloader's logo stays lit until omega-zero's first
//! frame, so a boot that lands on the shelf draws that same logo over its first frames, where
//! and as big as the bootloader drew it, and fades it out over the shelf already moving under
//! it. A press ends the fade at once, as it ends a slide, and is then handled. A boot that goes
//! straight back into a game, or to the clock first, shows its screen as it always has.

use omega_gfx::{out_h, out_w};
use omega_input::Action;
use omega_ui::kit::fade::stepped;
use omega_ui::{ease, Draw, TexId};

use crate::app::App;

/// How long the logo takes to fade out, in ms.
pub const FADE_MS: f32 = 500.0;

/// The most a frame moves the fade on, in ms: two frames' worth. The first frames at boot are
/// slow while the shelf's faces are made, and a slow frame slows the fade rather than skipping
/// part of it.
pub const STEP_MS: f32 = 1000.0 / 30.0;

/// The logo over the shelf at boot: its texture and size, and how far it has faded, from the
/// first frame drawn once there has been one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BootFade {
    tex: TexId,
    w: u32,
    h: u32,
    elapsed: Option<f32>,
}

/// How opaque the logo is `elapsed` ms into its fade: wholly as it starts, eased out to nothing
/// by the time it is over, in the kit's four steps (`kit::fade::stepped`) as the 16-bit
/// consoles faded, never in between.
pub fn alpha(elapsed: f32) -> f32 {
    stepped(1.0 - ease((elapsed / FADE_MS).clamp(0.0, 1.0)))
}

/// Whether `action` is a press, which ends the fade. A release is not, nor is the lid found
/// open, which the device reports at its first look.
fn presses(action: Action) -> bool {
    !matches!(action, Action::GbaUp(_) | Action::LidOpen)
}

impl App {
    /// Puts the boot logo `tex`, `w` by `h`, over the shelf, to fade from the first frame. It
    /// is only taken while the boot is still on the shelf it landed on.
    pub fn set_boot_logo(&mut self, tex: TexId, w: u32, h: u32) {
        if self.boot_fades() {
            self.boot_fade = Some(BootFade {
                tex,
                w,
                h,
                elapsed: None,
            });
        }
    }

    /// Whether the boot logo would fade now: on the shelf with nothing moving off it, as a boot
    /// that has not gone into a game or to the clock is.
    pub fn boot_fades(&self) -> bool {
        self.phase.is_home() && self.slide.is_none()
    }

    /// How opaque the boot logo is drawn, while it fades.
    pub fn boot_logo_alpha(&self) -> Option<f32> {
        self.boot_fade.map(|f| alpha(f.elapsed.unwrap_or(0.0)))
    }

    /// Ends the fade when `action` is a press.
    pub(crate) fn press_ends_boot_fade(&mut self, action: Action) {
        if presses(action) {
            self.boot_fade = None;
        }
    }

    /// Moves the fade on by `dt` seconds, from the first frame, and ends it once it is over or
    /// the shelf has gone.
    pub(crate) fn step_boot_fade(&mut self, dt: f32) {
        let on_shelf = self.phase.is_home();
        let Some(fade) = &mut self.boot_fade else {
            return;
        };
        let elapsed = match fade.elapsed {
            None => 0.0,
            Some(e) => e + (dt * 1000.0).min(STEP_MS),
        };
        fade.elapsed = Some(elapsed);
        if elapsed >= FADE_MS || !on_shelf {
            self.boot_fade = None;
        }
    }

    /// The logo over everything else on the shelf, in the middle of the canvas as the
    /// bootloader draws it on the panel, on whole pixels.
    pub(crate) fn draw_boot_logo(&self, out: &mut Vec<Draw>) {
        let (Some(fade), Some(alpha)) = (self.boot_fade, self.boot_logo_alpha()) else {
            return;
        };
        if !self.phase.is_home() {
            return;
        }
        out.push(Draw::Tex {
            x: (out_w().saturating_sub(fade.w) / 2) as f32,
            y: (out_h().saturating_sub(fade.h) / 2) as f32,
            w: fade.w as f32,
            h: fade.h as f32,
            tex: fade.tex,
            alpha,
        });
    }
}
