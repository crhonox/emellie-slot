//! Which shelf's carts need faces now, for the frontend to draw them only for the shelf in use
//! rather than for every cart on the card at boot: the shelf in view while it is shown, a game
//! in play included, and none on the menus. A face is about a quarter of a megabyte; a card of
//! a thousand games used to keep them all.

use omega_store::{Cart, Platform};
use omega_ui::TexId;

use crate::app::{App, Phase};

/// A shelf whose carts have faces of their own: a system's, or the list's in view, told from
/// the one before it by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShelfKey {
    System(Platform),
    List(u64),
}

/// The screens that draw the shelf's carts: the shelf, a cart going in, in play or coming
/// out, and the save states over a game.
fn shows_carts(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::Shelf
            | Phase::Inserting { .. }
            | Phase::Playing { .. }
            | Phase::Ejecting { .. }
            | Phase::Polaroids { .. }
            | Phase::Doze { cart: Some(_) }
    )
}

impl App {
    /// The shelf in view's key.
    pub fn shelf_key(&self) -> ShelfKey {
        match &self.list_shelf {
            Some(list) => ShelfKey::List(list.id),
            None => ShelfKey::System(self.shelves[self.shelf_at].0),
        }
    }

    /// The shelf whose carts are drawn now: the one in view, while the screen shown or the
    /// one it slides from draws them.
    pub fn shelf_in_use(&self) -> Option<ShelfKey> {
        let sliding_off_a_shelf = self.slide().is_some_and(|s| shows_carts(&s.from));
        (shows_carts(self.phase()) || sliding_off_a_shelf).then(|| self.shelf_key())
    }

    fn keyed(&self, key: ShelfKey) -> Option<&omega_ui::Shelf> {
        match key {
            ShelfKey::System(p) => self.shelves.iter().find(|(q, _)| *q == p).map(|(_, s)| s),
            ShelfKey::List(id) => self
                .list_shelf
                .as_ref()
                .filter(|l| l.id == id)
                .map(|l| &l.shelf),
        }
    }

    fn keyed_mut(&mut self, key: ShelfKey) -> Option<&mut omega_ui::Shelf> {
        match key {
            ShelfKey::System(p) => self
                .shelves
                .iter_mut()
                .find(|(q, _)| *q == p)
                .map(|(_, s)| s),
            ShelfKey::List(id) => self
                .list_shelf
                .as_mut()
                .filter(|l| l.id == id)
                .map(|l| &mut l.shelf),
        }
    }

    /// Whether every cart on `key`'s shelf has its face; a shelf no longer there has none.
    pub fn has_faces(&self, key: ShelfKey) -> bool {
        self.keyed(key).is_some_and(|s| s.has_faces())
    }

    /// The carts on `key`'s shelf, in its order, which its faces are given in.
    pub fn carts_on(&self, key: ShelfKey) -> Vec<Cart> {
        self.keyed(key).map(|s| s.carts.clone()).unwrap_or_default()
    }

    /// The carts on `key`'s shelf, the picked one first and then the nearest either way, the
    /// order their labels are drawn in.
    pub fn carts_on_nearest_first(&self, key: ShelfKey) -> Vec<Cart> {
        let Some(shelf) = self.keyed(key) else {
            return Vec::new();
        };
        let len = shelf.carts.len();
        let mut row: Vec<(usize, &Cart)> = shelf.carts.iter().enumerate().collect();
        row.sort_by_key(|(i, _)| {
            let d = i.abs_diff(shelf.index);
            d.min(len - d)
        });
        row.into_iter().map(|(_, c)| c.clone()).collect()
    }

    pub fn set_shelf_faces(&mut self, key: ShelfKey, faces: Vec<TexId>) {
        if let Some(shelf) = self.keyed_mut(key) {
            shelf.set_faces(faces);
        }
    }

    /// Takes `key`'s shelf's faces back, leaving its carts faceless until they are drawn
    /// again.
    pub fn take_shelf_faces(&mut self, key: ShelfKey) -> Vec<TexId> {
        self.keyed_mut(key)
            .map(|s| s.take_faces())
            .unwrap_or_default()
    }

    /// Every face the cart `stem` of `platform` is drawn on now: its system's shelf's, and the
    /// list's in view when it is on it.
    pub fn faces_of_cart(&self, platform: Platform, stem: &str) -> Vec<TexId> {
        let own = self.face_of(platform, stem);
        let listed = self.list_shelf.as_ref().and_then(|l| {
            let i = l
                .shelf
                .carts
                .iter()
                .position(|c| c.platform == platform && c.stem == stem)?;
            l.shelf.find(&l.shelf.carts[i].stem)?.1
        });
        own.into_iter().chain(listed).collect()
    }
}
