use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use super::chunk::{BlockType, ChunkData};
use crate::core::state::{WorldSeed, CHUNK_SIZE, SY};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Biome {
    Plains,
    Forest,
    Desert,
    Snow,
    Mountain,
}

// ============================================================
// КЭШ PERLIN
// ============================================================
//
// Perlin::new() строит таблицу пермутаций (~256 байт) и заполняет
// её через shuffle. Это недорого, но при 100+ чанках в секунду
// (async генерация) суммарно получается заметная нагрузка.
//
// Кэшируем 4 Perlin'а (base / temp / humid / mount) в одном
// ресурсе. При изменении seed (загрузка другого мира) — пересоздаём.
//
// Perlin реализует Copy, так что клонирование в задачу бесплатно.

#[derive(Clone, Copy)]
pub struct PerlinCache {
    pub seed: u32,
    pub base: Perlin,
    pub temp: Perlin,
    pub humid: Perlin,
    pub mount: Perlin,
}

impl PerlinCache {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            base: Perlin::new(seed),
            temp: Perlin::new(seed.wrapping_add(300)),
            humid: Perlin::new(seed.wrapping_add(400)),
            mount: Perlin::new(seed.wrapping_add(500)),
        }
    }
}

#[derive(Resource)]
pub struct PerlinCacheRes(pub PerlinCache);

impl Default for PerlinCacheRes {
    fn default() -> Self {
        // Дефолт с сидом 0. Реальная инициализация — в init_perlin_cache.
        Self(PerlinCache::new(0))
    }
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
        app.init_resource::<PerlinCacheRes>()
            .add_systems(Startup, init_perlin_cache)
            .add_systems(Update, update_perlin_cache);
        info!("WorldGeneratorPlugin loaded (per-chunk generation, cached Perlin).");
    }
}

/// Создаёт кэш с настоящим сидом на старте.
fn init_perlin_cache(
    seed: Res<WorldSeed>,
    mut cache: ResMut<PerlinCacheRes>,
) {
    cache.0 = PerlinCache::new(seed.0);
    info!("Perlin cache initialized for seed {}", seed.0);
}

/// Пересоздаёт кэш при смене seed (загрузка другого мира).
fn update_perlin_cache(
    seed: Res<WorldSeed>,
    mut cache: ResMut<PerlinCacheRes>,
) {
    if !seed.is_changed() { return; }
    cache.0 = PerlinCache::new(seed.0);
    info!("Perlin cache rebuilt for seed {}", seed.0);
}

/// Генерирует один чанк (cx, cz) с данным кэшем Perlin.
/// Детерминирован: те же (cache, cx, cz) → тот же результат.
pub fn generate_chunk(cache: &PerlinCache, cx: i32, cz: i32) -> ChunkData {
    let mut chunk = ChunkData::new();

    let base_perlin = &cache.base;
    let temp_noise  = &cache.temp;
    let humid_noise = &cache.humid;
    let mount_noise = &cache.mount;

    let base_x = cx * CHUNK_SIZE;
    let base_z = cz * CHUNK_SIZE;

    // 1. Считаем высоты и биомы для этого чанка.
    let mut heights = [[0i32; 16]; 16];
    let mut biomes  = [[Biome::Plains; 16]; 16];

    for lz in 0..16usize {
        for lx in 0..16usize {
            let fx = (base_x + lx as i32) as f64;
            let fz = (base_z + lz as i32) as f64;
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
            heights[lz][lx] = height;

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
            biomes[lz][lx] = biome;
        }
    }

    // 2. Заполняем блоки.
    for lz in 0..16usize {
        for lx in 0..16usize {
            let height = heights[lz][lx];
            let biome = biomes[lz][lx];
            for y in 0..=height {
                let block: BlockType = if y <= 1 {
                    BlockType::Obsidian
                } else if y < height - 3 {
                    if biome == Biome::Mountain && y < height - 6 {
                        let wx = (base_x + lx as i32) as f64;
                        let wz = (base_z + lz as i32) as f64;
                        let rv = (base_perlin.get([
                            wx * 0.7 + (y as f64) * 3.1,
                            wz * 0.7 + (y as f64) * 1.3,
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
                chunk.set_local(lx as i32, y, lz as i32, block);
            }
        }
    }

    // 3. Деревья.
    let chunk_seed = (cache.seed as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((cx as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((cz as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    let mut rng = Rng::new(chunk_seed);

    let attempts = 8;
    for _ in 0..attempts {
        let lx = rng.range_i32(3, 13) as usize;
        let lz = rng.range_i32(3, 13) as usize;
        let biome = biomes[lz][lx];
        let h = heights[lz][lx];

        if biome == Biome::Desert { continue; }
        if biome == Biome::Mountain && h > 28 { continue; }

        let top = chunk.get_local(lx as i32, h, lz as i32);
        let ok_top = matches!(top, BlockType::Grass | BlockType::Dirt | BlockType::Snow);
        if !ok_top { continue; }

        let mut blocked = false;
        'outer: for dx in -2i32..=2 {
            for dz in -2i32..=2 {
                let bx = lx as i32 + dx;
                let bz = lz as i32 + dz;
                if bx < 0 || bx >= 16 || bz < 0 || bz >= 16 { continue; }
                if chunk.get_local(bx, h + 4, bz) == BlockType::Log {
                    blocked = true;
                    break 'outer;
                }
            }
        }
        if blocked { continue; }

        let kind = if biome == Biome::Snow || biome == Biome::Mountain {
            TreeKind::Pine
        } else {
            TreeKind::Oak
        };

        match kind {
            TreeKind::Oak => build_oak_local(&mut chunk, &mut rng, lx as i32, h + 1, lz as i32),
            TreeKind::Pine => build_pine_local(&mut chunk, &mut rng, lx as i32, h + 1, lz as i32),
        }
    }

    chunk
}

#[derive(Clone, Copy)]
enum TreeKind { Oak, Pine }

fn build_oak_local(chunk: &mut ChunkData, rng: &mut Rng, lx: i32, y: i32, lz: i32) {
    let trunk_h = rng.range_i32(4, 7);
    for i in 0..trunk_h {
        chunk.set_local(lx, y + i, lz, BlockType::Log);
    }
    let top = y + trunk_h;

    for dy in -2i32..2 {
        let r: i32 = if dy <= -1 { 2 } else { 1 };
        for dx in -r..=r {
            for dz in -r..=r {
                if dx == 0 && dz == 0 && dy < 1 { continue; }
                if dx.abs() == r && dz.abs() == r && rng.next_f32() < 0.65 { continue; }
                let bx = lx + dx;
                let by = top + dy;
                let bz = lz + dz;
                if bx < 0 || bx >= 16 || bz < 0 || bz >= 16 { continue; }
                if by < 0 || by >= SY { continue; }
                if chunk.get_local(bx, by, bz) == BlockType::Air {
                    chunk.set_local(bx, by, bz, BlockType::Leaves);
                }
            }
        }
    }
    if top + 1 < SY
        && chunk.get_local(lx, top + 1, lz) == BlockType::Air
    {
        chunk.set_local(lx, top + 1, lz, BlockType::Leaves);
    }
}

fn build_pine_local(chunk: &mut ChunkData, rng: &mut Rng, lx: i32, y: i32, lz: i32) {
    let trunk_h = rng.range_i32(6, 9);
    for i in 0..trunk_h {
        chunk.set_local(lx, y + i, lz, BlockType::Log);
    }
    let top = y + trunk_h;

    for layer in 0i32..5 {
        let r: i32 = std::cmp::max(0, 2 - (layer / 2));
        let ly = top + layer - 4;
        if ly < 0 || ly >= SY { continue; }
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs() + dz.abs() > r + 1 { continue; }
                let bx = lx + dx;
                let bz = lz + dz;
                if bx < 0 || bx >= 16 || bz < 0 || bz >= 16 { continue; }
                if chunk.get_local(bx, ly, bz) == BlockType::Air {
                    chunk.set_local(bx, ly, bz, BlockType::Leaves);
                }
            }
        }
    }
}