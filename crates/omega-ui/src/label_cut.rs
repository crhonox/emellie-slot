//! A cart's label scans cut once to the size its face shows them at, and kept on the card beside
//! them. Decoding hundreds of whole scans held a boot on the boot logo long enough to look frozen;
//! the cuts are small and a face drawn from them is ready at once.
//!
//! Once cut, a scan is shrunk to the smallest of its own shape that still covers every face its
//! system draws it on: a face is never drawn bigger than a texel to a pixel, so what a scan holds
//! beyond that never reaches the screen, and a shrunk scan takes a fraction of the card.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use omega_store::{Cart, Platform};

use crate::art;
use crate::cart::{face_labels, label_scans};

/// The folder beside a system's label scans that their cuts are kept in.
pub const CUT_DIR: &str = ".cut";

/// Where the cut of the label scan at `scan` is kept.
pub fn cut_path(scan: &Path) -> Option<PathBuf> {
    Some(scan.parent()?.join(CUT_DIR).join(scan.file_name()?))
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The cut kept for `scan`, when it is `w` by `h` and was cut from the scan there now: a cut
/// carries its scan's time, so a scan put in its place, older or newer, has it cut again.
fn kept(scan: &Path, w: u32, h: u32) -> Option<PathBuf> {
    let cut = cut_path(scan)?;
    let current = modified(&cut).is_some_and(|t| Some(t) == modified(scan));
    (current && art::size(&cut) == Some((w, h))).then_some(cut)
}

/// The scan at `scan` as drawn at `w` by `h`: from its cut when one is kept, from the scan itself
/// when not.
pub(crate) fn label_art(scan: &Path, w: u32, h: u32, platform: Platform) -> Option<Vec<u8>> {
    kept(scan, w, h)
        .and_then(|cut| art::exact(&cut, w, h))
        .or_else(|| cover(scan, w, h, platform))
}

/// The scan at `scan` covering `w` by `h`: a DS scan of the whole sticker only by the rows of
/// its art, the card's own band being drawn over it.
fn cover(scan: &Path, w: u32, h: u32, platform: Platform) -> Option<Vec<u8>> {
    match platform {
        Platform::Nds | Platform::Ports => {
            art::cover_rows(scan, w, h, crate::models::nds::art_rows)
        }
        _ => art::cover(scan, w, h),
    }
}

/// Whether `cart`'s face is drawn with a label scan at all. A cart with no scan, or a PlayStation
/// game none of whose discs has one, wears its generated face whatever else is on the card.
pub fn wears_label(cart: &Cart) -> bool {
    !label_scans(cart).is_empty()
}

/// Whether `cart` has a label scan with no cut of it for its face, so drawing the face would
/// decode the whole scan.
pub fn label_uncut(cart: &Cart) -> bool {
    label_scans(cart)
        .iter()
        .any(|(scan, w, h)| kept(scan, *w, *h).is_none())
}

/// Cuts each of `cart`'s label scans that has no cut for its face and keeps the cuts beside them;
/// true when its face now has a cut to be drawn from.
pub fn cut_label(cart: &Cart) -> bool {
    let mut any = false;
    for (scan, w, h) in label_scans(cart) {
        if kept(&scan, w, h).is_some() || cut(&scan, w, h, cart.platform) {
            any = true;
        }
    }
    any
}

/// Cuts the scan at `scan` to `w` by `h` and keeps the cut, with the scan's time; false when the
/// scan does not decode or the cut could not be kept.
fn cut(scan: &Path, w: u32, h: u32, platform: Platform) -> bool {
    let (Some(cut), Some(scanned)) = (cut_path(scan), modified(scan)) else {
        return false;
    };
    let Some(png) = cover(scan, w, h, platform).and_then(|rgba| art::encode(&rgba, w, h)) else {
        return false;
    };
    match keep_dated(&cut, &png, scanned) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("omega-ui: label cut: {}: {e}", cut.display());
            false
        }
    }
}

/// The size a scan `sw` by `sh` of `platform` is kept at: the smallest of its own shape that
/// still covers each of the system's faces, so none is ever drawn from it stretched. Nothing when
/// the scan is no bigger than that already.
pub fn keep_size(platform: Platform, sw: u32, sh: u32) -> Option<(u32, u32)> {
    if sw == 0 || sh == 0 {
        return None;
    }
    let scale = face_labels(platform)
        .iter()
        .map(|&(w, h)| (w as f64 / sw as f64).max(h as f64 / sh as f64))
        .fold(0.0, f64::max);
    if scale == 0.0 || scale >= 1.0 {
        return None;
    }
    // rounded up, so the face the scale was taken from is covered to its last pixel
    let up = |side: u32| ((side as f64 * scale - 1e-6).ceil() as u32).clamp(1, side);
    let (kw, kh) = (up(sw), up(sh));
    (kw < sw || kh < sh).then_some((kw, kh))
}

/// Shrinks each of `cart`'s label scans that is bigger than [`keep_size`] says, keeping its time
/// so the cut made from it stays current. Run after the scan is cut, so the cut is made from the
/// whole scan. A scan that does not decode, or would come out no smaller, is left as it is.
/// A PlayStation game's own label, a copy of its first disc's that no face is drawn from when
/// that disc has its own, is shrunk with them.
pub fn shrink_label(cart: &Cart) {
    let mut scans: Vec<PathBuf> = label_scans(cart).into_iter().map(|(s, _, _)| s).collect();
    if let Some(own) = cart.label.as_ref().filter(|l| !scans.contains(l)) {
        scans.push(own.clone());
    }
    for scan in scans {
        shrink(&scan, cart.platform);
    }
}

fn shrink(scan: &Path, platform: Platform) {
    let Some((kw, kh)) = art::size(scan).and_then(|(sw, sh)| keep_size(platform, sw, sh)) else {
        return;
    };
    let Ok(meta) = std::fs::metadata(scan) else {
        return;
    };
    let Ok(scanned) = meta.modified() else {
        return;
    };
    let Some(png) = art::cover(scan, kw, kh).and_then(|rgba| art::encode(&rgba, kw, kh)) else {
        return;
    };
    if png.len() as u64 >= meta.len() {
        return;
    }
    if let Err(e) = keep_dated(scan, &png, scanned) {
        eprintln!("omega-ui: label shrink: {}: {e}", scan.display());
    }
}

/// Keeps `png` at `path`, dated `time` before it is there to be read, so no face is drawn from
/// the scan in between; dated again after when the rename did not keep the time.
fn keep_dated(path: &Path, png: &[u8], time: SystemTime) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    omega_store::atomic_write_dated(path, png, time)?;
    if modified(path) == Some(time) {
        return Ok(());
    }
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(time)
}
