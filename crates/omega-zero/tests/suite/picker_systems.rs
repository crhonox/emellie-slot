//! The core picker on the systems beyond the GBA: a SNES cart opens over a board with a socket
//! each for Snes9x and Supafaust, wider than the GBA's for their names, and picks between them as
//! a GBA cart picks between armGBA and mGBA; a PlayStation game has one core, and X opens nothing.

use crate::keys::root_with_games;

use omega_input::{Action, Btn};
use omega_store::{core_for_platform, write_app_state, AppState, Core, Platform};
use omega_ui::{board_at, on_board, Draw, Placed, TexId, BOARD_W, SNES_SOCKETS};
use omega_zero::app::App;
use tempfile::TempDir;

fn on_shelf(d: TempDir) -> (TempDir, App) {
    write_app_state(
        d.path(),
        &AppState {
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let app = App::boot(d.path());
    (d, app)
}

const SOCKET: usize = 910;
const CHIP: usize = 920;
const BLANK: usize = 906;
const SHADOW: usize = 907;
const WIDE_BLANK: usize = 930;
const WIDE_SHADOW: usize = 931;

fn id(n: usize) -> TexId {
    TexId::from_raw(n)
}

/// Every part's face, a socket and a chip for each core in `Core::ALL`'s order, and the SNES
/// board's own blank chip and shadow.
fn fake_faces(app: &mut App) {
    let each = |from: usize| (0..Core::ALL.len()).map(|i| id(from + i)).collect();
    app.set_core_part_faces(each(SOCKET), each(CHIP), id(BLANK), [id(SHADOW), id(908)]);
    app.set_chip_faces_for(SNES_SOCKETS, id(WIDE_BLANK), id(WIDE_SHADOW));
    app.set_core_board_faces(id(900), id(901));
}

fn frame(app: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    app.draw(&mut out);
    out
}

fn tex_at(out: &[Draw], want: TexId) -> Option<[f32; 4]> {
    out.iter().find_map(|d| match *d {
        Draw::Tex {
            x, y, w, h, tex, ..
        } if tex == want => Some([x, y, w, h]),
        _ => None,
    })
}

fn even(px: f32) -> f32 {
    (px / 2.0).round() * 2.0
}

/// Where a part `w` by `h` art pixels with its top left at `u, v` on the board is drawn.
fn part_at(board: Placed, u: f32, v: f32, w: u32, h: u32) -> [f32; 4] {
    let px = board.w / BOARD_W as f32;
    let (x, y) = on_board(board, u, v);
    [even(x), even(y), w as f32 * px, h as f32 * px]
}

fn near(a: [f32; 4], b: [f32; 4]) -> bool {
    a.iter().zip(b).all(|(p, q)| (p - q).abs() < 0.01)
}

#[test]
fn x_on_a_snes_cart_opens_a_socket_for_snes9x_and_one_for_supafaust() {
    let (_d, mut app) = on_shelf(root_with_games(Platform::Snes, &["Super Metroid", "Zelda"]));
    fake_faces(&mut app);
    app.apply(Action::GbaDown(Btn::X));
    assert_eq!(
        app.core_picker(),
        Some(Core::Snes9x),
        "a SNES cart with no line of its own runs Snes9x"
    );
    app.update(0.6);
    let out = frame(&app);
    let board = board_at(1.0);
    let s = SNES_SOCKETS;
    for (slot, core) in [Core::Snes9x, Core::Supafaust].into_iter().enumerate() {
        let at = tex_at(&out, id(SOCKET + core.index()))
            .unwrap_or_else(|| panic!("{core:?}'s socket is missing"));
        assert!(
            near(at, part_at(board, s.u[slot], s.v, s.w, s.h)),
            "{core:?}'s socket at {at:?}"
        );
    }
    for core in [Core::Mgba, Core::Armgba, Core::PcsxRearmed] {
        assert_eq!(
            tex_at(&out, id(SOCKET + core.index())),
            None,
            "the SNES board has a socket for {core:?}"
        );
    }
    let chip = tex_at(&out, id(CHIP + Core::Snes9x.index())).expect("no seated chip");
    assert!(
        near(
            chip,
            part_at(board, s.chip_u(0), s.chip_v(), s.chip_w, s.chip_h)
        ),
        "the chip is not in Snes9x's socket: {chip:?}"
    );
}

#[test]
fn right_and_a_on_a_snes_cart_write_supafaust() {
    let (d, mut app) = on_shelf(root_with_games(Platform::Snes, &["Super Metroid", "Zelda"]));
    fake_faces(&mut app);
    app.apply(Action::GbaDown(Btn::X));
    app.update(0.6);
    app.apply(Action::GbaDown(Btn::Left));
    assert_eq!(app.core_picker(), Some(Core::Snes9x), "left wrapped round");
    assert_ne!(app.core_picker_chip().unwrap().shake, 0.0);

    app.apply(Action::GbaDown(Btn::Right));
    assert_eq!(app.core_picker(), Some(Core::Supafaust));
    app.update(0.09);
    let out = frame(&app);
    assert!(
        tex_at(&out, id(WIDE_BLANK)).is_some(),
        "the chip in the air is not the SNES board's blank one"
    );
    assert_eq!(tex_at(&out, id(BLANK)), None, "the GBA's blank chip flew");
    let shadow = tex_at(&out, id(WIDE_SHADOW)).expect("no shadow under the chip in the air");
    assert_eq!(shadow[2], (SNES_SOCKETS.shadow_w() * 2) as f32);
    assert_eq!(tex_at(&out, id(SHADOW)), None);

    app.apply(Action::GbaDown(Btn::A));
    assert_eq!(
        core_for_platform(d.path(), "Super Metroid", Platform::Snes),
        Core::Supafaust
    );
    app.update(0.4);
    assert_eq!(app.core_picker(), None, "the picker stayed open");

    app.apply(Action::GbaDown(Btn::X));
    assert_eq!(
        app.core_picker(),
        Some(Core::Supafaust),
        "the chip did not start in the core the cart now uses"
    );
}

#[test]
fn x_on_a_playstation_game_opens_nothing() {
    let (_d, mut app) = on_shelf(root_with_games(Platform::Ps1, &["Ridge Racer", "Wipeout"]));
    fake_faces(&mut app);
    let before = frame(&app);
    app.apply(Action::GbaDown(Btn::X));
    assert_eq!(
        app.core_picker(),
        None,
        "a game with one core opened a picker"
    );
    assert_eq!(app.shelf_shake(), 0.0, "X was answered with a refusal");
    assert_eq!(frame(&app), before, "X changed what the shelf draws");
}
