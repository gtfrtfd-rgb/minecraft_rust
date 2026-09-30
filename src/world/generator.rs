use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use super::chunk::{BlockType, WorldData};
use crate::core::state::{WorldSeed, SX, SZ, SY};

pub struct WorldGeneratorPlugin;

impl Plugin for WorldGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_terrain);
    }
}

fn generate_terrain(mut world: ResMut<WorldData>, seed: Res<WorldSeed>) {
    info!("Generating terrain (seed = {})...", seed.0);
    let t0 = std::time::Instant::now();

    let perlin = Perlin::new(seed.0);
    let mut non_air = 0u64;

    for z in 0..SZ {
        for x in 0..SX {
            let n = perlin.get([x as f64 / 64.0, z as f64 / 64.0]);
            let height = ((n * 12.0 + 18.0) as i32).clamp(3, SY - 5);

            for y in 0..height {
                let block = if y < height - 3 {
                    BlockType::Stone
                } else if y == height - 1 {
                    BlockType::Grass
                } else {
                    BlockType::Dirt
                };
                world.set(x, y, z, block);
                non_air += 1;
            }
        }
    }

    info!(
        "Terrain generated in {:?}: {} blocks",
        t0.elapsed(),
        non_air
    );
}