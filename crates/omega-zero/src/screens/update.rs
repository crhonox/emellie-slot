//! The Update screen, opened with A on About: the version installed, the latest release, and a
//! row that checks for one or installs it.
//!
//! It is a lone panel as Labels is: INSTALLED, LATEST and the row A presses, then two lines
//! saying how a check or an install stands. A release found at boot is told with a toast, and
//! Settings' ABOUT row names it. (Settings has no room for a row of its own: GENERAL is full.)

use omega_input::{Action, Btn};
use omega_power::Charge;
use omega_ui::kit::palette::Palette;
use omega_ui::kit::panel::panel_face;
use omega_ui::kit::row::panel_height;
use omega_ui::{Draw, UndoFace};

use crate::app::{App, Phase};
use crate::list_view::{ListView, RowLabel, RowValue, ViewRow};
use crate::nav::Dir;
use crate::screens::lone::{hold, notes, notes_height, rows_height, Lone, NOTE_GAP};
use crate::update::{Fail, UpdateState, Updates};

/// How wide the panel is, in art pixels: as wide as Labels'.
pub const WIDTH: u32 = 280;

pub const TITLE: &str = "UPDATE";

/// The note lines the panel keeps room for, so it holds still as an install goes on.
pub const NOTE_LINES: usize = 2;

/// The row A presses, the panel's third.
pub const ACTION_ROW: usize = 2;

/// Below this charge, off the charger, an install is not started: the handheld must not go
/// flat with half a System/ on the card.
pub const MIN_BATTERY: u8 = 30;

const MB: u64 = 1024 * 1024;

/// What the Update screen shows of `updates`, `online` saying whether a network is joined and
/// `power_ok` whether the battery will see an install through.
pub fn update_view(updates: &Updates, online: bool, power_ok: bool) -> ListView {
    let state = updates.state();
    let latest = match state {
        UpdateState::Unknown => RowValue::Said("-"),
        UpdateState::Checking => RowValue::Said("Checking"),
        UpdateState::UpToDate => RowValue::Name(updates.installed().tag().to_uppercase()),
        UpdateState::Available(r)
        | UpdateState::Verifying(r)
        | UpdateState::Unpacking(r)
        | UpdateState::Downloading { release: r, .. } => {
            RowValue::Name(r.version.tag().to_uppercase())
        }
        UpdateState::Ready(v) => RowValue::Name(v.tag().to_uppercase()),
        UpdateState::Failed { release, .. } => match release {
            Some(r) => RowValue::Name(r.version.tag().to_uppercase()),
            None => RowValue::Said("-"),
        },
    };
    let installs = matches!(
        state,
        UpdateState::Available(_)
            | UpdateState::Failed {
                release: Some(_),
                ..
            }
    );
    let action = if installs || updates.installing() || updates.ready() {
        "Install Update"
    } else {
        "Check for Update"
    };
    let can_press = online && !updates.busy() && !updates.ready() && (!installs || power_ok);
    let rows = vec![
        ViewRow {
            label: RowLabel::Setting("Installed"),
            value: Some(RowValue::Name(updates.installed().tag().to_uppercase())),
            dim: true,
        },
        ViewRow {
            label: RowLabel::Setting("Latest"),
            value: Some(latest),
            dim: true,
        },
        ViewRow {
            label: RowLabel::Setting(action),
            value: None,
            dim: !can_press,
        },
    ];
    ListView {
        rows,
        lit: can_press.then_some(ACTION_ROW),
        status: status(updates, online, power_ok),
        legend: Vec::new(),
    }
}

fn status(updates: &Updates, online: bool, power_ok: bool) -> Vec<String> {
    let lines = |a: &str, b: &str| vec![a.to_string(), b.to_string()];
    match updates.state() {
        UpdateState::Unknown | UpdateState::UpToDate if !online => {
            lines("Join a Wi-Fi network", "to look for updates.")
        }
        UpdateState::Unknown => lines("Press A to look for", "a new release."),
        UpdateState::Checking => lines("Asking GitHub for", "the latest release..."),
        UpdateState::UpToDate => lines("This is the latest", "release."),
        UpdateState::Available(r) if !power_ok => vec![
            format!("{} is out.", r.version.tag()),
            format!("Charge to {MIN_BATTERY}% or plug in."),
        ],
        UpdateState::Available(r) => vec![
            format!("{} is out. A installs it.", r.version.tag()),
            r.notes.clone(),
        ],
        UpdateState::Downloading { got, total, .. } => {
            let first = match (got * 100).checked_div(*total) {
                Some(pct) => format!("Downloading {}%", pct.min(100)),
                None => "Downloading".to_string(),
            };
            vec![first, format!("{} of {} MB", got / MB, total.div_ceil(MB))]
        }
        UpdateState::Verifying(_) => lines("Checking the download...", ""),
        UpdateState::Unpacking(_) => lines("Unpacking...", "Keep the handheld on."),
        UpdateState::Ready(v) => vec![format!("Restarting into {}", v.tag()), String::new()],
        UpdateState::Failed { fail, .. } => {
            let why = match fail {
                Fail::Unreachable => "GitHub could not be reached.",
                Fail::Download => "The download did not finish.",
                Fail::Corrupt => "The download was damaged.",
                Fail::Unpack => "It would not unpack.",
                Fail::Stopped => "Stopped.",
            };
            lines(why, "Press A to try again.")
        }
    }
}

/// The UPDATE panel holding `view`'s rows and its notes under them, in `pal`.
pub fn update_face(view: &ListView, pal: &Palette) -> UndoFace {
    let rows = rows_height(view.rows.len());
    let content = rows + notes_height(NOTE_LINES);
    let mut panel = panel_face(WIDTH, panel_height(content), TITLE, true, pal);
    hold(&mut panel, &view.rows, view.lit, pal);
    let shown = &view.status[..view.status.len().min(NOTE_LINES)];
    notes(&mut panel, shown, rows + NOTE_GAP, pal);
    panel.face
}

impl App {
    /// The battery will see an install through: on the charger, or charged enough.
    pub(crate) fn power_ok_for_update(&self) -> bool {
        match self.battery {
            Some(b) => {
                matches!(b.charge, Charge::Charging | Charge::Full) || b.percent >= MIN_BATTERY
            }
            None => true,
        }
    }

    pub(crate) fn update_input(&mut self, action: Action) {
        match action {
            Action::GbaDown(Btn::A) => {
                let view = update_view(&self.updates, self.online(), self.power_ok_for_update());
                if view.lit != Some(ACTION_ROW) {
                    return self.refuse();
                }
                if self.updates.available().is_some()
                    || matches!(
                        self.updates.state(),
                        UpdateState::Failed {
                            release: Some(_),
                            ..
                        }
                    )
                {
                    self.updates.install();
                } else {
                    self.updates.check();
                }
            }
            // an install keeps the screen until it is done: half a System/ is no place to leave
            Action::GbaDown(Btn::B) if !self.updates.installing() && !self.updates.ready() => {
                self.go(Phase::About, Dir::Back)
            }
            _ => {}
        }
    }

    /// Opens Update from About; opened with nothing known yet, it looks at once.
    pub(crate) fn open_update(&mut self) {
        if self.online()
            && matches!(
                self.updates.state(),
                UpdateState::Unknown | UpdateState::Failed { release: None, .. }
            )
        {
            self.updates.check();
        }
        self.go(Phase::Update, Dir::Deeper);
    }

    /// The Update screen's list, while it is open.
    pub fn update_view(&self) -> Option<ListView> {
        match &self.phase {
            Phase::Update => Some(update_view(
                &self.updates,
                self.online(),
                self.power_ok_for_update(),
            )),
            _ => None,
        }
    }

    pub fn set_update_panel(&mut self, panel: Lone) {
        self.update_panel = Some(panel);
    }

    pub(crate) fn draw_update(&self, out: &mut Vec<Draw>) {
        if let Some(panel) = &self.update_panel {
            panel.draw(out);
        }
    }

    /// The updates' news, each tick: a newer release found told with a toast, and an install
    /// that is ready leaving Emellie Slot for `launch_frontend.sh` to put it in place.
    pub(crate) fn take_updates(&mut self) {
        self.updates.tick();
        if self.updates.take_news().is_some() {
            self.hud.toast(omega_ui::Toast::UpdateAvailable, self.now());
        }
        // the check at boot, once a network is joined, unless a game was put straight back in
        if self.update_check_due && self.online() && !self.in_a_game() {
            self.update_check_due = false;
            self.updates.check();
        }
    }

    /// A game is going in, in the handheld, or coming out: no time to ask GitHub anything.
    fn in_a_game(&self) -> bool {
        matches!(
            self.phase,
            Phase::Inserting { .. }
                | Phase::Playing { .. }
                | Phase::Ejecting { .. }
                | Phase::Polaroids { .. }
                | Phase::Doze { .. }
        )
    }

    /// An update is unpacked and waiting: Emellie Slot is to leave, its saves settled.
    pub fn update_ready(&self) -> bool {
        self.updates.ready()
    }

    /// Settles everything to the card before Emellie Slot leaves for an update.
    pub fn leave_for_update(&mut self) {
        self.settle_saves();
        self.persist();
    }
}
