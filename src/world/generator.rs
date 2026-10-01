use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use super::chunk::{BlockType, WorldData};
use crate::core::state::{WorldSeed, SX, SZ, SY};
use crate::save::persistence::{LoadedSave, rle_decode};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
enum Biome {
    Plains,
    Forest,
    Desert,
    Snow,
    Mountain,
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self { Self { state: seed | 1 } }
    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        (self.state as f32) / (0x7fffffff as f32)
    }
    fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        if max <= min { return min; }
        min + (self.next_f32() * (max - min) as f32) as i32
    }
}

pub struct WorldGeneratorPlugin;

impl Plugin for WorldGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_or_generate);
    }
}

fn load_or_generate(
    mut world: ResMut<WorldData>,
    mut seed: ResMut<WorldSeed>,
    loaded: Res<LoadedSave>,
) {
    if let (Some(meta), Some(rle)) = (&loaded.meta, &loaded.rle_bytes) {
        info!("Applying save data...");
        let t0 = std::time::Instant::now();

        seed.0 = meta.seed;
        world.blocks = rle_decode(rle, (SX * SY * SZ) as usize);

        let non_air = world.blocks.iter().filter(|b| b.is_solid()).count();
        info!(
            "Loaded world from save: seed={}, {} solid blocks in {:?}",
            meta.seed, non_air, t0.elapsed()
        );
        return;
    }

    info!("Generating new terrain (seed = {})...", seed.0);
    let t0 = std::time::Instant::now();

    let base_perlin = Perlin::new(seed.0);
    let temp_noise  = Perlin::new(seed.0.wrapping_add(300));
    let humid_noise = Perlin::new(seed.0.wrapping_add(400));
    let mount_noise = Perlin::new(seed.0.wrapping_add(500));

    let mut heights: Vec<i32> = vec![0; (SX * SZ) as usize];
    let mut biomes:  Vec<Biome> = vec![Biome::Plains; (SX * SZ) as usize];

    let idx2 = |x: i32, z: i32| -> usize { (z * SX + x) as usize };

    for z in 0..SZ {
        for x in 0..SX {
            let fx = x as f64;
            let fz = z as f64;
            let wx = fx + (base_perlin.get([fx / 40.0, fz / 40.0 + 700.0]) * 15.0);
            let wz = fz + (base_perlin.get([fx / 40.0 + 800.0, fz / 40.0]) * 15.0);

            let mut h: f64 = 0.0;
            h += base_perlin.get([wx / 128.0, wz / 128.0]) * 16.0;
            h += base_perlin.get([wx / 64.0,  wz / 64.0])  * 8.0;
            h += base_perlin.get([wx / 32.0,  wz / 32.0])  * 4.0;
            h += base_perlin.get([wx / 16.0,  wz / 16.0])  * 2.0;
            h += base_perlin.get([wx / 8.0,   wz / 8.0])   * 1.0;
            h += base_perlin.get([wx / 4.0,   wz / 4.0])   * 0.5;

            let mut height = (h + 6.0) as i32;
            height = height.clamp(4, SY - 12);
            heights[idx2(x, z)] = height;

            let temp  = (temp_noise.get([fx / 180.0, fz / 180.0]) * 0.5 + 0.5) as f32;
            let humid = (humid_noise.get([fx / 140.0, fz / 140.0]) * 0.5 + 0.5) as f32;
            let mount = (mount_noise.get([fx / 90.0, fz / 90.0]) * 0.5 + 0.5) as f32;

            let biome = if height > 30 || mount > 0.72 {
                Biome::Mountain
            } else if temp < 0.34 {
                Biome::Snow
            } else if temp > 0.66 && humid < 0.42 {
                Biome::Desert
            } else if humid > 0.58 {
                Biome::Forest
            } else {
                Biome::Plains
            };
            biomes[idx2(x, z)] = biome;
        }
    }

    let mut non_air = 0u64;
    for z in 0..SZ {
        for x in 0..SX {
            let height = heights[idx2(x, z)];
            let biome = biomes[idx2(x, z)];
            for y in 0..=height {
                let block: BlockType = if y <= 1 {
                    BlockType::Obsidian
                } else if y < height - 3 {
                    if biome == Biome::Mountain && y < height - 6 {
                        let rv = (base_perlin.get([
                            (x as f64) * 0.7 + (y as f64) * 3.1,
                            (z as f64) * 0.7 + (y as f64) * 1.3,
                        ]) * 0.5 + 0.5) as f32;
                        if rv > 0.985 { BlockType::Obsidian } else { BlockType::Stone }
                    } else {
                        BlockType::Stone
                    }
                } else if y < height {
                    match biome {
                        Biome::Desert => BlockType::Sand,
                        Biome::Snow => BlockType::Dirt,
                        Biome::Mountain => BlockType::Stone,
                        _ => BlockType::Dirt,
                    }
                } else {
                    match biome {
                        Biome::Desert => BlockType::Sand,
                        Biome::Snow => BlockType::Snow,
                        Biome::Mountain => {
                            if height > 32 { BlockType::Snow } else { BlockType::Stone }
                        }
                        _ => BlockType::Grass,
                    }
                };
                world.set(x, y, z, block);
                non_air += 1;
            }
        }
    }

    let mut rng = Rng::new(seed.0 as u64);
    let mut trees_planted = 0;
    trees_planted += plant_trees(&mut world, &heights, &biomes, &mut rng,
        250, Biome::Forest, TreeKind::Oak);
    trees_planted += plant_trees(&mut world, &heights, &biomes, &mut rng,
        60, Biome::Plains, TreeKind::Oak);
    trees_planted += plant_trees(&mut world, &heights, &biomes, &mut rng,
        100, Biome::Snow, TreeKind::Pine);
    trees_planted += plant_trees(&mut world, &heights, &biomes, &mut rng,
        30, Biome::Mountain, TreeKind::Pine);

    // НИКАКОЙ стены — физический барьер обеспечивает
    // WorldData::is_solid() (возвращает true за пределами мира).
    // Визуально граница скрыта туманом (см. camera.rs).

    info!(
        "Terrain generated in {:?}: {} blocks, {} trees",
        t0.elapsed(),
        non_air,
        trees_planted
    );
}

#[derive(Clone, Copy)]
enum TreeKind { Oak, Pine }

fn plant_trees(
    world: &mut WorldData,
    heights: &[i32],
    biomes: &[Biome],
    rng: &mut Rng,
    count: i32,
    target: Biome,
    kind: TreeKind,
) -> i32 {
    let idx2 = |x: i32, z: i32| -> usize { (z * SX + x) as usize };
    let mut planted = 0;
    let mut attempts = 0;
    let max_attempts = count * 6;

    while planted < count && attempts < max_attempts {
        attempts += 1;
        let x = rng.range_i32(4, SX - 4);
        let z = rng.range_i32(4, SZ - 4);
        if biomes[idx2(x, z)] != target { continue; }
        let h = heights[idx2(x, z)];
        if target == Biome::Mountain && h > 28 { continue; }

        let top = world.get(x, h, z);
        let ok_top = matches!(top, BlockType::Grass | BlockType::Dirt | BlockType::Snow);
        if !ok_top { continue; }

        let mut blocked = false;
        'outer: for dx in -2i32..=2 {
            for dz in -2i32..=2 {
                if world.get(x + dx, h + 4, z + dz) == BlockType::Log {
                    blocked = true;
                    break 'outer;
                }
            }
        }
        if blocked { continue; }

        match kind {
            TreeKind::Oak => build_oak(world, rng, x, h + 1, z),
            TreeKind::Pine => build_pine(world, rng, x, h + 1, z),
        }
        planted += 1;
    }
    planted
}

fn build_oak(world: &mut WorldData, rng: &mut Rng, x: i32, y: i32, z: i32) {
    let trunk_h = rng.range_i32(4, 7);
    for i in 0..trunk_h {
        world.set(x, y + i, z, BlockType::Log);
    }
    let top = y + trunk_h;

    for dy in -2i32..2 {
        let r: i32 = if dy <= -1 { 2 } else { 1 };
        for dx in -r..=r {
            for dz in -r..=r {
                if dx == 0 && dz == 0 && dy < 1 { continue; }
                if dx.abs() == r && dz.abs() == r && rng.next_f32() < 0.65 { continue; }
                let bx = x + dx;
                let by = top + dy;
                let bz = z + dz;
                if !WorldData::in_bounds(bx, by, bz) { continue; }
                if world.get(bx, by, bz) == BlockType::Air {
                    world.set(bx, by, bz, BlockType::Leaves);
                }
            }
        }
    }
    if WorldData::in_bounds(x, top + 1, z) {
        if world.get(x, top + 1, z) == BlockType::Air {
            world.set(x, top + 1, z, BlockType::Leaves);
        }
    }
}

fn build_pine(world: &mut WorldData, rng: &mut Rng, x: i32, y: i32, z: i32) {
    let trunk_h = rng.range_i32(6, 9);
    for i in 0..trunk_h {
        world.set(x, y + i, z, BlockType::Log);
    }
    let top = y + trunk_h;

    for layer in 0i32..5 {
        let r: i32 = std::cmp::max(0, 2 - (layer / 2));
        let ly = top + layer - 4;
        if ly < 0 || ly >= SY { continue; }
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs() + dz.abs() > r + 1 { continue; }
                let bx = x + dx;
                let bz = z + dz;
                if !WorldData::in_bounds(bx, ly, bz) { continue; }
                if world.get(bx, ly, bz) == BlockType::Air {
                    world.set(bx, ly, bz, BlockType::Leaves);
                }
            }
        }
    }
}