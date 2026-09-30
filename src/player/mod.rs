pub mod controller;
pub mod camera;

use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            controller::PlayerControllerPlugin,
            camera::PlayerCameraPlugin,
        ));
        info!("PlayerPlugin loaded.");
    }
}