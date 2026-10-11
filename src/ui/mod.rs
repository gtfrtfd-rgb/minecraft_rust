pub mod hotbar;
pub mod hud;
pub mod menu;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hotbar::HotbarPlugin,
            hud::HudPlugin,
            menu::MenuPlugin,
        ));
        info!("UiPlugin loaded.");
    }
}