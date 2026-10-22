//! The Labels screen, opened from Settings: the player's ScreenScraper account, and a sync
//! that fetches the real labels of the carts that have none: every cart's, one shelf's, or
//! the picked cart's again, as left and right choose on Fetch Labels.

use std::path::Path;

use omega_input::{Action, Btn};
use omega_store::Platform;
use omega_ui::Keyboard;

use crate::label_sync::{
    Account, LabelJob, LabelJobs, LabelNews, Quota, SyncFail, Tally, Want, Who,
};
use crate::list_view::{LegendHint, ListView, RowLabel, RowValue, ViewRow};
use crate::scope::{self, Scope};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    Working {
        done: usize,
        total: usize,
        title: String,
    },
    Done(Tally),
    Failed(SyncFail),
}

pub struct Labels {
    jobs: Box<dyn LabelJobs>,
    account: Option<Account>,
    state: SyncState,
    quota: Option<Quota>,
    /// Labels saved since the shelf last took them.
    fresh: Vec<(Platform, String)>,
}

impl Labels {
    pub fn new(jobs: Box<dyn LabelJobs>, account: Option<Account>) -> Self {
        Labels {
            jobs,
            account,
            state: SyncState::Idle,
            quota: None,
            fresh: Vec::new(),
        }
    }

    pub fn account(&self) -> Option<&Account> {
        self.account.as_ref()
    }

    pub fn state(&self) -> &SyncState {
        &self.state
    }

    pub fn working(&self) -> bool {
        matches!(self.state, SyncState::Working { .. })
    }

    pub fn sign_in(&mut self, account: Account, root: Option<&Path>) {
        if let Some(root) = root {
            let _ = account.save(root);
        }
        self.account = Some(account);
        self.quota = None;
        self.state = SyncState::Idle;
    }

    pub fn sign_out(&mut self, root: Option<&Path>) {
        if let Some(root) = root {
            Account::forget(root);
        }
        self.account = None;
        self.quota = None;
        self.state = SyncState::Idle;
    }

    /// Looks up `wants`, the carts with no label yet.
    pub fn start(&mut self, wants: Vec<Want>) {
        let Some(account) = self.account.clone() else {
            return;
        };
        if wants.is_empty() {
            self.state = SyncState::Done(Tally::default());
            return;
        }
        self.state = SyncState::Working {
            done: 0,
            total: wants.len(),
            title: wants[0].title(),
        };
        self.jobs.ask(LabelJob::Sync { account, wants });
    }

    pub fn stop(&mut self) {
        self.jobs.ask(LabelJob::Stop);
    }

    pub fn tick(&mut self) {
        for news in self.jobs.news() {
            match news {
                LabelNews::Working { done, total, title } => {
                    self.state = SyncState::Working { done, total, title }
                }
                LabelNews::Saved { platform, stem } => self.fresh.push((platform, stem)),
                LabelNews::Quota(q) => self.quota = Some(q),
                LabelNews::Finished(tally) => self.state = SyncState::Done(tally),
                LabelNews::Failed(fail) => self.state = SyncState::Failed(fail),
            }
        }
    }

    /// The carts whose labels arrived since this was last asked.
    pub fn take_fresh(&mut self) -> Vec<(Platform, String)> {
        std::mem::take(&mut self.fresh)
    }

    /// The lines under the list. `online` is whether Wi-Fi has a network joined, `scope` the
    /// carts a fetch would go through and `picked` the title of the cart picked on the shelf.
    pub fn status_lines(&self, online: bool, scope: Scope, picked: Option<&str>) -> Vec<String> {
        if !self.jobs.present() || self.state == SyncState::Failed(SyncFail::NoKey) {
            return said(
                "This build has no ScreenScraper key.",
                "Labels can still be copied into Labels/.",
            );
        }
        match &self.state {
            SyncState::Working { done, total, title } => {
                return said(
                    &format!("Looking up {} of {total}: {title}", done + 1),
                    "Labels go on the shelf as they arrive.",
                )
            }
            SyncState::Failed(fail) => return failed_lines(*fail),
            _ => {}
        }
        if self.account.is_none() {
            return said(
                "Sign in with your free ScreenScraper account",
                "to fetch the real labels of your carts.",
            );
        }
        if !online {
            return said("Join a Wi-Fi network to fetch labels.", "");
        }
        if let SyncState::Done(tally) = &self.state {
            return done_lines(tally);
        }
        let left = self
            .quota
            .map(|q| format!("{} lookups left today.", q.left))
            .unwrap_or_default();
        let what = match (scope, picked) {
            (Scope::Folder(p), _) => {
                format!("Fetches labels for {} carts with none.", p.dir_name())
            }
            (Scope::Game, Some(title)) => format!("Fetches the label of {title} again."),
            _ => "Fetches real labels for carts with none.".to_string(),
        };
        said(&what, &left)
    }
}

fn said(first: &str, second: &str) -> Vec<String> {
    [first, second]
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

fn failed_lines(fail: SyncFail) -> Vec<String> {
    match fail {
        SyncFail::Refused(Who::Player) => said(
            "ScreenScraper refused that name or password.",
            "Press A on ScreenScraper to sign in again.",
        ),
        SyncFail::Refused(Who::Program) => said(
            "ScreenScraper refused this build.",
            "A newer Emellie Slot may be let in.",
        ),
        SyncFail::Spent => said(
            "Today's ScreenScraper lookups are spent.",
            "They come back at midnight, French time.",
        ),
        SyncFail::Down => said(
            "ScreenScraper is not answering just now.",
            "Try again later.",
        ),
        SyncFail::Offline | SyncFail::NoKey => said(
            "Could not reach ScreenScraper.",
            "Check the Wi-Fi and try again.",
        ),
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn done_lines(t: &Tally) -> Vec<String> {
    if *t == Tally::default() {
        return said("Every cart already has a label.", "");
    }
    let first = format!("Done: {}.", plural(t.saved, "new label", "new labels"));
    let second = match (t.bare, t.unknown) {
        (0, 0) => String::new(),
        (bare, 0) => format!("{} no label scan.", plural(bare, "cart has", "carts have")),
        (0, unknown) => format!(
            "{} not on ScreenScraper.",
            plural(unknown, "cart is", "carts are")
        ),
        (bare, unknown) => format!(
            "{} no scan, {} not on ScreenScraper.",
            plural(bare, "has", "have"),
            plural(unknown, "is", "are")
        ),
    };
    said(&first, &second)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sel {
    Account,
    Sync,
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
            Step::User => "ScreenScraper user name".to_string(),
            Step::Password => format!("ScreenScraper password for {}", self.user),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelsScreen {
    pub sel: Sel,
    pub typing: Option<AccountTyping>,
    /// The carts Fetch Labels goes through.
    pub scope: Scope,
}

impl Default for LabelsScreen {
    fn default() -> Self {
        LabelsScreen {
            sel: Sel::Account,
            typing: None,
            scope: Scope::All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    /// Start a sync of the carts in the scope: the screen does not know the carts, its owner
    /// does.
    Sync(Scope),
}

/// What A on Fetch Labels and START do: stop the fetch going, or ask for one once signed in.
fn fetch(labels: &mut Labels, scope: Scope) -> Outcome {
    if labels.working() {
        labels.stop();
    } else if labels.account().is_some() {
        return Outcome::Sync(scope);
    }
    Outcome::Stay
}

impl LabelsScreen {
    /// Acts on `action`, `scopes` being the choices Fetch Labels steps through.
    pub fn input(
        &mut self,
        labels: &mut Labels,
        action: Action,
        root: Option<&Path>,
        scopes: &[Scope],
    ) -> Outcome {
        if self.typing.is_some() {
            self.type_input(labels, action, root);
            return Outcome::Stay;
        }
        match action {
            Action::GbaDown(Btn::B) | Action::QuickMenu => return Outcome::Close,
            Action::GbaDown(Btn::Up | Btn::Down) => {
                self.sel = match self.sel {
                    Sel::Account => Sel::Sync,
                    Sel::Sync => Sel::Account,
                }
            }
            Action::GbaDown(Btn::A) if self.sel == Sel::Account && !labels.working() => {
                let keys = Keyboard {
                    text: labels.account().map(|a| a.user.clone()).unwrap_or_default(),
                    ..Keyboard::default()
                };
                self.typing = Some(AccountTyping {
                    step: Step::User,
                    user: String::new(),
                    keys,
                });
            }
            Action::GbaDown(Btn::Y) if self.sel == Sel::Account && !labels.working() => {
                labels.sign_out(root)
            }
            Action::GbaDown(Btn::Left | Btn::Right)
                if self.sel == Sel::Sync && !labels.working() =>
            {
                let by = if action == Action::GbaDown(Btn::Left) {
                    -1
                } else {
                    1
                };
                self.scope = scope::step(scopes, self.scope, by);
            }
            // A on Fetch Labels, or START on either row, starts a fetch or stops the one going
            Action::GbaDown(Btn::A) if self.sel == Sel::Sync => return fetch(labels, self.scope),
            Action::GbaDown(Btn::Start) => return fetch(labels, self.scope),
            _ => {}
        }
        Outcome::Stay
    }

    fn type_input(&mut self, labels: &mut Labels, action: Action, root: Option<&Path>) {
        let Some(t) = &mut self.typing else {
            return;
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
                return;
            }
            Action::GbaDown(Btn::B) => keys.delete(),
            Action::GbaDown(Btn::X) => keys.shift(),
            Action::GbaDown(Btn::Y) => keys.type_char(' '),
            Action::GbaDown(Btn::L1 | Btn::R1) => keys.symbols(),
            Action::GbaDown(Btn::Start) => submit = true,
            Action::QuickMenu => {
                self.typing = None;
                return;
            }
            _ => return,
        }
        if !submit || t.keys.text.is_empty() {
            return;
        }
        let typed = std::mem::take(&mut t.keys.text);
        match t.step {
            Step::User => {
                t.step = Step::Password;
                t.user = typed;
                t.keys = Keyboard::default();
            }
            Step::Password => {
                let user = std::mem::take(&mut t.user);
                self.typing = None;
                self.sel = Sel::Sync;
                labels.sign_in(
                    Account {
                        user,
                        password: typed,
                    },
                    root,
                );
            }
        }
    }

    /// The list, `picked` being the title of the cart picked on the shelf.
    pub fn view(&self, labels: &Labels, online: bool, picked: Option<&str>) -> ListView {
        let account = labels.account().map(|a| a.user.clone());
        let progress = match labels.state() {
            SyncState::Working { done, total, .. } => {
                RowValue::Name(format!("{} / {total}", done + 1))
            }
            _ => RowValue::Choice(self.scope.name()),
        };
        let rows = vec![
            ViewRow {
                label: RowLabel::Setting("ScreenScraper"),
                value: Some(match account {
                    Some(user) => RowValue::Name(user),
                    None => RowValue::Said("Sign in"),
                }),
                dim: false,
            },
            ViewRow {
                label: RowLabel::Setting("Fetch Labels"),
                value: Some(progress),
                dim: false,
            },
        ];
        let mut legend = vec![LegendHint::Back];
        if self.sel == Sel::Sync && !labels.working() {
            legend.push(LegendHint::Change);
        }
        if self.sel == Sel::Account && !labels.working() {
            legend.push(LegendHint::SignIn);
            if labels.account().is_some() {
                legend.push(LegendHint::SignOut);
            }
        }
        // START stops a fetch or starts one from either row
        if labels.working() {
            legend.push(LegendHint::Stop);
        } else if labels.account().is_some() && online {
            legend.push(LegendHint::Start);
        }
        ListView {
            rows,
            lit: Some(match self.sel {
                Sel::Account => 0,
                Sel::Sync => 1,
            }),
            status: labels.status_lines(online, self.scope, picked),
            legend,
        }
    }
}
