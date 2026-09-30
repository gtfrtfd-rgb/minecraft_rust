use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use super::chunk::{BlockType, Chunk};
use crate::core::state::{WorldSeed, SX, SZ, SY, CHUNK_SIZE};

pub struct WorldGeneratorPlugin;

impl Plugin for WorldGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_terrain);
    }
}

fn generate_terrain(mut commands: Commands, seed: Res<WorldSeed>) {
    let perlin = Perlin::new(seed.0);
    let mut chunk_count = 0;

    for cx in 0..(SX / CHUNK_SIZE) {
        for cz in 0..(SZ / CHUNK_SIZE) {
            let mut chunk = Chunk::new(cx, cz);
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let wx = (cx * CHUNK_SIZE + x) as f64;
                    let wz = (cz * CHUNK_SIZE + z) as f64;
                    let n = perlin.get([wx / 64.0, wz / 64.0]);
                    let height = ((n * 16.0 + 16.0) as i32).clamp(3, SY - 5);
                    for y in 0..height {
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
            chunk_count += 1;
        }
    }
    info!("Generated {} chunks", chunk_count);
}