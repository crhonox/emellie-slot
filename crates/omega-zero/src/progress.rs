//! A progress screen drawn straight to the panel while omega-zero is not running,
//! `omega-zero --progress CARD TITLE`: `System/ports.sh` pipes 7-Zip's progress (`-bsp1`) into
//! it while it lays PortMaster out on the card, the first time a minute or more. Each `NN%` read
//! is drawn as a bar under TITLE, in the menus' pixel letters and in the colours of the
//! backdrop the player picked; it leaves when its input ends.
//!
//! It writes the framebuffer itself, every buffer of it, so whichever one the panel shows when
//! omega-zero let go carries the screen. No GL: nothing else holds the panel then.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use omega_store::read_app_state;
use omega_ui::kit::palette::{palette, Palette};
use omega_ui::kit::pixel;
use omega_ui::UndoFace;

const FB_SYS: &str = "/sys/class/graphics/fb0";
const FB_DEV: &str = "/dev/fb0";

/// How many panel pixels an art pixel takes, by the panel's width: 4 on the 640 and 720 panels.
const ART_W: u32 = 160;

/// The percentages in 7-Zip's progress, which it rewrites in place with backspaces: each run of
/// figures that a `%` closes.
#[derive(Default)]
pub struct Percents {
    figures: Option<u32>,
}

impl Percents {
    /// The percentages `bytes` finish, in order, at most 100.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for &b in bytes {
            match b {
                b'0'..=b'9' => {
                    let n = self.figures.unwrap_or(0);
                    self.figures = Some(n.saturating_mul(10).saturating_add(u32::from(b - b'0')));
                }
                b'%' => {
                    if let Some(n) = self.figures.take() {
                        out.push(n.min(100) as u8);
                    }
                }
                _ => self.figures = None,
            }
        }
        out
    }
}

/// The screen, `w` by `h` art pixels: TITLE in the middle, the bar under it filled to `percent`,
/// the figure under the bar.
pub fn screen(w: u32, h: u32, title: &str, percent: u8, pal: &Palette) -> UndoFace {
    let mut face = pixel::blank(w, h);
    pixel::fill(&mut face, [0, 0, w as i32, h as i32], pal.bg);
    let line = pixel::LINE as i32;
    let centre = |text: &str| (w as i32 - pixel::text_width(text) as i32) / 2;
    let mid = h as i32 / 2;

    let title_y = mid - line - 8;
    pixel::ink(
        &mut face,
        &pixel::letters(title, pixel::ADVANCE),
        centre(title),
        title_y,
        pal.cream,
    );

    let bar_w = (w as i32 * 3 / 5).max(20);
    let (bar_x, bar_y, bar_h) = ((w as i32 - bar_w) / 2, mid - 3, 7);
    pixel::fill(&mut face, [bar_x, bar_y, bar_w, bar_h], pal.dark);
    pixel::fill(
        &mut face,
        [bar_x + 1, bar_y + 1, bar_w - 2, bar_h - 2],
        pal.well,
    );
    let inner = (bar_w - 2) * i32::from(percent) / 100;
    pixel::fill(
        &mut face,
        [bar_x + 1, bar_y + 1, inner, bar_h - 2],
        pal.label,
    );

    let figure = format!("{percent}%");
    pixel::ink(
        &mut face,
        &pixel::letters(&figure, pixel::ADVANCE),
        centre(&figure),
        bar_y + bar_h + 5,
        pal.dim,
    );
    face
}

/// The framebuffer as sysfs tells it: the panel's size, the bytes of each line, and how many
/// panel heights the virtual screen holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Fb {
    pub w: u32,
    pub h: u32,
    pub stride: u32,
    pub bpp: u32,
    pub buffers: u32,
}

impl Fb {
    pub fn read(sys: &Path) -> Option<Fb> {
        let attr = |name: &str| std::fs::read_to_string(sys.join(name)).ok();
        let pair = |text: String| -> Option<(u32, u32)> {
            let (a, b) = text.trim().split_once(',')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        };
        let (vw, vh) = pair(attr("virtual_size")?)?;
        let (w, h) = attr("modes")
            .and_then(|m| panel_mode(&m))
            .unwrap_or((vw, vh));
        let bpp: u32 = attr("bits_per_pixel")?.trim().parse().ok()?;
        let stride = attr("stride")
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(vw * bpp / 8);
        Some(Fb {
            w,
            h,
            stride,
            bpp,
            buffers: (vh / h.max(1)).max(1),
        })
    }

    /// One panel height of the framebuffer showing `face` scaled up `scale` times, centred.
    pub fn frame(&self, face: &UndoFace, scale: u32) -> Vec<u8> {
        let bytes = (self.bpp / 8) as usize;
        let mut out = vec![0u8; (self.stride * self.h) as usize];
        let (ox, oy) = (
            (self.w.saturating_sub(face.w * scale)) / 2,
            (self.h.saturating_sub(face.h * scale)) / 2,
        );
        for y in 0..self.h {
            let fy = y.checked_sub(oy).map(|v| v / scale).filter(|v| *v < face.h);
            for x in 0..self.w {
                let fx = x.checked_sub(ox).map(|v| v / scale).filter(|v| *v < face.w);
                let (r, g, b) = match (fx, fy) {
                    (Some(fx), Some(fy)) => {
                        let i = ((fy * face.w + fx) * 4) as usize;
                        (face.rgba[i], face.rgba[i + 1], face.rgba[i + 2])
                    }
                    _ => (0, 0, 0),
                };
                let at = (y * self.stride) as usize + x as usize * bytes;
                match bytes {
                    4 => out[at..at + 4].copy_from_slice(&[b, g, r, 255]),
                    2 => {
                        let v = (u16::from(r >> 3) << 11)
                            | (u16::from(g >> 2) << 5)
                            | u16::from(b >> 3);
                        out[at..at + 2].copy_from_slice(&v.to_le_bytes());
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

/// The panel's size in the first of sysfs' `modes`, as `U:720x720p-60`.
fn panel_mode(text: &str) -> Option<(u32, u32)> {
    let body = text.lines().next()?.rsplit(':').next()?;
    let (w, rest) = body.split_once('x')?;
    let h: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let (w, h) = (w.trim().parse().ok()?, h.parse().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

fn show(fb: &Fb, frame: &[u8]) {
    let Ok(mut dev) = OpenOptions::new().write(true).open(FB_DEV) else {
        return;
    };
    for buffer in 0..fb.buffers {
        let at = u64::from(buffer) * frame.len() as u64;
        if dev.seek(SeekFrom::Start(at)).is_err() || dev.write_all(frame).is_err() {
            return;
        }
    }
}

pub fn run(root: &Path, title: &str) {
    let pal = palette(read_app_state(root).backdrop_colour);
    let fb = Fb::read(Path::new(FB_SYS));
    let scale = fb.map_or(1, |fb| (fb.w / ART_W).max(1));
    let draw = |percent: u8| {
        if let Some(fb) = fb {
            let face = screen(fb.w / scale, fb.h / scale, title, percent, pal);
            show(&fb, &fb.frame(&face, scale));
        }
    };
    draw(0);
    let mut shown = 0;
    let mut percents = Percents::default();
    let mut input = std::io::stdin().lock();
    let mut chunk = [0u8; 512];
    while let Ok(n) = input.read(&mut chunk) {
        if n == 0 {
            break;
        }
        if let Some(&p) = percents.feed(&chunk[..n]).last() {
            if p != shown {
                shown = p;
                draw(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_percentages_are_read_through_7_zips_backspaces() {
        let mut p = Percents::default();
        assert_eq!(
            p.feed(b"  0 M Scan /x/\x08\x08   0%\x08\x08\x08\x08    \x08\x08\x08\x08  7% 822"),
            vec![0, 7]
        );
        // a figure split between two reads
        assert_eq!(p.feed(b"\x08\x08 2"), vec![]);
        assert_eq!(p.feed(b"2% 1262\x08"), vec![22]);
        assert_eq!(p.feed(b"1284 - files"), vec![]);
        assert_eq!(p.feed(b"999%"), vec![100]);
    }

    #[test]
    fn the_bar_fills_with_the_percentage() {
        let pal = palette(0);
        let at = |face: &UndoFace, x: u32, y: u32| {
            let i = ((y * face.w + x) * 4) as usize;
            [face.rgba[i], face.rgba[i + 1], face.rgba[i + 2]]
        };
        let (w, h) = (160, 120);
        let bar_x = (w as i32 - w as i32 * 3 / 5) / 2;
        let y = (h / 2) as u32;
        let empty = screen(w, h, "INSTALLING", 0, pal);
        let half = screen(w, h, "INSTALLING", 50, pal);
        let full = screen(w, h, "INSTALLING", 100, pal);
        let left = bar_x as u32 + 2;
        let right = (bar_x + w as i32 * 3 / 5) as u32 - 3;
        assert_eq!(at(&empty, left, y), pal.well);
        assert_eq!(at(&half, left, y), pal.label);
        assert_eq!(at(&half, right, y), pal.well);
        assert_eq!(at(&full, right, y), pal.label);
        assert_eq!(at(&full, 0, 0), pal.bg);
    }

    #[test]
    fn the_frame_is_the_face_scaled_and_centred_in_the_panel_s_format() {
        let mut face = pixel::blank(2, 1);
        pixel::fill(&mut face, [0, 0, 1, 1], [10, 20, 30]);
        pixel::fill(&mut face, [1, 0, 1, 1], [40, 50, 60]);
        let fb = Fb {
            w: 6,
            h: 4,
            stride: 6 * 4 + 8,
            bpp: 32,
            buffers: 3,
        };
        let frame = fb.frame(&face, 2);
        assert_eq!(frame.len(), (fb.stride * fb.h) as usize);
        let px = |x: usize, y: usize| &frame[y * fb.stride as usize + x * 4..][..4];
        // two art pixels across, each 2x2, from x 1 and y 1
        assert_eq!(px(0, 1), &[0, 0, 0, 255]);
        assert_eq!(px(1, 1), &[30, 20, 10, 255]);
        assert_eq!(px(2, 2), &[30, 20, 10, 255]);
        assert_eq!(px(3, 1), &[60, 50, 40, 255]);
        assert_eq!(px(5, 1), &[0, 0, 0, 255]);
        assert_eq!(px(1, 0), &[0, 0, 0, 255]);
    }

    #[test]
    fn the_framebuffer_is_read_from_sysfs() {
        let dir = tempfile::tempdir().unwrap();
        let put = |n: &str, v: &str| std::fs::write(dir.path().join(n), v).unwrap();
        put("virtual_size", "720,2160\n");
        put("modes", "U:720x720p-60\n");
        put("bits_per_pixel", "32\n");
        put("stride", "2880\n");
        assert_eq!(
            Fb::read(dir.path()),
            Some(Fb {
                w: 720,
                h: 720,
                stride: 2880,
                bpp: 32,
                buffers: 3
            })
        );
    }
}
