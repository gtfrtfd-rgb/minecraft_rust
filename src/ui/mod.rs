pub mod hotbar;
pub mod hud;
pub mod keybinds;
pub mod menu;
pub mod settings;
pub mod worlds;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hotbar::HotbarPlugin,
            hud::HudPlugin,
            menu::MenuPlugin,
            settings::SettingsPlugin,
            keybinds::KeybindsPlugin,
            worlds::WorldsPlugin,
        ));
        info!("UiPlugin loaded.");
    }
}