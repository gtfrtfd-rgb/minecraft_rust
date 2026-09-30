pub mod hotbar;
pub mod hud;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hotbar::HotbarPlugin,
            hud::HudPlugin,
        ));
        info!("UiPlugin loaded.");
    }
}