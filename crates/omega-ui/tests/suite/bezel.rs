use omega_store::Platform;
use omega_ui::{bezel_mark_face, mark_height, mark_rect, rim, BEZEL_INK, RIM_W};

#[test]
fn the_mark_is_printed_at_a_size_the_band_can_carry() {
    assert_eq!(
        mark_height(48.0),
        30,
        "the band a 640x480 panel leaves inside the rim"
    );
    assert_eq!(
        mark_height(150.0),
        36,
        "a deep band prints no larger than 36"
    );
    assert_eq!(
        mark_height(20.0),
        16,
        "a thin band prints no smaller than 16"
    );
}

#[test]
fn every_system_has_its_mark_at_the_height_asked_for() {
    for platform in [Platform::Gba, Platform::Gb, Platform::Gbc] {
        let face = bezel_mark_face(platform, 20).expect("no mark");
        assert_eq!(face.h, 20, "{platform:?}");
        assert!(
            (150..400).contains(&face.w),
            "{platform:?}: a {}x20 mark is not a wordmark's shape",
            face.w
        );
        assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
        let inked = face.rgba.chunks_exact(4).filter(|p| p[3] > 128).count();
        assert!(
            inked > (face.w * face.h) as usize / 8,
            "{platform:?}: only {inked} pixels of ink"
        );
    }
    for platform in [Platform::Gb, Platform::Gbc] {
        let face = bezel_mark_face(platform, 20).expect("no mark");
        assert!(
            face.rgba.chunks_exact(4).all(|p| p[..3] == BEZEL_INK),
            "{platform:?}: the mark is not printed in the bezel ink"
        );
    }
}

#[test]
fn the_gba_mark_is_white_edged_in_black() {
    let face = bezel_mark_face(Platform::Gba, 30).expect("no mark");
    let solid = |p: &&[u8]| p[3] == 255;
    let white = face
        .rgba
        .chunks_exact(4)
        .filter(solid)
        .filter(|p| p[0] > 240)
        .count();
    let black = face
        .rgba
        .chunks_exact(4)
        .filter(solid)
        .filter(|p| p[0] < 16)
        .count();
    assert!(white > 400, "only {white} white pixels");
    assert!(black > 400, "only {black} black pixels round the white");
    let (x, y) = (0, face.h as usize / 2);
    let edge = &face.rgba[(y * face.w as usize + x) * 4..][..4];
    assert!(
        edge[3] == 0 || edge[0] < 64,
        "the mark's left side is not edged: {edge:?}"
    );
}

#[test]
fn the_gba_mark_reads_game_boy_advance_and_no_more() {
    // GAME BOY ADVANCE alone is under 9 times as wide as it is tall; with the SP logo's "SP"
    // it would be over 10.
    let face = bezel_mark_face(Platform::Gba, 30).expect("no mark");
    let aspect = face.w as f32 / face.h as f32;
    assert!((8.0..9.0).contains(&aspect), "a {}x{} mark", face.w, face.h);
}

#[test]
fn a_band_under_the_picture_carries_the_screens_rim_along_its_top() {
    let picture = [0.0, 0.0, 640.0, 427.0];
    let (rest, rim_at) = rim([0.0, 427.0, 640.0, 53.0], picture);
    assert_eq!(rim_at, Some([0.0, 427.0, 640.0, RIM_W]));
    assert_eq!(rest, [0.0, 427.0 + RIM_W, 640.0, 53.0 - RIM_W]);
}

#[test]
fn a_rim_reaches_round_the_corners_of_a_narrower_picture_and_no_further() {
    let picture = [80.0, 24.0, 480.0, 432.0];
    let (_, above) = rim([0.0, 0.0, 640.0, 24.0], picture);
    assert_eq!(
        above,
        Some([80.0 - RIM_W, 24.0 - RIM_W, 480.0 + 2.0 * RIM_W, RIM_W])
    );
    let (rest, left) = rim([0.0, 24.0, 80.0, 432.0], picture);
    assert_eq!(left, Some([80.0 - RIM_W, 24.0, RIM_W, 432.0]));
    assert_eq!(rest, [0.0, 24.0, 80.0 - RIM_W, 432.0]);
    let (rest, right) = rim([560.0, 24.0, 80.0, 432.0], picture);
    assert_eq!(right, Some([560.0, 24.0, RIM_W, 432.0]));
    assert_eq!(rest, [560.0 + RIM_W, 24.0, 80.0 - RIM_W, 432.0]);
}

#[test]
fn a_band_that_meets_the_picture_nowhere_has_no_rim() {
    let band = [0.0, 0.0, 720.0, 90.0];
    assert_eq!(rim(band, [0.0, 200.0, 720.0, 480.0]), (band, None));
}

#[test]
fn a_mark_shrunk_small_keeps_its_strokes_rather_than_dropping_them() {
    let big = bezel_mark_face(Platform::Gba, 40).expect("no mark");
    let small = bezel_mark_face(Platform::Gba, 16).expect("no mark");
    let ink = |f: &omega_ui::UndoFace| {
        f.rgba.chunks_exact(4).map(|p| p[3] as f32).sum::<f32>() / (f.w * f.h) as f32
    };
    assert!(
        (ink(&big) - ink(&small)).abs() < ink(&big) * 0.15,
        "the mark's ink went from {} to {} a pixel",
        ink(&big),
        ink(&small)
    );
}

#[test]
fn the_mark_is_centred_across_its_band_and_inside_it() {
    let narrow = [0.0, 427.0, 640.0, 53.0];
    let r = mark_rect(narrow, 262, 20);
    assert_eq!(r[0], 189.0);
    assert!(r[1] >= 427.0 && r[1] + r[3] <= 480.0, "{r:?}");
    assert_eq!(
        r[1], 444.0,
        "a narrow band carries the mark across its middle"
    );

    let deep = [0.0, 570.0, 720.0, 150.0];
    let r = mark_rect(deep, 420, 32);
    assert_eq!(r[0], 150.0);
    assert!(r[1] > 570.0 && r[1] + r[3] < 720.0 - 32.0, "{r:?}");
}

fn frame_overlay(w: u32, h: u32, hole: [u32; 4]) -> Vec<u8> {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let inside = (hole[0]..hole[0] + hole[2]).contains(&x)
                && (hole[1]..hole[1] + hole[3]).contains(&y);
            if !inside {
                rgba[((y * w + x) * 4 + 3) as usize] = 255;
            }
        }
    }
    rgba
}

#[test]
fn an_overlay_with_a_hole_where_the_game_shows_is_used() {
    let rgba = frame_overlay(640, 480, [80, 24, 480, 432]);
    assert!(omega_ui::overlay_clear(
        &rgba,
        640,
        480,
        [80.0, 24.0, 480.0, 432.0]
    ));
}

#[test]
fn an_overlay_that_would_hide_the_game_is_refused() {
    let gb_hole = frame_overlay(640, 480, [80, 24, 480, 432]);
    assert!(
        !omega_ui::overlay_clear(&gb_hole, 640, 480, [0.0, 0.0, 640.0, 427.0]),
        "a Game Boy frame was laid over a GBA picture"
    );
    let solid = vec![255u8; 640 * 480 * 4];
    assert!(!omega_ui::overlay_clear(
        &solid,
        640,
        480,
        [80.0, 24.0, 480.0, 432.0]
    ));
    assert!(
        !omega_ui::overlay_clear(&solid[..100], 640, 480, [0.0, 0.0, 10.0, 10.0]),
        "a short buffer"
    );
}

#[test]
fn a_few_stray_pixels_over_the_game_are_forgiven() {
    let mut rgba = frame_overlay(640, 480, [0, 0, 640, 427]);
    for x in 0..640 {
        rgba[((200 * 640 + x) * 4 + 3) as usize] = 255;
    }
    assert!(
        omega_ui::overlay_clear(&rgba, 640, 480, [0.0, 0.0, 640.0, 427.0]),
        "one line over the picture is under 1% of it"
    );
}

/// An LCD grid as overlays draw one: every game pixel's edges darkened, its middle left light,
/// and the console's name solid in the band under the picture.
fn grid_overlay(w: u32, h: u32, picture_h: u32, strongest: u8) -> Vec<u8> {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let a = if y >= picture_h {
                255
            } else if x % 3 == 2 || y % 3 == 2 {
                strongest
            } else {
                40
            };
            rgba[((y * w + x) * 4 + 3) as usize] = a;
        }
    }
    rgba
}

#[test]
fn a_grid_the_game_shows_through_everywhere_is_used() {
    let rgba = grid_overlay(640, 480, 427, 234);
    assert!(
        omega_ui::overlay_clear(&rgba, 640, 480, [0.0, 0.0, 640.0, 427.0]),
        "an LCD grid over the GBA picture was refused"
    );
}

#[test]
fn a_grid_solid_anywhere_over_the_game_is_refused() {
    let rgba = grid_overlay(640, 480, 427, 255);
    assert!(
        !omega_ui::overlay_clear(&rgba, 640, 480, [0.0, 0.0, 640.0, 427.0]),
        "a grid with solid lines hides the game"
    );
    let laid_too_low = grid_overlay(640, 480, 400, 234);
    assert!(
        !omega_ui::overlay_clear(&laid_too_low, 640, 480, [0.0, 0.0, 640.0, 427.0]),
        "a grid whose solid band runs over the picture was used"
    );
}

#[test]
fn a_frame_a_pixel_over_the_pictures_edge_is_used() {
    // a Game Boy's 640x576 at (40, 61) on 720x720, its grid over every fourth line and the
    // frame's last row and column a pixel into the picture, as a RetroArch overlay has them
    let (w, h) = (720u32, 720u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let inside = (40..679).contains(&x) && (61..636).contains(&y);
            let a = if !inside {
                255
            } else if (x - 40) % 4 == 3 || (y - 61) % 4 == 3 {
                153
            } else {
                0
            };
            rgba[((y * w + x) * 4 + 3) as usize] = a;
        }
    }
    assert!(omega_ui::overlay_clear(
        &rgba,
        w,
        h,
        [40.0, 61.0, 640.0, 576.0]
    ));
    // laid for a picture 30 pixels higher, its frame covers a row of the game's
    assert!(!omega_ui::overlay_clear(
        &rgba,
        w,
        h,
        [40.0, 31.0, 640.0, 576.0]
    ));
}
