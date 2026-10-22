use omega_gfx::{out_h, out_w, Bezel, Compositor, Draw, HeadlessSurface, TexId};
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

const FILL: [f32; 4] = [0.2, 0.4, 0.6, 1.0];
const FILL_RGB: [u8; 3] = [51, 102, 153];

fn band() -> [f32; 4] {
    [0.0, out_h() as f32 - 40.0, out_w() as f32, 40.0]
}

fn bezel(mark: TexId) -> Bezel {
    Bezel {
        bands: vec![band()],
        fill: FILL,
        rims: vec![[0.0, out_h() as f32 - 40.0, out_w() as f32, 4.0]],
        marks: vec![(mark, [10.0, out_h() as f32 - 30.0, 20.0, 20.0])],
        overlay: None,
    }
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    (0..3).all(|i| a[i].abs_diff(b[i]) <= 1)
}

#[test]
fn the_body_is_painted_over_the_bands_the_picture_leaves() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let mark = c.create_texture(1, 1, &[255, 255, 255, 255]);
    c.set_bezel(Some(bezel(mark)));
    c.begin_frame();
    c.draw_list(&[Draw::Game]);
    let frame = c.read_frame();
    let bottom = out_h() as usize - 5;
    assert!(near(px(&frame, out_w() as usize - 5, bottom), FILL_RGB));
    assert_eq!(
        px(&frame, 15, out_h() as usize - 20),
        [255, 255, 255],
        "the mark is not printed on the band"
    );
    assert_eq!(
        px(&frame, 100, out_h() as usize - 38),
        [0, 0, 0],
        "the screen has no rim where it meets the band"
    );
}

#[test]
fn a_screenshot_gets_the_bands_without_the_mark() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let mark = c.create_texture(1, 1, &[255, 255, 255, 255]);
    let shot = c.create_texture(1, 1, &[0, 0, 0, 255]);
    c.set_bezel(Some(bezel(mark)));
    c.begin_frame();
    c.draw_list(&[Draw::Shot { tex: shot }]);
    let frame = c.read_frame();
    assert!(near(px(&frame, 15, out_h() as usize - 20), FILL_RGB));
    assert_eq!(
        px(&frame, 100, out_h() as usize - 38),
        [0, 0, 0],
        "a screenshot's screen has no rim"
    );
}

#[test]
fn without_a_bezel_nothing_is_drawn_over_the_picture() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let grey = vec![0x80u8; (omega_gfx::SRC_W * omega_gfx::SRC_H * 4) as usize];
    c.upload_game(&grey);
    c.begin_frame();
    c.draw_game();
    let plain = c.read_frame();
    c.set_bezel(None);
    c.begin_frame();
    c.draw_list(&[Draw::Game]);
    assert!(c.read_frame() == plain);
}
