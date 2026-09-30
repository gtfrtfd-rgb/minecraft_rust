pub mod generator;
pub mod chunk;

use bevy::prelude::*;
use chunk::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            generator::WorldGeneratorPlugin,
            chunk::ChunkPlugin,
        ));
        info!("WorldPlugin loaded.");
    }
}