//! The DS game card, drawn from a picture of one (assets/nds_cart.png, 489 by 520): the grey
//! plastic round a sticker, NINTENDO DS on its band at the top, a well for the game's art in its
//! middle with a shadow round its edge, and the arrow moulded in the plastic below. The picture
//! is laid over the art, its well clear but for the shadow, so a label only fills the well and
//! the band and the plastic stay the card's own.
//!
//! The real card is 35 by 33 mm and 3.8 mm thick. Its outline is the picture's: corners a
//! little round, and the corner bottom left cut away on a curve, as the card's is so it goes
//! into the slot one way only.

use std::sync::OnceLock;

use crate::cart::{Board, CartFace, Spec};
use crate::silhouette::{depth_map, Detail};

const ART_PNG: &[u8] = include_bytes!("../../assets/nds_cart.png");

/// The picture's size, in its own pixels.
const ART_W: u32 = 489;
const ART_H: u32 = 520;

/// The face's size in pixels: as the picture, at about 7 pixels to the millimetre.
pub const NDS_W: u32 = 252;
pub const NDS_H: u32 = (NDS_W * ART_H + ART_W / 2) / ART_W;

/// How thick the card is, as a share of its width: 3.8 mm on its 35.
pub const THICK: f32 = 3.8 / 35.0;

/// How big the card stands on the shelf, as a share of its own size.
pub const SHELF_SCALE: f32 = 1.0;

/// The well the game's art shows through, in the picture's pixels: under NINTENDO DS's band
/// and over the band at the foot of the sticker.
const WELL: (u32, u32, u32, u32) = (54, 104, 392, 304);

/// The sticker the well is cut in, in the picture's pixels, from the top of its band to the
/// foot of its own: a scan of the whole sticker keeps only the rows the well shows.
const STICKER_TOP: u32 = 36;
const STICKER_FOOT: u32 = 460;

/// The card's outline, in the picture's pixels, clockwise from the top left: its round corners,
/// and the corner cut away bottom left.
const OUTLINE_PX: &[(f32, f32)] = &[
    (9.0, 0.0),
    (476.0, 0.0),
    (484.0, 4.0),
    (488.0, 9.0),
    (488.0, 514.0),
    (486.0, 516.0),
    (483.0, 519.0),
    (34.0, 519.0),
    (21.0, 510.0),
    (11.0, 501.0),
    (5.0, 491.0),
    (0.0, 483.0),
    (0.0, 8.0),
    (4.0, 3.0),
];

/// The outline in drawing units, 240 across as every cart's drawing is, and the drawing's size.
pub(crate) fn outline() -> (Vec<(f32, f32)>, (f32, f32)) {
    let k = 240.0 / ART_W as f32;
    let points = OUTLINE_PX.iter().map(|&(x, y)| (x * k, y * k)).collect();
    (points, (240.0, ART_H as f32 * k))
}

fn sx(x: u32) -> u32 {
    (x * NDS_W + ART_W / 2) / ART_W
}

fn sy(y: u32) -> u32 {
    (y * NDS_H + ART_H / 2) / ART_H
}

/// The well, in the face's pixels: where a label goes.
pub(crate) fn label_rect() -> (u32, u32, u32, u32) {
    let (x, y, w, h) = WELL;
    (sx(x), sy(y), sx(x + w) - sx(x), sy(y + h) - sy(y))
}

/// Which share of a scan's rows its art takes, top to foot: all of a scan of the art alone,
/// and the rows under the band and over the foot of a scan of the whole sticker, which is
/// taller than it is wide where the art alone is half as wide again as it is tall.
pub fn art_rows(sw: u32, sh: u32) -> (f32, f32) {
    if sw == 0 || sh == 0 || (sw as f32 / sh as f32) > 1.1 {
        return (0.0, 1.0);
    }
    let tall = (STICKER_FOOT - STICKER_TOP) as f32;
    let (_, y, _, h) = WELL;
    (
        (y - STICKER_TOP) as f32 / tall,
        (y + h - STICKER_TOP) as f32 / tall,
    )
}

/// The picture at the face's size.
fn art() -> &'static [u8] {
    static ART: OnceLock<Vec<u8>> = OnceLock::new();
    ART.get_or_init(|| {
        crate::art::decode_bytes(ART_PNG)
            .map(|(rgba, w, h)| crate::art::resample(&rgba, w, h, NDS_W, NDS_H))
            .unwrap_or_else(|| vec![0; (NDS_W * NDS_H * 4) as usize])
    })
}

fn mask() -> &'static [u8] {
    static MASK: OnceLock<Vec<u8>> = OnceLock::new();
    MASK.get_or_init(|| art().chunks_exact(4).map(|px| px[3]).collect())
}

fn depth() -> &'static [u8] {
    static DEPTH: OnceLock<Vec<u8>> = OnceLock::new();
    DEPTH.get_or_init(|| depth_map(mask(), NDS_W as usize, NDS_H as usize))
}

fn detail() -> &'static Detail {
    static DETAIL: OnceLock<Detail> = OnceLock::new();
    DETAIL.get_or_init(|| Detail::blank(NDS_W, NDS_H))
}

/// How the card's face is laid out. A clear shell would show its board, small and at its foot.
pub(crate) fn spec() -> Spec {
    let label = label_rect();
    Spec {
        w: NDS_W,
        h: NDS_H,
        label,
        label_round: 0,
        mask: mask(),
        depth: depth(),
        detail: detail(),
        max_h: (label.3 - 2 * crate::cart::PAD) as f32,
        rim: 0,
        board: Board {
            x: (0.12, 0.88),
            top: 0.2,
            pins: (0.15, 0.85),
            contacts_from: 0.9,
            traces_from: 0.8,
        },
    }
}

/// The card's face: `label`, the art the well shows, `label_rect()`'s size, with the picture
/// laid over it.
pub(crate) fn face(label: &[u8]) -> CartFace {
    let (lx, ly, lw, lh) = label_rect();
    let mut rgba = vec![0u8; (NDS_W * NDS_H * 4) as usize];
    for y in 0..lh {
        let from = (y * lw * 4) as usize;
        let to = (((ly + y) * NDS_W + lx) * 4) as usize;
        rgba[to..to + (lw * 4) as usize].copy_from_slice(&label[from..from + (lw * 4) as usize]);
    }
    for (d, s) in rgba.chunks_exact_mut(4).zip(art().chunks_exact(4)) {
        let a = s[3] as u32;
        if a == 0 {
            continue;
        }
        // the picture over the art: its plastic opaque, the well's shadow part of the way
        let under = d[3] as u32 * (255 - a) / 255;
        let out = a + under;
        for c in 0..3 {
            d[c] = ((s[c] as u32 * a + d[c] as u32 * under + out / 2) / out) as u8;
        }
        d[3] = out as u8;
    }
    CartFace {
        rgba,
        w: NDS_W,
        h: NDS_H,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_decodes_at_the_face_size() {
        assert_eq!(art().len(), (NDS_W * NDS_H * 4) as usize);
        // the well's middle shows what is under it, the plastic at the side does not
        let at = |x: u32, y: u32| art()[((y * NDS_W + x) * 4 + 3) as usize];
        let (lx, ly, lw, lh) = label_rect();
        assert!(at(lx + lw / 2, ly + lh / 2) < 64);
        assert_eq!(at(2, NDS_H / 2), 255);
    }

    #[test]
    fn a_whole_sticker_keeps_only_the_rows_of_its_art() {
        assert_eq!(art_rows(400, 300), (0.0, 1.0));
        let (top, foot) = art_rows(386, 424);
        assert!(top > 0.1 && top < 0.2, "{top}");
        assert!(foot > 0.8 && foot < 0.95, "{foot}");
    }

    #[test]
    fn the_outline_spans_the_drawing() {
        let (points, (w, h)) = outline();
        assert_eq!(w, 240.0);
        assert!((h - 255.2).abs() < 0.1);
        assert!(points.iter().all(|&(x, y)| x <= w && y <= h + 0.01));
    }
}
