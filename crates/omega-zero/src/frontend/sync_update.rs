//! The UPDATE panel, baked again whenever what it shows or the palette changes.

use omega_gfx::Compositor;

use crate::app::App;
use crate::frontend::lone::Baked;
use crate::list_view::ListView;
use crate::screens::update::update_face;

pub(crate) fn sync_update(
    app: &mut App,
    compositor: &mut Compositor,
    panel: &mut Baked<(ListView, u8)>,
) {
    let Some(view) = app.update_view() else {
        return;
    };
    let pal = app.palette();
    let from = (view, app.palette_colour());
    if let Some(baked) = panel.bake(compositor, from, |(view, _)| update_face(view, pal)) {
        app.set_update_panel(baked);
    }
}
