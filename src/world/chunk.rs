use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::ImageSampler;
use bevy::render::render_resource::Face;
use bevy::light::GlobalAmbientLight;
use std::collections::HashMap;
use std::collections::HashSet;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};

use crate::core::state::{
    SY, CHUNK_SIZE, CHUNK_AREA, CHUNK_VOLUME, Settings, AppState,
};
use crate::player::controller::Player;
use super::generator::{self, PerlinCacheRes};

// ============================================================
// БЛОКИ
// ============================================================
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockType {
    Air = 0, Grass, Dirt, Stone, Cobblestone, Sand,
    Log, Leaves, Planks, Brick, Glass, Snow, Obsidian,
}

impl BlockType {
    #[inline] pub fn is_solid(self) -> bool { !matches!(self, BlockType::Air) }
    #[inline] pub fn to_u8(self) -> u8 { self as u8 }
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => BlockType::Grass, 2 => BlockType::Dirt, 3 => BlockType::Stone,
            4 => BlockType::Cobblestone, 5 => BlockType::Sand, 6 => BlockType::Log,
            7 => BlockType::Leaves, 8 => BlockType::Planks, 9 => BlockType::Brick,
            10 => BlockType::Glass, 11 => BlockType::Snow, 12 => BlockType::Obsidian,
            _ => BlockType::Air,
        }
    }
    pub fn icon_color(self) -> Color {
        use BlockType::*;
        match self {
            Air => Color::srgba(0.0, 0.0, 0.0, 0.0),
            Grass => Color::srgb(0.30, 0.60, 0.15), Dirt => Color::srgb(0.45, 0.32, 0.20),
            Stone => Color::srgb(0.45, 0.45, 0.45), Cobblestone => Color::srgb(0.38, 0.38, 0.38),
            Sand => Color::srgb(0.80, 0.75, 0.58), Log => Color::srgb(0.42, 0.30, 0.18),
            Leaves => Color::srgb(0.25, 0.50, 0.25), Planks => Color::srgb(0.65, 0.52, 0.32),
            Brick => Color::srgb(0.59, 0.26, 0.20), Glass => Color::srgb(0.78, 0.88, 0.94),
            Snow => Color::srgb(0.92, 0.96, 0.99), Obsidian => Color::srgb(0.15, 0.11, 0.23),
        }
    }
    #[inline]
    pub fn top_tile(self) -> u32 {
        match self {
            BlockType::Air | BlockType::Stone => 3, BlockType::Grass => 0, BlockType::Dirt => 2,
            BlockType::Cobblestone => 4, BlockType::Sand => 5, 
            BlockType::Log => 7, 
            BlockType::Planks => 9, // <-- Доски теперь одинаковые со всех сторон
            BlockType::Leaves => 8, BlockType::Brick => 10, BlockType::Glass => 11,
            BlockType::Snow => 12, BlockType::Obsidian => 13,
        }
    }
    #[inline]
    pub fn side_tile(self) -> u32 {
        match self {
            BlockType::Air => 3, BlockType::Grass => 1, BlockType::Dirt => 2, BlockType::Stone => 3,
            BlockType::Cobblestone => 4, BlockType::Sand => 5, BlockType::Log => 6,
            BlockType::Leaves => 8, BlockType::Planks => 9, BlockType::Brick => 10,
            BlockType::Glass => 11, BlockType::Snow => 12, BlockType::Obsidian => 13,
        }
    }
    #[inline]
    pub fn bottom_tile(self) -> u32 {
        match self {
            BlockType::Air | BlockType::Stone => 3, BlockType::Grass | BlockType::Dirt => 2,
            BlockType::Cobblestone => 4, BlockType::Sand => 5, 
            BlockType::Log => 7, 
            BlockType::Planks => 9, // <-- То же самое снизу
            BlockType::Leaves => 8, BlockType::Brick => 10, BlockType::Glass => 11,
            BlockType::Snow => 12, BlockType::Obsidian => 13,
        }
    }
}

// ============================================================
// ДАННЫЕ ЧАНКА И СНИМОК
// ============================================================
#[derive(Clone)]
pub struct ChunkData { pub blocks: Vec<BlockType> }

impl ChunkData {
    pub fn new() -> Self { Self { blocks: vec![BlockType::Air; CHUNK_VOLUME] } }
    #[inline] fn idx(lx: i32, y: i32, lz: i32) -> usize {
        (y as usize) * CHUNK_AREA + (lz as usize) * (CHUNK_SIZE as usize) + (lx as usize)
    }
    #[inline]
    pub fn get_local(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        if y < 0 || y >= SY || lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE { return BlockType::Air; }
        self.blocks[Self::idx(lx, y, lz)]
    }
    #[inline(always)]
    pub fn get_local_fast(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        debug_assert!(y >= 0 && y < SY && lx >= 0 && lx < CHUNK_SIZE && lz >= 0 && lz < CHUNK_SIZE);
        unsafe { *self.blocks.get_unchecked(Self::idx(lx, y, lz)) }
    }
    #[inline]
    pub fn set_local(&mut self, lx: i32, y: i32, lz: i32, block: BlockType) {
        if y < 0 || y >= SY || lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE { return; }
        self.blocks[Self::idx(lx, y, lz)] = block;
    }
}

pub const PAD_SIZE: i32 = CHUNK_SIZE + 2;
pub const PAD_AREA: usize = (PAD_SIZE * PAD_SIZE) as usize;

#[derive(Clone)]
pub struct ChunkSnapshot { pub coord: IVec2, pub blocks: Vec<BlockType> }

impl ChunkSnapshot {
    pub fn new(coord: IVec2) -> Self { Self { coord, blocks: vec![BlockType::Air; PAD_AREA * SY as usize] } }
    #[inline(always)] fn idx(lx: i32, y: i32, lz: i32) -> usize {
        let px = (lx + 1) as usize; let pz = (lz + 1) as usize;
        (y as usize) * PAD_AREA + pz * (PAD_SIZE as usize) + px
    }
    #[inline(always)] pub fn get(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        if y < 0 || y >= SY || lx < -1 || lx > CHUNK_SIZE || lz < -1 || lz > CHUNK_SIZE { return BlockType::Air; }
        unsafe { *self.blocks.get_unchecked(Self::idx(lx, y, lz)) }
    }
}

pub fn build_snapshot(world: &WorldData, cx: i32, cz: i32) -> Option<ChunkSnapshot> {
    let coord = IVec2::new(cx, cz);
    let chunk = world.chunks.get(&coord)?;
    let mut snap = ChunkSnapshot::new(coord);
    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                snap.blocks[ChunkSnapshot::idx(lx, y, lz)] = chunk.get_local_fast(lx, y, lz);
            }
        }
    }
    let west = world.chunks.get(&IVec2::new(cx - 1, cz));
    let east = world.chunks.get(&IVec2::new(cx + 1, cz));
    let north = world.chunks.get(&IVec2::new(cx, cz - 1));
    let south = world.chunks.get(&IVec2::new(cx, cz + 1));
    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            if let Some(w) = west { snap.blocks[ChunkSnapshot::idx(-1, y, lz)] = w.get_local_fast(CHUNK_SIZE - 1, y, lz); }
            if let Some(e) = east { snap.blocks[ChunkSnapshot::idx(CHUNK_SIZE, y, lz)] = e.get_local_fast(0, y, lz); }
        }
        for lx in 0..CHUNK_SIZE {
            if let Some(n) = north { snap.blocks[ChunkSnapshot::idx(lx, y, -1)] = n.get_local_fast(lx, y, CHUNK_SIZE - 1); }
            if let Some(s) = south { snap.blocks[ChunkSnapshot::idx(lx, y, CHUNK_SIZE)] = s.get_local_fast(lx, y, 0); }
        }
    }
    Some(snap)
}

#[derive(Resource)]
pub struct WorldData {
    pub chunks: HashMap<IVec2, ChunkData>,
    pub edits: HashMap<IVec2, HashMap<IVec3, BlockType>>,
}

impl WorldData {
    pub fn new() -> Self { Self { chunks: HashMap::new(), edits: HashMap::new() } }
    #[inline] pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < 0 || y >= SY { return BlockType::Air; }
        let Some(chunk) = self.chunks.get(&IVec2::new(x.div_euclid(CHUNK_SIZE), z.div_euclid(CHUNK_SIZE))) else { return BlockType::Air; };
        chunk.get_local(x.rem_euclid(CHUNK_SIZE), y, z.rem_euclid(CHUNK_SIZE))
    }
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if y < 0 || y >= SY { return; }
        let key = IVec2::new(x.div_euclid(CHUNK_SIZE), z.div_euclid(CHUNK_SIZE));
        self.edits.entry(key).or_default().insert(IVec3::new(x, y, z), block);
        if let Some(chunk) = self.chunks.get_mut(&key) { chunk.set_local(x.rem_euclid(CHUNK_SIZE), y, z.rem_euclid(CHUNK_SIZE), block); }
    }
    pub fn set_natural(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if y < 0 || y >= SY { return; }
        if let Some(chunk) = self.chunks.get_mut(&IVec2::new(x.div_euclid(CHUNK_SIZE), z.div_euclid(CHUNK_SIZE))) {
            chunk.set_local(x.rem_euclid(CHUNK_SIZE), y, z.rem_euclid(CHUNK_SIZE), block);
        }
    }
    #[inline] pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if y < 0 { return true; } if y >= SY { return false; } self.get(x, y, z).is_solid()
    }
    pub fn insert_chunk(&mut self, cx: i32, cz: i32, mut chunk: ChunkData) {
        let key = IVec2::new(cx, cz);
        if let Some(edits) = self.edits.get(&key) {
            for (pos, &block) in edits { chunk.set_local(pos.x.rem_euclid(CHUNK_SIZE), pos.y, pos.z.rem_euclid(CHUNK_SIZE), block); }
        }
        self.chunks.insert(key, chunk);
    }
    pub fn has_chunk(&self, cx: i32, cz: i32) -> bool { self.chunks.contains_key(&IVec2::new(cx, cz)) }
}

#[derive(Resource, Default)] pub struct DirtyChunks(pub Vec<IVec2>);
impl DirtyChunks {
    pub fn mark(&mut self, x: i32, z: i32) {
        let cx = x.div_euclid(CHUNK_SIZE); let cz = z.div_euclid(CHUNK_SIZE);
        let lx = x.rem_euclid(CHUNK_SIZE); let lz = z.rem_euclid(CHUNK_SIZE);
        self.0.push(IVec2::new(cx, cz));
        if lx == 0 { self.0.push(IVec2::new(cx - 1, cz)); }
        if lx == CHUNK_SIZE - 1 { self.0.push(IVec2::new(cx + 1, cz)); }
        if lz == 0 { self.0.push(IVec2::new(cx, cz - 1)); }
        if lz == CHUNK_SIZE - 1 { self.0.push(IVec2::new(cx, cz + 1)); }
    }
}

pub struct PendingChunkTask { pub coord: IVec2, pub task: Task<ChunkData> }
#[derive(Resource, Default)] pub struct PendingChunks { pub tasks: Vec<PendingChunkTask> }
pub struct PendingMeshTask { pub coord: IVec2, pub task: Task<Mesh> }
#[derive(Resource, Default)] pub struct PendingMeshes { pub tasks: Vec<PendingMeshTask> }

const MAX_GEN_IN_FLIGHT: usize = 32;
const MAX_MESH_IN_FLIGHT: usize = 16;
const TASKS_SPAWN_PER_FRAME: usize = 6;
const MESH_TASKS_PER_FRAME: usize = 8;
const SYNC_STARTUP_RADIUS: i32 = 1;

// ============================================================
// АТЛАС
// ============================================================
const ATLAS_TILE: u32 = 16;
const ATLAS_COLS: u32 = 4;
const ATLAS_ROWS: u32 = 4;

fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    h = h.wrapping_add(seed.wrapping_mul(1013904223));
    h ^= h >> 13; h = h.wrapping_mul(1274126177); h ^= h >> 16;
    (h & 0xFF) as f32 / 255.0
}

fn draw_tile(data: &mut [u8], atlas_w: u32, tile_idx: u32, seed: u32, painter: impl Fn(u32, u32, u32) -> [u8; 4]) {
    let tx = (tile_idx % ATLAS_COLS) * ATLAS_TILE;
    let ty = (tile_idx / ATLAS_COLS) * ATLAS_TILE;
    for py in 0..ATLAS_TILE {
        for px in 0..ATLAS_TILE {
            let color = painter(px, py, seed);
            let o = (((ty + py) * atlas_w + (tx + px)) * 4) as usize;
            data[o] = color[0]; data[o + 1] = color[1]; data[o + 2] = color[2]; data[o + 3] = color[3];
        }
    }
}

fn make_atlas() -> Vec<u8> {
    let w = ATLAS_TILE * ATLAS_COLS; let h = ATLAS_TILE * ATLAS_ROWS;
    let mut data = vec![0u8; (w * h * 4) as usize];

    draw_tile(&mut data, w, 0, 1, |x, y, s| { let n = hash(x, y, s); let b = (0.32 + n * 0.14) * 255.0; [(b * 0.50) as u8, b as u8, (b * 0.28) as u8, 255] });
    draw_tile(&mut data, w, 1, 2, |x, y, s| { let n = hash(x, y, s); let edge = 3 + (hash(x, 0, s + 7) * 2.0) as u32; if y < edge { let b = (0.32 + n * 0.14) * 255.0; [(b * 0.50) as u8, b as u8, (b * 0.28) as u8, 255] } else { let b = (0.45 + n * 0.12) * 255.0; [b as u8, (b * 0.70) as u8, (b * 0.45) as u8, 255] } });
    draw_tile(&mut data, w, 2, 3, |x, y, s| { let n = hash(x, y, s); let b = (0.45 + n * 0.12) * 255.0; [b as u8, (b * 0.70) as u8, (b * 0.45) as u8, 255] });
    draw_tile(&mut data, w, 3, 4, |x, y, s| { let n = hash(x, y, s); let b = (0.42 + n * 0.12) * 255.0; [b as u8, b as u8, b as u8, 255] });
    draw_tile(&mut data, w, 4, 5, |x, y, _s| { let gx = x / 4; let gy = y / 4; let v = ((gx * 7 + gy * 13) % 5) as f32 * 0.03; let edge = if x % 4 == 0 || y % 4 == 0 { -0.08 } else { 0.0 }; let b = (0.40 + v + edge).clamp(0.0, 1.0) * 255.0; [b as u8, b as u8, b as u8, 255] });
    draw_tile(&mut data, w, 5, 6, |x, y, s| { let n = hash(x, y, s); let b = (0.80 + n * 0.08) * 255.0; [b as u8, (b * 0.94) as u8, (b * 0.72) as u8, 255] });
    draw_tile(&mut data, w, 6, 7, |x, y, s| { let n = hash(x, y, s); let stripe = ((x as f32 * 0.9).sin() * 0.05).abs(); let b = (0.42 + n * 0.08 + stripe) * 255.0; [b as u8, (b * 0.72) as u8, (b * 0.42) as u8, 255] });
    draw_tile(&mut data, w, 7, 8, |x, y, _s| { let dx = x as f32 - 7.5; let dy = y as f32 - 7.5; let r = (dx * dx + dy * dy).sqrt(); let ring = (r * 2.4).sin() * 0.05; let b = (0.55 + ring) * 255.0; [b as u8, (b * 0.78) as u8, (b * 0.48) as u8, 255] });
    draw_tile(&mut data, w, 8, 9, |x, y, s| { let n = hash(x, y, s); let dark = if n < 0.35 { -0.10 } else { 0.0 }; let b = (0.18 + n * 0.18 + dark).clamp(0.0, 1.0) * 255.0; let b = b as u8; [b, ((b as f32) * 1.7).min(255.0) as u8, b, 255] });
    
    // Текстура Досок (tile 9)
    draw_tile(&mut data, w, 9, 10, |x, y, s| {
        let n = hash(x, y, s);
        let row = y / 4;
        // Один вертикальный стык на ряд, смещённый в случайную позицию
        let off = (hash(row as u32, 0, s + 99) * 14.0) as u32 + 1;
        let line = if y % 4 == 0 || x == off { -0.15 } else { 0.0 };
        let b = (0.65 + n * 0.06 + line).clamp(0.0, 1.0) * 255.0;
        let b = b as u8;
        [b, (b as f32 * 0.80) as u8, (b as f32 * 0.50) as u8, 255]
    });

    draw_tile(&mut data, w, 10, 11, |x, y, s| { let n = hash(x, y, s); let row = y / 4; let off = (row % 2) * 4; if y % 4 == 0 || (x + off) % 8 == 0 { [196, 190, 184, 255] } else { let r = (0.59 + n * 0.06) * 255.0; [r as u8, 66, 51, 255] } });
    draw_tile(&mut data, w, 11, 12, |_, _, _| [200, 226, 240, 255]);
    draw_tile(&mut data, w, 12, 13, |x, y, s| { let n = hash(x, y, s); let b = (0.92 + n * 0.05) * 255.0; [b as u8, (b * 0.98) as u8, (b * 0.99) as u8, 255] });
    draw_tile(&mut data, w, 13, 14, |x, y, s| { let n = hash(x, y, s); let b = (0.15 + n * 0.10) * 255.0; [b as u8, (b * 0.75) as u8, (b * 1.25).min(255.0) as u8, 255] });
    data
}

#[inline]
fn tile_uv(tile: u32) -> [[f32; 2]; 4] {
    let tc = (tile % ATLAS_COLS) as f32; let tr = (tile / ATLAS_COLS) as f32;
    let cols = ATLAS_COLS as f32; let rows = ATLAS_ROWS as f32;
    let pad = 0.5 / (ATLAS_TILE * ATLAS_COLS) as f32;
    let u0 = tc / cols + pad; let u1 = (tc + 1.0) / cols - pad;
    let v_top = tr / rows + pad; let v_bot = (tr + 1.0) / rows - pad;
    [[u0, v_bot], [u1, v_bot], [u1, v_top], [u0, v_top]]
}

const FACES: [([i32; 3], [[f32; 3]; 4]); 6] = [
    ([1, 0, 0],  [[1.0,0.0,1.0],[1.0,0.0,0.0],[1.0,1.0,0.0],[1.0,1.0,1.0]]),
    ([-1, 0, 0], [[0.0,0.0,0.0],[0.0,0.0,1.0],[0.0,1.0,1.0],[0.0,1.0,0.0]]),
    ([0, 1, 0],  [[0.0,1.0,1.0],[1.0,1.0,1.0],[1.0,1.0,0.0],[0.0,1.0,0.0]]),
    ([0, -1, 0], [[0.0,0.0,0.0],[1.0,0.0,0.0],[1.0,0.0,1.0],[0.0,0.0,1.0]]),
    ([0, 0, 1],  [[0.0,0.0,1.0],[1.0,0.0,1.0],[1.0,1.0,1.0],[0.0,1.0,1.0]]),
    ([0, 0, -1], [[1.0,0.0,0.0],[0.0,0.0,0.0],[0.0,1.0,0.0],[1.0,1.0,0.0]]),
];

/// Вычисляет AO-фактор для вершины грани.
/// Проверяет 3 соседних блока в «наружной» плоскости грани:
/// два по сторонам (side1, side2) и один по диагонали (corner).
/// Возвращает 0.4..1.0 — множитель к цвету текстуры.
#[inline]
fn vertex_ao(
    snap: &ChunkSnapshot,
    bx: i32, by: i32, bz: i32,
    nx: i32, ny: i32, nz: i32,
    cx: i32, cy: i32, cz: i32,
) -> f32 {
    let (s1, s2, corner) = if nx != 0 {
        let sy = if cy == 0 { -1 } else { 1 };
        let sz = if cz == 0 { -1 } else { 1 };
        (
            snap.get(bx + nx, by + sy, bz).is_solid(),
            snap.get(bx + nx, by, bz + sz).is_solid(),
            snap.get(bx + nx, by + sy, bz + sz).is_solid(),
        )
    } else if ny != 0 {
        let sx = if cx == 0 { -1 } else { 1 };
        let sz = if cz == 0 { -1 } else { 1 };
        (
            snap.get(bx + sx, by + ny, bz).is_solid(),
            snap.get(bx, by + ny, bz + sz).is_solid(),
            snap.get(bx + sx, by + ny, bz + sz).is_solid(),
        )
    } else {
        let sx = if cx == 0 { -1 } else { 1 };
        let sy = if cy == 0 { -1 } else { 1 };
        (
            snap.get(bx + sx, by, bz + nz).is_solid(),
            snap.get(bx, by + sy, bz + nz).is_solid(),
            snap.get(bx + sx, by + sy, bz + nz).is_solid(),
        )
    };

    let level = if s1 && s2 {
        0
    } else {
        3 - (s1 as u8 + s2 as u8 + corner as u8)
    };

    const AO: [f32; 4] = [0.55, 0.7, 0.85, 1.0];
    AO[level as usize]
}

pub fn build_mesh_from_snapshot(snap: &ChunkSnapshot) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(4096);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(4096);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(4096);
    let mut colors:    Vec<[f32; 4]> = Vec::with_capacity(4096);
    let mut indices:   Vec<u32>      = Vec::with_capacity(6144);

    let x_start = snap.coord.x * CHUNK_SIZE;
    let z_start = snap.coord.y * CHUNK_SIZE;

    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let block = snap.get(lx, y, lz);
                if !block.is_solid() { continue; }

                let x = x_start + lx;
                let z = z_start + lz;

                for (i, (dir, corners)) in FACES.iter().enumerate() {
                    let nlx = lx + dir[0]; let ny = y + dir[1]; let nlz = lz + dir[2];
                    let solid = if ny < 0 { true } else if ny >= SY { false } else { snap.get(nlx, ny, nlz).is_solid() };
                    if solid { continue; }

                    let tile = match i { 2 => block.top_tile(), 3 => block.bottom_tile(), _ => block.side_tile() };
                    let tuv = tile_uv(tile);
                    let base = positions.len() as u32;

                    let mut ao = [1.0_f32; 4];
                    for j in 0..4 {
                        let corner = corners[j];
                        ao[j] = vertex_ao(
                            snap, lx, y, lz,
                            dir[0], dir[1], dir[2],
                            corner[0] as i32, corner[1] as i32, corner[2] as i32,
                        );
                    }

                    for (j, corner) in corners.iter().enumerate() {
                        positions.push([x as f32 + corner[0], y as f32 + corner[1], z as f32 + corner[2]]);
                        normals.push([dir[0] as f32, dir[1] as f32, dir[2] as f32]);
                        uvs.push(tuv[j]);
                        colors.push([ao[j], ao[j], ao[j], 1.0]);
                    }

                    if ao[0] + ao[2] > ao[1] + ao[3] {
                        indices.extend_from_slice(&[
                            base + 1, base + 2, base + 3,
                            base + 1, base + 3, base,
                        ]);
                    } else {
                        indices.extend_from_slice(&[
                            base, base + 1, base + 2,
                            base, base + 2, base + 3,
                        ]);
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[derive(Component)] pub struct ChunkMesh { pub coord: IVec2 }

#[derive(Resource)]
pub struct ChunkRenderAssets { pub material: Handle<StandardMaterial> }

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 1200.0, affects_lightmapped_meshes: true })
        .insert_resource(WorldData::new())
        .init_resource::<DirtyChunks>()
        .init_resource::<PendingChunks>()
        .init_resource::<PendingMeshes>()
        .add_systems(Startup, (setup_chunk_render_assets, setup_lights))
        .add_systems(PostUpdate, (
            stream_chunks.run_if(in_state(AppState::InGame)),
            rebuild_dirty_chunks.run_if(in_state(AppState::InGame)),
        ).chain());
        info!("ChunkPlugin loaded.");
    }
}

fn setup_chunk_render_assets(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let atlas_data = make_atlas();
    let mut atlas_image = Image::new(
        Extent3d { width: ATLAS_TILE * ATLAS_COLS, height: ATLAS_TILE * ATLAS_ROWS, depth_or_array_layers: 1 },
        TextureDimension::D2, atlas_data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default(),
    );
    atlas_image.sampler = ImageSampler::nearest();
    let atlas_handle = images.add(atlas_image);

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(atlas_handle),
        unlit: true,
        cull_mode: Some(Face::Back),
        ..default()
    });

    commands.insert_resource(ChunkRenderAssets { material });
    info!("Atlas generated, vertex AO baked, unlit material ready");
}

fn setup_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight { illuminance: 8000.0, shadow_maps_enabled: false, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.9, -0.5, 0.0)),
    ));
}

fn spawn_chunk_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    coord: IVec2,
    mesh: Mesh,
) {
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material.clone()),
        Transform::default(),
        ChunkMesh { coord },
        Name::new(format!("Chunk({}, {})", coord.x, coord.y)),
    ));
}

fn stream_chunks(
    settings: Res<Settings>,
    perlin: Res<PerlinCacheRes>,
    mut world: ResMut<WorldData>,
    mut pending_gen: ResMut<PendingChunks>,
    mut pending_mesh: ResMut<PendingMeshes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Option<Res<ChunkRenderAssets>>,
    chunk_q: Query<(Entity, &ChunkMesh)>,
    player_q: Query<&Transform, With<Player>>,
) {
    let Some(assets) = assets else { return; };
    let Ok(pt) = player_q.single() else { return; };

    let pcx = (pt.translation.x.floor() as i32).div_euclid(CHUNK_SIZE);
    let pcz = (pt.translation.z.floor() as i32).div_euclid(CHUNK_SIZE);
    let rd = settings.render_distance_chunks;
    let load_rd = rd + 2;

    if world.chunks.is_empty() {
        pending_gen.tasks.clear(); pending_mesh.tasks.clear();
        for dz in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
            for dx in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
                let cx = pcx + dx; let cz = pcz + dz;
                if world.has_chunk(cx, cz) { continue; }
                world.insert_chunk(cx, cz, generator::generate_chunk(&perlin.0, cx, cz));
            }
        }
        for dz in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
            for dx in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
                let key = IVec2::new(pcx + dx, pcz + dz);
                let Some(snap) = build_snapshot(&world, key.x, key.y) else { continue; };
                spawn_chunk_entity(&mut commands, &mut meshes, &assets.material, key, build_mesh_from_snapshot(&snap));
            }
        }
    }

    let mut done_gen = Vec::new();
    pending_gen.tasks.retain_mut(|p| {
        if let Some(chunk) = block_on(poll_once(&mut p.task)) { done_gen.push((p.coord, chunk)); false } else { true }
    });
    for (coord, chunk) in done_gen {
        if world.chunks.contains_key(&coord) { continue; }
        world.insert_chunk(coord.x, coord.y, chunk);
    }

    let rendered: HashSet<IVec2> = chunk_q.iter().map(|(_, c)| c.coord).collect();
    let mut done_mesh = Vec::new();
    pending_mesh.tasks.retain_mut(|p| {
        if let Some(mesh) = block_on(poll_once(&mut p.task)) { done_mesh.push((p.coord, mesh)); false } else { true }
    });
    for (coord, mesh) in done_mesh {
        if !world.has_chunk(coord.x, coord.y) || rendered.contains(&coord) { continue; }
        spawn_chunk_entity(&mut commands, &mut meshes, &assets.material, coord, mesh);
    }

    let pending_mesh_coords: HashSet<IVec2> = pending_mesh.tasks.iter().map(|p| p.coord).collect();
    let mut mesh_spawned = 0usize;
    'mesh_sched: for dz in -rd..=rd {
        for dx in -rd..=rd {
            if mesh_spawned >= MESH_TASKS_PER_FRAME || pending_mesh.tasks.len() >= MAX_MESH_IN_FLIGHT { break 'mesh_sched; }
            let key = IVec2::new(pcx + dx, pcz + dz);
            if rendered.contains(&key) || pending_mesh_coords.contains(&key) { continue; }
            if !world.has_chunk(key.x, key.y) { continue; }
            let Some(snap) = build_snapshot(&world, key.x, key.y) else { continue; };
            let pool = AsyncComputeTaskPool::get();
            let task = pool.spawn(async move { build_mesh_from_snapshot(&snap) });
            pending_mesh.tasks.push(PendingMeshTask { coord: key, task });
            mesh_spawned += 1;
        }
    }

    for (entity, chunk) in chunk_q.iter() {
        let dx = (chunk.coord.x - pcx).abs(); let dz = (chunk.coord.y - pcz).abs();
        if dx > rd || dz > rd { commands.entity(entity).despawn(); }
    }

    let mem_rd = rd + 4;
    let to_remove: Vec<IVec2> = world.chunks.keys().filter(|k| (k.x - pcx).abs() > mem_rd || (k.y - pcz).abs() > mem_rd).copied().collect();
    for k in to_remove { world.chunks.remove(&k); }

    if pending_gen.tasks.len() >= MAX_GEN_IN_FLIGHT { return; }
    let in_flight: HashSet<IVec2> = pending_gen.tasks.iter().map(|p| p.coord).collect();
    let fwd = pt.forward().as_vec3();
    let mut to_load: Vec<(f32, IVec2)> = Vec::with_capacity(64);
    for dz in -load_rd..=load_rd {
        for dx in -load_rd..=load_rd {
            let key = IVec2::new(pcx + dx, pcz + dz);
            if world.chunks.contains_key(&key) || in_flight.contains(&key) { continue; }
            let fdx = dx as f32; let fdz = dz as f32;
            let dist_sq = fdx * fdx + fdz * fdz; let dot = fdx * fwd.x + fdz * fwd.z;
            let score = if dot > 0.0 { dist_sq } else { dist_sq * 3.0 };
            to_load.push((score, key));
        }
    }
    to_load.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let to_spawn = MAX_GEN_IN_FLIGHT.saturating_sub(pending_gen.tasks.len()).min(TASKS_SPAWN_PER_FRAME);
    let pool = AsyncComputeTaskPool::get();
    let cache_snapshot = perlin.0;
    for (_, key) in to_load.into_iter().take(to_spawn) {
        let (cx, cz) = (key.x, key.y); let cache = cache_snapshot;
        let task = pool.spawn(async move { generator::generate_chunk(&cache, cx, cz) });
        pending_gen.tasks.push(PendingChunkTask { coord: key, task });
    }
}

fn rebuild_dirty_chunks(
    world: Res<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    _assets: Option<Res<ChunkRenderAssets>>,
    mut chunk_q: Query<(Entity, &ChunkMesh, &mut Mesh3d)>,
    mut pending_mesh: ResMut<PendingMeshes>,
) {
    if dirty.0.is_empty() { return; }
    dirty.0.sort_by_key(|c| (c.x, c.y)); dirty.0.dedup();
    let coords: Vec<IVec2> = std::mem::take(&mut dirty.0);

    for coord in coords {
        if !world.has_chunk(coord.x, coord.y) { continue; }
        pending_mesh.tasks.retain(|p| p.coord != coord);
        let Some(snap) = build_snapshot(&world, coord.x, coord.y) else { continue; };
        let new_mesh = build_mesh_from_snapshot(&snap);
        let new_handle = meshes.add(new_mesh);
        for (_, chunk, mut mesh3d) in chunk_q.iter_mut() {
            if chunk.coord == coord { mesh3d.0 = new_handle.clone(); break; }
        }
    }
}