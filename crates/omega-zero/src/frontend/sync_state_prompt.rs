//! The screen prompt box's faces, made while it is open: its panel, when what it asks or the
//! palette changes, and the bar's actions, when the palette or the font does.

use omega_gfx::{out_w, Compositor, TexId};
use omega_ui::kit::action::{action_pair, Font, Side};
use omega_ui::kit::bar::bar_layout;
use omega_ui::{UndoFace, SPRITE_PX};

use crate::app::App;
use crate::disc_screen::DiscBar;
use crate::frontend::faces::upload_nearest;
use crate::frontend::lone::Baked;
use crate::screens::bar::BarSet;
use crate::state_prompt::{prompt_action, prompt_panel_face, PromptView};

#[derive(Default)]
pub(crate) struct PromptTexes {
    panel: Baked<(PromptView, u8)>,
    actions: [Option<TexId>; 2],
    built: Option<(u8, Font)>,
}

pub(crate) fn sync_state_prompt(
    app: &mut App,
    compositor: &mut Compositor,
    texes: &mut PromptTexes,
) {
    let Some(view) = app.prompt_view() else {
        return;
    };
    let (colour, font, pal) = (app.palette_colour(), app.font(), app.palette());
    if let Some(panel) = texes.panel.bake(compositor, (view, colour), |(view, _)| {
        prompt_panel_face(view, pal)
    }) {
        app.set_prompt_panel(panel);
    }
    if texes.built == Some((colour, font)) {
        return;
    }
    let [left, right] = action_pair(
        prompt_action(Side::Left),
        prompt_action(Side::Right),
        font,
        pal,
    );
    let layout = bar_layout(out_w() / SPRITE_PX, &left, &right);
    let mut up = |i: usize, face: &UndoFace| {
        (
            upload_nearest(compositor, &mut texes.actions[i], face),
            face.w,
            face.h,
        )
    };
    let set = BarSet {
        layout,
        left: up(0, &left.face),
        right: up(1, &right.face),
    };
    app.set_prompt_bar(DiscBar { set, hints: None });
    texes.built = Some((colour, font));
}
