//! The player's lists of games across systems, kept on the card: Favourites, Recents (the
//! games last put in, newest first) and Collections, each a list the player names. A list is
//! a text file of one game a line, `<SYSTEM>\t<stem>`, the system as its Games/ folder is
//! named; a game since taken off the card is passed over where the list is shown, not dropped
//! from the file, so it comes back with its file.

use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::platform::Platform;

pub const FAVORITES_FILE: &str = "Config/favorites.txt";
pub const RECENTS_FILE: &str = "Config/recents.txt";
pub const COLLECTIONS_DIR: &str = "Config/collections";

/// How many games Recents keeps.
pub const RECENTS_MAX: usize = 20;

/// The longest name a collection may have, in characters.
pub const COLLECTION_NAME_MAX: usize = 40;

/// A game on a list: its system and its file's stem, as the shelf names a cart.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameRef {
    pub platform: Platform,
    pub stem: String,
}

impl GameRef {
    pub fn new(platform: Platform, stem: impl Into<String>) -> GameRef {
        GameRef {
            platform,
            stem: stem.into(),
        }
    }

    fn line(&self) -> String {
        format!("{}\t{}\n", self.platform.dir_name(), self.stem)
    }

    fn parse(line: &str) -> Option<GameRef> {
        let (system, stem) = line.trim_end_matches(['\r', '\n']).split_once('\t')?;
        let platform = Platform::ALL
            .into_iter()
            .find(|p| system.eq_ignore_ascii_case(p.dir_name()))?;
        (!stem.is_empty()).then(|| GameRef::new(platform, stem))
    }
}

/// The games listed in `path`, in its order, each once; none when it is missing.
pub fn read_list(path: &Path) -> Vec<GameRef> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut out: Vec<GameRef> = Vec::new();
    for game in text.lines().filter_map(GameRef::parse) {
        if !out.contains(&game) {
            out.push(game);
        }
    }
    out
}

/// Writes `games` to `path`, making its folder; an empty list removes the file.
pub fn write_list(path: &Path, games: &[GameRef]) -> std::io::Result<()> {
    if games.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text: String = games.iter().map(GameRef::line).collect();
    atomic_write(path, text.as_bytes())
}

pub fn favorites(root: &Path) -> Vec<GameRef> {
    read_list(&root.join(FAVORITES_FILE))
}

/// Adds `game` to the Favourites at the end, or takes it off when it is there; says whether
/// it is a favourite now.
pub fn toggle_favorite(root: &Path, game: &GameRef) -> std::io::Result<bool> {
    let path = root.join(FAVORITES_FILE);
    let mut list = read_list(&path);
    let now = match list.iter().position(|g| g == game) {
        Some(i) => {
            list.remove(i);
            false
        }
        None => {
            list.push(game.clone());
            true
        }
    };
    write_list(&path, &list)?;
    Ok(now)
}

pub fn recents(root: &Path) -> Vec<GameRef> {
    read_list(&root.join(RECENTS_FILE))
}

/// Puts `game` first in Recents, once, keeping the newest [`RECENTS_MAX`].
pub fn push_recent(root: &Path, game: &GameRef) -> std::io::Result<()> {
    let path = root.join(RECENTS_FILE);
    let mut list = read_list(&path);
    if list.first() == Some(game) {
        return Ok(());
    }
    list.retain(|g| g != game);
    list.insert(0, game.clone());
    list.truncate(RECENTS_MAX);
    write_list(&path, &list)
}

/// A collection's name as its file can hold it: no path separators or control characters,
/// trimmed, at most [`COLLECTION_NAME_MAX`] characters; nothing when nothing is left.
pub fn collection_name(name: &str) -> Option<String> {
    let clean: String = name
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .take(COLLECTION_NAME_MAX)
        .collect();
    let clean = clean.trim().trim_start_matches('.').trim().to_string();
    (!clean.is_empty()).then_some(clean)
}

fn collection_path(root: &Path, name: &str) -> Option<PathBuf> {
    let name = collection_name(name)?;
    Some(root.join(COLLECTIONS_DIR).join(format!("{name}.txt")))
}

/// The collections on the card, by name, in alphabetical order whatever the case.
pub fn collections(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(COLLECTIONS_DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "txt") {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .filter(|n| !n.starts_with('.'))
            } else {
                None
            }
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

pub fn collection(root: &Path, name: &str) -> Vec<GameRef> {
    collection_path(root, name)
        .map(|p| read_list(&p))
        .unwrap_or_default()
}

/// Adds `game` to the collection `name`, making it when there is none; says the name the
/// collection was kept under.
pub fn add_to_collection(root: &Path, name: &str, game: &GameRef) -> std::io::Result<String> {
    let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidInput, "no collection name");
    let clean = collection_name(name).ok_or_else(invalid)?;
    let path = collection_path(root, &clean).ok_or_else(invalid)?;
    let mut list = read_list(&path);
    if !list.contains(game) {
        list.push(game.clone());
    }
    write_list(&path, &list)?;
    Ok(clean)
}

/// Takes `game` off the collection `name`; the collection is gone once it is empty.
pub fn remove_from_collection(root: &Path, name: &str, game: &GameRef) -> std::io::Result<()> {
    let Some(path) = collection_path(root, name) else {
        return Ok(());
    };
    let mut list = read_list(&path);
    list.retain(|g| g != game);
    write_list(&path, &list)
}
