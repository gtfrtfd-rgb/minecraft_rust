pub mod ai;
pub mod spawn;

use bevy::prelude::*;

pub struct MobPlugin;

impl Plugin for MobPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ai::MobAiPlugin,
            spawn::MobSpawnPlugin,
        ));
        info!("MobPlugin loaded.");
    }
}