use crate::common;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use omega_input::Action;
use omega_store::Core;
use omega_ui::kit::action::{action_pair, Font, Side};
use omega_ui::kit::bar::bar_layout;
use omega_ui::kit::palette::palette;
use omega_ui::link_art::{link_art, PLUG_H};
use omega_ui::{out_h, out_w, CartFace, Draw, HudFace, TexId, Toast, UndoFace, SPRITE_PX};
use omega_zero::app::{App, GameMenu, LinkRow};
use omega_zero::link_kind::LinkKind;
use omega_zero::link_net::Cancel;
use omega_zero::link_radio::{LinkRole, RadioJob, RadioJobs};
use omega_zero::link_screen::{
    draw_link_art, link_panel_face, port_top, LinkActions, LinkBar, LinkSprites, Sprite, PORT_UP,
};
use omega_zero::link_start::{LinkFail, LinkStarter, LinkStep};
use omega_zero::screens::bar::BarSet;
use omega_zero::screens::lone::Lone;

struct Face {
    rgba: Vec<u8>,
    w: u32,
    h: u32,
}

impl From<CartFace> for Face {
    fn from(f: CartFace) -> Self {
        Face {
            rgba: f.rgba,
            w: f.w,
            h: f.h,
        }
    }
}

impl From<UndoFace> for Face {
    fn from(f: UndoFace) -> Self {
        Face {
            rgba: f.rgba,
            w: f.w,
            h: f.h,
        }
    }
}

/// The art for Night, each face under a texture of its own, as the frontend uploads it.
fn sprites_and_faces() -> (LinkSprites, Vec<(TexId, Face)>) {
    let art = link_art(palette(0), out_w() / SPRITE_PX, PORT_UP);
    let mut faces = Vec::new();
    let mut n = 0;
    let mut put = |f: UndoFace| {
        n += 1;
        let tex = TexId::from_raw(n);
        let sprite = Sprite {
            tex,
            w: f.w,
            h: f.h,
        };
        faces.push((tex, f.into()));
        sprite
    };
    let [ar0, ar1, ar2] = art.arcs_right;
    let [al0, al1, al2] = art.arcs_left;
    let sprites = LinkSprites {
        plug_host: put(art.plug_host),
        plug_join: put(art.plug_join),
        cable: put(art.cable),
        adapter: put(art.adapter),
        port: put(art.port),
        arcs_right: [put(ar0), put(ar1), put(ar2)],
        arcs_left: [put(al0), put(al1), put(al2)],
        clicks: put(art.clicks),
    };
    (sprites, faces)
}

const GROUND: [u8; 3] = [0x05, 0x05, 0x08];

/// `out` drawn over the Night backdrop's colour, each face stretched to its quad by its nearest
/// texel, as the compositor draws a NEAREST texture.
fn composite(out: &[Draw], faces: &[(TexId, Face)]) -> Vec<u8> {
    let (w, h) = (out_w() as usize, out_h() as usize);
    let mut px = vec![0u8; w * h * 4];
    for i in 0..w * h {
        px[i * 4..i * 4 + 4].copy_from_slice(&[GROUND[0], GROUND[1], GROUND[2], 255]);
    }
    for d in out {
        let Draw::Tex {
            x,
            y,
            w: dw,
            h: dh,
            tex,
            alpha,
        } = *d
        else {
            continue;
        };
        let Some((_, face)) = faces.iter().find(|(t, _)| *t == tex) else {
            continue;
        };
        for qy in y as i32..(y + dh) as i32 {
            for qx in x as i32..(x + dw) as i32 {
                if qx < 0 || qy < 0 || qx >= w as i32 || qy >= h as i32 {
                    continue;
                }
                let fx = ((qx as f32 - x) * face.w as f32 / dw) as usize;
                let fy = ((qy as f32 - y) * face.h as f32 / dh) as usize;
                let s = (fy * face.w as usize + fx) * 4;
                let a = face.rgba[s + 3] as f32 / 255.0 * alpha;
                let o = (qy as usize * w + qx as usize) * 4;
                for k in 0..3 {
                    px[o + k] =
                        (face.rgba[s + k] as f32 * a + px[o + k] as f32 * (1.0 - a)).round() as u8;
                }
            }
        }
    }
    px
}

fn at(px: &[u8], x: usize, y: usize) -> [u8; 3] {
    let o = (y * out_w() as usize + x) * 4;
    [px[o], px[o + 1], px[o + 2]]
}

/// The pixel at art pixel (`x`, `y`) of the screen.
fn art_at(px: &[u8], x: i32, y: i32) -> [u8; 3] {
    let s = SPRITE_PX as i32;
    at(px, (x * s) as usize, (y * s) as usize)
}

fn mid() -> i32 {
    (out_w() / SPRITE_PX / 2) as i32
}

fn render(menu: GameMenu, kind: LinkKind, now: u64, name: &str) -> Vec<u8> {
    let (sprites, faces) = sprites_and_faces();
    let mut out = Vec::new();
    draw_link_art(menu, kind, now, &sprites, 8, &mut out);
    let px = composite(&out, &faces);
    dump(&px, name);
    px
}

fn dump(px: &[u8], name: &str) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let path = format!("{dir}/link-{name}.png");
    let file = std::fs::File::create(&path).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(file), out_w(), out_h());
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(px).unwrap();
    println!("wrote {path}");
}

/// The plug's housing, under its ridges, left of the middle.
fn housing(px: &[u8], menu: GameMenu, now: u64) -> [u8; 3] {
    let bottom = port_top() + omega_zero::link_screen::plug_bottom(menu, now);
    art_at(px, mid() - 4, bottom - PLUG_H as i32 + 22)
}

#[test]
fn the_host_picks_a_purple_plug_over_the_port() {
    let menu = GameMenu::Pick(LinkRow::Host);
    let px = render(menu, LinkKind::Cable, 0, "cable-host-pick");
    let h = housing(&px, menu, 0);
    assert!(h[2] > h[1] + 30, "no purple housing over the port: {h:?}");
    let slot = art_at(&px, mid(), port_top() + 1);
    let pal = palette(0);
    assert_eq!(slot, pal.dark, "no port's opening under the plug");
    let gap = art_at(&px, mid(), port_top() - 2);
    assert_eq!(gap, GROUND, "the plug is already in the port");
}

#[test]
fn a_joiner_picks_a_grey_plug() {
    let menu = GameMenu::Pick(LinkRow::Join);
    let px = render(menu, LinkKind::Cable, 0, "cable-join-pick");
    let h = housing(&px, menu, 0);
    assert!(
        (h[0] as i32 - h[2] as i32).abs() < 14 && h[0] > 0x70,
        "not grey: {h:?}"
    );
}

#[test]
fn a_wireless_link_seats_the_adapter_on_the_port() {
    let menu = GameMenu::Linked {
        role: LinkRow::Host,
        worked: 0,
        since: 600,
        opened: false,
    };
    let px = render(menu, LinkKind::Wireless, 2000, "wireless-linked");
    let body = art_at(&px, mid(), port_top() - 4);
    assert_ne!(body, GROUND, "no adapter sitting on the port");
    assert!(
        (body[0] as i32 - body[2] as i32).abs() < 30,
        "the adapter is not its grey: {body:?}"
    );
}

#[test]
fn a_failed_plug_leaves_where_it_waited() {
    let working = GameMenu::Working {
        role: LinkRow::Host,
        step: LinkStep::Waiting,
        since: 0,
    };
    let waiting = render(working, LinkKind::Cable, 1200, "cable-working");
    let failed = GameMenu::Failed {
        role: LinkRow::Host,
        fail: LinkFail::NobodyCame,
        worked: 0,
        since: 1200,
    };
    let gone = render(failed, LinkKind::Cable, 1600, "cable-failed");
    let spot = (mid() - 10, port_top() - 2);
    assert_ne!(
        art_at(&waiting, spot.0, spot.1),
        art_at(&gone, spot.0, spot.1),
        "the plug never lifted away"
    );
}

#[test]
fn ending_a_link_pulls_the_plug_back_out_of_the_port() {
    let menu = GameMenu::Unplug {
        role: LinkRow::Host,
        since: 0,
    };
    let seated = render(menu, LinkKind::Cable, 0, "cable-unplug-start");
    let gone = render(menu, LinkKind::Cable, 400, "cable-unplug-end");
    let (x, y) = (mid() - 10, port_top() - 2);
    assert_ne!(
        art_at(&seated, x, y),
        GROUND,
        "no plug on the port on the frame the unplug begins"
    );
    assert_eq!(
        art_at(&gone, x, y),
        GROUND,
        "the plug never came back out of the port"
    );
}

#[test]
fn ending_a_wireless_link_lifts_the_adapter_off_the_port() {
    let menu = GameMenu::Unplug {
        role: LinkRow::Join,
        since: 0,
    };
    let seated = render(menu, LinkKind::Wireless, 0, "wireless-unplug-start");
    let gone = render(menu, LinkKind::Wireless, 400, "wireless-unplug-end");
    let (x, y) = (mid(), port_top() - 2);
    assert_ne!(
        art_at(&seated, x, y),
        GROUND,
        "no adapter on the port on the frame the unplug begins"
    );
    assert_eq!(
        art_at(&gone, x, y),
        GROUND,
        "the adapter never lifted off the port"
    );
}

/// The app with its link screen's faces in, as sync_link puts them: the art, the panel for what
/// it shows, and the bar.
struct Shown {
    faces: Vec<(TexId, Face)>,
}

impl Shown {
    fn new(app: &mut App) -> Shown {
        let (sprites, mut faces) = sprites_and_faces();
        app.set_link_sprites(sprites);
        let pal = palette(0);
        let width = out_w() / SPRITE_PX;
        let mut n = 500;
        let mut put = |face: UndoFace, faces: &mut Vec<(TexId, Face)>| {
            n += 1;
            let tex = TexId::from_raw(n);
            let (w, h) = (face.w, face.h);
            faces.push((tex, face.into()));
            (tex, w, h)
        };
        let sets = LinkActions::ALL.map(|a| {
            let [left, right] =
                action_pair(a.action(Side::Left), a.action(Side::Right), Font::Bold, pal);
            BarSet {
                layout: bar_layout(width, &left, &right),
                left: put(left.face, &mut faces),
                right: put(right.face, &mut faces),
            }
        });
        app.set_link_bar(LinkBar { sets, hints: None });
        if let Some(view) = app.link_view() {
            let (tex, w, h) = put(link_panel_face(&view, pal), &mut faces);
            app.set_link_panel(Lone { tex, w, h });
        }
        Shown { faces }
    }

    fn pixels(&self, app: &App, name: &str) -> Vec<u8> {
        let mut out = Vec::new();
        app.draw(&mut out);
        let px = composite(&out, &self.faces);
        dump(&px, name);
        px
    }
}

fn lit(px: &[u8], o: usize) -> bool {
    px[o] != GROUND[0] || px[o + 1] != GROUND[1] || px[o + 2] != GROUND[2]
}

fn ink(px: &[u8]) -> usize {
    (0..px.len() / 4).filter(|i| lit(px, i * 4)).count()
}

fn inked_rows(px: &[u8]) -> (usize, usize) {
    let rows: Vec<usize> = (0..out_h() as usize)
        .filter(|y| (0..out_w() as usize).any(|x| lit(px, (y * out_w() as usize + x) * 4)))
        .collect();
    (
        *rows.first().expect("nothing on the frame"),
        *rows.last().expect("nothing on the frame"),
    )
}

fn playing(title: &str, code: &str) -> (App, tempfile::TempDir) {
    let d = common::tmp_root_with_carts(&["Zzz"]);
    common::write_retail_header(&d, "Cart", title, code);
    let mut app = common::boot(d.path());
    app.apply(Action::Insert);
    app.set_core(Core::Armgba);
    app.on_core_ready();
    for _ in 0..120 {
        app.update(1.0 / 60.0);
    }
    (app, d)
}

#[test]
fn the_whole_link_screen_is_drawn_in_the_kit_over_the_backdrop() {
    let (mut app, _d) = playing("POKEMON RUBY", "AXVE");
    app.apply(Action::GameMenu);
    let shown = Shown::new(&mut app);
    let px = shown.pixels(&app, "screen-pick");
    let pal = palette(0);
    // the panel's frame at the top, the strip's lit edge over the bar, and the bar at the bottom
    let [panel_x, panel_y] = omega_zero::link_screen::panel_at(&Lone {
        tex: TexId::from_raw(1),
        w: omega_zero::link_screen::PANEL_W,
        h: 1,
    });
    let strip = panel_x + omega_zero::link_screen::PANEL_W as i32 - 4;
    assert_eq!(
        art_at(&px, strip, panel_y + 4),
        pal.frame_hi,
        "no LINK panel"
    );
    assert_eq!(art_at(&px, 0, port_top()), pal.frame, "no port's strip");
    let bottom = (out_h() / SPRITE_PX) as i32 - 1;
    assert_ne!(
        art_at(&px, 0, bottom),
        pal.well,
        "no bar over the strip's foot"
    );
}

#[test]
fn the_connected_screen_offers_return_and_end_link() {
    let (mut app, _d) = playing("POKEMON RUBY", "AXVE");
    app.begin_link(0);
    app.apply(Action::GameMenu);
    assert!(
        matches!(app.game_menu(), Some(GameMenu::Linked { opened: true, .. })),
        "the shortcut did not open the connected screen over a live session"
    );
    assert_eq!(app.link_bar_shown(), Some((LinkActions::Linked, 0)));
    assert_eq!(app.link_view().and_then(|v| v.status), Some("Linked"));
    let shown = Shown::new(&mut app);
    let px = shown.pixels(&app, "screen-connected");
    assert!(ink(&px) > 0);
}

#[test]
fn a_failed_link_says_why_and_offers_only_ok() {
    let (mut app, _d) = playing("POKEMON RUBY", "AXVE");
    app.apply(Action::GameMenu);
    app.start_link(
        LinkStarter::spawn_with(
            Box::new(|_, _| Ok(())),
            Box::new(|| {}),
            LinkRole::Host,
            0,
            Box::new(|_, _: &Cancel| Err(io::Error::new(io::ErrorKind::TimedOut, "from a test"))),
        ),
        0,
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while matches!(app.game_menu(), Some(GameMenu::Working { .. })) {
        app.update(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(1));
        assert!(Instant::now() < deadline, "the link never failed");
    }
    assert!(
        matches!(
            app.game_menu(),
            Some(GameMenu::Failed {
                fail: LinkFail::NobodyCame,
                ..
            })
        ),
        "{:?}",
        app.game_menu()
    );
    assert_eq!(
        app.link_view().and_then(|v| v.status),
        Some(LinkFail::NobodyCame.line())
    );
    for _ in 0..30 {
        app.update(1.0 / 60.0);
    }
    assert_eq!(app.link_bar_shown(), Some((LinkActions::Failed, 0)));
    let shown = Shown::new(&mut app);
    let px = shown.pixels(&app, "screen-failed");
    assert!(ink(&px) > 0);
}

const TOAST: TexId = TexId::from_raw(800);

/// `t` as the frontend draws it for the HUD, doubled as it shows on the screen, two pixels to
/// each of its art pixels.
fn toast_face(t: Toast) -> Face {
    let f = t.face(palette(0));
    let (w, h) = (f.w * 2, f.h * 2);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let at = (((y / 2) * f.w + x / 2) * 4) as usize;
            rgba.extend_from_slice(&f.rgba[at..at + 4]);
        }
    }
    Face { rgba, w, h }
}

fn banner_pixels(core: Core, title: &str, code: &str, name: &str) -> Vec<u8> {
    let d = common::tmp_root_with_carts(&["Zzz"]);
    common::write_retail_header(&d, "Cart", title, code);
    let mut app = common::boot(d.path());
    app.apply(Action::Insert);
    app.set_core(core);
    app.on_core_ready();
    for _ in 0..120 {
        app.update(1.0 / 60.0);
    }
    app.apply(Action::GameMenu);
    assert!(
        !app.game_menu_open(),
        "{code} on {core:?} opened a link screen instead of refusing"
    );
    let said = app.hud().last_toast().expect("the refusal said nothing");
    let face = toast_face(said);
    let (w, h) = (face.w / 2, face.h / 2);
    app.hud_mut()
        .set_toast_face(said, HudFace { tex: TOAST, w, h });
    let mut out = Vec::new();
    app.draw(&mut out);
    let banner: Vec<Draw> = out
        .into_iter()
        .filter(|d| matches!(*d, Draw::Tex { tex, .. } if tex == TOAST))
        .collect();
    let px = composite(&banner, &[(TOAST, face)]);
    dump(&px, name);
    px
}

fn banner_ink(t: Toast) -> usize {
    let f = toast_face(t);
    let (w, h) = (f.w as f32, f.h as f32);
    let draw = Draw::Tex {
        x: 100.0,
        y: 20.0,
        w,
        h,
        tex: TOAST,
        alpha: 1.0,
    };
    ink(&composite(&[draw], &[(TOAST, f)]))
}

#[test]
fn an_adapter_cart_on_mgba_reads_please_switch_to_armgba_on_the_glass() {
    let px = banner_pixels(Core::Mgba, "POKEMON EMER", "BPEE", "banner-emerald-mgba");
    assert!(
        ink(&px) > 0,
        "the refusal put no banner on the frame at all"
    );
    assert_eq!(
        ink(&px),
        banner_ink(Toast::NeedsArmgba),
        "the banner is not the PLEASE SWITCH TO ARMGBA line"
    );
    assert_ne!(
        banner_ink(Toast::NoLink),
        banner_ink(Toast::NeedsArmgba),
        "the two sentences carry the same ink, so this frame proves nothing"
    );
    let (first, last) = inked_rows(&px);
    assert!(
        last < out_h() as usize / 4,
        "the banner is not up in the plate band: rows {first}..{last}"
    );
}

#[derive(Clone, Default)]
struct FakeRadio {
    warm: Arc<AtomicBool>,
    asked: Arc<Mutex<Vec<RadioJob>>>,
}

impl RadioJobs for FakeRadio {
    fn ask(&mut self, job: RadioJob) {
        self.asked.lock().expect("radio log").push(job);
    }

    fn warmed(&self) -> bool {
        self.warm.load(Ordering::SeqCst)
    }
}

/// The LINK panel on the first step of a link, its radio `warm` or not, and what the screen asked
/// of the radio.
fn first_step_pixels(warm: bool, name: &str) -> (Vec<u8>, Option<&'static str>, Vec<RadioJob>) {
    let (mut app, _d) = playing("POKEMON RUBY", "AXVE");
    let radio = FakeRadio::default();
    radio.warm.store(warm, Ordering::SeqCst);
    app.set_radio_jobs(Box::new(radio.clone()));
    app.apply(Action::GameMenu);
    assert!(app.game_menu_open(), "the link screen never opened");
    let (release, held) = channel::<()>();
    app.start_link(
        LinkStarter::spawn_with(
            Box::new(move |_, _| {
                let _ = held.recv();
                Ok(())
            }),
            Box::new(|| {}),
            LinkRole::Host,
            0,
            Box::new(|_, _: &Cancel| Err(io::Error::new(io::ErrorKind::TimedOut, "from a test"))),
        ),
        0,
    );
    assert!(
        matches!(
            app.game_menu(),
            Some(GameMenu::Working {
                step: LinkStep::Radio,
                ..
            })
        ),
        "the screen is not on the step this is about: {:?}",
        app.game_menu()
    );
    let status = app.link_view().and_then(|v| v.status);
    let shown = Shown::new(&mut app);
    let px = shown.pixels(&app, name);
    drop(release);
    let asked = radio.asked.lock().expect("radio log").clone();
    (px, status, asked)
}

#[test]
fn a_cold_radio_says_it_is_bringing_the_radio_up() {
    let (px, status, asked) = first_step_pixels(false, "step-radio-cold");
    assert!(ink(&px) > 0, "the screen drew nothing at all");
    assert!(
        asked.contains(&RadioJob::Warm),
        "the screen never asked for a warm, so this frame says nothing about finishing one"
    );
    assert_eq!(
        status,
        Some(LinkStep::Radio.line()),
        "a screen whose warm has not finished is not saying the radio is coming up"
    );
}

#[test]
fn a_warm_radio_goes_straight_to_looking_for_the_other_player() {
    let (px, status, _) = first_step_pixels(true, "step-radio-warm");
    assert_eq!(
        status,
        Some(LinkStep::Waiting.line()),
        "a warm radio is still being announced as coming up"
    );
    let (first, _) = inked_rows(&px);
    assert!(
        first < out_h() as usize / 4,
        "the panel is not up at the top"
    );
}

#[test]
fn the_status_is_a_line_in_the_panels_well_that_grows_it() {
    let pal = palette(0);
    let view = |status: Option<&'static str>| omega_zero::link_screen::LinkView {
        role: LinkRow::Host,
        kind: LinkKind::Cable,
        picking: status.is_none(),
        status,
    };
    let bare = link_panel_face(&view(None), pal);
    let told = link_panel_face(&view(Some(LinkStep::Waiting.line())), pal);
    assert_eq!(bare.w, told.w);
    assert!(told.h > bare.h, "the status took no room");
    let dim = told
        .rgba
        .chunks_exact(4)
        .filter(|p| p[..3] == pal.dim)
        .count();
    assert!(dim > 0, "the status is not in the dim colour of a note");
}
