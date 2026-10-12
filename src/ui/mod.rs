pub mod hotbar;
pub mod hud;
pub mod menu;
pub mod settings;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hotbar::HotbarPlugin,
            hud::HudPlugin,
            menu::MenuPlugin,
            settings::SettingsPlugin,
        ));
        info!("UiPlugin loaded.");
    }
}