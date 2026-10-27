//! The menu's theme: the main menu's and the grid of systems' pictures, font, colours and
//! layout, from the card's Menu folder where it has them, else from the look built in
//! (spruce.rs). A picture or a setting is looked for first in the folder for the panel's size,
//! Menu/640x480, Menu/720x480 or Menu/720x720, then in Menu itself; anything not found there
//! is the built-in one. Read once, as the menu's pictures are first made: a theme changed on
//! the card shows from the next start. System/menu-template holds a theme to start from, and
//! its README.md says what each file and setting is.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};
use omega_store::Platform;

use crate::screens::home::Entry;
use crate::spruce::{self, Picture};

/// The theme's folder on the card.
pub const MENU_DIR: &str = "Menu";
/// Its settings, beside its pictures.
pub const MENU_INI: &str = "menu.ini";

/// A system's icon's file name in the theme, as spruce names them.
pub fn icon_file(platform: Platform) -> &'static str {
    match platform {
        Platform::Gba => "gba",
        Platform::Gb => "gb",
        Platform::Gbc => "gbc",
        Platform::Nes => "fc",
        Platform::Snes => "sfc",
        Platform::Md => "md",
        Platform::Ps1 => "ps",
        Platform::Nds => "nds",
        Platform::Pico8 => "pico",
        Platform::Ports => "ports",
    }
}

/// A main menu entry's icon's file name in the theme, as spruce names them.
pub fn entry_file(entry: Entry) -> &'static str {
    match entry {
        Entry::Recents => "recent",
        Entry::Favorites => "favorite",
        Entry::Collections => "collection",
        Entry::Games => "game",
        Entry::Settings => "setting",
    }
}

/// The menu's look for one canvas.
pub struct MenuTheme {
    /// Drawn over the whole screen behind the menu; none leaves omega-zero's own background.
    pub background: Option<Picture>,
    /// Drawn behind the system picked on the grid; none draws nothing behind it.
    pub frame: Option<Picture>,
    pub grid_cols: u32,
    pub grid_rows: u32,
    /// The size names are written at, on the grid and on the main menu, in screen pixels.
    pub grid_text_px: f32,
    pub home_text_px: f32,
    pub text: [u8; 3],
    pub text_lit: [u8; 3],
    /// Whether the grid and the main menu write each one's name under its icon.
    pub names: bool,
    /// Each system's icon and lit icon.
    pub icons: Vec<(Platform, Picture, Picture)>,
    /// Each main menu entry's icon and lit icon.
    pub entries: Vec<(Entry, Picture, Picture)>,
    font: Option<Font>,
}

/// Where the theme's files for a canvas `w` by `h` are looked for, the panel's own folder first.
fn folders(root: &Path, w: u32, h: u32) -> [PathBuf; 2] {
    let menu = root.join(MENU_DIR);
    [menu.join(format!("{w}x{h}")), menu]
}

fn find(folders: &[PathBuf], name: &str) -> Option<Vec<u8>> {
    folders
        .iter()
        .find_map(|d| std::fs::read(d.join(name)).ok())
}

/// The theme's settings, the panel's folder's over Menu's own.
fn settings(folders: &[PathBuf]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for dir in folders.iter().rev() {
        if let Ok(text) = std::fs::read_to_string(dir.join(MENU_INI)) {
            out.extend(omega_store::ini::parse(&text));
        }
    }
    out
}

/// A colour written `#RRGGBB` or `RRGGBB`.
pub fn colour(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn yes(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "on" | "true" | "1" => Some(true),
        "no" | "off" | "false" | "0" => Some(false),
        _ => None,
    }
}

impl MenuTheme {
    /// The theme for a canvas `w` by `h`, from the card at `root` where it has one.
    pub fn load(root: Option<&Path>, w: u32, h: u32) -> MenuTheme {
        let folders: Vec<PathBuf> = root.map(|r| folders(r, w, h).to_vec()).unwrap_or_default();
        let set = spruce::set_for(w, h);
        let ini = settings(&folders);
        let get = |key: &str| ini.get(key).map(String::as_str);
        let number = |key: &str, default: u32, max: u32| {
            get(key)
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|n| (1..=max).contains(n))
                .unwrap_or(default)
        };
        let size = |key: &str, default: f32| {
            get(key)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|n| (6.0..=96.0).contains(n))
                .unwrap_or(default)
        };
        let picture = |name: &str, default: Option<&[u8]>| -> Option<Picture> {
            match find(&folders, name) {
                Some(bytes) => spruce::decode(&bytes).or_else(|| {
                    eprintln!("omega-zero: menu theme: {name} is not a PNG it can read");
                    default.and_then(spruce::decode)
                }),
                None => default.and_then(spruce::decode),
            }
        };
        let pair = |plain: String, lit: String, defaults: (&[u8], &[u8])| {
            let plain = picture(&plain, Some(defaults.0));
            let lit = picture(&lit, Some(defaults.1)).or_else(|| plain.clone());
            Some((plain?, lit?))
        };
        let icons = Platform::ALL
            .into_iter()
            .filter_map(|p| {
                let file = icon_file(p);
                let (plain, lit) = pair(
                    format!("icons/{file}.png"),
                    format!("icons/sel/{file}.png"),
                    set.icon(p),
                )?;
                Some((p, plain, lit))
            })
            .collect();
        let entries = Entry::ALL
            .into_iter()
            .filter_map(|e| {
                let file = entry_file(e);
                let (plain, lit) = pair(
                    format!("ic-{file}-n.png"),
                    format!("ic-{file}-f.png"),
                    crate::screens::home::home_icon(e, w, h),
                )?;
                Some((e, plain, lit))
            })
            .collect();
        let frame = match get("frame").map(str::trim) {
            Some("none") => None,
            _ => picture("frame.png", Some(set.frame)),
        };
        let font = find(&folders, "font.ttf").and_then(|bytes| {
            let font = Font::from_bytes(bytes, FontSettings::default());
            if let Err(e) = &font {
                eprintln!("omega-zero: menu theme: font.ttf: {e}");
            }
            font.ok()
        });
        MenuTheme {
            background: picture("background.png", None),
            frame,
            grid_cols: number("grid_columns", set.cols, 8),
            grid_rows: number("grid_rows", set.rows, 6),
            grid_text_px: size("grid_text_size", set.text_px),
            home_text_px: size("home_text_size", crate::screens::home::home_text_px(h)),
            text: get("text_colour").and_then(colour).unwrap_or(spruce::TEXT),
            text_lit: get("text_lit_colour")
                .and_then(colour)
                .unwrap_or(spruce::TEXT_LIT),
            names: get("names").and_then(yes).unwrap_or(true),
            icons,
            entries,
            font,
        }
    }

    /// `text` in the theme's font at `px`, in `colour`.
    pub fn text(&self, text: &str, px: f32, colour: [u8; 3]) -> Picture {
        match &self.font {
            Some(font) => spruce::text_in(font, text, px, colour),
            None => spruce::text(text, px, colour),
        }
    }

    /// How tall a line of the theme's font is at `px`.
    pub fn line_height(&self, px: f32) -> u32 {
        match &self.font {
            Some(font) => spruce::line_height_in(font, px),
            None => spruce::line_height(px),
        }
    }
}
