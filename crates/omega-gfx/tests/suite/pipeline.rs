use omega_gfx::{
    blue_light_gain, lcd3x_mask, out_h, out_w, spun_x, Compositor, Draw, Filter, GameView,
    HeadlessSurface, MeshVertex, SRC_H, SRC_W,
};
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

fn flat_shot(rgb: [u8; 3]) -> Vec<u8> {
    std::iter::repeat_n([rgb[0], rgb[1], rgb[2], 255], (SRC_W * SRC_H) as usize)
        .flatten()
        .collect()
}

const GB_X: usize = 40;
const GB_Y: usize = 8;
const GB_W: usize = 160;
const GB_H: usize = 144;

const GB_RECT: [f32; 4] = [
    GB_X as f32 / SRC_W as f32,
    GB_Y as f32 / SRC_H as f32,
    GB_W as f32 / SRC_W as f32,
    GB_H as f32 / SRC_H as f32,
];

const WHOLE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// The GBA frame at 3x, centred on the canvas: where every view through the LCD mask lays
/// the game texture, overhanging a canvas narrower than 720.
fn frame_rect() -> [f32; 4] {
    [
        (out_w() as f32 - 720.0) / 2.0,
        (out_h() as f32 - 480.0) / 2.0,
        720.0,
        480.0,
    ]
}

fn at_3x(uv: [f32; 4]) -> GameView {
    GameView {
        rect: frame_rect(),
        uv,
        filter: Filter::Lcd3x,
    }
}

/// The canvas pixel `x`, `y` of the 3x frame lands on, if it is on the canvas at all.
fn on_canvas(x: usize, y: usize) -> Option<(usize, usize)> {
    let r = frame_rect();
    let (cx, cy) = (x as f32 + r[0], y as f32 + r[1]);
    let inside = cx >= 0.0 && cy >= 0.0 && cx < out_w() as f32 && cy < out_h() as f32;
    inside.then_some((cx as usize, cy as usize))
}

fn gb_shaped(inside: impl Fn(usize, usize) -> [u8; 3], margin: [u8; 3]) -> Vec<u8> {
    let mut buf = Vec::with_capacity((SRC_W * SRC_H * 4) as usize);
    for y in 0..SRC_H as usize {
        for x in 0..SRC_W as usize {
            let in_window = (GB_X..GB_X + GB_W).contains(&x) && (GB_Y..GB_Y + GB_H).contains(&y);
            let rgb = match in_window {
                true => inside(x - GB_X, y - GB_Y),
                false => margin,
            };
            buf.extend_from_slice(&[rgb[2], rgb[1], rgb[0], 0]);
        }
    }
    buf
}

fn masked(rgb: [u8; 3], ox: usize, oy: usize) -> [i32; 3] {
    let cell = lcd3x_mask()[oy][ox];
    [0, 1, 2].map(|ch| (rgb[ch] as f32 * cell[ch]).round() as i32)
}

fn period_x(frame: &[u8], xs: std::ops::Range<usize>, ys: std::ops::Range<usize>) -> Option<usize> {
    (1..=8).find(|p| {
        (xs.start..xs.end - p).all(|x| ys.clone().all(|y| px(frame, x, y) == px(frame, x + p, y)))
    })
}

fn period_y(frame: &[u8], xs: std::ops::Range<usize>, ys: std::ops::Range<usize>) -> Option<usize> {
    (1..=8).find(|p| {
        (ys.start..ys.end - p).all(|y| xs.clone().all(|x| px(frame, x, y) == px(frame, x, y + p)))
    })
}

#[test]
fn game_pass_multiplies_every_output_cell_by_the_lcd3x_mask() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let grey = vec![0x80u8; (SRC_W * SRC_H * 4) as usize];
    c.begin_frame();
    c.upload_game(&grey);
    c.draw_game();
    let frame = c.read_frame();

    let mask = lcd3x_mask();
    let mut worst = 0i32;
    for y in 0..out_h() as usize {
        for x in 0..out_w() as usize {
            let got = px(&frame, x, y);
            for (ch, gain) in mask[y % 3][x % 3].iter().enumerate() {
                let want = (128.0 * gain).round() as i32;
                worst = worst.max((want - got[ch] as i32).abs());
            }
        }
    }
    assert!(worst <= 1, "max channel deviation {worst} from the mask");
}

#[test]
fn the_game_frame_keeps_its_orientation_from_upload_to_readback() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let mut src = vec![0u8; (SRC_W * SRC_H * 4) as usize];
    src[0..3].copy_from_slice(&[255, 255, 255]);
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let frame = c.read_frame();

    for y in 0..3 {
        for x in 0..3 {
            assert!(
                px(&frame, x, y)[0] > 100,
                "top left source pixel missing at {x},{y}"
            );
        }
    }
    assert_eq!(px(&frame, 3, 0), [0, 0, 0], "bled one cell right");
    assert_eq!(px(&frame, 0, 3), [0, 0, 0], "bled one cell down");
    assert_eq!(
        px(&frame, 0, out_h() as usize - 1),
        [0, 0, 0],
        "frame is upside down"
    );
    assert_eq!(
        px(&frame, out_w() as usize - 1, 0),
        [0, 0, 0],
        "frame is mirrored"
    );
}

#[test]
fn the_picture_strikes_as_a_band_at_the_centre_before_it_fills_the_frame() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let grey = vec![0x80u8; (SRC_W * SRC_H * 4) as usize];
    let peak = |c: &mut Compositor, t: f32| {
        c.set_screen_power(t);
        c.begin_frame();
        c.upload_game(&grey);
        c.draw_game();
        let frame = c.read_frame();
        let brightest = frame.chunks_exact(4).map(|p| p[0]).max().unwrap_or(0);
        (frame, brightest)
    };

    let (striking, bright) = peak(&mut c, 0.35);
    let lit = |frame: &[u8], y: usize| (0..out_w() as usize).any(|x| px(frame, x, y) != [0, 0, 0]);
    assert!(
        lit(&striking, out_h() as usize / 2),
        "nothing at the centre"
    );
    assert!(!lit(&striking, 1), "the picture already reaches the top");
    assert!(
        !lit(&striking, out_h() as usize - 2),
        "the picture already reaches the bottom"
    );

    let (settled, normal) = peak(&mut c, 1.0);
    assert!(
        lit(&settled, 1) && lit(&settled, out_h() as usize - 2),
        "the settled picture is short"
    );
    assert!(
        bright > normal,
        "the strike at {bright} is no brighter than the settled picture at {normal}"
    );
}

#[test]
fn the_game_marker_draws_the_picture_where_it_sits_in_the_list() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let white = vec![0xffu8; (SRC_W * SRC_H * 4) as usize];
    let full = |colour: [f32; 4]| Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: out_w() as f32,
        h: out_h() as f32,
        colour,
    };
    c.set_screen_power(1.0);

    c.begin_frame();
    c.upload_game(&white);
    c.draw_list(&[full([1.0, 0.0, 0.0, 1.0]), Draw::Game]);
    let under = c.read_frame();
    let blue = under.chunks_exact(4).map(|p| p[2]).max().unwrap_or(0);
    assert!(blue > 100, "the picture never drew over the red rect");

    c.begin_frame();
    c.draw_list(&[Draw::Game, full([0.0, 0.0, 1.0, 1.0])]);
    let over = c.read_frame();
    assert_eq!(
        px(&over, out_w() as usize / 2, out_h() as usize / 2),
        [0, 0, 255],
        "the picture was drawn over the item listed after it"
    );
}

#[test]
fn a_saved_shot_is_drawn_through_the_lcd_pass() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let shot = flat_shot([200, 200, 200]);
    let tex = c.create_texture_nearest(SRC_W, SRC_H, &shot);
    c.set_screen_power(1.0);

    c.begin_frame();
    c.draw_list(&[Draw::Tex {
        x: 0.0,
        y: 0.0,
        w: out_w() as f32,
        h: out_h() as f32,
        tex,
        alpha: 1.0,
    }]);
    let plain = c.read_frame();

    c.begin_frame();
    c.draw_list(&[Draw::Shot { tex }]);
    let lit = c.read_frame();

    assert_ne!(plain, lit, "the shot is not going through the lcd pass");
    for (y, row) in lcd3x_mask().iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let got = px(&lit, x, y)[0] as f32;
            let want = 200.0 * cell[0];
            assert!(
                (got - want).abs() < 3.0,
                "cell {x},{y} does not match the mask: {got} against {want}"
            );
        }
    }
}

#[test]
fn a_still_is_not_cropped_by_the_picture_mode() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let mut shot = flat_shot([120, 120, 120]);
    for (x, y) in [(0, 0), (SRC_W as usize - 1, SRC_H as usize - 1)] {
        let o = (y * SRC_W as usize + x) * 4;
        shot[o..o + 3].copy_from_slice(&[255, 0, 0]);
    }
    let tex = c.create_texture_nearest(SRC_W, SRC_H, &shot);
    let src = gb_shaped(|x, y| [(x * 3) as u8, (y * 5) as u8, 200], [0, 0, 0]);
    c.set_screen_power(1.0);

    let framed = |c: &mut Compositor, rect: [f32; 4]| {
        c.set_game_source_rect(rect);
        c.begin_frame();
        c.upload_game(&src);
        c.draw_list(&[Draw::Shot { tex }]);
        let still = c.read_frame();
        c.begin_frame();
        c.draw_game();
        (still, c.read_frame())
    };

    let (still_actual, game_actual) = framed(&mut c, WHOLE);
    let (still_full, game_full) = framed(&mut c, GB_RECT);

    assert!(
        still_actual == still_full,
        "the polaroid changed shape when the panel did"
    );
    assert!(
        game_actual != game_full,
        "the game did not stretch, so this proves nothing about the still"
    );
}

#[test]
fn draw_list_rects_land_in_top_left_pixel_space() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    c.begin_frame();
    c.draw_list(&[Draw::Rect {
        x: 2.0,
        y: 1.0,
        w: 4.0,
        h: 2.0,
        colour: [1.0, 0.0, 0.0, 1.0],
    }]);
    let frame = c.read_frame();

    assert_eq!(px(&frame, 2, 1), [255, 0, 0]);
    assert_eq!(px(&frame, 5, 2), [255, 0, 0]);
    assert_ne!(px(&frame, 1, 1), [255, 0, 0], "rect starts one pixel early");
    assert_ne!(px(&frame, 6, 2), [255, 0, 0], "rect runs one pixel long");
    assert_ne!(px(&frame, 2, 3), [255, 0, 0], "rect runs one row long");
}

#[test]
fn a_replaced_texture_draws_its_new_contents() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let tex = c.create_texture(1, 1, &[255, 0, 0, 255]);
    c.update_texture(tex, 1, 1, &[0, 0, 255, 255]);
    c.begin_frame();
    c.draw_list(&[Draw::Tex {
        x: 0.0,
        y: 0.0,
        w: 4.0,
        h: 4.0,
        tex,
        alpha: 1.0,
    }]);
    let frame = c.read_frame();
    assert_eq!(px(&frame, 1, 1), [0, 0, 255]);
}

#[test]
fn blue_light_warms_monotonically_and_clamps_at_the_last_step() {
    assert_eq!(blue_light_gain(0), [1.0, 1.0, 1.0]);
    let warmest = blue_light_gain(9);
    assert!(
        (warmest[1] - 0.82).abs() < 0.005 && (warmest[2] - 0.62).abs() < 0.005,
        "warmest step is {warmest:?}"
    );
    assert_eq!(blue_light_gain(200), warmest, "step must clamp, not wrap");
    for step in 0..9 {
        let a = blue_light_gain(step);
        let b = blue_light_gain(step + 1);
        assert_eq!(b[0], 1.0, "red must not move");
        assert!(b[1] < a[1] && b[2] < a[2], "step {step} did not warm");
        assert!(b[2] < b[1], "blue must fall faster than green");
    }
}

#[test]
fn an_unturned_image_draws_exactly_as_a_plain_one() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let rgba: Vec<u8> = (0..16u32 * 8)
        .flat_map(|i| [(i * 7) as u8, (i * 13) as u8, (i * 29) as u8, 255])
        .collect();
    let tex = c.create_texture(16, 8, &rgba);

    c.begin_frame();
    c.draw_list(&[Draw::Tex {
        x: 101.3,
        y: 57.6,
        w: 37.0,
        h: 19.0,
        tex,
        alpha: 0.8,
    }]);
    let plain = c.read_frame();

    c.begin_frame();
    c.draw_list(&[Draw::Turned {
        x: 101.3,
        y: 57.6,
        w: 37.0,
        h: 19.0,
        tex,
        alpha: 0.8,
        turn: 0.0,
    }]);
    let turned = c.read_frame();

    assert!(plain == turned, "a turn of zero moved something");
}

#[test]
fn a_quarter_turn_takes_the_top_left_corner_to_the_top_right() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let mut rgba = [0u8, 0, 255, 255].repeat(8);
    rgba[0..4].copy_from_slice(&[255, 0, 0, 255]);
    let tex = c.create_texture_nearest(4, 2, &rgba);

    c.begin_frame();
    c.draw_list(&[Draw::Turned {
        x: 100.0,
        y: 100.0,
        w: 40.0,
        h: 20.0,
        tex,
        alpha: 1.0,
        turn: std::f32::consts::FRAC_PI_2,
    }]);
    let frame = c.read_frame();

    assert_eq!(
        px(&frame, 127, 93),
        [255, 0, 0],
        "the red texel is not top right"
    );
    assert_eq!(
        px(&frame, 113, 93),
        [0, 0, 255],
        "the top left did not move"
    );
    assert_eq!(px(&frame, 113, 127), [0, 0, 255]);
}

#[test]
fn the_default_source_rect_draws_exactly_as_before() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let src: Vec<u8> = (0..(SRC_W * SRC_H) as usize)
        .flat_map(|i| {
            let (x, y) = (i % SRC_W as usize, i / SRC_W as usize);
            let rgb = [
                (x * 7 + y * 3) as u8,
                (x * 13 + y * 29) as u8,
                (x * 31 + y * 11) as u8,
            ];
            [rgb[2], rgb[1], rgb[0], 0]
        })
        .collect();
    c.set_screen_power(1.0);

    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let default = c.read_frame();

    c.set_game_view(GameView::canvas(WHOLE));
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    assert!(
        c.read_frame() == default,
        "the default view is not the whole texture over the whole canvas"
    );

    c.set_game_view(at_3x(WHOLE));
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let tripled = c.read_frame();
    for (sx, sy) in [
        (0, 0),
        (1, 0),
        (0, 1),
        (113, 37),
        (120, 80),
        (239, 159),
        (20, 5),
    ] {
        let o = (sy * SRC_W as usize + sx) * 4;
        let want_rgb = [src[o + 2], src[o + 1], src[o]];
        for oy in 0..3 {
            for ox in 0..3 {
                let Some((x, y)) = on_canvas(sx * 3 + ox, sy * 3 + oy) else {
                    continue;
                };
                let got = px(&tripled, x, y);
                for (ch, want) in masked(want_rgb, ox, oy).iter().enumerate() {
                    assert!(
                        (want - got[ch] as i32).abs() <= 1,
                        "source {sx},{sy} cell {ox},{oy} channel {ch}: {} against {want}",
                        got[ch],
                    );
                }
            }
        }
    }

    c.set_game_source_rect(WHOLE);
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    assert!(
        c.read_frame() == default,
        "the whole texture asked for is not the whole texture by default"
    );
    if (out_w(), out_h()) == (720, 480) {
        assert!(
            tripled == default,
            "on 720x480 the 3x frame is not the whole canvas"
        );
    }
}

#[test]
fn fullscreen_puts_the_pictures_corners_in_the_panels_corners() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let corner = |x: usize, y: usize| match (x, y) {
        (0, 0) => Some([255, 0, 0]),
        (x, 0) if x == GB_W - 1 => Some([0, 255, 0]),
        (0, y) if y == GB_H - 1 => Some([0, 0, 255]),
        (x, y) if x == GB_W - 1 && y == GB_H - 1 => Some([255, 255, 0]),
        _ => None,
    };
    const MARGIN: [u8; 3] = [0, 255, 255];
    let src = gb_shaped(|x, y| corner(x, y).unwrap_or([90, 90, 90]), MARGIN);
    c.set_screen_power(1.0);

    c.set_game_source_rect(WHOLE);
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let actual = c.read_frame();
    let got = px(&actual, 0, 0);
    for (ch, want) in masked(MARGIN, 0, 0).iter().enumerate() {
        assert!(
            (want - got[ch] as i32).abs() <= 1,
            "at actual size the panel's corner is not the margin: {got:?}"
        );
    }

    c.set_game_source_rect(GB_RECT);
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let full = c.read_frame();

    let last_x = out_w() as usize - 1;
    let last_y = out_h() as usize - 1;
    for (name, (x, y), rgb) in [
        ("top left", (0, 0), [255, 0, 0]),
        ("top right", (last_x, 0), [0, 255, 0]),
        ("bottom left", (0, last_y), [0, 0, 255]),
        ("bottom right", (last_x, last_y), [255, 255, 0]),
    ] {
        let cell = masked(rgb, x % 3, y % 3);
        let got = px(&full, x, y);
        for (ch, want) in cell.iter().enumerate() {
            assert!(
                (want - got[ch] as i32).abs() <= 1,
                "{name}: {got:?} against {cell:?}"
            );
        }
    }

    let strayed = (0..out_h() as usize).any(|y| {
        (0..out_w() as usize).any(|x| {
            let p = px(&full, x, y);
            p[1] > 60 && p[2] > 60 && p[0] < 20
        })
    });
    assert!(!strayed, "the margin is still on screen in fullscreen");
}

#[test]
fn the_mask_period_is_three_pixels_in_both_modes() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let src = gb_shaped(|_, _| [0x80, 0x80, 0x80], [0, 0, 0]);
    c.set_screen_power(1.0);

    c.set_game_view(at_3x(WHOLE));
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let actual = c.read_frame();

    c.set_game_source_rect(GB_RECT);
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let full = c.read_frame();

    let r = frame_rect();
    let fx = (GB_X as f32 * 3.0 + r[0]) as usize;
    let fy = (GB_Y as f32 * 3.0 + r[1]) as usize;
    let lit_x = fx..fx + GB_W * 3;
    let lit_y = fy..fy + GB_H * 3;
    assert_eq!(
        period_x(&actual, lit_x.clone(), lit_y.clone()),
        Some(3),
        "actual size: the grille does not repeat every 3 px across"
    );
    assert_eq!(
        period_y(&actual, lit_x, lit_y),
        Some(3),
        "actual size: the grille does not repeat every 3 px down"
    );

    assert_eq!(
        period_x(&full, 0..out_w() as usize, 0..out_h() as usize),
        Some(3),
        "fullscreen: the grille does not repeat every 3 px across"
    );
    assert_eq!(
        period_y(&full, 0..out_w() as usize, 0..out_h() as usize),
        Some(3),
        "fullscreen: the grille does not repeat every 3 px down"
    );
}

fn spun(x: f32, y: f32, w: f32, h: f32, tex: omega_gfx::TexId, yaw: f32) -> Draw {
    Draw::Spun {
        x,
        y,
        w,
        h,
        tex,
        colour: [1.0, 1.0, 1.0, 1.0],
        turn: 0.0,
        yaw,
        depth: 0.0,
        clip: None,
    }
}

fn lit(frame: &[u8], x: usize, y: usize) -> bool {
    px(frame, x, y)[0] > 128
}

/// How many rows of column `x` a white face covers.
fn lit_rows(frame: &[u8], x: usize) -> usize {
    (0..out_h() as usize).filter(|y| lit(frame, x, *y)).count()
}

/// The first and last columns a white face covers along row `y`.
fn lit_span(frame: &[u8], y: usize) -> (usize, usize) {
    let lit_at: Vec<usize> = (0..out_w() as usize)
        .filter(|x| lit(frame, *x, y))
        .collect();
    (lit_at[0], *lit_at.last().unwrap())
}

#[test]
fn an_unspun_face_draws_exactly_as_a_plain_one() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let rgba: Vec<u8> = (0..16u32 * 8)
        .flat_map(|i| [(i * 7) as u8, (i * 13) as u8, (i * 29) as u8, 255])
        .collect();
    let tex = c.create_texture(16, 8, &rgba);

    c.begin_frame();
    c.draw_list(&[Draw::Tex {
        x: 101.3,
        y: 57.6,
        w: 37.0,
        h: 19.0,
        tex,
        alpha: 0.8,
    }]);
    let plain = c.read_frame();

    c.begin_frame();
    c.draw_list(&[Draw::Spun {
        x: 101.3,
        y: 57.6,
        w: 37.0,
        h: 19.0,
        tex,
        colour: [1.0, 1.0, 1.0, 0.8],
        turn: 0.0,
        yaw: 0.0,
        depth: 0.0,
        clip: None,
    }]);
    let spun = c.read_frame();

    assert!(plain == spun, "a spin of zero moved something");
}

#[test]
fn a_spun_face_grows_on_the_side_that_turns_towards_the_eye() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let tex = c.create_texture_nearest(2, 2, &[255u8; 16]);
    let (x, w, yaw) = (200.0, 200.0, 0.3);

    c.begin_frame();
    c.draw_list(&[spun(x, 100.0, w, 100.0, tex, yaw)]);
    let frame = c.read_frame();

    let (left, right) = lit_span(&frame, 150);
    let near = lit_rows(&frame, left + 2);
    let far = lit_rows(&frame, right - 2);
    assert!(
        near > far + 2,
        "the side turning towards the eye is {near} rows tall and the far side {far}"
    );
    let want = x + w / 2.0 + spun_x(-w / 2.0, yaw, 0.0);
    assert!(
        (left as f32 - want).abs() <= 1.0,
        "the near edge landed at {left}, where spun_x puts it at {want}"
    );
    assert!(
        ((right - left) as f32) < w - 4.0,
        "the face did not narrow as it turned: {left}..{right}"
    );
}

#[test]
fn a_clip_keeps_a_spun_face_inside_it_and_ends_with_it() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let tex = c.create_texture_nearest(2, 2, &[255u8; 16]);
    let face = |clip| Draw::Spun {
        x: 200.0,
        y: 100.0,
        w: 200.0,
        h: 100.0,
        tex,
        colour: [1.0, 1.0, 1.0, 1.0],
        turn: 0.0,
        yaw: 0.2,
        depth: 0.0,
        clip,
    };

    c.begin_frame();
    c.draw_list(&[
        face(Some([250.0, 0.0, 50.0, out_h() as f32])),
        Draw::Rect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
            colour: [1.0, 1.0, 1.0, 1.0],
        },
    ]);
    let frame = c.read_frame();

    assert!(lit(&frame, 260, 150), "nothing was drawn inside the clip");
    assert!(
        !lit(&frame, 240, 150) && !lit(&frame, 310, 150),
        "the face spilled out of its clip"
    );
    assert!(
        lit(&frame, 20, 20),
        "the clip outlived its face and hid what came after it"
    );
}

/// A box `deep` widths deep over the whole of its rect, as a cart's model is laid out: its face
/// in one piece, and with `sides`, a wall down each edge.
fn box_mesh(deep: f32, sides: bool) -> Vec<MeshVertex> {
    let v = |x: f32, y: f32, z: f32, normal: [f32; 3]| MeshVertex {
        xyz: [x, y, z],
        normal,
        uv: [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)],
        shade: 1.0,
    };
    let mut out = Vec::new();
    if sides {
        // clockwise round the face, each wall's normal out of it
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let normals = [
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
        ];
        for (i, n) in normals.iter().enumerate() {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let (a0, b0) = (v(a.0, a.1, 0.0, *n), v(b.0, b.1, 0.0, *n));
            let (a1, b1) = (v(a.0, a.1, deep, *n), v(b.0, b.1, deep, *n));
            out.extend([a0, b0, b1, a0, b1, a1]);
        }
    }
    let f = |x, y| v(x, y, 0.0, [0.0, 0.0, -1.0]);
    out.extend([
        f(0.0, 0.0),
        f(1.0, 0.0),
        f(1.0, 1.0),
        f(0.0, 0.0),
        f(1.0, 1.0),
        f(0.0, 1.0),
    ]);
    out
}

fn model(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mesh: omega_gfx::MeshId,
    tex: omega_gfx::TexId,
    yaw: f32,
) -> Draw {
    Draw::Model {
        x,
        y,
        w,
        h,
        mesh,
        tex,
        colour: [1.0; 4],
        turn: 0.0,
        yaw,
    }
}

#[test]
fn a_model_square_to_the_eye_covers_exactly_its_rect_and_none_of_its_sides() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let white = c.create_texture(4, 4, &[255; 64]);
    let mesh = c.create_mesh(&box_mesh(0.3, true));

    c.begin_frame();
    c.draw_list(&[model(100.0, 60.0, 120.0, 80.0, mesh, white, 0.0)]);
    let frame = c.read_frame();

    assert_eq!(
        lit_span(&frame, 100),
        (100, 219),
        "the face is not where its rect is"
    );
    assert_eq!(
        lit_rows(&frame, 160),
        80,
        "the face is not as tall as its rect"
    );
}

/// Only a model's corners land on the grid: a turned face's top and bottom edges run slanted
/// between them, so its sides are checked turned and its top and bottom square to the eye.
#[test]
fn every_corner_of_a_model_lands_on_the_two_pixel_grid() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let white = c.create_texture(4, 4, &[255; 64]);
    let mesh = c.create_mesh(&box_mesh(0.3, false));

    c.begin_frame();
    c.draw_list(&[model(101.3, 57.6, 117.0, 81.0, mesh, white, 0.21)]);
    let (left, right) = lit_span(&c.read_frame(), 100);
    assert!(
        left % 2 == 0 && right % 2 == 1,
        "turned, a row runs from column {left} to {right}, not from one grid line to another"
    );

    c.begin_frame();
    c.draw_list(&[model(101.3, 57.6, 117.0, 81.0, mesh, white, 0.0)]);
    let frame = c.read_frame();
    let rows: Vec<usize> = (0..out_h() as usize)
        .filter(|y| lit(&frame, 160, *y))
        .collect();
    let (top, bottom) = (rows[0], *rows.last().unwrap());
    assert!(
        top % 2 == 0 && bottom % 2 == 1,
        "a column runs from row {top} to {bottom}, not from one grid line to another"
    );
}

#[test]
fn a_turned_model_shows_the_side_that_turns_towards_the_eye_and_hides_the_other() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let white = c.create_texture(4, 4, &[255; 64]);
    let face = c.create_mesh(&box_mesh(0.3, false));
    let solid = c.create_mesh(&box_mesh(0.3, true));
    let row = 100;

    // a yaw that takes the right edge back brings the left side round towards the eye
    c.begin_frame();
    c.draw_list(&[model(100.0, 60.0, 120.0, 80.0, face, white, 0.35)]);
    let (face_left, face_right) = lit_span(&c.read_frame(), row);
    c.begin_frame();
    c.draw_list(&[model(100.0, 60.0, 120.0, 80.0, solid, white, 0.35)]);
    let (left, right) = lit_span(&c.read_frame(), row);

    assert!(
        left + 6 <= face_left,
        "the left side does not show: the model starts at {left}, its face at {face_left}"
    );
    assert_eq!(
        right, face_right,
        "the side turned away shows past the face"
    );
}

/// A picture `w` by `h` whose every texel tells where it is: red across, green down.
fn located(w: usize, h: usize) -> (Vec<u8>, impl Fn(usize, usize) -> [u8; 3]) {
    let at = move |x: usize, y: usize| [(x * 255 / (w - 1)) as u8, (y * 255 / (h - 1)) as u8, 0x40];
    let mut bgrx = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let [r, g, b] = at(x, y);
            bgrx.extend_from_slice(&[b, g, r, 0]);
        }
    }
    (bgrx, at)
}

fn sharp(rect: [f32; 4]) -> GameView {
    GameView {
        rect,
        uv: WHOLE,
        filter: Filter::Sharp,
    }
}

/// Every texel of a `w` by `h` picture drawn at 2x from the canvas's top left lands on its own
/// 2x2 block of the canvas.
fn assert_at_2x(frame: &[u8], w: usize, h: usize, at: &impl Fn(usize, usize) -> [u8; 3]) {
    for (x, y) in [
        (0, 0),
        (w - 1, 0),
        (0, h - 1),
        (w - 1, h - 1),
        (w / 2, h / 3),
    ] {
        for (dx, dy) in [(0, 0), (1, 1)] {
            assert_eq!(
                px(frame, 2 * x + dx, 2 * y + dy),
                at(x, y),
                "texel {x},{y} of the {w}x{h} picture is not at 2x"
            );
        }
    }
}

#[test]
fn a_console_picture_is_drawn_whole_at_its_own_size() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    c.set_screen_power(1.0);
    for (w, h) in [(320, 240), (256, 224), (320, 240), (240, 160)] {
        let (picture, at) = located(w, h);
        c.set_game_view(sharp([0.0, 0.0, 2.0 * w as f32, 2.0 * h as f32]));
        c.begin_frame();
        c.upload_game_sized(&picture, (w as u32, h as u32));
        assert_eq!(c.game_size(), (w as u32, h as u32));
        c.draw_game();
        assert_at_2x(&c.read_frame(), w, h, &at);
    }
}

#[test]
fn a_gba_frame_uploaded_after_a_console_picture_fills_the_texture_again() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let (big, _) = located(640, 480);
    c.upload_game_sized(&big, (640, 480));
    let (gba, at) = located(SRC_W as usize, SRC_H as usize);
    c.upload_game(&gba);
    assert_eq!(c.game_size(), (SRC_W, SRC_H));
    c.set_screen_power(1.0);
    c.set_game_view(sharp([0.0, 0.0, 480.0, 320.0]));
    c.begin_frame();
    c.draw_game();
    assert_at_2x(&c.read_frame(), SRC_W as usize, SRC_H as usize, &at);
}

#[test]
fn a_short_picture_is_not_uploaded() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    c.upload_game_sized(&[0u8; 16], (320, 240));
    assert_eq!(
        c.game_size(),
        (SRC_W, SRC_H),
        "a frame too short for its size was taken"
    );
}

#[test]
fn a_console_still_is_drawn_whole_through_its_view() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let (w, h) = (320usize, 240usize);
    let (bgrx, at) = located(w, h);
    let rgba: Vec<u8> = bgrx
        .chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], 255])
        .collect();
    let tex = c.create_texture_nearest(w as u32, h as u32, &rgba);
    c.set_screen_power(1.0);
    c.set_still_view(sharp([0.0, 0.0, 640.0, 480.0]));
    c.begin_frame();
    c.draw_list(&[Draw::Shot { tex }]);
    assert_at_2x(&c.read_frame(), w, h, &at);
}
