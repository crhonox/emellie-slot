use omega_ui::kit::badge::BADGE_H;
use omega_ui::kit::gauge::GAUGE_H;
use omega_ui::kit::palette::palette;
use omega_ui::{
    out_h, out_w, CornerBadge, Draw, FfState, Gauge, GaugeIcon, Hud, HudFace, HudKind, LinkBadge,
    TexId, MARGIN, SCREEN_MARGIN, SPRITE_PX,
};

const GAUGE_TEX: TexId = TexId::from_raw(700);
const BADGE_TEX: TexId = TexId::from_raw(900);

/// `h` given a face for the gauge it shows now, 157 art pixels wide as VOLUME's.
fn with_gauge_face(h: &mut Hud) {
    let gauge = h.gauge().expect("no gauge to draw a face for");
    let face = HudFace {
        tex: GAUGE_TEX,
        w: 157,
        h: GAUGE_H,
    };
    h.set_gauge_face(gauge, face);
}

/// `h` given a face for the badge it shows now, `w` art pixels wide.
fn with_badge_face(h: &mut Hud, w: u32) {
    let badge = h.badge().expect("no badge to draw a face for");
    let face = HudFace {
        tex: BADGE_TEX,
        w,
        h: BADGE_H,
    };
    h.set_badge_face(badge, face);
}

fn drawn(h: &Hud, now: u64, top: f32) -> Vec<Draw> {
    let mut out = Vec::new();
    h.draw_at(now, top, &mut out);
    out
}

fn tex(out: &[Draw], tex: TexId) -> Option<(f32, f32, f32, f32, f32)> {
    out.iter().find_map(|d| match *d {
        Draw::Tex {
            x,
            y,
            w,
            h,
            tex: t,
            alpha,
        } if t == tex => Some((x, y, w, h, alpha)),
        _ => None,
    })
}

fn lit(kind: HudKind, value: u8) -> u32 {
    let mut h = Hud::new();
    h.show(kind, value, false, 0);
    h.gauge().expect("no gauge").lit
}

#[test]
fn hud_fades_after_1500ms() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 5, false, 1000);
    assert!(h.visible(2499));
    assert!(!h.visible(2500));
}

#[test]
fn nothing_shows_until_something_is_adjusted() {
    let h = Hud::new();
    assert!(!h.visible(0));
    assert_eq!(h.gauge(), None);
    assert!(drawn(&h, 0, 0.0).is_empty());
}

#[test]
fn volume_reads_against_100_not_the_nine_step_scale() {
    assert_eq!(lit(HudKind::Brightness, 9), 9, "brightness 9 is not full");
    assert_eq!(
        lit(HudKind::Volume, 9),
        2,
        "volume 9 is being read against the brightness scale"
    );
    assert_eq!(lit(HudKind::Volume, 100), 20);
    assert_eq!(lit(HudKind::Sunlight, 5), 5);
    assert_eq!(lit(HudKind::Rewind, 70), 14);
}

#[test]
fn each_level_has_its_title_and_cells() {
    let kinds = [
        (HudKind::Volume, "VOLUME", 20),
        (HudKind::Brightness, "BRIGHTNESS", 9),
        (HudKind::Sunlight, "SUNLIGHT", 10),
        (HudKind::Rewind, "REWIND", 20),
    ];
    for (kind, label, cells) in kinds {
        assert_eq!((kind.label(), kind.cells()), (label, cells), "{kind:?}");
    }
}

#[test]
fn the_gauge_stands_at_the_top_on_its_own_with_no_plate() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    with_gauge_face(&mut h);
    let out = drawn(&h, 0, 0.0);
    assert_eq!(out.len(), 1, "the gauge came with company: {out:?}");
    let (_, y, _, gh, alpha) = tex(&out, GAUGE_TEX).expect("the gauge is not drawn");
    assert_eq!(y, SCREEN_MARGIN, "the gauge is not at the top");
    assert_eq!(gh, (GAUGE_H * SPRITE_PX) as f32, "not drawn at 2x");
    assert_eq!(alpha, 1.0);
    assert!(y + gh < out_h() as f32 / 2.0);
}

#[test]
fn the_gauge_is_centred_on_the_art_grid() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    with_gauge_face(&mut h);
    let (x, y, w, _, _) = tex(&drawn(&h, 0, 0.0), GAUGE_TEX).expect("no gauge");
    assert_eq!(x % 2.0, 0.0, "x {x} is off the art grid");
    assert_eq!(y % 2.0, 0.0, "y {y} is off the art grid");
    assert!(
        (x + w / 2.0 - out_w() as f32 / 2.0).abs() <= 1.0,
        "the gauge at {x} is not centred"
    );
}

#[test]
fn under_a_top_bar_the_gauge_stands_just_below_it_on_the_grid() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    with_gauge_face(&mut h);
    let y = |top: f32| tex(&drawn(&h, 0, top), GAUGE_TEX).expect("no gauge").1;
    assert_eq!(y(36.0), 36.0 + MARGIN);
    let sliding = y(23.3);
    assert_eq!(sliding % 2.0, 0.0, "{sliding} is off the grid mid-slide");
    assert!((sliding - (23.3 + MARGIN)).abs() <= 1.0);
}

#[test]
fn the_gauge_fades_out_in_four_even_steps() {
    let mut h = Hud::new();
    h.show(HudKind::Brightness, 5, false, 0);
    with_gauge_face(&mut h);
    let mut seen = Vec::new();
    for now in 0..1500 {
        let alpha = tex(&drawn(&h, now, 0.0), GAUGE_TEX).map_or(0.0, |t| t.4);
        assert!(
            [0.0, 0.25, 0.5, 0.75, 1.0].contains(&alpha),
            "{alpha} at {now} ms is between the steps"
        );
        if seen.last() != Some(&alpha) {
            seen.push(alpha);
        }
    }
    assert_eq!(seen, vec![1.0, 0.75, 0.5, 0.25, 0.0]);
}

#[test]
fn a_face_drawn_for_another_level_is_never_shown() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    with_gauge_face(&mut h);
    h.show(HudKind::Volume, 55, false, 10);
    assert!(
        tex(&drawn(&h, 10, 0.0), GAUGE_TEX).is_none(),
        "the face of the level before was drawn for the level now"
    );
}

#[test]
fn muted_volume_uses_the_muted_icon() {
    assert_eq!(HudKind::Volume.icon(false, false), GaugeIcon::Volume);
    assert_eq!(HudKind::Volume.icon(true, false), GaugeIcon::Muted);
    let mut h = Hud::new();
    h.show(HudKind::Volume, 0, true, 0);
    assert_eq!(
        h.gauge().map(|g| (g.icon, g.lit)),
        Some((GaugeIcon::Muted, 0))
    );
}

#[test]
fn volume_on_headphones_shows_headphones() {
    assert_eq!(HudKind::Volume.icon(false, true), GaugeIcon::Headphones);
    assert_eq!(
        HudKind::Volume.icon(true, true),
        GaugeIcon::Muted,
        "muted headphones should say muted"
    );
    assert_eq!(HudKind::Brightness.icon(false, true), GaugeIcon::Brightness);
    assert_eq!(HudKind::Rewind.icon(false, false), GaugeIcon::Rewind);
}

#[test]
fn the_rewind_gauge_is_held_open_rather_than_fading() {
    let mut h = Hud::new();
    h.show(HudKind::Rewind, 80, false, 0);
    with_gauge_face(&mut h);
    assert!(
        h.visible(60_000),
        "the rewind gauge timed out while still held"
    );
    assert_eq!(
        tex(&drawn(&h, 60_000, 0.0), GAUGE_TEX).map(|t| t.4),
        Some(1.0)
    );
    h.release_rewind();
    assert!(!h.visible(60_000));
}

#[test]
fn an_empty_rewind_buffer_still_draws_an_empty_gauge() {
    let mut h = Hud::new();
    h.show(HudKind::Rewind, 0, false, 0);
    assert_eq!(
        h.gauge(),
        Some(Gauge {
            kind: HudKind::Rewind,
            lit: 0,
            icon: GaugeIcon::Rewind
        })
    );
    with_gauge_face(&mut h);
    assert!(
        tex(&drawn(&h, 0, 0.0), GAUGE_TEX).is_some(),
        "the gauge disappeared when the buffer emptied"
    );
}

#[test]
fn releasing_rewind_leaves_a_level_gauge_alone() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    h.release_rewind();
    assert!(h.visible(0));
}

#[test]
fn held_and_latched_fast_forward_differ_and_carry_the_speed() {
    let mut h = Hud::new();
    h.set_ff(FfState::Held, 4);
    let held = h.ff_badge().expect("held draws nothing");
    h.set_ff(FfState::Latched, 4);
    let latched = h.ff_badge().expect("latched draws nothing");
    assert_eq!(
        held,
        CornerBadge::FastForward {
            latched: false,
            speed: 4
        }
    );
    assert_eq!(
        latched,
        CornerBadge::FastForward {
            latched: true,
            speed: 4
        }
    );
    let pal = palette(0);
    assert_ne!(
        held.face(pal).rgba,
        latched.face(pal).rgba,
        "the latch is indistinguishable from the hold"
    );
    h.set_ff(FfState::Off, 4);
    assert_eq!(h.ff_badge(), None);
}

#[test]
fn each_speed_has_its_own_badge() {
    let pal = palette(0);
    let faces: Vec<_> = [2, 3, 4, 6]
        .map(|speed| {
            CornerBadge::FastForward {
                latched: false,
                speed,
            }
            .face(pal)
        })
        .into_iter()
        .map(|f| f.rgba)
        .collect();
    for (i, a) in faces.iter().enumerate() {
        for b in &faces[i + 1..] {
            assert_ne!(a, b, "two speeds read the same");
        }
    }
}

#[test]
fn the_badge_outlives_the_gauge_timer_without_a_plate() {
    let mut h = Hud::new();
    h.set_ff(FfState::Latched, 2);
    with_badge_face(&mut h, 25);
    let out = drawn(&h, 60_000, 0.0);
    assert_eq!(out.len(), 1, "the badge came with company: {out:?}");
    assert!(tex(&out, BADGE_TEX).is_some(), "the badge itself went away");
}

#[test]
fn the_badge_sits_in_the_top_right_corner_on_the_grid() {
    let mut h = Hud::new();
    h.set_ff(FfState::Held, 4);
    with_badge_face(&mut h, 25);
    // the bar across the top never moves the badge
    let (x, y, w, bh, _) = tex(&drawn(&h, 0, 36.0), BADGE_TEX).expect("no badge");
    assert_eq!((w, bh), (50.0, 18.0), "not drawn at 2x");
    assert_eq!((x % 2.0, y % 2.0), (0.0, 0.0), "off the art grid");
    let right = out_w() as f32 - (x + w);
    assert_eq!(right, y, "{right} from the right but {y} from the top");
    assert_eq!(
        y + bh / 2.0,
        SCREEN_MARGIN + (GAUGE_H * SPRITE_PX) as f32 / 2.0,
        "the badge is not level with a gauge's middle"
    );
}

#[test]
fn the_badge_leaves_when_fast_forward_stops() {
    let mut h = Hud::new();
    h.set_ff(FfState::Held, 4);
    with_badge_face(&mut h, 25);
    h.set_ff(FfState::Off, 4);
    assert_eq!(h.badge(), None);
    assert!(
        drawn(&h, 0, 0.0).is_empty(),
        "the badge outlived the fast forward"
    );
}

#[test]
fn the_link_badge_outranks_fast_forward() {
    let mut h = Hud::new();
    h.set_ff(FfState::Latched, 4);
    h.set_link(LinkBadge::JoinedLost);
    assert_eq!(h.badge(), Some(CornerBadge::Link(LinkBadge::JoinedLost)));
    with_badge_face(&mut h, 11);
    let out = drawn(&h, 0, 0.0);
    assert_eq!(out.len(), 1);
    assert!(tex(&out, BADGE_TEX).is_some());
}

#[test]
fn each_link_badge_looks_its_own_and_off_has_none() {
    let mut h = Hud::new();
    h.set_link(LinkBadge::Off);
    assert_eq!(h.badge(), None);
    let pal = palette(0);
    let faces: Vec<_> = [
        LinkBadge::Hosting,
        LinkBadge::Joined,
        LinkBadge::HostingLost,
        LinkBadge::JoinedLost,
    ]
    .iter()
    .map(|b| CornerBadge::Link(*b).face(pal).rgba)
    .collect();
    for (i, a) in faces.iter().enumerate() {
        for b in &faces[i + 1..] {
            assert_ne!(a, b, "two of the link's badges read the same");
        }
    }
}

#[test]
fn hosting_and_joining_are_told_apart_by_the_palette() {
    let pal = palette(7);
    let has = |b: LinkBadge, colour: [u8; 3]| {
        CornerBadge::Link(b)
            .face(pal)
            .rgba
            .chunks(4)
            .any(|p| p[3] == 255 && p[..3] == colour)
    };
    assert!(has(LinkBadge::Hosting, pal.frame), "hosting is not lit");
    assert!(
        has(LinkBadge::Joined, pal.dark),
        "joined is not the dark cap"
    );
    assert!(!has(LinkBadge::Joined, pal.frame));
}

#[test]
fn the_faces_follow_the_palette() {
    let gauge = Gauge {
        kind: HudKind::Volume,
        lit: 10,
        icon: GaugeIcon::Volume,
    };
    let (night, amber) = (palette(0), palette(7));
    assert_ne!(gauge.face(night).rgba, gauge.face(amber).rgba);
    let face = gauge.face(amber);
    assert!(
        face.rgba
            .chunks(4)
            .any(|p| p[3] == 255 && p[..3] == amber.label),
        "the lit cells are not in the palette's label colour"
    );
}
