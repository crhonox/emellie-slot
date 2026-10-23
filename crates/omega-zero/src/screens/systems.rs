//! The grid of systems: one cell for each system with games on the card, its icon over its
//! name, the one picked lit in its frame, as the menu's theme draws them (menu_theme.rs). A
//! opens the system's shelf; B on that shelf comes back here, to it; START is Settings. The
//! shelf's bars stay: its top bar across the top, and the bar of actions along the bottom.

use omega_input::{Action, Btn};
use omega_store::Platform;
use omega_ui::{out_h, out_w, Draw, TexId};
use omega_ui::{BAR_H, TOP_BAR_H};

use crate::app::{App, Phase};
use crate::nav::{Dir, Offsets};
use crate::screens::home::Base;
use crate::spruce;

/// A picture uploaded for the grid, and its size in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face {
    pub tex: TexId,
    pub w: u32,
    pub h: u32,
}

/// The menu's pictures for one canvas, as its theme has them: the background the main menu
/// and the grid share, if any, the frame, and each system's icon and name, plain and lit (no
/// names where the theme writes none); and how many cells the grid has across and down.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemsFaces {
    pub canvas: (u32, u32),
    pub background: Option<Face>,
    pub frame: Option<Face>,
    pub icons: Vec<(Platform, Face, Face)>,
    pub names: Vec<(Platform, Face, Face)>,
    pub text_h: u32,
    pub cols: u32,
    pub rows: u32,
}

impl SystemsFaces {
    fn icon(&self, platform: Platform, lit: bool) -> Option<Face> {
        self.icons
            .iter()
            .find(|(p, _, _)| *p == platform)
            .map(|(_, plain, glow)| if lit { *glow } else { *plain })
    }

    fn name(&self, platform: Platform, lit: bool) -> Option<Face> {
        self.names
            .iter()
            .find(|(p, _, _)| *p == platform)
            .map(|(_, plain, glow)| if lit { *glow } else { *plain })
    }
}

fn tex(face: Face, x: f32, y: f32) -> Draw {
    Draw::Tex {
        x: x.round(),
        y: y.round(),
        w: face.w as f32,
        h: face.h as f32,
        tex: face.tex,
        alpha: 1.0,
    }
}

impl Phase {
    /// The screen a boot or a menu comes home to: a shelf, or the grid of systems.
    pub fn is_home(&self) -> bool {
        matches!(
            self,
            Phase::Shelf | Phase::Systems { .. } | Phase::Home { .. }
        )
    }
}

impl App {
    /// The systems with games on the card, in the shelves' order: the grid's cells.
    pub fn systems(&self) -> Vec<Platform> {
        self.shelves
            .iter()
            .filter(|(_, s)| !s.carts.is_empty())
            .map(|(p, _)| *p)
            .collect()
    }

    /// Whether there is a grid to choose a system on: games on more than one system.
    pub fn uses_grid(&self) -> bool {
        self.systems().len() > 1
    }

    /// The shelf in view's cell in the grid.
    pub(crate) fn system_at(&self) -> usize {
        let platform = self.shelves[self.shelf_at].0;
        self.systems()
            .iter()
            .position(|p| *p == platform)
            .unwrap_or(0)
    }

    /// Where a menu left comes back to: the grid, when the shelf was left for it, else the
    /// shelf.
    pub(crate) fn home_phase(&self) -> Phase {
        match self.base {
            Base::Home if self.uses_menu() => self.home(),
            Base::Grid if self.uses_grid() => Phase::Systems {
                at: self.system_at(),
            },
            _ => Phase::Shelf,
        }
    }

    /// B on a shelf: back to the grid, at its system.
    pub(crate) fn back_to_grid(&mut self) {
        self.play_held = None;
        self.shelf_mut().release_hold();
        self.base = Base::Grid;
        self.persist();
        self.go(
            Phase::Systems {
                at: self.system_at(),
            },
            Dir::Back,
        );
    }

    /// A on the grid: the shelf of the system at `at`.
    fn open_system(&mut self, at: usize) {
        let Some(platform) = self.systems().get(at).copied() else {
            return;
        };
        let Some(to) = self.shelves.iter().position(|(p, _)| *p == platform) else {
            return;
        };
        if to != self.shelf_at || self.list_shelf.is_some() {
            self.shelf_at = to;
            self.pill_letter = None;
            self.shelf_named = None;
        }
        self.name_pending = true;
        self.list_shelf = None;
        self.base = Base::Shelf;
        self.persist();
        self.go(Phase::Shelf, Dir::Deeper);
    }

    pub(crate) fn systems_input(&mut self, at: usize, action: Action) {
        let n = self.systems().len();
        let cols = self
            .systems_faces
            .as_ref()
            .map_or(spruce::set_for(out_w(), out_h()).cols, |f| f.cols) as usize;
        let to = match action {
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => spruce::step(at, n, cols, -1, 0),
            Action::GbaDown(Btn::Right) | Action::ShelfRight => spruce::step(at, n, cols, 1, 0),
            Action::GbaDown(Btn::Up) => spruce::step(at, n, cols, 0, -1),
            Action::GbaDown(Btn::Down) => spruce::step(at, n, cols, 0, 1),
            Action::GbaDown(Btn::A) | Action::Insert => return self.open_system(at),
            Action::GbaDown(Btn::Start) => return self.open_settings(),
            Action::GbaDown(Btn::B) if self.uses_menu() => {
                self.base = Base::Home;
                return self.go(self.home(), Dir::Back);
            }
            _ => return,
        };
        self.phase = Phase::Systems { at: to };
    }

    pub fn set_systems_faces(&mut self, faces: SystemsFaces) {
        self.systems_faces = Some(faces);
    }

    pub fn systems_faces(&self) -> Option<&SystemsFaces> {
        self.systems_faces.as_ref()
    }

    /// The grid with `at` picked, moved by `off`: its cells with the screen, the top bar its
    /// own way, as the shelf's.
    pub(crate) fn draw_systems(&self, at: usize, off: Offsets, out: &mut Vec<Draw>) {
        let start = out.len();
        if let Some(faces) = &self.systems_faces {
            let (w, h) = (out_w() as f32, out_h() as f32);
            if let Some(bg) = faces.background {
                out.push(Draw::Tex {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                    tex: bg.tex,
                    alpha: 1.0,
                });
            }
            let (cols, rows) = (faces.cols, faces.rows);
            let per_page = (cols * rows) as usize;
            let page = at / per_page;
            let systems = self.systems();
            let text_h = faces.text_h as f32;
            for (i, platform) in systems
                .iter()
                .enumerate()
                .skip(page * per_page)
                .take(per_page)
            {
                let lit = i == at;
                let cell = spruce::cell(
                    (i - page * per_page) as u32,
                    cols,
                    rows,
                    w,
                    TOP_BAR_H,
                    h - BAR_H,
                    text_h,
                );
                if let (true, Some(f)) = (lit, faces.frame) {
                    out.push(tex(
                        f,
                        cell.x - f.w as f32 / 2.0,
                        cell.frame_y - f.h as f32 / 2.0,
                    ));
                }
                if let Some(f) = faces.icon(*platform, lit) {
                    out.push(tex(
                        f,
                        cell.x - f.w as f32 / 2.0,
                        cell.icon_y - f.h as f32 / 2.0,
                    ));
                }
                if let Some(f) = faces.name(*platform, lit) {
                    out.push(tex(
                        f,
                        cell.x - f.w as f32 / 2.0,
                        cell.text_bottom - f.h as f32,
                    ));
                }
            }
        }
        for d in &mut out[start..] {
            *d = d.shifted(off.dx, off.dy);
        }
        let bar = out.len();
        self.draw_top_bar(out);
        for d in &mut out[bar..] {
            *d = d.shifted(0.0, off.bar_dy);
        }
    }
}
