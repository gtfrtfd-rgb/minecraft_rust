use bevy::prelude::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, _app: &mut App) {
        info!("HudPlugin loaded (stub).");
    }
}