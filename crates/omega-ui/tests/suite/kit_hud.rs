//! The kit's faces for what shows over a game, the gauge, the toast and the badge, held to
//! screenshots of omega-gauge, omega-toast and omega-badge in Chrome (kit/ref/, made by
//! kit/make-refs.sh), pixel for pixel, as kit.rs holds the menus' faces.

use omega_ui::kit::badge::{badge_face, badge_width, BadgeIcon, BADGE_H};
use omega_ui::kit::gauge::{gauge_face, gauge_width, lit_cells, GaugeIcon, GAUGE_H};
use omega_ui::kit::toast::{toast_face, toast_width, TOAST_H};

use crate::kit::{matches, RAMP};

#[test]
fn the_gauges_are_chromes() {
    let gauges = [
        (
            "VOLUME",
            55,
            100,
            20,
            Some(GaugeIcon::Volume),
            "gauge-volume",
        ),
        ("VOLUME", 0, 100, 20, Some(GaugeIcon::Muted), "gauge-muted"),
        (
            "VOLUME",
            35,
            100,
            20,
            Some(GaugeIcon::Headphones),
            "gauge-headphones",
        ),
        (
            "BRIGHTNESS",
            6,
            9,
            9,
            Some(GaugeIcon::Brightness),
            "gauge-brightness",
        ),
        (
            "SUNLIGHT",
            5,
            10,
            10,
            Some(GaugeIcon::Sun),
            "gauge-sun",
        ),
        (
            "REWIND",
            70,
            100,
            20,
            Some(GaugeIcon::Rewind),
            "gauge-rewind",
        ),
        (
            "SPEED",
            4,
            8,
            8,
            Some(GaugeIcon::FastForward),
            "gauge-speed",
        ),
        ("LEVEL", 0, 9, 9, None, "gauge-empty"),
        ("LEVEL", 9, 9, 9, None, "gauge-full"),
    ];
    for (label, value, max, cells, icon, name) in gauges {
        matches(&gauge_face(label, value, max, cells, icon, &RAMP), name);
    }
}

#[test]
fn a_gauge_is_as_wide_as_its_title_cells_and_icon_and_as_tall_as_the_bar() {
    let face = gauge_face("VOLUME", 55, 100, 20, Some(GaugeIcon::Volume), &RAMP);
    assert_eq!((face.w, face.h), (157, GAUGE_H));
    assert_eq!(gauge_width("VOLUME", 20, true), 157);
    assert_eq!(gauge_width("BRIGHTNESS", 9, true), 137);
    assert_eq!(gauge_width("LEVEL", 9, false), 97);
}

#[test]
fn a_gauge_lights_one_cell_to_a_step_from_none_to_all() {
    assert_eq!(lit_cells(0, 9, 9), 0);
    assert_eq!(lit_cells(6, 9, 9), 6);
    assert_eq!(lit_cells(9, 9, 9), 9);
    assert_eq!(lit_cells(55, 100, 20), 11);
    assert_eq!(lit_cells(100, 100, 20), 20);
    // 2.5% is half a cell, which rounds up as JS's Math.round does
    assert_eq!(lit_cells(3, 100, 20), 1);
    assert_eq!(lit_cells(2, 100, 20), 0);
    assert_eq!(
        lit_cells(250, 100, 20),
        20,
        "a level past full lit more than all"
    );
}

#[test]
fn toasts_are_chromes() {
    let toasts = [
        ("SAVE STATE", "State saved", "toast-saved"),
        ("LINK", "Link was ended", "toast-ended"),
        ("LINK", "Please switch to gpSP", "toast-gpsp"),
        ("colour", "Correction off", "toast-colour"),
    ];
    for (label, text, name) in toasts {
        matches(&toast_face(label, text, &RAMP), name);
    }
}

#[test]
fn a_toast_is_as_wide_as_its_header_or_its_line() {
    assert_eq!(
        toast_width("SAVE STATE", "State saved"),
        84,
        "the tab's width"
    );
    assert_eq!(
        toast_width("LINK", "Link was ended"),
        96,
        "the line's width"
    );
    assert_eq!(toast_face("LINK", "Link ended", &RAMP).h, TOAST_H);
    assert_eq!(TOAST_H, 34);
}

#[test]
fn badges_are_chromes() {
    let badges = [
        (BadgeIcon::FastForward, "×2", false, "badge-ff-2"),
        (BadgeIcon::FastForward, "×6", true, "badge-ff-6-lit"),
        (BadgeIcon::Link, "", false, "badge-link"),
        (BadgeIcon::Link, "", true, "badge-link-lit"),
        (BadgeIcon::Unlinked, "", false, "badge-unlinked"),
        (BadgeIcon::Unlinked, "", true, "badge-unlinked-lit"),
        (BadgeIcon::FastForward, "HOST", false, "badge-word"),
    ];
    for (icon, text, lit, name) in badges {
        matches(&badge_face(icon, text, lit, &RAMP), name);
    }
}

#[test]
fn a_badge_is_a_key_caps_height_and_as_wide_as_what_it_holds() {
    assert_eq!(badge_width(""), 11);
    assert_eq!(badge_width("×4"), 25);
    let face = badge_face(BadgeIcon::Link, "", true, &RAMP);
    assert_eq!((face.w, face.h), (11, BADGE_H));
}
