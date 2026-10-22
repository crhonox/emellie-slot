//! The Achievements screen, opened from Settings: RetroAchievements on or off, the player's
//! account, typed on the keyboard as the Labels screen's ScreenScraper one is, and Offline
//! Cache, which keeps on the card what every game, one shelf's or the picked game would fetch
//! from RetroAchievements, so it can be played offline with its achievements.

use std::path::Path;

use omega_cheevos::config::{self, Account};
use omega_input::{Action, Btn};
use omega_ui::Keyboard;

use crate::cheevos::{CacheFail, Caching, Status};
use crate::list_view::{LegendHint, ListView, RowLabel, RowValue, ViewRow};
use crate::scope::{self, Scope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sel {
    Enabled,
    Account,
    Cache,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    User,
    Password,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountTyping {
    pub step: Step,
    /// The user name typed, once the password is being asked for.
    pub user: String,
    pub keys: Keyboard,
}

impl AccountTyping {
    pub fn prompt(&self) -> String {
        match self.step {
            Step::User => "RetroAchievements user name".to_string(),
            Step::Password => format!("RetroAchievements password for {}", self.user),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheevosScreen {
    pub sel: Sel,
    pub typing: Option<AccountTyping>,
    /// The games Offline Cache goes through.
    pub scope: Scope,
}

impl Default for CheevosScreen {
    fn default() -> Self {
        CheevosScreen {
            sel: Sel::Enabled,
            typing: None,
            scope: Scope::All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    /// The account or the switch changed on the card: the sign-in starts over.
    Changed,
    /// Cache the games in the scope: the screen does not know them, its owner does.
    Cache(Scope),
    StopCache,
}

fn said(first: &str, second: &str) -> Vec<String> {
    [first, second]
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// The lines under the list: where the account stands. `online` is whether Wi-Fi has a
/// network joined.
pub fn status_lines(account: &Account, status: &Status, online: bool) -> Vec<String> {
    if !account.enabled {
        return said(
            "RetroAchievements is off.",
            "Turn it on to earn achievements.",
        );
    }
    if !account.signed || account.username.is_empty() {
        return said(
            "Sign in with your RetroAchievements",
            "account to earn achievements.",
        );
    }
    match status {
        Status::SignedIn(user) => said(
            &format!("Signed in as {user}."),
            "States, rewind and fast forward stay on.",
        ),
        Status::Rejected(_) => said(
            "RetroAchievements refused that password.",
            "Press A on Account to sign in again.",
        ),
        Status::SigningIn => said(&format!("Signing in as {}...", account.username), ""),
        Status::Waiting if !online => said("Join a Wi-Fi network to sign in.", ""),
        Status::Waiting | Status::NoAccount => {
            said(&format!("Signing in as {}...", account.username), "")
        }
        Status::Off => said("Signs in when Emellie Slot next starts.", ""),
    }
}

/// The lines under the list while Offline Cache is picked or going: how the caching stands, or
/// what it would do. `picked` is the title of the game picked on the shelf.
pub fn cache_lines(caching: &Caching, scope: Scope, picked: Option<&str>) -> Vec<String> {
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    match caching {
        Caching::Working { done, total, title } => said(
            &format!("Caching {} of {total}: {title}", done + 1),
            "Achievements kept for offline play.",
        ),
        Caching::Done(t) if t.cached == 0 && t.unknown == 0 => said("No games to cache.", ""),
        Caching::Done(t) => said(
            &format!("Done: {} cached.", plural(t.cached, "game", "games")),
            &match t.unknown {
                0 => String::new(),
                n => format!(
                    "{} not on RetroAchievements.",
                    plural(n, "game is", "games are")
                ),
            },
        ),
        Caching::Failed(CacheFail::SignedOut) => {
            said("Sign in first, with Wi-Fi on,", "to cache achievements.")
        }
        Caching::Failed(CacheFail::Offline) => said(
            "Could not reach RetroAchievements.",
            "Check the Wi-Fi and try again.",
        ),
        Caching::Failed(CacheFail::Refused) => said(
            "RetroAchievements refused the account.",
            "Press A on Account to sign in again.",
        ),
        Caching::Idle => said(
            &match (scope, picked) {
                (Scope::Folder(p), _) => format!("Keeps {} achievements on the card", p.dir_name()),
                (Scope::Game, Some(title)) => format!("Keeps {title}'s achievements"),
                _ => "Keeps every game's achievements".to_string(),
            },
            "to play offline. A starts, A again stops.",
        ),
    }
}

impl CheevosScreen {
    /// Acts on `action`, `account` the card's as it stands, written back through `root`;
    /// `scopes` are the choices Offline Cache steps through, `caching` whether it is going.
    pub fn input(
        &mut self,
        account: &Account,
        action: Action,
        root: Option<&Path>,
        scopes: &[Scope],
        caching: bool,
    ) -> Outcome {
        if self.typing.is_some() {
            return self.type_input(action, root);
        }
        let Some(root) = root else {
            return match action {
                Action::GbaDown(Btn::B) | Action::QuickMenu => Outcome::Close,
                _ => Outcome::Stay,
            };
        };
        let saved = |r: std::io::Result<()>| match r {
            Ok(()) => Outcome::Changed,
            Err(e) => {
                eprintln!("omega-zero: cheevos: {e}");
                Outcome::Stay
            }
        };
        match action {
            Action::GbaDown(Btn::B) | Action::QuickMenu => Outcome::Close,
            Action::GbaDown(Btn::Up) => {
                self.sel = match self.sel {
                    Sel::Enabled => Sel::Cache,
                    Sel::Account => Sel::Enabled,
                    Sel::Cache => Sel::Account,
                };
                Outcome::Stay
            }
            Action::GbaDown(Btn::Down) => {
                self.sel = match self.sel {
                    Sel::Enabled => Sel::Account,
                    Sel::Account => Sel::Cache,
                    Sel::Cache => Sel::Enabled,
                };
                Outcome::Stay
            }
            Action::GbaDown(Btn::Left | Btn::Right) if self.sel == Sel::Cache && !caching => {
                let by = if action == Action::GbaDown(Btn::Left) {
                    -1
                } else {
                    1
                };
                self.scope = scope::step(scopes, self.scope, by);
                Outcome::Stay
            }
            Action::GbaDown(Btn::A | Btn::Start) if self.sel == Sel::Cache => {
                if caching {
                    Outcome::StopCache
                } else {
                    Outcome::Cache(self.scope)
                }
            }
            Action::GbaDown(Btn::Left | Btn::Right | Btn::A) if self.sel == Sel::Enabled => {
                saved(config::set_enabled(root, !account.enabled))
            }
            Action::GbaDown(Btn::A) if self.sel == Sel::Account => {
                let keys = Keyboard {
                    text: account.username.clone(),
                    ..Keyboard::default()
                };
                self.typing = Some(AccountTyping {
                    step: Step::User,
                    user: String::new(),
                    keys,
                });
                Outcome::Stay
            }
            Action::GbaDown(Btn::Y) if self.sel == Sel::Account && account.signed => {
                saved(config::sign_out(root))
            }
            _ => Outcome::Stay,
        }
    }

    fn type_input(&mut self, action: Action, root: Option<&Path>) -> Outcome {
        let Some(t) = &mut self.typing else {
            return Outcome::Stay;
        };
        let keys = &mut t.keys;
        let mut submit = false;
        match action {
            Action::GbaDown(Btn::Up) => keys.up(),
            Action::GbaDown(Btn::Down) => keys.down(),
            Action::GbaDown(Btn::Left) => keys.left(),
            Action::GbaDown(Btn::Right) => keys.right(),
            Action::GbaDown(Btn::A) => submit = keys.press(),
            Action::GbaDown(Btn::B) if keys.text.is_empty() => {
                self.typing = None;
                return Outcome::Stay;
            }
            Action::GbaDown(Btn::B) => keys.delete(),
            Action::GbaDown(Btn::X) => keys.shift(),
            Action::GbaDown(Btn::Y) => keys.type_char(' '),
            Action::GbaDown(Btn::L1 | Btn::R1) => keys.symbols(),
            Action::GbaDown(Btn::Start) => submit = true,
            Action::QuickMenu => {
                self.typing = None;
                return Outcome::Stay;
            }
            _ => return Outcome::Stay,
        }
        if !submit || t.keys.text.is_empty() {
            return Outcome::Stay;
        }
        let typed = std::mem::take(&mut t.keys.text);
        match t.step {
            Step::User => {
                t.step = Step::Password;
                t.user = typed;
                t.keys = Keyboard::default();
                Outcome::Stay
            }
            Step::Password => {
                let user = std::mem::take(&mut t.user);
                self.typing = None;
                let Some(root) = root else {
                    return Outcome::Stay;
                };
                match config::sign_in_with(root, &user, &typed) {
                    Ok(()) => Outcome::Changed,
                    Err(e) => {
                        eprintln!("omega-zero: cheevos: {e}");
                        Outcome::Stay
                    }
                }
            }
        }
    }

    /// The list; `caching` is how Offline Cache stands and `picked` the title of the game
    /// picked on the shelf.
    pub fn view(
        &self,
        account: &Account,
        status: &Status,
        online: bool,
        caching: &Caching,
        picked: Option<&str>,
    ) -> ListView {
        let cache = match caching {
            Caching::Working { done, total, .. } => {
                RowValue::Name(format!("{} / {total}", done + 1))
            }
            _ => RowValue::Choice(self.scope.name()),
        };
        let rows = vec![
            ViewRow {
                label: RowLabel::Setting("RetroAchievements"),
                value: Some(RowValue::Setting(account.enabled)),
                dim: false,
            },
            ViewRow {
                label: RowLabel::Setting("Account"),
                value: Some(if account.signed && !account.username.is_empty() {
                    RowValue::Name(account.username.clone())
                } else {
                    RowValue::Said("Sign in")
                }),
                dim: false,
            },
            ViewRow {
                label: RowLabel::Setting("Offline Cache"),
                value: Some(cache),
                dim: false,
            },
        ];
        let mut legend = vec![LegendHint::Back];
        match self.sel {
            Sel::Enabled => legend.push(LegendHint::Change),
            Sel::Account => {
                legend.push(LegendHint::SignIn);
                if account.signed {
                    legend.push(LegendHint::SignOut);
                }
            }
            Sel::Cache if matches!(caching, Caching::Working { .. }) => {
                legend.push(LegendHint::Stop)
            }
            Sel::Cache => legend.push(LegendHint::Change),
        }
        let going = matches!(caching, Caching::Working { .. });
        ListView {
            rows,
            lit: Some(match self.sel {
                Sel::Enabled => 0,
                Sel::Account => 1,
                Sel::Cache => 2,
            }),
            status: if self.sel == Sel::Cache || going {
                cache_lines(caching, self.scope, picked)
            } else {
                status_lines(account, status, online)
            },
            legend,
        }
    }
}
