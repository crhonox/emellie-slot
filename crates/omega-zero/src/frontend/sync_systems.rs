//! The menu's pictures, made once for the canvas from its theme (menu_theme.rs): the
//! background, the frame, each system's icon and name for the grid, and each entry's icon and
//! name for the main menu, plain and lit.

use omega_gfx::Compositor;
use omega_ui::{out_h, out_w};

use crate::app::App;
use crate::menu_theme::MenuTheme;
use crate::screens::home::HomeFaces;
use crate::screens::systems::{Face, SystemsFaces};
use crate::spruce::Picture;

fn upload(compositor: &mut Compositor, picture: &Picture) -> Face {
    Face {
        tex: compositor.create_texture(picture.w, picture.h, &picture.rgba),
        w: picture.w,
        h: picture.h,
    }
}

/// Makes the menu's pictures the first time a card with a menu needs them on this canvas.
pub(crate) fn sync_systems(app: &mut App, compositor: &mut Compositor) {
    let canvas = (out_w(), out_h());
    if !app.uses_menu() || app.systems_faces().is_some_and(|f| f.canvas == canvas) {
        return;
    }
    let theme = MenuTheme::load(app.root(), canvas.0, canvas.1);
    let mut up = |p: &Picture| upload(compositor, p);
    let icons = theme
        .icons
        .iter()
        .map(|(p, plain, lit)| (*p, up(plain), up(lit)))
        .collect();
    let mut names = Vec::new();
    let mut entry_names = Vec::new();
    if theme.names {
        for (platform, _, _) in &theme.icons {
            let name = crate::spruce::system_name(*platform);
            names.push((
                *platform,
                up(&theme.text(name, theme.grid_text_px, theme.text)),
                up(&theme.text(name, theme.grid_text_px, theme.text_lit)),
            ));
        }
        for (entry, _, _) in &theme.entries {
            entry_names.push((
                *entry,
                up(&theme.text(entry.name(), theme.home_text_px, theme.text)),
                up(&theme.text(entry.name(), theme.home_text_px, theme.text_lit)),
            ));
        }
    }
    let entries = theme
        .entries
        .iter()
        .map(|(e, plain, lit)| (*e, up(plain), up(lit)))
        .collect();
    let faces = SystemsFaces {
        canvas,
        background: theme.background.as_ref().map(&mut up),
        frame: theme.frame.as_ref().map(&mut up),
        icons,
        names,
        text_h: theme.line_height(theme.grid_text_px),
        cols: theme.grid_cols,
        rows: theme.grid_rows,
    };
    app.set_systems_faces(faces);
    app.set_home_faces(HomeFaces {
        canvas,
        icons: entries,
        names: entry_names,
    });
}
