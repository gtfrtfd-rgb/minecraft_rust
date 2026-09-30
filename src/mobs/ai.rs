use bevy::prelude::*;

pub struct MobAiPlugin;

impl Plugin for MobAiPlugin {
    fn build(&self, _app: &mut App) {
        info!("MobAiPlugin loaded (stub).");
    }
}