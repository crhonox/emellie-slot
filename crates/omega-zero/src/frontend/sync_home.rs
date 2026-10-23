//! The Collections and Collect panels, baked again whenever what they show or the palette
//! changes. The main menu's pictures are made with the grid's (sync_systems.rs).

use omega_gfx::Compositor;

use crate::app::App;
use crate::frontend::lone::Baked;
use crate::list_view::ListView;
use crate::screens::home::list_face;

pub(crate) fn sync_list_panel(
    app: &mut App,
    compositor: &mut Compositor,
    panel: &mut Baked<(ListView, &'static str, u8)>,
) {
    let (view, title) = match (app.collections_view(), app.collect_view()) {
        (Some(view), _) => (view, "COLLECTIONS"),
        (None, Some(view)) => (view, "ADD TO"),
        _ => return,
    };
    let pal = app.palette();
    let from = (view, title, app.palette_colour());
    if let Some(baked) = panel.bake(compositor, from, |(view, title, _)| {
        list_face(view, title, pal)
    }) {
        app.set_list_panel(baked);
    }
}
