pub mod chunk;
pub mod generator;

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((chunk::ChunkPlugin, generator::WorldGeneratorPlugin));
        info!("WorldPlugin loaded.");
    }
}