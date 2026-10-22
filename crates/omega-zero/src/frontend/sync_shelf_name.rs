//! The selected cart's file name over the shelf, baked again only for another cart or palette.

use omega_gfx::Compositor;

use crate::app::App;
use crate::frontend::lone::Baked;
use crate::screens::shelf::shelf_name_face;

pub(crate) fn sync_shelf_name(
    app: &mut App,
    compositor: &mut Compositor,
    held: &mut Baked<(String, u8)>,
) {
    let Some(stem) = app.selected_stem().map(str::to_owned) else {
        return;
    };
    let pal = app.palette();
    if let Some(name) = held.bake(
        compositor,
        (stem.clone(), app.palette_colour()),
        |(stem, _)| shelf_name_face(stem, pal),
    ) {
        app.set_shelf_name(stem, name);
    }
}
