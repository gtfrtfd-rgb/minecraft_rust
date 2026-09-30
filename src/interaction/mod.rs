pub mod breaking;
pub mod raycast;

use bevy::prelude::*;

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((raycast::RaycastPlugin, breaking::BreakingPlugin));
        info!("InteractionPlugin loaded.");
    }
}