use std::path::Path;

pub fn cover(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    cover_rows(path, w, h, |_, _| (0.0, 1.0))
}

/// As [`cover`], of only the rows `rows` gives as a share of the picture's height, top to foot,
/// for its size: a DS label scan of the whole sticker keeps the rows of its art.
pub fn cover_rows(
    path: &Path,
    w: u32,
    h: u32,
    rows: impl Fn(u32, u32) -> (f32, f32),
) -> Option<Vec<u8>> {
    let (src, sw, full_h) = decode(path)?;
    if sw == 0 || full_h == 0 {
        return None;
    }
    let (top, foot) = rows(sw, full_h);
    let first = ((top.clamp(0.0, 1.0) * full_h as f32) as u32).min(full_h - 1);
    let last = ((foot.clamp(0.0, 1.0) * full_h as f32).ceil() as u32).clamp(first + 1, full_h);
    let row_bytes = (sw * 4) as usize;
    let src = &src[first as usize * row_bytes..last as usize * row_bytes];
    let sh = last - first;

    let scale = (w as f32 / sw as f32).max(h as f32 / sh as f32);
    let ox = (sw as f32 - w as f32 / scale) / 2.0;
    let oy = (sh as f32 - h as f32 / scale) / 2.0;

    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        let y0 = oy + y as f32 / scale;
        let y1 = oy + (y + 1) as f32 / scale;
        for x in 0..w {
            let x0 = ox + x as f32 / scale;
            let x1 = ox + (x + 1) as f32 / scale;
            let px = box_average(src, sw, sh, x0, y0, x1, y1);
            out[((y * w + x) * 4) as usize..][..4].copy_from_slice(&px);
        }
    }
    Some(out)
}

/// The png at `path` as RGBA, only if it is exactly `w` by `h`.
/// `src`, `sw` by `sh` RGBA, at `w` by `h`: each pixel the average of the ones it covers.
pub fn resample(src: &[u8], sw: u32, sh: u32, w: u32, h: u32) -> Vec<u8> {
    let (kx, ky) = (sw as f32 / w as f32, sh as f32 / h as f32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let px = box_average(
                src,
                sw,
                sh,
                x as f32 * kx,
                y as f32 * ky,
                (x + 1) as f32 * kx,
                (y + 1) as f32 * ky,
            );
            out[((y * w + x) * 4) as usize..][..4].copy_from_slice(&px);
        }
    }
    out
}

pub fn exact(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    let (rgba, sw, sh) = decode(path)?;
    ((sw, sh) == (w, h)).then_some(rgba)
}

/// How big the png at `path` is, read from its header alone.
pub fn size(path: &Path) -> Option<(u32, u32)> {
    let file = std::fs::File::open(path).ok()?;
    let reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .ok()?;
    Some((reader.info().width, reader.info().height))
}

/// `rgba`, `w` by `h`, as a png: without its alpha when every pixel is opaque, as a cart's label
/// is, and each row filtered as suits it. Compressing harder saves under one byte in a hundred
/// on a label and takes a third as long again.
pub fn encode(rgba: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let opaque = rgba.chunks_exact(4).all(|px| px[3] == 255);
    let rgb: Vec<u8>;
    let (colour, data) = if opaque {
        rgb = rgba
            .chunks_exact(4)
            .flat_map(|px| &px[..3])
            .copied()
            .collect();
        (png::ColorType::Rgb, &rgb[..])
    } else {
        (png::ColorType::Rgba, rgba)
    };
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(colour);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
    let mut writer = enc.write_header().ok()?;
    writer.write_image_data(data).ok()?;
    writer.finish().ok()?;
    Some(out)
}

fn box_average(src: &[u8], sw: u32, sh: u32, x0: f32, y0: f32, x1: f32, y1: f32) -> [u8; 4] {
    let xa = (x0.floor().max(0.0) as u32).min(sw - 1);
    let ya = (y0.floor().max(0.0) as u32).min(sh - 1);
    let xb = ((x1.ceil() as u32).max(xa + 1)).min(sw);
    let yb = ((y1.ceil() as u32).max(ya + 1)).min(sh);

    let mut acc = [0u32; 4];
    let mut n = 0u32;
    for y in ya..yb {
        for x in xa..xb {
            let i = ((y * sw + x) * 4) as usize;
            for c in 0..4 {
                acc[c] += src[i + c] as u32;
            }
            n += 1;
        }
    }
    let mut out = [0u8; 4];
    for c in 0..4 {
        out[c] = ((acc[c] + n / 2) / n) as u8;
    }
    out
}

/// The png at `path` as RGBA, with its width and height.
pub(crate) fn decode(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    let file = std::fs::File::open(path).ok()?;
    decode_from(std::io::BufReader::new(file))
}

/// A png held in memory as RGBA, with its width and height.
pub(crate) fn decode_bytes(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    decode_from(std::io::Cursor::new(bytes))
}

fn decode_from<R: std::io::BufRead + std::io::Seek>(input: R) -> Option<(Vec<u8>, u32, u32)> {
    let mut dec = png::Decoder::new(input);
    dec.set_transformations(png::Transformations::normalize_to_color8());
    dec.set_limits(png::Limits { bytes: 64 << 20 });

    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let src = &buf[..info.buffer_size()];
    let n = (info.width * info.height) as usize;

    let mut rgba = vec![255u8; n * 4];
    match info.color_type {
        png::ColorType::Rgba => rgba.copy_from_slice(src),
        png::ColorType::Rgb => {
            for i in 0..n {
                rgba[i * 4..i * 4 + 3].copy_from_slice(&src[i * 3..i * 3 + 3]);
            }
        }
        png::ColorType::Grayscale => {
            for i in 0..n {
                rgba[i * 4..i * 4 + 3].fill(src[i]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for i in 0..n {
                rgba[i * 4..i * 4 + 3].fill(src[i * 2]);
                rgba[i * 4 + 3] = src[i * 2 + 1];
            }
        }
        png::ColorType::Indexed => return None,
    }
    Some((rgba, info.width, info.height))
}
