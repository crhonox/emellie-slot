use std::collections::HashMap;

use omega_gfx::{Canvas, Filter};
use omega_store::Platform;
use omega_zero::video_mode::{parse_grid, parse_picture, picture_rect, placed};

fn keys(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn an_overlays_picture_is_read_as_retroarch_says_it() {
    let square = Canvas::SQUARE;
    let k = keys(&[("x", "40"), ("y", "61"), ("w", "640"), ("h", "576")]);
    assert_eq!(parse_picture(&k, square), Some([40.0, 61.0, 640.0, 576.0]));
    // off the canvas, or not all there, it is not used
    let off = keys(&[("x", "100"), ("y", "61"), ("w", "640"), ("h", "576")]);
    assert_eq!(parse_picture(&off, square), None);
    assert_eq!(
        parse_picture(&keys(&[("x", "0"), ("y", "0")]), square),
        None
    );
}

#[test]
fn a_game_boy_under_its_overlay_fills_the_overlays_window_sharply() {
    let rect = [40.0, 61.0, 640.0, 576.0];
    for platform in [Platform::Gb, Platform::Gbc] {
        let s = placed(Canvas::SQUARE, platform, rect, 0);
        assert_eq!(s.view.filter, Filter::Sharp);
        // the picture is the Game Boy's own window, so the overlay is checked against it
        let shown = picture_rect(platform, s.view);
        for (a, b) in shown.iter().zip(rect) {
            assert!((a - b).abs() < 0.01, "{shown:?}");
        }
        assert!(!s.bands.is_empty());
    }
}

#[test]
fn a_console_under_its_overlay_fills_it_with_its_whole_frame() {
    let rect = [0.0, 90.0, 720.0, 540.0];
    let s = placed(Canvas::SQUARE, Platform::Snes, rect, 0);
    assert_eq!(s.view.rect, rect);
    assert_eq!(picture_rect(Platform::Snes, s.view), rect);
}

#[test]
fn an_overlays_grid_is_read_in_hundredths_and_draws_through_the_grid_filter() {
    assert_eq!(parse_grid(&keys(&[("grid", "0.35")])), 35);
    assert_eq!(parse_grid(&keys(&[("grid", "2")])), 100);
    assert_eq!(parse_grid(&keys(&[("grid", "-1")])), 0);
    assert_eq!(parse_grid(&keys(&[("grid", "dark")])), 0);
    assert_eq!(parse_grid(&keys(&[])), 0);
    let rect = [40.0, 61.0, 640.0, 576.0];
    let s = placed(Canvas::SQUARE, Platform::Gbc, rect, 35);
    assert_eq!(s.view.filter, Filter::Grid(35));
    assert_eq!(s.still, s.view);
}
