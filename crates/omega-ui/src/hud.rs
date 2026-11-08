//! What shows over a game, the shelf or a menu for a moment, or while it lasts: a gauge for the
//! level a button just moved (kit::gauge), a toast saying what just happened (kit::toast), and
//! a badge in the corner while the game runs fast or is linked (kit::badge).
//!
//! The gauge or the toast stands centred at the top, `SCREEN_MARGIN` down, or `MARGIN` under the
//! bar across the top where one shows; a toast takes the gauge's place while it lasts. Each comes
//! at once and goes in the even steps of kit::fade. The badge stays in the top right corner,
//! level with a gauge's middle, for as long as what it says is so.
//!
//! The HUD only says what it shows. The frontend draws each face the first time it is asked for
//! and again when the palette changes, and hands it back with `set_gauge_face`,
//! `set_toast_face` and `set_badge_face`; a face is drawn only while the HUD still shows what
//! it was drawn for.

use omega_gfx::{out_w, Draw, TexId};
use omega_store::{BRIGHTNESS_MAX, SUNLIGHT_MAX, VOLUME_MAX};

use crate::kit::badge::{badge_face, BadgeIcon, BADGE_H};
use crate::kit::fade::stepped;
use crate::kit::gauge::{gauge_face, lit_cells, GaugeIcon, GAUGE_H};
use crate::kit::palette::Palette;
use crate::kit::pixel::quad;
use crate::plate::UndoFace;
use crate::sprite::SPRITE_PX;
use crate::Toast;

pub type Millis = u64;

pub const HUD_MS: Millis = 1500;
const FADE_MS: Millis = 250;

/// The white the save-state switcher prints in, and the top bar on a card whose theme sets the
/// chrome.
pub const HUD_INK: [u8; 3] = [0xf5, 0xf2, 0xef];

/// How far the gauge and the toast stand below the bar across the top, where one shows, in
/// screen pixels: two art pixels.
pub const MARGIN: f32 = 4.0;

/// How far they stand below the top of the screen where no bar shows, over a game or a menu, in
/// screen pixels: four art pixels, room to breathe at the panel's edge.
pub const SCREEN_MARGIN: f32 = 8.0;

/// How far the badge stands below the top of the screen, level with a gauge's middle, and in
/// from its right edge, as far: in screen pixels.
const BADGE_TOP: u32 = SCREEN_MARGIN as u32 + (GAUGE_H - BADGE_H) / 2 * SPRITE_PX;
const BADGE_RIGHT: u32 = BADGE_TOP;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum HudKind {
    #[default]
    Brightness,
    Sunlight,
    Volume,
    Rewind,
}

impl HudKind {
    /// The gauge's icon: the speaker, or the headphones while they are in, and either muted
    /// as the speaker muted.
    pub fn icon(self, muted: bool, headphones: bool) -> GaugeIcon {
        match self {
            HudKind::Brightness => GaugeIcon::Brightness,
            HudKind::Sunlight => GaugeIcon::Sun,
            HudKind::Volume if muted => GaugeIcon::Muted,
            HudKind::Volume if headphones => GaugeIcon::Headphones,
            HudKind::Volume => GaugeIcon::Volume,
            HudKind::Rewind => GaugeIcon::Rewind,
        }
    }

    /// The name in the gauge's tab.
    pub fn label(self) -> &'static str {
        match self {
            HudKind::Brightness => "BRIGHTNESS",
            HudKind::Sunlight => "SUNLIGHT",
            HudKind::Volume => "VOLUME",
            HudKind::Rewind => "REWIND",
        }
    }

    /// How many cells the gauge has: one to a step for the levels of 0 to 9 (0 to 10 for the sunlight), one to 5% for
    /// the volume and the rewind left.
    pub fn cells(self) -> u32 {
        match self {
            HudKind::Brightness => 9,
            HudKind::Sunlight => 10,
            HudKind::Volume | HudKind::Rewind => 20,
        }
    }

    fn max(self) -> u8 {
        match self {
            HudKind::Brightness => BRIGHTNESS_MAX,
            HudKind::Sunlight => SUNLIGHT_MAX,
            HudKind::Volume => VOLUME_MAX,
            HudKind::Rewind => 100,
        }
    }
}

/// What a gauge shows: which level, how many of its cells are lit, and its icon.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Gauge {
    pub kind: HudKind,
    pub lit: u32,
    pub icon: GaugeIcon,
}

impl Gauge {
    pub fn face(&self, pal: &Palette) -> UndoFace {
        let cells = self.kind.cells();
        let icon = Some(self.icon);
        gauge_face(self.kind.label(), self.lit, cells, cells, icon, pal)
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum FfState {
    #[default]
    Off,
    Held,
    Latched,
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum LinkBadge {
    #[default]
    Off,
    Hosting,
    Joined,
    HostingLost,
    JoinedLost,
}

/// What the badge in the corner says: fast forward, latched or held, and how fast; or the link,
/// which outranks it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CornerBadge {
    FastForward { latched: bool, speed: u8 },
    Link(LinkBadge),
}

impl CornerBadge {
    /// The badge as the kit draws it. Fast forward holds its speed, "×4", lit once latched; the
    /// link is lit while hosting and dark when joined, and broken once it is lost.
    pub fn face(&self, pal: &Palette) -> UndoFace {
        match *self {
            CornerBadge::FastForward { latched, speed } => {
                badge_face(BadgeIcon::FastForward, &format!("×{speed}"), latched, pal)
            }
            CornerBadge::Link(link) => {
                let icon = match link {
                    LinkBadge::HostingLost | LinkBadge::JoinedLost => BadgeIcon::Unlinked,
                    _ => BadgeIcon::Link,
                };
                let hosting = matches!(link, LinkBadge::Hosting | LinkBadge::HostingLost);
                badge_face(icon, "", hosting, pal)
            }
        }
    }
}

/// A face the frontend drew for the HUD: its texture, and its size in art pixels.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HudFace {
    pub tex: TexId,
    pub w: u32,
    pub h: u32,
}

#[derive(Default)]
pub struct Hud {
    pub kind: HudKind,
    pub value: u8,
    pub shown_at: Option<Millis>,
    held: bool,
    muted: bool,
    headphones: bool,
    ff: FfState,
    ff_speed: u8,
    said: Option<(Toast, Millis)>,
    link: LinkBadge,
    gauge_face: Option<(Gauge, HudFace)>,
    toast_face: Option<(Toast, HudFace)>,
    badge_face: Option<(CornerBadge, HudFace)>,
}

impl Hud {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toast(&mut self, toast: Toast, now: Millis) {
        self.said = Some((toast, now));
    }

    pub fn toast_visible(&self, now: Millis) -> bool {
        self.toast_alpha(now) > 0.0
    }

    pub fn said(&self, now: Millis) -> Option<Toast> {
        self.toast_visible(now)
            .then(|| self.said.map(|(t, _)| t))
            .flatten()
    }

    pub fn show(&mut self, kind: HudKind, value: u8, muted: bool, now: Millis) {
        self.kind = kind;
        self.value = value;
        self.muted = muted;
        self.shown_at = Some(now);
        self.held = kind == HudKind::Rewind;
    }

    pub fn release_rewind(&mut self) {
        if self.kind == HudKind::Rewind {
            self.shown_at = None;
        }
        self.held = false;
    }

    pub fn glyph(&self) -> GaugeIcon {
        self.kind.icon(self.muted, self.headphones)
    }

    pub fn set_headphones(&mut self, on: bool) {
        self.headphones = on;
    }

    /// Fast forward as `ff`, at `speed` times.
    pub fn set_ff(&mut self, ff: FfState, speed: u8) {
        self.ff = ff;
        self.ff_speed = speed;
    }

    /// The badge fast forward shows, while it runs.
    pub fn ff_badge(&self) -> Option<CornerBadge> {
        let speed = self.ff_speed;
        match self.ff {
            FfState::Off => None,
            FfState::Held => Some(CornerBadge::FastForward {
                latched: false,
                speed,
            }),
            FfState::Latched => Some(CornerBadge::FastForward {
                latched: true,
                speed,
            }),
        }
    }

    pub fn set_link(&mut self, badge: LinkBadge) {
        self.link = badge;
    }

    pub fn link(&self) -> LinkBadge {
        self.link
    }

    /// The gauge last shown, while it shows and after: the face the frontend keeps drawn.
    pub fn gauge(&self) -> Option<Gauge> {
        self.shown_at?;
        Some(Gauge {
            kind: self.kind,
            lit: lit_cells(self.value as u32, self.kind.max() as u32, self.kind.cells()),
            icon: self.glyph(),
        })
    }

    /// The toast last said, while it shows and after.
    pub fn last_toast(&self) -> Option<Toast> {
        self.said.map(|(t, _)| t)
    }

    /// The badge in the corner: the link's, or else fast forward's.
    pub fn badge(&self) -> Option<CornerBadge> {
        match self.link {
            LinkBadge::Off => self.ff_badge(),
            link => Some(CornerBadge::Link(link)),
        }
    }

    pub fn set_gauge_face(&mut self, gauge: Gauge, face: HudFace) {
        self.gauge_face = Some((gauge, face));
    }

    pub fn set_toast_face(&mut self, toast: Toast, face: HudFace) {
        self.toast_face = Some((toast, face));
    }

    pub fn set_badge_face(&mut self, badge: CornerBadge, face: HudFace) {
        self.badge_face = Some((badge, face));
    }

    pub fn visible(&self, now: Millis) -> bool {
        self.alpha(now) > 0.0
    }

    pub fn draw(&self, now: Millis, out: &mut Vec<Draw>) {
        self.draw_at(now, 0.0, out);
    }

    /// The gauge or the toast `top` pixels down the screen, under whatever bar stands across
    /// the top, and the badge in the corner.
    pub fn draw_at(&self, now: Millis, top: f32, out: &mut Vec<Draw>) {
        let y = match top > 0.0 {
            true => (top / 2.0).round() * 2.0 + MARGIN,
            false => SCREEN_MARGIN,
        };
        let toast = stepped(self.toast_alpha(now));
        let gauge = stepped(self.alpha(now));
        let toast_face = face_of(self.toast_face, self.last_toast());
        let gauge_face = face_of(self.gauge_face, self.gauge());
        if let (true, Some(face)) = (toast > 0.0, toast_face) {
            out.push(faded(centred(face, y), toast));
        } else if let (true, Some(face)) = (gauge > 0.0, gauge_face) {
            out.push(faded(centred(face, y), gauge));
        }
        if let Some(face) = face_of(self.badge_face, self.badge()) {
            let x = out_w().saturating_sub(BADGE_RIGHT + face.w * SPRITE_PX);
            out.push(quad(face.tex, face.w, face.h, x as f32, BADGE_TOP as f32));
        }
    }

    fn alpha(&self, now: Millis) -> f32 {
        let Some(shown) = self.shown_at else {
            return 0.0;
        };
        if self.held {
            return 1.0;
        }
        fade(shown, now)
    }

    fn toast_alpha(&self, now: Millis) -> f32 {
        match self.said {
            Some((_, shown)) => fade(shown, now),
            None => 0.0,
        }
    }
}

/// `face` while it was drawn for what is shown now.
fn face_of<K: PartialEq>(face: Option<(K, HudFace)>, shown: Option<K>) -> Option<HudFace> {
    face.filter(|(k, _)| Some(k) == shown.as_ref())
        .map(|(_, f)| f)
}

/// `face` across the middle of the screen, rounded to a whole art pixel, its top at `y`.
fn centred(face: HudFace, y: f32) -> Draw {
    let x = (out_w() / SPRITE_PX).saturating_sub(face.w) / 2 * SPRITE_PX;
    quad(face.tex, face.w, face.h, x as f32, y)
}

fn faded(mut draw: Draw, by: f32) -> Draw {
    if let Draw::Tex { alpha, .. } = &mut draw {
        *alpha = by;
    }
    draw
}

/// How much of the gauge or the toast shows `now`, smoothly: all of it, then less over its last
/// `FADE_MS`, and none from `HUD_MS` on. kit::fade::stepped moves it to its steps to draw.
fn fade(shown: Millis, now: Millis) -> f32 {
    let age = now.saturating_sub(shown);
    if age >= HUD_MS {
        return 0.0;
    }
    ((HUD_MS - age) as f32 / FADE_MS as f32).min(1.0)
}
