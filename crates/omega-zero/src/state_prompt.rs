//! The box a screen prompt opens over a game (screen_watch.rs): on its password screen, SAVE
//! STATE asks whether to keep a state there; on its title screen, LOAD STATE asks whether to
//! go back to the state the password screen last kept. A says yes and B no; the game is paused
//! under it, dimmed.
//!
//! The state saved is an ordinary one, among the game's others in its polaroids; its stamp is
//! noted beside them (`screen-prompt.txt`) so the title screen loads that one and not whatever
//! state was saved last. The title screen asks only while that state is still there.

use omega_gfx::{out_h, out_w};
use omega_input::{Action, Btn, Millis};
use omega_ui::kit::action::Side;
use omega_ui::kit::bar::BarLayout;
use omega_ui::kit::palette::Palette;
use omega_ui::kit::panel::panel_face;
use omega_ui::kit::pixel::quad;
use omega_ui::kit::row::{panel_height, row_face, row_rect};
use omega_ui::{clean_label, Draw, UndoFace, SPRITE_PX};

use crate::app::{App, Phase};
use crate::disc_screen::DiscBar;
use crate::screen_watch::{note_path, Ask};
use crate::screens::lone::{notes, notes_height, over, rows_height, Lone, Words, NOTE_GAP};

/// How wide the box is, in art pixels: the DISC panel's.
pub const PANEL_W: u32 = crate::disc_screen::PANEL_W;

/// How long the box shuts its ears after it opens: a button already being pressed through the
/// game's screens does not answer it.
pub const DEAF_MS: Millis = 600;

/// How dark the game is under the box.
const DIM: f32 = 0.55;

/// The bar's actions under the box.
pub const ACTIONS: [(&str, &str); 2] = [("NO", "B"), ("YES", "A")];

/// The box while it is open: what it asks, and when it opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatePrompt {
    pub ask: Ask,
    pub since: Millis,
}

/// What the box shows: what it asks, of which game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptView {
    pub ask: Ask,
    pub title: String,
}

/// The box's panel in `pal`: SAVE STATE or LOAD STATE, its question lit as a row, and the game's
/// title as a note under it.
pub fn prompt_panel_face(view: &PromptView, pal: &Palette) -> UndoFace {
    let (title, question, note) = match view.ask {
        Ask::Save => ("SAVE STATE", "SAVE ON THIS SCREEN?", "PASSWORD SCREEN"),
        Ask::Load => (
            "LOAD STATE",
            "LOAD THE SAVED STATE?",
            "FROM THE PASSWORD SCREEN",
        ),
    };
    let row = Words {
        label: question.into(),
        value: None,
        carets: false,
        dim: false,
    };
    let held = rows_height(1);
    let lines = [note.to_string(), view.title.clone()];
    let mut panel = panel_face(
        PANEL_W,
        panel_height(held + notes_height(lines.len())),
        title,
        true,
        pal,
    );
    let [x, y, w, _] = row_rect(panel.well, 0);
    over(&mut panel.face, &row_face(w, &row.row(true), pal), x, y);
    notes(&mut panel, &lines, held + NOTE_GAP, pal);
    panel.face
}

/// The box's actions as the bar draws them at `side`.
pub fn prompt_action(side: Side) -> (&'static str, &'static str) {
    match side {
        Side::Left => ACTIONS[0],
        Side::Right => ACTIONS[1],
    }
}

fn px(art: i32) -> f32 {
    (art * SPRITE_PX as i32) as f32
}

impl App {
    /// A screen of the game in play has just come up and asks `ask`: the box opens, when nothing
    /// else is over the game and, for the title screen, a state from the password screen is
    /// there to load.
    pub fn screen_asks(&mut self, ask: Ask) {
        let busy = !matches!(self.phase, Phase::Playing { .. })
            || self.game_menu_open()
            || self.core_picker.is_some()
            || self.link_active()
            || self.snapshot.is_none();
        if busy {
            return;
        }
        if ask == Ask::Load && self.prompted_state().is_none() {
            return;
        }
        self.state_prompt = Some(StatePrompt {
            ask,
            since: self.now(),
        });
    }

    pub fn state_prompt_open(&self) -> bool {
        self.state_prompt.is_some()
    }

    /// The state the password screen last kept, while it is still among the game's states.
    fn prompted_state(&self) -> Option<std::path::PathBuf> {
        let ring = self.ring()?;
        let stamp = std::fs::read_to_string(note_path(ring.dir())).ok()?;
        let stamp = stamp.trim();
        ring.list()
            .ok()?
            .into_iter()
            .find(|e| e.stamp == stamp)
            .map(|e| e.state)
    }

    pub(crate) fn state_prompt_input(&mut self, action: Action) {
        let Some(prompt) = self.state_prompt else {
            return;
        };
        if self.now().saturating_sub(prompt.since) < DEAF_MS {
            return;
        }
        match action {
            Action::GbaDown(Btn::A) => {
                self.state_prompt = None;
                match prompt.ask {
                    Ask::Save => self.save_prompted(),
                    Ask::Load => self.load_prompted(),
                }
            }
            Action::GbaDown(Btn::B) | Action::GameMenu => self.state_prompt = None,
            _ => {}
        }
    }

    fn save_prompted(&mut self) {
        let Some(stamp) = self.push_state() else {
            return;
        };
        let Some(ring) = self.ring() else { return };
        let note = note_path(ring.dir());
        if let Err(e) = omega_store::atomic_write(&note, stamp.as_bytes()) {
            eprintln!("omega-zero: screen prompts: could not note {stamp}: {e}");
        }
    }

    fn load_prompted(&mut self) {
        match self.prompted_state() {
            Some(state) => {
                self.load_file(&state);
            }
            None => self.refuse(),
        }
    }

    /// What the box shows, while it is open.
    pub fn prompt_view(&self) -> Option<PromptView> {
        let prompt = self.state_prompt?;
        Some(PromptView {
            ask: prompt.ask,
            title: self.seated().map(clean_label).unwrap_or_default(),
        })
    }

    pub fn set_prompt_panel(&mut self, panel: Lone) {
        self.prompt_panel = Some(panel);
    }

    pub fn set_prompt_bar(&mut self, bar: DiscBar) {
        self.prompt_bar = Some(bar);
    }

    /// The box over the game: the game dimmed, the panel in the middle and the bar.
    pub(crate) fn draw_state_prompt(&self, out: &mut Vec<Draw>) {
        if self.state_prompt.is_none() {
            return;
        }
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: out_w() as f32,
            h: out_h() as f32,
            colour: [0.0, 0.0, 0.0, DIM],
        });
        if let Some(panel) = &self.prompt_panel {
            panel.draw(out);
        }
        let Some(bar) = &self.prompt_bar else {
            return;
        };
        let h = out_h();
        for ((tex, w, th), [x, y]) in [
            (bar.set.left, bar.set.layout.left),
            (bar.set.right, bar.set.layout.right),
        ] {
            out.push(quad(tex, w, th, px(x), BarLayout::top(h) + px(y)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_box_fits_its_panel() {
        let pal = omega_ui::kit::palette::palette(0);
        for ask in [Ask::Save, Ask::Load] {
            let view = PromptView {
                ask,
                title: "Castlevania - Bloodlines (USA)".into(),
            };
            let face = prompt_panel_face(&view, pal);
            assert_eq!(face.w, PANEL_W);
            assert!(face.h > 0);
        }
    }
}
