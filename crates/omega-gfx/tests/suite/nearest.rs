//! Pixel art is uploaded once with nearest filtering and its pixels replaced in place after:
//! what it is drawn with has to stay sharp through both.

use omega_gfx::{out_w, Compositor, Draw, HeadlessSurface};
use std::sync::{Mutex, MutexGuard, PoisonError};

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

fn px(frame: &[u8], x: usize, y: usize) -> [u8; 3] {
    let o = (y * out_w() as usize + x) * 4;
    [frame[o], frame[o + 1], frame[o + 2]]
}

/// Two texels side by side, `a` then `b`, drawn at 4x: 8 by 4 pixels from (100, 100).
fn drawn_at_4x(c: &mut Compositor, tex: omega_gfx::TexId) -> Vec<u8> {
    c.begin_frame();
    c.draw_list(&[Draw::Tex {
        x: 100.0,
        y: 100.0,
        w: 8.0,
        h: 4.0,
        tex,
        alpha: 1.0,
    }]);
    c.read_frame()
}

#[test]
fn a_nearest_texture_stays_nearest_when_its_pixels_are_replaced() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let red_blue = [255, 0, 0, 255, 0, 0, 255, 255];
    let tex = c.create_texture_nearest(2, 1, &red_blue);
    let frame = drawn_at_4x(&mut c, tex);
    assert_eq!(px(&frame, 103, 101), [255, 0, 0], "the first texel blurred");
    assert_eq!(
        px(&frame, 104, 101),
        [0, 0, 255],
        "the second texel blurred"
    );

    let green_white = [0, 255, 0, 255, 255, 255, 255, 255];
    c.update_texture(tex, 2, 1, &green_white);
    let frame = drawn_at_4x(&mut c, tex);
    assert_eq!(px(&frame, 103, 101), [0, 255, 0], "the update blurred");
    assert_eq!(px(&frame, 104, 101), [255, 255, 255], "the update blurred");
}
