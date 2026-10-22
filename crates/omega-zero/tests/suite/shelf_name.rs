use omega_ui::kit::palette::palette;
use omega_ui::kit::text::text_face;
use omega_zero::screens::shelf::{name_scroll, shelf_name_face};

#[test]
fn the_file_name_is_baked_whole_however_long() {
    let stem = "Castlevania - Aria of Sorrow (USA) [Hack] ".repeat(4);
    let face = shelf_name_face(&stem, palette(0));
    assert_eq!(face.w, text_face(&stem, palette(0).cream).w);
}

#[test]
fn a_long_name_holds_at_its_start_scrolls_to_its_end_holds_and_comes_back() {
    let over = 200.0;
    assert_eq!(name_scroll(over, 0.0), 0.0);
    assert_eq!(
        name_scroll(over, 1_400.0),
        0.0,
        "it moved before its hold was over"
    );
    let mid = name_scroll(over, 1_500.0 + 2_500.0);
    assert!(mid > 0.0 && mid < over, "{mid}");
    assert_eq!(
        name_scroll(over, 1_500.0 + 5_000.0 + 100.0),
        over,
        "it did not reach its end"
    );
    assert_eq!(
        name_scroll(over, 2.0 * 1_500.0 + 5_000.0 + 10.0),
        0.0,
        "it did not come back"
    );
}
