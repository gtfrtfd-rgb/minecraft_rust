use bevy::prelude::*;

pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, _app: &mut App) {
        info!("RaycastPlugin loaded (stub).");
    }
}