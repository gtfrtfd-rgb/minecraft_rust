use bevy::prelude::*;

pub struct MobSpawnPlugin;

impl Plugin for MobSpawnPlugin {
    fn build(&self, _app: &mut App) {
        info!("MobSpawnPlugin loaded (stub).");
    }
}