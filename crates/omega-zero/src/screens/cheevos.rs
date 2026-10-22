//! The Achievements screen, opened from Settings: RetroAchievements ON or OFF, the account
//! (SIGN IN, or its name), then two lines saying how the sign-in stands. Y SIGN OUT is a hint
//! on the bottom bar between its actions; the account is typed on the keyboard screen.

use omega_input::Action;
use omega_store::Platform;
use omega_ui::kit::bar::Hint;
use omega_ui::kit::palette::Palette;
use omega_ui::kit::panel::panel_face;
use omega_ui::kit::row::panel_height;
use omega_ui::{Draw, SettingsRow, UndoFace};

use crate::app::{App, Phase};
use crate::cheevos::CacheGame;
use crate::cheevos_screen::Outcome;
use crate::list_view::{LegendHint, ListView};
use crate::nav::Dir;
use crate::scope::Scope;
use crate::screens::lone::{hold, notes, notes_height, rows_height, Lone, NOTE_GAP};

/// How wide the panel is, in art pixels: as wide as the Labels one.
pub const WIDTH: u32 = 280;

pub const TITLE: &str = "ACHIEVEMENTS";

/// The note lines the panel keeps room for, whatever it says, so it holds still.
pub const NOTE_LINES: usize = 2;

/// The ACHIEVEMENTS panel holding `view`'s rows and its notes under them, in `pal`.
pub fn cheevos_face(view: &ListView, pal: &Palette) -> UndoFace {
    let rows = rows_height(view.rows.len());
    let content = rows + notes_height(NOTE_LINES);
    let mut panel = panel_face(WIDTH, panel_height(content), TITLE, true, pal);
    hold(&mut panel, &view.rows, view.lit, pal);
    let shown = &view.status[..view.status.len().min(NOTE_LINES)];
    notes(&mut panel, shown, rows + NOTE_GAP, pal);
    panel.face
}

/// The keys beyond B and A the bottom bar shows: Y to sign out.
pub fn cheevos_hints(legend: &[LegendHint]) -> Vec<Hint<'static>> {
    legend
        .iter()
        .filter_map(|hint| match hint {
            LegendHint::SignOut => Some(Hint {
                key: "Y",
                word: "SIGN OUT",
            }),
            _ => None,
        })
        .collect()
}

impl App {
    pub(crate) fn cheevos_input(&mut self, action: Action) {
        let root = self.root.clone();
        let account = self.cheevos_account.clone();
        let scopes = self.scope_choices(|p| omega_cheevos::game::console_for(p, &[]).is_some());
        let caching = crate::cheevos::is_caching();
        let Phase::Cheevos(screen) = &mut self.phase else {
            return;
        };
        let before = screen.clone();
        let outcome = screen.input(&account, action, root.as_deref(), &scopes, caching);
        match (before.typing.is_some(), screen.typing.is_some()) {
            (false, true) => self.slide_from(Phase::Cheevos(before), Dir::Deeper),
            (true, false) => self.slide_from(Phase::Cheevos(before), Dir::Back),
            _ => {}
        }
        match outcome {
            Outcome::Close => self.go(
                Phase::Settings {
                    row: SettingsRow::Achievements,
                },
                Dir::Back,
            ),
            Outcome::Changed => {
                if let Some(root) = root.as_deref() {
                    self.cheevos_account = omega_cheevos::config::account(root);
                    crate::cheevos::account_changed(root);
                }
            }
            Outcome::Cache(scope) => {
                let games = self.cache_games(scope);
                crate::cheevos::cache_games(games);
            }
            Outcome::StopCache => crate::cheevos::stop_caching(),
            Outcome::Stay => {}
        }
    }

    /// The games in `scope` RetroAchievements has achievements for, each with the files it
    /// hashes: every disc of a PlayStation game.
    fn cache_games(&self, scope: Scope) -> Vec<CacheGame> {
        self.carts_in(scope)
            .into_iter()
            .filter(|c| omega_cheevos::game::console_for(c.platform, &[]).is_some())
            .map(|c| CacheGame {
                platform: c.platform,
                files: match c.platform {
                    Platform::Ps1 => omega_store::ps1::discs(&c.rom),
                    _ => vec![c.rom.clone()],
                },
                title: crate::label_sync::title(&c.stem),
            })
            .collect()
    }

    /// The Achievements screen's list, while it is open and not taking an account.
    pub fn cheevos_view(&self) -> Option<ListView> {
        match &self.phase {
            Phase::Cheevos(screen) if screen.typing.is_none() => {
                let picked = self
                    .picked_cart()
                    .map(|c| crate::label_sync::title(&c.stem));
                Some(screen.view(
                    &self.cheevos_account,
                    &crate::cheevos::status(),
                    self.online(),
                    &crate::cheevos::caching(),
                    picked.as_deref(),
                ))
            }
            _ => None,
        }
    }

    pub fn set_cheevos_panel(&mut self, panel: Lone) {
        self.cheevos_panel = Some(panel);
    }

    pub(crate) fn draw_cheevos(&self, out: &mut Vec<Draw>) {
        if let Some(panel) = &self.cheevos_panel {
            panel.draw(out);
        }
    }
}
