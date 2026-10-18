pub mod chunk;
pub mod generator;
pub mod grass;
pub mod sky;
pub mod face_textures;
pub mod greedy;

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            chunk::ChunkPlugin,
            generator::WorldGeneratorPlugin,
            grass::GrassSpreadPlugin,
            sky::SkyPlugin,
        ));
        info!("WorldPlugin loaded.");
    }
}