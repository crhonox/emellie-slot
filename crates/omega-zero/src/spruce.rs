//! The menu's look when the card has none of its own (menu_theme.rs): from spruceOS's SPRUCE
//! theme (licenses/spruce-theme.txt), each system's icon and lit icon, the frame behind the one
//! picked, and its font, cut down to Latin letters, over omega-zero's own background. Each
//! panel size has its own set, as the theme has; the grid is laid out the way spruce's PyUI
//! lays its system grid out.

use std::sync::OnceLock;

use fontdue::{Font, FontSettings};
use omega_store::Platform;

/// The pictures for one panel size, as PNG bytes.
pub struct Set {
    pub frame: &'static [u8],
    /// The grid's columns and rows.
    pub cols: u32,
    pub rows: u32,
    /// The size the names under the icons are written at, in screen pixels.
    pub text_px: f32,
    icons: fn(Platform) -> (&'static [u8], &'static [u8]),
}

impl Set {
    /// `platform`'s icon, and the one lit for when it is picked.
    pub fn icon(&self, platform: Platform) -> (&'static [u8], &'static [u8]) {
        (self.icons)(platform)
    }
}

macro_rules! icons {
    ($dir:literal) => {
        |p: Platform| match p {
            Platform::Gba => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/gba.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/gba.png"
                ))[..],
            ),
            Platform::Gb => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/gb.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/gb.png"
                ))[..],
            ),
            Platform::Gbc => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/gbc.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/gbc.png"
                ))[..],
            ),
            Platform::Nes => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/fc.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/fc.png"
                ))[..],
            ),
            Platform::Snes => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/sfc.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/sfc.png"
                ))[..],
            ),
            Platform::Md => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/md.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/md.png"
                ))[..],
            ),
            Platform::Ps1 => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/ps.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/ps.png"
                ))[..],
            ),
            Platform::Nds => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/nds.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/nds.png"
                ))[..],
            ),
            Platform::Pico8 => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/pico.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/pico.png"
                ))[..],
            ),
            Platform::Ports => (
                &include_bytes!(concat!("../../../assets/spruce/", $dir, "/icons/ports.png"))[..],
                &include_bytes!(concat!(
                    "../../../assets/spruce/",
                    $dir,
                    "/icons/sel/ports.png"
                ))[..],
            ),
        }
    };
}

/// The 4:3 panels' set, 640x480, and the wide 720x480 one's.
const FOUR_THREE: Set = Set {
    frame: include_bytes!("../../../assets/spruce/640x480/bg-game-item-f.png"),
    cols: 4,
    rows: 2,
    text_px: 18.0,
    icons: icons!("640x480"),
};

const SQUARE: Set = Set {
    frame: include_bytes!("../../../assets/spruce/720x720/bg-game-item-f.png"),
    cols: 4,
    rows: 3,
    text_px: 20.0,
    icons: icons!("720x720"),
};

/// The set for a canvas `w` by `h`.
pub fn set_for(w: u32, h: u32) -> &'static Set {
    let _ = w;
    if h >= 600 {
        &SQUARE
    } else {
        &FOUR_THREE
    }
}

/// A system's name under its icon.
pub fn system_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Gba => "GBA",
        Platform::Gb => "Game Boy",
        Platform::Gbc => "GB Color",
        Platform::Nes => "NES",
        Platform::Snes => "SNES",
        Platform::Md => "Mega Drive",
        Platform::Ps1 => "PlayStation",
        Platform::Nds => "DS",
        Platform::Pico8 => "PICO-8",
        Platform::Ports => "Ports",
    }
}

/// The name's colour, and the picked system's.
pub const TEXT: [u8; 3] = [0x7c, 0x6f, 0x64];
pub const TEXT_LIT: [u8; 3] = [0xfb, 0xf1, 0xc7];

/// An RGBA picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// A PNG's pixels as RGBA, whatever its colour type.
pub fn decode(png_bytes: &[u8]) -> Option<Picture> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let px = (info.width * info.height) as usize;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..px * 4].to_vec(),
        png::ColorType::Rgb => buf[..px * 3]
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf[..px * 2]
            .chunks_exact(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => buf[..px].iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Picture {
        w: info.width,
        h: info.height,
        rgba,
    })
}

const FONT_TTF: &[u8] = include_bytes!("../../../assets/spruce/nunwen-latin.ttf");

/// The theme's font when the card gives none.
pub fn font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::from_bytes(FONT_TTF, FontSettings::default()).ok())
        .as_ref()
}

/// How tall a line of the theme's font is at `px`, as PyUI makes room for a name.
pub fn line_height(px: f32) -> u32 {
    font().map_or(px.ceil() as u32, |f| line_height_in(f, px))
}

/// How tall a line of `font` is at `px`.
pub fn line_height_in(font: &Font, px: f32) -> u32 {
    font.horizontal_line_metrics(px)
        .map_or(px.ceil() as u32, |m| m.new_line_size.ceil() as u32)
}

/// `text` written in the theme's font at `px`, in `colour`, on a clear picture a line tall.
pub fn text(text: &str, px: f32, colour: [u8; 3]) -> Picture {
    match font() {
        Some(font) => text_in(font, text, px, colour),
        None => Picture {
            w: 1,
            h: 1,
            rgba: vec![0; 4],
        },
    }
}

/// `text` written in `font` at `px`, in `colour`, on a clear picture a line tall.
pub fn text_in(font: &Font, text: &str, px: f32, colour: [u8; 3]) -> Picture {
    let line = font.horizontal_line_metrics(px);
    let ascent = line.map_or(px, |m| m.ascent);
    let h = line_height_in(font, px).max(1);
    let glyphs: Vec<_> = text.chars().map(|c| font.rasterize(c, px)).collect();
    let w = glyphs
        .iter()
        .map(|(m, _)| m.advance_width)
        .sum::<f32>()
        .ceil()
        .max(1.0) as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut pen = 0.0f32;
    for (m, coverage) in &glyphs {
        let left = (pen + m.xmin as f32).round() as i32;
        let top = (ascent - m.ymin as f32 - m.height as f32).round() as i32;
        for gy in 0..m.height {
            for gx in 0..m.width {
                let (x, y) = (left + gx as i32, top + gy as i32);
                if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                    continue;
                }
                let a = coverage[gy * m.width + gx];
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                if a > rgba[i + 3] {
                    rgba[i..i + 4].copy_from_slice(&[colour[0], colour[1], colour[2], a]);
                }
            }
        }
        pen += m.advance_width;
    }
    Picture { w, h, rgba }
}

/// Where a cell of the grid stands on the screen: the middle of its icon and frame, and the
/// bottom middle of its name, in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub x: f32,
    pub icon_y: f32,
    pub frame_y: f32,
    pub text_bottom: f32,
}

/// Cell `i` of a page of a `cols` by `rows` grid over a screen `w` wide between `top` and
/// `bottom`, names a line `text_h` tall: as PyUI's grid view lays it, 10 pixels in from each
/// side, each row its share of the height, the icon raised by its name.
pub fn cell(i: u32, cols: u32, rows: u32, w: f32, top: f32, bottom: f32, text_h: f32) -> Cell {
    let pad = 10.0;
    let col_w = (w - 2.0 * pad) / cols as f32;
    let row_h = (bottom - top) / rows as f32;
    let (col, row) = ((i % cols) as f32, (i / cols % rows) as f32);
    let x = pad + col * col_w + col_w / 2.0;
    let row_bottom = top + (row + 1.0) * row_h;
    let middle = row_bottom - row_h / 2.0;
    Cell {
        x,
        icon_y: middle - text_h / 2.0,
        frame_y: middle,
        text_bottom: row_bottom - text_h / 2.0,
    }
}

/// Where `at` lands moving by `dx` across and `dy` down a grid `cols` wide of `n` cells: across
/// it goes round the whole list, down it keeps its column and stops at the ends.
pub fn step(at: usize, n: usize, cols: usize, dx: i32, dy: i32) -> usize {
    if n == 0 {
        return 0;
    }
    let at = at.min(n - 1) as i64;
    if dx != 0 {
        return (at + dx as i64).rem_euclid(n as i64) as usize;
    }
    let to = at + dy as i64 * cols as i64;
    if (0..n as i64).contains(&to) {
        to as usize
    } else if dy > 0 && at / cols as i64 != (n as i64 - 1) / cols as i64 {
        // down onto a last row that stops short of this column: its last cell
        n - 1
    } else {
        at as usize
    }
}
