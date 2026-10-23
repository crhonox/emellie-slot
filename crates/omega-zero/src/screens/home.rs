//! The main menu, as spruce's: a row of what there is to open, Recents, Favorites,
//! Collections, Games and Settings, each spruce's icon over its name, on spruce's background.
//! Games opens the grid of systems (screens/systems.rs); Recents, Favorites and a collection
//! open a shelf of their own, carts of any system on it, which B leaves for where it was opened
//! from. Collections lists the player's collections, and MENU on a shelf puts the picked game
//! on the Favorites or a collection, or a new one named on the keyboard.

use omega_input::{Action, Btn};
use omega_store::lists::{self, GameRef};
use omega_store::Platform;
use omega_ui::kit::palette::Palette;
use omega_ui::kit::panel::panel_face;
use omega_ui::kit::row::panel_height;
use omega_ui::{out_h, out_w, Draw, Keyboard, Shelf, Toast, UndoFace, BAR_H, TOP_BAR_H};

use crate::app::{App, Phase};
use crate::list_view::{LegendHint, ListView, RowLabel, RowValue, ViewRow};
use crate::nav::{Dir, Offsets};
use crate::screens::lone::{hold, notes, notes_height, rows_height, Lone, NOTE_GAP};
use crate::screens::systems::Face;

/// What the main menu opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Recents,
    Favorites,
    Collections,
    Games,
    Settings,
}

impl Entry {
    pub const ALL: [Entry; 5] = [
        Entry::Recents,
        Entry::Favorites,
        Entry::Collections,
        Entry::Games,
        Entry::Settings,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Entry::Recents => "Recents",
            Entry::Favorites => "Favorites",
            Entry::Collections => "Collections",
            Entry::Games => "Games",
            Entry::Settings => "Settings",
        }
    }

    pub fn index(self) -> usize {
        Entry::ALL.iter().position(|e| *e == self).unwrap_or(0)
    }
}

/// A shelf of the player's own: its carts any system's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum List {
    Recents,
    Favorites,
    Collection(String),
}

impl List {
    /// What the pill on the bar says, in place of a system.
    pub fn pill(&self) -> &'static str {
        match self {
            List::Recents => "RECENTS",
            List::Favorites => "FAVORITES",
            List::Collection(_) => "COLLECTION",
        }
    }

    fn games(&self, root: &std::path::Path) -> Vec<GameRef> {
        match self {
            List::Recents => lists::recents(root),
            List::Favorites => lists::favorites(root),
            List::Collection(name) => lists::collection(root, name),
        }
    }
}

/// Where a menu left comes back to, and B on a shelf goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Base {
    /// The main menu.
    Home,
    /// The grid of systems.
    Grid,
    /// A shelf: a system's, or a list's.
    #[default]
    Shelf,
}

/// A list's shelf, and the screen B on it goes back to.
pub struct ListShelf {
    /// Which list's shelf this is, of all opened: its faces are its own.
    pub id: u64,
    pub list: List,
    pub shelf: Shelf,
    pub back: Phase,
}

/// The main menu's pictures for one canvas: each entry's icon and name, plain and lit.
#[derive(Clone, Debug, PartialEq)]
pub struct HomeFaces {
    pub canvas: (u32, u32),
    pub icons: Vec<(Entry, Face, Face)>,
    pub names: Vec<(Entry, Face, Face)>,
}

/// The Collect panel: the picked game on the Favorites, then a new collection, then each
/// collection with whether the game is on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectScreen {
    pub at: usize,
    pub game: GameRef,
    pub typing: Option<Keyboard>,
}

/// The Collections panel's and Collect's look: a lone panel of rows, as Labels'.
pub const LIST_WIDTH: u32 = 280;
/// How many rows a list panel shows at once; a longer list scrolls by the one lit.
pub const LIST_ROWS: usize = 9;
const LIST_NOTES: usize = 1;

/// A lone panel titled `title` holding `view`'s rows and its first note, in `pal`.
pub fn list_face(view: &ListView, title: &str, pal: &Palette) -> UndoFace {
    let rows = rows_height(view.rows.len().max(1));
    let content = rows + notes_height(LIST_NOTES);
    let mut panel = panel_face(LIST_WIDTH, panel_height(content), title, true, pal);
    hold(&mut panel, &view.rows, view.lit, pal);
    let shown = &view.status[..view.status.len().min(LIST_NOTES)];
    notes(&mut panel, shown, rows + NOTE_GAP, pal);
    panel.face
}

/// `rows`, cut to a window of [`LIST_ROWS`] that holds `lit`, and where `lit` is in it.
fn window(rows: Vec<ViewRow>, lit: usize) -> (Vec<ViewRow>, usize) {
    if rows.len() <= LIST_ROWS {
        return (rows, lit);
    }
    let first = lit
        .saturating_sub(LIST_ROWS / 2)
        .min(rows.len() - LIST_ROWS);
    let shown = rows.into_iter().skip(first).take(LIST_ROWS).collect();
    (shown, lit - first)
}

/// Where cell `i` of the main menu's one row stands: its icon's middle and its name's
/// bottom, as PyUI lays a grid of one row, the icons in the middle of the room between the
/// bars and each name a little under its icon.
pub fn home_cell(i: usize, w: f32, h: f32) -> (f32, f32, f32) {
    let cols = Entry::ALL.len() as f32;
    let pad = 10.0;
    let col_w = (w - 2.0 * pad) / cols;
    let x = pad + i as f32 * col_w + col_w / 2.0;
    let middle = TOP_BAR_H + (h - TOP_BAR_H - BAR_H) / 2.0;
    // spruce's icons, 140 or 156 tall, and a name's line under them
    let (half, line) = if h >= 600.0 {
        (78.0, 34.0)
    } else {
        (70.0, 30.0)
    };
    let icon_y = middle - line / 2.0;
    (x, icon_y, icon_y + half + 14.0 + line)
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

impl App {
    /// Whether the main menu leads the way in: a card with games on more than one system.
    pub fn uses_menu(&self) -> bool {
        self.uses_grid()
    }

    /// The main menu with the entry last opened lit.
    pub(crate) fn home(&self) -> Phase {
        Phase::Home {
            at: self.state.home_at.min(Entry::ALL.len() as u8 - 1) as usize,
        }
    }

    pub(crate) fn home_input(&mut self, at: usize, action: Action) {
        let n = Entry::ALL.len();
        let to = match action {
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => (at + n - 1) % n,
            Action::GbaDown(Btn::Right) | Action::ShelfRight => (at + 1) % n,
            Action::GbaDown(Btn::A) | Action::Insert => return self.open_entry(Entry::ALL[at]),
            Action::GbaDown(Btn::Start) => return self.open_entry(Entry::Settings),
            _ => return,
        };
        self.state.home_at = to as u8;
        self.phase = Phase::Home { at: to };
    }

    fn open_entry(&mut self, entry: Entry) {
        self.state.home_at = entry.index() as u8;
        self.persist();
        match entry {
            Entry::Recents => self.open_list(List::Recents, self.home()),
            Entry::Favorites => self.open_list(List::Favorites, self.home()),
            Entry::Collections => {
                let names = self.collection_names();
                if names.is_empty() {
                    self.hud.toast(Toast::NoCollections, self.now());
                    return;
                }
                self.base = Base::Home;
                self.go(Phase::Collections { at: 0 }, Dir::Deeper);
            }
            Entry::Games if self.uses_grid() => {
                self.list_shelf = None;
                self.base = Base::Grid;
                self.go(
                    Phase::Systems {
                        at: self.system_at(),
                    },
                    Dir::Deeper,
                );
            }
            Entry::Games => {
                self.list_shelf = None;
                self.base = Base::Shelf;
                self.name_pending = true;
                self.go(Phase::Shelf, Dir::Deeper);
            }
            Entry::Settings => {
                self.base = Base::Home;
                self.open_settings();
            }
        }
    }

    pub(crate) fn collection_names(&self) -> Vec<String> {
        self.root
            .as_deref()
            .map(lists::collections)
            .unwrap_or_default()
    }

    /// The carts `list` names that are on the card, in its order.
    fn list_carts(&self, list: &List) -> Vec<omega_store::Cart> {
        let Some(root) = self.root.as_deref() else {
            return Vec::new();
        };
        list.games(root)
            .iter()
            .filter_map(|g| {
                let (_, shelf) = self.shelves.iter().find(|(p, _)| *p == g.platform)?;
                shelf.carts.iter().find(|c| c.stem == g.stem).cloned()
            })
            .collect()
    }

    /// The shelf of `list`, which B leaves for `back`; a list with none of its games on the
    /// card says so and opens nothing.
    pub(crate) fn open_list(&mut self, list: List, back: Phase) {
        let carts = self.list_carts(&list);
        if carts.is_empty() {
            self.hud.toast(Toast::NothingHere, self.now());
            return;
        }
        // its carts' faces are drawn for it by the frontend, as any shelf's in use
        let mut shelf = Shelf::new(carts);
        if let Some((_, looks)) = self.shelves.iter().find(|(_, s)| !s.carts.is_empty()) {
            shelf.take_looks(looks);
        }
        self.list_ids += 1;
        let id = self.list_ids;
        self.list_shelf = Some(ListShelf {
            id,
            list,
            shelf,
            back,
        });
        self.base = Base::Shelf;
        self.pill_letter = None;
        self.shelf_named = None;
        self.name_pending = true;
        self.go(Phase::Shelf, Dir::Deeper);
    }

    /// The list whose shelf is in view, if it is one.
    pub fn list_in_view(&self) -> Option<&List> {
        self.list_shelf.as_ref().map(|l| &l.list)
    }

    /// B on a list's shelf: back where it was opened from.
    pub(crate) fn leave_list(&mut self) {
        let Some(back) = self.list_shelf.as_ref().map(|l| l.back.clone()) else {
            return;
        };
        self.play_held = None;
        self.shelf_mut().release_hold();
        self.base = Base::Home;
        self.go(back, Dir::Back);
    }

    /// The system of the shelf in view, or of the cart picked on a list's.
    pub(crate) fn shelf_system(&self) -> Platform {
        match &self.list_shelf {
            Some(l) => l
                .shelf
                .carts
                .get(l.shelf.index)
                .map_or(Platform::Gba, |c| c.platform),
            None => self.shelves[self.shelf_at].0,
        }
    }

    pub(crate) fn collections_input(&mut self, at: usize, action: Action) {
        let names = self.collection_names();
        if names.is_empty() {
            return self.go(self.home(), Dir::Back);
        }
        let n = names.len();
        let at = at.min(n - 1);
        let to = match action {
            Action::GbaDown(Btn::Up) => (at + n - 1) % n,
            Action::GbaDown(Btn::Down) => (at + 1) % n,
            Action::GbaDown(Btn::B) | Action::QuickMenu => {
                self.base = Base::Home;
                return self.go(self.home(), Dir::Back);
            }
            Action::GbaDown(Btn::A) => {
                let list = List::Collection(names[at].clone());
                return self.open_list(list, Phase::Collections { at });
            }
            _ => return,
        };
        self.phase = Phase::Collections { at: to };
    }

    pub fn collections_view(&self) -> Option<ListView> {
        let Phase::Collections { at } = &self.phase else {
            return None;
        };
        let root = self.root.as_deref()?;
        let rows: Vec<ViewRow> = self
            .collection_names()
            .into_iter()
            .map(|name| {
                let n = lists::collection(root, &name).len();
                ViewRow {
                    label: RowLabel::Name(name),
                    value: Some(RowValue::Name(n.to_string())),
                    dim: false,
                }
            })
            .collect();
        let (rows, lit) = window(rows, *at);
        Some(ListView {
            rows,
            lit: Some(lit),
            status: vec!["MENU on a game adds it to one.".into()],
            legend: vec![LegendHint::Back],
        })
    }

    /// MENU on a shelf: the Collect panel for the picked game.
    pub(crate) fn open_collect(&mut self) {
        let Some(cart) = self.picked_cart() else {
            return;
        };
        let game = GameRef::new(cart.platform, cart.stem.clone());
        self.go(
            Phase::Collect(CollectScreen {
                at: 0,
                game,
                typing: None,
            }),
            Dir::Aside,
        );
    }

    pub(crate) fn collect_input(&mut self, action: Action) {
        let names = self.collection_names();
        let Phase::Collect(screen) = &mut self.phase else {
            return;
        };
        if screen.typing.is_some() {
            return self.collect_typing(action);
        }
        let n = names.len() + 2;
        match action {
            Action::GbaDown(Btn::Up) => screen.at = (screen.at + n - 1) % n,
            Action::GbaDown(Btn::Down) => screen.at = (screen.at + 1) % n,
            Action::GbaDown(Btn::B) | Action::QuickMenu => self.go(Phase::Shelf, Dir::FromAside),
            Action::GbaDown(Btn::A) => {
                let (at, game) = (screen.at, screen.game.clone());
                match at {
                    0 => self.toggle_favorite(),
                    1 => {
                        let before = self.phase.clone();
                        if let Phase::Collect(screen) = &mut self.phase {
                            screen.typing = Some(Keyboard::default());
                        }
                        self.slide_from(before, Dir::Deeper);
                    }
                    i => self.toggle_collection(&names[i - 2], &game),
                }
            }
            _ => {}
        }
    }

    fn collect_typing(&mut self, action: Action) {
        let Phase::Collect(screen) = &mut self.phase else {
            return;
        };
        let Some(keys) = &mut screen.typing else {
            return;
        };
        let mut submit = false;
        match action {
            Action::GbaDown(Btn::Up) => keys.up(),
            Action::GbaDown(Btn::Down) => keys.down(),
            Action::GbaDown(Btn::Left) => keys.left(),
            Action::GbaDown(Btn::Right) => keys.right(),
            Action::GbaDown(Btn::A) => submit = keys.press(),
            Action::GbaDown(Btn::B) if !keys.text.is_empty() => keys.delete(),
            Action::GbaDown(Btn::B) | Action::QuickMenu => {
                let before = self.phase.clone();
                if let Phase::Collect(screen) = &mut self.phase {
                    screen.typing = None;
                }
                return self.slide_from(before, Dir::Back);
            }
            Action::GbaDown(Btn::X) => keys.shift(),
            Action::GbaDown(Btn::Y) => keys.type_char(' '),
            Action::GbaDown(Btn::L1 | Btn::R1) => keys.symbols(),
            Action::GbaDown(Btn::Start) => submit = true,
            _ => return,
        }
        if !submit {
            return;
        }
        let name = keys.text.clone();
        let game = screen.game.clone();
        let Some(clean) = lists::collection_name(&name) else {
            return;
        };
        let before = self.phase.clone();
        self.toggle_collection(&clean, &game);
        let names = self.collection_names();
        if let Phase::Collect(screen) = &mut self.phase {
            screen.typing = None;
            screen.at = names.iter().position(|n| *n == clean).map_or(0, |i| i + 2);
        }
        self.slide_from(before, Dir::Back);
    }

    /// Puts `game` on the collection `name`, or takes it off, saying which.
    fn toggle_collection(&mut self, name: &str, game: &GameRef) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let on = lists::collection(&root, name).contains(game);
        let done = if on {
            lists::remove_from_collection(&root, name, game).map(|_| Toast::CollectionRemoved)
        } else {
            lists::add_to_collection(&root, name, game).map(|_| Toast::CollectionAdded)
        };
        match done {
            Ok(said) => self.hud.toast(said, self.now()),
            Err(e) => eprintln!("omega-zero: collections: {e}"),
        }
    }

    pub fn collect_view(&self) -> Option<ListView> {
        let Phase::Collect(screen) = &self.phase else {
            return None;
        };
        if screen.typing.is_some() {
            return None;
        }
        let root = self.root.as_deref()?;
        let mut rows = vec![
            ViewRow {
                label: RowLabel::Setting("Favorite"),
                value: Some(RowValue::Setting(
                    self.is_favorite(screen.game.platform, &screen.game.stem),
                )),
                dim: false,
            },
            ViewRow {
                label: RowLabel::Setting("New Collection"),
                value: None,
                dim: false,
            },
        ];
        for name in self.collection_names() {
            let on = lists::collection(root, &name).contains(&screen.game);
            rows.push(ViewRow {
                label: RowLabel::Name(name),
                value: Some(RowValue::Setting(on)),
                dim: false,
            });
        }
        let (rows, lit) = window(rows, screen.at);
        let title = crate::label_sync::title(&screen.game.stem);
        Some(ListView {
            rows,
            lit: Some(lit),
            status: vec![title],
            legend: vec![LegendHint::Back],
        })
    }

    pub(crate) fn set_list_panel(&mut self, panel: Lone) {
        self.list_panel = Some(panel);
    }

    pub(crate) fn draw_list_panel(&self, out: &mut Vec<Draw>) {
        if let Some(panel) = &self.list_panel {
            panel.draw(out);
        }
    }

    pub fn set_home_faces(&mut self, faces: HomeFaces) {
        self.home_faces = Some(faces);
    }

    pub fn home_faces(&self) -> Option<&HomeFaces> {
        self.home_faces.as_ref()
    }

    /// The main menu with `at` lit, moved by `off`; the top bar its own way, as the shelf's.
    pub(crate) fn draw_home(&self, at: usize, off: Offsets, out: &mut Vec<Draw>) {
        let start = out.len();
        if let (Some(faces), Some(grid)) = (&self.home_faces, &self.systems_faces) {
            let (w, h) = (out_w() as f32, out_h() as f32);
            if let Some(bg) = grid.background {
                out.push(Draw::Tex {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                    tex: bg.tex,
                    alpha: 1.0,
                });
            }
            for (i, entry) in Entry::ALL.iter().enumerate() {
                let lit = i == at;
                let (x, icon_y, text_bottom) = home_cell(i, w, h);
                let pick = |set: &Vec<(Entry, Face, Face)>| {
                    set.iter()
                        .find(|(e, _, _)| e == entry)
                        .map(|(_, plain, glow)| if lit { *glow } else { *plain })
                };
                if let Some(f) = pick(&faces.icons) {
                    out.push(tex(f, x - f.w as f32 / 2.0, icon_y - f.h as f32 / 2.0));
                }
                if let Some(f) = pick(&faces.names) {
                    out.push(tex(f, x - f.w as f32 / 2.0, text_bottom - f.h as f32));
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

/// The main menu's icons for a canvas `w` by `h`, as PNG bytes: each entry's, plain and lit.
pub fn home_icon(entry: Entry, w: u32, h: u32) -> (&'static [u8], &'static [u8]) {
    macro_rules! pair {
        ($dir:literal, $name:literal) => {
            (
                &include_bytes!(concat!(
                    "../../../../assets/spruce/",
                    $dir,
                    "/ic-",
                    $name,
                    "-n.png"
                ))[..],
                &include_bytes!(concat!(
                    "../../../../assets/spruce/",
                    $dir,
                    "/ic-",
                    $name,
                    "-f.png"
                ))[..],
            )
        };
    }
    let square = h >= 600;
    let _ = w;
    match (entry, square) {
        (Entry::Recents, false) => pair!("640x480", "recent"),
        (Entry::Favorites, false) => pair!("640x480", "favorite"),
        (Entry::Collections, false) => pair!("640x480", "collection"),
        (Entry::Games, false) => pair!("640x480", "game"),
        (Entry::Settings, false) => pair!("640x480", "setting"),
        (Entry::Recents, true) => pair!("720x720", "recent"),
        (Entry::Favorites, true) => pair!("720x720", "favorite"),
        (Entry::Collections, true) => pair!("720x720", "collection"),
        (Entry::Games, true) => pair!("720x720", "game"),
        (Entry::Settings, true) => pair!("720x720", "setting"),
    }
}

/// The size the main menu's names are written at on a canvas `h` tall, spruce's grid1x4.
pub fn home_text_px(h: u32) -> f32 {
    // spruce writes them at 25 and 28 with four to a row; five are a little smaller
    if h >= 600 {
        24.0
    } else {
        21.0
    }
}
