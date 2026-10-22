//! The shelf: the carts in a row over the background and the bar across the top. Each is a
//! layer of its own as the shelf slides away to a menu and back. The bar across the bottom, with
//! the shelf's system in its pill, is drawn over every screen (bar.rs). Over the selected cart
//! stands its file name, without the extension, so two games that wear the same label (a game
//! and a romhack of it) can be told apart.

use omega_ui::kit::palette::Palette;
use omega_ui::kit::pixel::quad;
use omega_ui::kit::text::text_face;
use omega_ui::{
    ease, model_of, model_shelf_box, out_w, rest_y, sway_reach, Draw, TopBar, UndoFace, SPRITE_PX,
};

use crate::app::App;
use crate::nav::Offsets;
use crate::screens::lone::Lone;

const CORE_PICKER_RECEDE: f32 = 0.26;
const CORE_PICKER_DIM: f32 = 0.614;

/// How far the name stands above the reach of the selected cart's sway, and below the top bar,
/// in screen pixels.
const NAME_GAP: f32 = 8.0;
/// The dark behind the name, in art pixels around it, so it reads over any background.
const NAME_PAD: u32 = 3;
/// The margin each side of the name, in art pixels: a longer name scrolls between them.
const NAME_MARGIN: u32 = 12;
/// A name too long to fit scrolls from its start to its end and jumps back: it stands this long
/// at each end, in ms, and moves this many screen pixels a second.
const NAME_HOLD_MS: f32 = 1_500.0;
const NAME_SPEED: f32 = 40.0;

impl App {
    /// The shelf, its carts moved `off.carts_dx` across and `off.carts_dy` down, its top bar
    /// `off.bar_dy` down.
    pub(crate) fn draw_shelf(&self, off: Offsets, out: &mut Vec<Draw>) {
        let carts = out.len();
        match (self.core_picker_shown(), self.selected_stem()) {
            (Some(picker), Some(stem)) => {
                let open = ease(picker.openness(self.now()));
                let dim = 1.0 + (CORE_PICKER_DIM - 1.0) * open;
                self.shelf()
                    .draw_row(Some(stem), 0.0, CORE_PICKER_RECEDE * open, dim, out);
                shift(&mut out[carts..], off.carts_dx, off.carts_dy);
            }
            _ => {
                self.shelf()
                    .draw_row(None, self.shelf_shake(), 0.0, 1.0, out);
                shift(&mut out[carts..], off.carts_dx, off.carts_dy);
            }
        }
        if self.core_picker_shown().is_none() {
            let name = out.len();
            self.draw_shelf_name(out);
            shift(&mut out[name..], off.carts_dx, off.carts_dy);
        }
        let bar = out.len();
        self.draw_top_bar(out);
        shift(&mut out[bar..], 0.0, off.bar_dy);
    }

    pub fn set_shelf_name(&mut self, stem: String, name: Lone) {
        self.shelf_name = Some((stem, name, self.now()));
    }

    /// The selected cart's file name, centred just over the highest its sway reaches and never
    /// under the top bar, on a dark band; one wider than the canvas between its margins
    /// scrolls, from when it was baked for the cart.
    fn draw_shelf_name(&self, out: &mut Vec<Draw>) {
        let shelf = self.shelf();
        let (Some(cart), Some((stem, name, since))) =
            (shelf.carts.get(shelf.index), &self.shelf_name)
        else {
            return;
        };
        if *stem != cart.stem {
            return;
        }
        let (cw, ch) = model_shelf_box(model_of(cart));
        let px = SPRITE_PX as f32;
        let (w, h) = (name.w as f32 * px, name.h as f32 * px);
        let pad = NAME_PAD as f32 * px;
        let top = omega_ui::TOP_BAR_H + NAME_GAP + pad;
        let y = (rest_y(ch) - sway_reach(cw, ch) - NAME_GAP - pad - h)
            .max(top)
            .round();
        let room = out_w() as f32 - 2.0 * NAME_MARGIN as f32 * px;
        let shown = w.min(room);
        let left = ((out_w() as f32 - shown) / 2.0).round();
        out.push(Draw::Rect {
            x: left - pad,
            y: y - pad,
            w: shown + 2.0 * pad,
            h: h + 2.0 * pad,
            colour: [0.0, 0.0, 0.0, 0.55],
        });
        if w <= room {
            out.push(quad(name.tex, name.w, name.h, left, y));
            return;
        }
        let scrolled = name_scroll(w - room, self.now().saturating_sub(*since) as f32);
        out.push(Draw::Spun {
            x: (left - scrolled).round(),
            y,
            w,
            h,
            tex: name.tex,
            colour: [1.0, 1.0, 1.0, 1.0],
            turn: 0.0,
            yaw: 0.0,
            depth: 0.0,
            clip: Some([left, y, shown, h]),
        });
    }

    /// The bar across the top, in the chrome's colours, its faces baked in its ink.
    pub(crate) fn draw_top_bar(&self, out: &mut Vec<Draw>) {
        let r = self.top_readouts;
        TopBar {
            colours: self.chrome_colours(),
            links: self.links(),
            faces: self.link_faces.as_ref(),
            clock: r.clock,
            battery: self.battery,
            percent: r.percent,
            bolt: r.bolt,
        }
        .draw(out);
    }
}

/// Moves each of `drawn` `dx` across and `dy` down.
fn shift(drawn: &mut [Draw], dx: f32, dy: f32) {
    for d in drawn {
        *d = d.shifted(dx, dy);
    }
}

/// `stem` in Pixel Code in the palette's cream, whole: one too long to fit scrolls.
pub fn shelf_name_face(stem: &str, pal: &Palette) -> UndoFace {
    text_face(stem, pal.cream)
}

/// How far a name `over` screen pixels wider than its room has scrolled `ms` after it came up:
/// held at its start, moved to its end, held there, and back to its start again.
pub fn name_scroll(over: f32, ms: f32) -> f32 {
    let travel = over / NAME_SPEED * 1_000.0;
    let at = ms % (2.0 * NAME_HOLD_MS + travel);
    ((at - NAME_HOLD_MS) / travel).clamp(0.0, 1.0) * over
}
