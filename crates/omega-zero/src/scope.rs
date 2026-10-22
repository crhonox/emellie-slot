//! Which carts a ScreenScraper fetch or a RetroAchievements caching goes through: every cart,
//! one shelf's, or the one picked on the shelf. Left and right step through the choices the
//! card has, which the screens' owner lists, since only it knows the shelves.

use omega_store::{Cart, Platform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    All,
    Folder(Platform),
    /// The cart picked on the shelf when Settings was opened.
    Game,
}

impl Scope {
    /// What the row says the scope is.
    pub fn name(self) -> &'static str {
        match self {
            Scope::All => "All",
            Scope::Folder(p) => p.dir_name(),
            Scope::Game => "This game",
        }
    }

    /// Whether `cart` is in this scope, `picked` being the cart picked on the shelf.
    pub fn holds(self, cart: &Cart, picked: Option<&Cart>) -> bool {
        match self {
            Scope::All => true,
            Scope::Folder(p) => cart.platform == p,
            Scope::Game => {
                picked.is_some_and(|c| c.platform == cart.platform && c.stem == cart.stem)
            }
        }
    }
}

/// The choices for carts of the platforms `takes`: All, each shelf with such carts, then the
/// picked cart when it is one.
pub fn choices<'a>(
    shelves: impl IntoIterator<Item = (Platform, &'a [Cart])>,
    picked: Option<&Cart>,
    takes: impl Fn(Platform) -> bool,
) -> Vec<Scope> {
    let mut out = vec![Scope::All];
    for (platform, carts) in shelves {
        if takes(platform) && !carts.is_empty() && !out.contains(&Scope::Folder(platform)) {
            out.push(Scope::Folder(platform));
        }
    }
    if picked.is_some_and(|c| takes(c.platform)) {
        out.push(Scope::Game);
    }
    out
}

/// The choice `by` steps from `at` among `choices`, round and round.
pub fn step(choices: &[Scope], at: Scope, by: i32) -> Scope {
    let n = choices.len() as i32;
    if n == 0 {
        return at;
    }
    let i = choices.iter().position(|s| *s == at).unwrap_or(0) as i32;
    choices[(i + by).rem_euclid(n) as usize]
}
