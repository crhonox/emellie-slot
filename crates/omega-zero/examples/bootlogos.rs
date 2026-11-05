//! Lays the slot mark from one boot logo out again on every panel size slot runs on.
//!
//! `bootlogos SRC OUT_DIR` reads SRC, a 24-bit bmp on a background that is flat or a pattern
//! repeating every two pixels both ways (as its top left 2x2 has it), and writes
//! OUT_DIR/bootlogo-<w>x<h>.bmp for each panel with the mark centred at its original size, on the
//! same background. The panel SRC was drawn for gets SRC itself, untouched.

use std::path::{Path, PathBuf};

use omega_zero::bootlogo::dims;

const PANELS: [(u32, u32); 3] = [(640, 480), (720, 480), (720, 720)];

const HEADER: usize = 54;

struct Bmp {
    w: usize,
    h: usize,
    rows: Vec<Vec<[u8; 3]>>,
}

impl Bmp {
    fn read(path: &Path) -> Bmp {
        let bytes = std::fs::read(path).expect("read the source logo");
        let (w, h) = dims(&bytes).expect("the source is not an uncompressed 24-bit bmp");
        let (w, h) = (w as usize, h as usize);
        let stride = (w * 3).next_multiple_of(4);
        let rows = (0..h)
            .map(|y| {
                let row = &bytes[HEADER + (h - 1 - y) * stride..];
                (0..w)
                    .map(|x| [row[x * 3], row[x * 3 + 1], row[x * 3 + 2]])
                    .collect()
            })
            .collect();
        Bmp { w, h, rows }
    }

    /// A bmp `w` by `h` with `tile` repeated over it.
    fn blank(w: usize, h: usize, tile: [[[u8; 3]; 2]; 2]) -> Bmp {
        Bmp {
            w,
            h,
            rows: (0..h)
                .map(|y| (0..w).map(|x| tile[y % 2][x % 2]).collect())
                .collect(),
        }
    }

    fn write(&self, path: &Path) {
        let stride = (self.w * 3).next_multiple_of(4);
        let size = HEADER + stride * self.h;
        let mut b = vec![0u8; HEADER];
        b[0..2].copy_from_slice(b"BM");
        b[2..6].copy_from_slice(&(size as u32).to_le_bytes());
        b[10..14].copy_from_slice(&(HEADER as u32).to_le_bytes());
        b[14..18].copy_from_slice(&40u32.to_le_bytes());
        b[18..22].copy_from_slice(&(self.w as u32).to_le_bytes());
        b[22..26].copy_from_slice(&(self.h as u32).to_le_bytes());
        b[26..28].copy_from_slice(&1u16.to_le_bytes());
        b[28..30].copy_from_slice(&24u16.to_le_bytes());
        b[34..38].copy_from_slice(&((stride * self.h) as u32).to_le_bytes());
        b[38..42].copy_from_slice(&3780u32.to_le_bytes());
        b[42..46].copy_from_slice(&3780u32.to_le_bytes());
        for row in self.rows.iter().rev() {
            let start = b.len();
            row.iter().for_each(|px| b.extend_from_slice(px));
            b.resize(start + stride, 0);
        }
        std::fs::write(path, b).expect("write the logo");
    }
}

fn main() {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let [src, out] = args.as_slice() else {
        eprintln!("usage: bootlogos SRC OUT_DIR");
        std::process::exit(2);
    };
    let logo = Bmp::read(src);
    let tile = [
        [logo.rows[0][0], logo.rows[0][1]],
        [logo.rows[1][0], logo.rows[1][1]],
    ];
    let inked = |x: usize, y: usize| logo.rows[y][x] != tile[y % 2][x % 2];
    let xs: Vec<usize> = (0..logo.w)
        .filter(|&x| (0..logo.h).any(|y| inked(x, y)))
        .collect();
    let ys: Vec<usize> = (0..logo.h)
        .filter(|&y| (0..logo.w).any(|x| inked(x, y)))
        .collect();
    let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) =
        (xs.first(), xs.last(), ys.first(), ys.last())
    else {
        panic!("the source logo is all background");
    };
    let (mark_w, mark_h) = (x1 - x0 + 1, y1 - y0 + 1);
    eprintln!("bootlogos: a {mark_w}x{mark_h} mark at ({x0}, {y0})");
    for (w, h) in PANELS {
        let (w, h) = (w as usize, h as usize);
        let path = out.join(format!("bootlogo-{w}x{h}.bmp"));
        if (w, h) == (logo.w, logo.h) {
            let same = std::fs::canonicalize(&path).ok() == std::fs::canonicalize(src).ok();
            if !same {
                std::fs::copy(src, &path).expect("copy the source logo");
            }
            eprintln!("bootlogos: {} is the source, as drawn", path.display());
            continue;
        }
        assert!(mark_w <= w && mark_h <= h, "the mark does not fit {w}x{h}");
        let mut panel = Bmp::blank(w, h, tile);
        // as odd or even as in SRC, so the mark keeps its place in the pattern
        let even = |n: usize| n - n % 2;
        let (left, top) = (
            even((w - mark_w) / 2) + x0 % 2,
            even((h - mark_h) / 2) + y0 % 2,
        );
        for y in 0..mark_h {
            panel.rows[top + y][left..left + mark_w]
                .copy_from_slice(&logo.rows[y0 + y][x0..x0 + mark_w]);
        }
        panel.write(&path);
        eprintln!("bootlogos: wrote {}", path.display());
    }
}
