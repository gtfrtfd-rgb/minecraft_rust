use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use super::chunk::{Chunk, BlockType};
use crate::core::state::{SX, SY, SZ, CHUNK_SIZE};

pub struct WorldGeneratorPlugin;

impl Plugin for WorldGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_terrain);
    }
}

fn generate_terrain(
    mut commands: Commands,
    seed: Res<crate::core::state::WorldSeed>,
) {
    let perlin = Perlin::new(seed.0);
    for cx in 0..(SX / CHUNK_SIZE) {
        for cz in 0..(SZ / CHUNK_SIZE) {
            let mut chunk = Chunk::new(cx, cz);
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let world_x = cx * CHUNK_SIZE + x;
                    let world_z = cz * CHUNK_SIZE + z;
                    let noise_val = perlin.get([world_x as f64 / 64.0, world_z as f64 / 64.0]);
                    let height = (noise_val * 16.0 + 16.0) as i32;
                    for y in 0..height.min(SY) {
                        let block = match y {
                            0..=2 => BlockType::Stone,
                            _ if y == height - 1 => BlockType::Grass,
                            _ => BlockType::Dirt,
                        };
                        chunk.set_block(x, y, z, block);
                    }
                }
            }
            commands.spawn(chunk);
        }
    }
    info!("Terrain generation finished.");
}