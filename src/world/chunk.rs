use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::ImageSampler;
use bevy::render::render_resource::Face;
use bevy::light::GlobalAmbientLight;
use std::collections::HashMap;

use crate::core::state::{
    SY, CHUNK_SIZE, CHUNK_AREA, CHUNK_VOLUME, Settings, WorldSeed, AppState,
};
use crate::player::controller::Player;
use super::generator;

// ============================================================
// БЛОКИ
// ============================================================
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockType {
    Air = 0,
    Grass,
    Dirt,
    Stone,
    Cobblestone,
    Sand,
    Log,
    Leaves,
    Planks,
    Brick,
    Glass,
    Snow,
    Obsidian,
}

impl BlockType {
    #[inline]
    pub fn is_solid(self) -> bool {
        !matches!(self, BlockType::Air)
    }

    #[inline]
    pub fn to_u8(self) -> u8 { self as u8 }

    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => BlockType::Air,
            1 => BlockType::Grass,
            2 => BlockType::Dirt,
            3 => BlockType::Stone,
            4 => BlockType::Cobblestone,
            5 => BlockType::Sand,
            6 => BlockType::Log,
            7 => BlockType::Leaves,
            8 => BlockType::Planks,
            9 => BlockType::Brick,
            10 => BlockType::Glass,
            11 => BlockType::Snow,
            12 => BlockType::Obsidian,
            _ => BlockType::Air,
        }
    }

    #[inline]
    pub fn top_tile(self) -> u32 {
        match self {
            BlockType::Air | BlockType::Stone => 3,
            BlockType::Grass => 0,
            BlockType::Dirt => 2,
            BlockType::Cobblestone => 4,
            BlockType::Sand => 5,
            BlockType::Log | BlockType::Planks => 7,
            BlockType::Leaves => 8,
            BlockType::Brick => 10,
            BlockType::Glass => 11,
            BlockType::Snow => 12,
            BlockType::Obsidian => 13,
        }
    }

    #[inline]
    pub fn side_tile(self) -> u32 {
        match self {
            BlockType::Air => 3,
            BlockType::Grass => 1,
            BlockType::Dirt => 2,
            BlockType::Stone => 3,
            BlockType::Cobblestone => 4,
            BlockType::Sand => 5,
            BlockType::Log => 6,
            BlockType::Leaves => 8,
            BlockType::Planks => 9,
            BlockType::Brick => 10,
            BlockType::Glass => 11,
            BlockType::Snow => 12,
            BlockType::Obsidian => 13,
        }
    }

    #[inline]
    pub fn bottom_tile(self) -> u32 {
        match self {
            BlockType::Air | BlockType::Stone => 3,
            BlockType::Grass | BlockType::Dirt => 2,
            BlockType::Cobblestone => 4,
            BlockType::Sand => 5,
            BlockType::Log | BlockType::Planks => 7,
            BlockType::Leaves => 8,
            BlockType::Brick => 10,
            BlockType::Glass => 11,
            BlockType::Snow => 12,
            BlockType::Obsidian => 13,
        }
    }

    pub fn icon_color(self) -> Color {
        match self {
            BlockType::Air => Color::srgba(0.0, 0.0, 0.0, 0.0),
            BlockType::Grass => Color::srgb(0.30, 0.60, 0.15),
            BlockType::Dirt => Color::srgb(0.45, 0.32, 0.20),
            BlockType::Stone => Color::srgb(0.45, 0.45, 0.45),
            BlockType::Cobblestone => Color::srgb(0.38, 0.38, 0.38),
            BlockType::Sand => Color::srgb(0.80, 0.75, 0.58),
            BlockType::Log => Color::srgb(0.42, 0.30, 0.18),
            BlockType::Leaves => Color::srgb(0.25, 0.50, 0.25),
            BlockType::Planks => Color::srgb(0.65, 0.52, 0.32),
            BlockType::Brick => Color::srgb(0.59, 0.26, 0.20),
            BlockType::Glass => Color::srgb(0.78, 0.88, 0.94),
            BlockType::Snow => Color::srgb(0.92, 0.96, 0.99),
            BlockType::Obsidian => Color::srgb(0.15, 0.11, 0.23),
        }
    }
}

// ============================================================
// ДАННЫЕ ЧАНКА
// ============================================================
#[derive(Clone)]
pub struct ChunkData {
    pub blocks: Vec<BlockType>,
}

impl ChunkData {
    pub fn new() -> Self {
        Self { blocks: vec![BlockType::Air; CHUNK_VOLUME] }
    }

    #[inline]
    fn idx(lx: i32, y: i32, lz: i32) -> usize {
        (y as usize) * CHUNK_AREA
            + (lz as usize) * (CHUNK_SIZE as usize)
            + (lx as usize)
    }

    #[inline]
    pub fn get_local(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        if y < 0 || y >= SY { return BlockType::Air; }
        if lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE {
            return BlockType::Air;
        }
        self.blocks[Self::idx(lx, y, lz)]
    }

    #[inline]
    pub fn set_local(&mut self, lx: i32, y: i32, lz: i32, block: BlockType) {
        if y < 0 || y >= SY { return; }
        if lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE { return; }
        self.blocks[Self::idx(lx, y, lz)] = block;
    }
}

// ============================================================
// МИР
// ============================================================
#[derive(Resource)]
pub struct WorldData {
    pub chunks: HashMap<IVec2, ChunkData>,
    pub edits: HashMap<IVec2, HashMap<IVec3, BlockType>>,
}

impl WorldData {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            edits: HashMap::new(),
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < 0 || y >= SY { return BlockType::Air; }
        let cx = x.div_euclid(CHUNK_SIZE);
        let cz = z.div_euclid(CHUNK_SIZE);
        let Some(chunk) = self.chunks.get(&IVec2::new(cx, cz)) else {
            return BlockType::Air;
        };
        let lx = x.rem_euclid(CHUNK_SIZE);
        let lz = z.rem_euclid(CHUNK_SIZE);
        chunk.get_local(lx, y, lz)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if y < 0 || y >= SY { return; }
        let cx = x.div_euclid(CHUNK_SIZE);
        let cz = z.div_euclid(CHUNK_SIZE);
        let key = IVec2::new(cx, cz);

        self.edits.entry(key).or_default().insert(IVec3::new(x, y, z), block);

        if let Some(chunk) = self.chunks.get_mut(&key) {
            let lx = x.rem_euclid(CHUNK_SIZE);
            let lz = z.rem_euclid(CHUNK_SIZE);
            chunk.set_local(lx, y, lz, block);
        }
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if y < 0 { return true; }
        if y >= SY { return false; }
        self.get(x, y, z).is_solid()
    }

    pub fn insert_chunk(&mut self, cx: i32, cz: i32, mut chunk: ChunkData) {
        let key = IVec2::new(cx, cz);
        if let Some(edits) = self.edits.get(&key) {
            for (pos, &block) in edits {
                let lx = pos.x.rem_euclid(CHUNK_SIZE);
                let lz = pos.z.rem_euclid(CHUNK_SIZE);
                chunk.set_local(lx, pos.y, lz, block);
            }
        }
        self.chunks.insert(key, chunk);
    }

    pub fn has_chunk(&self, cx: i32, cz: i32) -> bool {
        self.chunks.contains_key(&IVec2::new(cx, cz))
    }
}

#[derive(Resource, Default)]
pub struct DirtyChunks(pub Vec<IVec2>);

impl DirtyChunks {
    pub fn mark(&mut self, x: i32, z: i32) {
        let cx = x.div_euclid(CHUNK_SIZE);
        let cz = z.div_euclid(CHUNK_SIZE);
        let lx = x.rem_euclid(CHUNK_SIZE);
        let lz = z.rem_euclid(CHUNK_SIZE);

        self.0.push(IVec2::new(cx, cz));

        let near_min_x = lx == 0;
        let near_max_x = lx == CHUNK_SIZE - 1;
        let near_min_z = lz == 0;
        let near_max_z = lz == CHUNK_SIZE - 1;

        if near_min_x { self.0.push(IVec2::new(cx - 1, cz)); }
        if near_max_x { self.0.push(IVec2::new(cx + 1, cz)); }
        if near_min_z { self.0.push(IVec2::new(cx, cz - 1)); }
        if near_max_z { self.0.push(IVec2::new(cx, cz + 1)); }

        if near_min_x && near_min_z { self.0.push(IVec2::new(cx - 1, cz - 1)); }
        if near_min_x && near_max_z { self.0.push(IVec2::new(cx - 1, cz + 1)); }
        if near_max_x && near_min_z { self.0.push(IVec2::new(cx + 1, cz - 1)); }
        if near_max_x && near_max_z { self.0.push(IVec2::new(cx + 1, cz + 1)); }
    }
}

// ============================================================
// АТЛАС ТЕКСТУР
// ============================================================
const ATLAS_TILE: u32 = 16;
const ATLAS_COLS: u32 = 4;
const ATLAS_ROWS: u32 = 4;

fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    h = h.wrapping_add(seed.wrapping_mul(1013904223));
    h ^= h >> 13;
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xFF) as f32 / 255.0
}

fn draw_tile(
    data: &mut [u8],
    atlas_w: u32,
    tile_idx: u32,
    seed: u32,
    painter: impl Fn(u32, u32, u32) -> [u8; 4],
) {
    let tx = (tile_idx % ATLAS_COLS) * ATLAS_TILE;
    let ty = (tile_idx / ATLAS_COLS) * ATLAS_TILE;
    for py in 0..ATLAS_TILE {
        for px in 0..ATLAS_TILE {
            let color = painter(px, py, seed);
            let atlas_x = tx + px;
            let atlas_y = ty + py;
            let o = ((atlas_y * atlas_w + atlas_x) * 4) as usize;
            data[o] = color[0];
            data[o + 1] = color[1];
            data[o + 2] = color[2];
            data[o + 3] = color[3];
        }
    }
}

fn make_atlas() -> Vec<u8> {
    let w = ATLAS_TILE * ATLAS_COLS;
    let h = ATLAS_TILE * ATLAS_ROWS;
    let mut data = vec![0u8; (w * h * 4) as usize];

    draw_tile(&mut data, w, 0, 1, |x, y, s| {
        let n = hash(x, y, s);
        let base = (0.32 + n * 0.14) * 255.0;
        [(base * 0.50) as u8, base as u8, (base * 0.28) as u8, 255]
    });
    draw_tile(&mut data, w, 1, 2, |x, y, s| {
        let n = hash(x, y, s);
        let edge = 3 + (hash(x, 0, s + 7) * 2.0) as u32;
        if y < edge {
            let base = (0.32 + n * 0.14) * 255.0;
            [(base * 0.50) as u8, base as u8, (base * 0.28) as u8, 255]
        } else {
            let base = (0.45 + n * 0.12) * 255.0;
            [base as u8, (base * 0.70) as u8, (base * 0.45) as u8, 255]
        }
    });
    draw_tile(&mut data, w, 2, 3, |x, y, s| {
        let n = hash(x, y, s);
        let base = (0.45 + n * 0.12) * 255.0;
        [base as u8, (base * 0.70) as u8, (base * 0.45) as u8, 255]
    });
    draw_tile(&mut data, w, 3, 4, |x, y, s| {
        let n = hash(x, y, s);
        let base = (0.42 + n * 0.12) * 255.0;
        [base as u8, base as u8, base as u8, 255]
    });
    draw_tile(&mut data, w, 4, 5, |x, y, _s| {
        let gx = x / 4;
        let gy = y / 4;
        let v = ((gx * 7 + gy * 13) % 5) as f32 * 0.03;
        let edge = if x % 4 == 0 || y % 4 == 0 { -0.08 } else { 0.0 };
        let base = (0.40 + v + edge).clamp(0.0, 1.0) * 255.0;
        let base = base as u8;
        [base, base, base, 255]
    });
    draw_tile(&mut data, w, 5, 6, |x, y, s| {
        let n = hash(x, y, s);
        let base = (0.80 + n * 0.08) * 255.0;
        [base as u8, (base * 0.94) as u8, (base * 0.72) as u8, 255]
    });
    draw_tile(&mut data, w, 6, 7, |x, y, s| {
        let n = hash(x, y, s);
        let stripe = ((x as f32 * 0.9).sin() * 0.05).abs();
        let base = (0.42 + n * 0.08 + stripe) * 255.0;
        [base as u8, (base * 0.72) as u8, (base * 0.42) as u8, 255]
    });
    draw_tile(&mut data, w, 7, 8, |x, y, _s| {
        let dx = x as f32 - 7.5;
        let dy = y as f32 - 7.5;
        let r = (dx * dx + dy * dy).sqrt();
        let ring = (r * 2.4).sin() * 0.05;
        let base = (0.55 + ring) * 255.0;
        [base as u8, (base * 0.78) as u8, (base * 0.48) as u8, 255]
    });
    draw_tile(&mut data, w, 8, 9, |x, y, s| {
        let n = hash(x, y, s);
        let dark = if n < 0.35 { -0.10 } else { 0.0 };
        let base = (0.18 + n * 0.18 + dark).clamp(0.0, 1.0) * 255.0;
        let base = base as u8;
        [base, ((base as f32) * 1.7).min(255.0) as u8, base, 255]
    });
    draw_tile(&mut data, w, 9, 10, |x, y, s| {
        let n = hash(x, y, s);
        let row = y / 4;
        let off = (row % 2) * 4;
        let line = if y % 4 == 0 || (x + off) % 8 == 0 { -0.12 } else { 0.0 };
        let base = (0.65 + n * 0.06 + line).clamp(0.0, 1.0) * 255.0;
        let base = base as u8;
        [base, (base as f32 * 0.80) as u8, (base as f32 * 0.50) as u8, 255]
    });
    draw_tile(&mut data, w, 10, 11, |x, y, s| {
        let n = hash(x, y, s);
        let row = y / 4;
        let off = (row % 2) * 4;
        if y % 4 == 0 || (x + off) % 8 == 0 {
            [196, 190, 184, 255]
        } else {
            let r = (0.59 + n * 0.06) * 255.0;
            [r as u8, 66, 51, 255]
        }
    });
    draw_tile(&mut data, w, 11, 12, |_x, _y, _s| [200, 226, 240, 255]);
    draw_tile(&mut data, w, 12, 13, |x, y, s| {
        let n = hash(x, y, s);
        let base = (0.92 + n * 0.05) * 255.0;
        [base as u8, (base * 0.98) as u8, (base * 0.99) as u8, 255]
    });
    draw_tile(&mut data, w, 13, 14, |x, y, s| {
        let n = hash(x, y, s);
        let base = (0.15 + n * 0.10) * 255.0;
        [base as u8, (base * 0.75) as u8, (base * 1.25).min(255.0) as u8, 255]
    });
    data
}

#[inline]
fn tile_uv(tile: u32) -> [[f32; 2]; 4] {
    let tc = (tile % ATLAS_COLS) as f32;
    let tr = (tile / ATLAS_COLS) as f32;
    let cols = ATLAS_COLS as f32;
    let rows = ATLAS_ROWS as f32;
    let pad = 0.5 / (ATLAS_TILE * ATLAS_COLS) as f32;
    let u0 = tc / cols + pad;
    let u1 = (tc + 1.0) / cols - pad;
    let v_top = tr / rows + pad;
    let v_bot = (tr + 1.0) / rows - pad;
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

pub fn build_chunk_mesh(world: &WorldData, cx: i32, cz: i32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut uvs:       Vec<[f32; 2]> = Vec::new();
    let mut indices:   Vec<u32>      = Vec::new();

    let x_start = cx * CHUNK_SIZE;
    let z_start = cz * CHUNK_SIZE;

    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let x = x_start + lx;
                let z = z_start + lz;
                let block = world.get(x, y, z);
                if !block.is_solid() { continue; }

                for (i, (dir, corners)) in FACES.iter().enumerate() {
                    if world.is_solid(x + dir[0], y + dir[1], z + dir[2]) {
                        continue;
                    }
                    let tile = match i {
                        2 => block.top_tile(),
                        3 => block.bottom_tile(),
                        _ => block.side_tile(),
                    };
                    let tuv = tile_uv(tile);

                    let base = positions.len() as u32;
                    for (j, corner) in corners.iter().enumerate() {
                        positions.push([
                            x as f32 + corner[0],
                            y as f32 + corner[1],
                            z as f32 + corner[2],
                        ]);
                        normals.push([dir[0] as f32, dir[1] as f32, dir[2] as f32]);
                        uvs.push(tuv[j]);
                    }
                    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[derive(Component)]
pub struct ChunkMesh {
    pub coord: IVec2,
}

#[derive(Resource)]
pub struct ChunkRenderAssets {
    pub material: Handle<StandardMaterial>,
    #[allow(dead_code)]
    pub atlas: Handle<Image>,
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 1200.0,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(WorldData::new())
        .init_resource::<DirtyChunks>()
        .add_systems(Startup, setup_chunk_render_assets)
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
    info!("Generating atlas...");
    let t_atlas = std::time::Instant::now();
    let atlas_data = make_atlas();
    let mut atlas_image = Image::new(
        Extent3d {
            width: ATLAS_TILE * ATLAS_COLS,
            height: ATLAS_TILE * ATLAS_ROWS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        atlas_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    atlas_image.sampler = ImageSampler::nearest();
    let atlas_handle = images.add(atlas_image);
    info!("Atlas generated in {:?}", t_atlas.elapsed());

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(atlas_handle.clone()),
        unlit: false,
        cull_mode: Some(Face::Back),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    commands.insert_resource(ChunkRenderAssets {
        material,
        atlas: atlas_handle,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.9, -0.5, 0.0)),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.8, 2.5, 0.0)),
    ));
}

/// Бюджет генерации в «стабильном» режиме — сколько новых чанков
/// генерировать за кадр при движении.
const STREAM_BUDGET_PER_FRAME: usize = 6;

/// Бюджет генерации в «стартовом» режиме — первые 3 секунды после
/// входа в игру. Загружаем быстро, но не всё сразу.
const STREAM_BUDGET_STARTUP: usize = 24;

/// Бюджет на самом первом кадре. Не `usize::MAX`, чтобы не
/// фризить окно на несколько секунд при большом render distance.
const STREAM_BUDGET_FIRST_FRAME: usize = 32;

/// Длительность «стартовой» фазы (секунды).
const STARTUP_PHASE: f32 = 3.0;

fn stream_chunks(
    time: Res<Time>,
    settings: Res<Settings>,
    seed: Res<WorldSeed>,
    mut world: ResMut<WorldData>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Option<Res<ChunkRenderAssets>>,
    chunk_q: Query<(Entity, &ChunkMesh)>,
    player_q: Query<&Transform, With<Player>>,
    mut startup_time: Local<f32>,
    mut initialized: Local<bool>,
) {
    let Some(assets) = assets else { return; };
    let Ok(pt) = player_q.single() else { return; };

    let pcx = (pt.translation.x.floor() as i32).div_euclid(CHUNK_SIZE);
    let pcz = (pt.translation.z.floor() as i32).div_euclid(CHUNK_SIZE);

    let rd = settings.render_distance_chunks;
    let load_rd = rd + 2;

    let rendered: std::collections::HashSet<IVec2> =
        chunk_q.iter().map(|(_, c)| c.coord).collect();

    // Плавная загрузка вместо «фриза на первом кадре».
    let budget = if !*initialized {
        *initialized = true;
        STREAM_BUDGET_FIRST_FRAME
    } else if *startup_time < STARTUP_PHASE {
        *startup_time += time.delta_secs();
        STREAM_BUDGET_STARTUP
    } else {
        STREAM_BUDGET_PER_FRAME
    };

    if budget > 0 {
        let fwd = pt.forward().as_vec3();
        let fwd_x = fwd.x;
        let fwd_z = fwd.z;

        let mut to_load: Vec<(f32, IVec2)> = Vec::new();
        for dz in -load_rd..=load_rd {
            for dx in -load_rd..=load_rd {
                let cx = pcx + dx;
                let cz = pcz + dz;
                let key = IVec2::new(cx, cz);
                if world.chunks.contains_key(&key) { continue; }
                let fdx = dx as f32;
                let fdz = dz as f32;
                let dist_sq = fdx * fdx + fdz * fdz;
                let dot = fdx * fwd_x + fdz * fwd_z;
                let score = if dot > 0.0 { dist_sq } else { dist_sq * 3.0 };
                to_load.push((score, key));
            }
        }
        to_load.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (_, key) in to_load.into_iter().take(budget) {
            let chunk = generator::generate_chunk(seed.0, key.x, key.y);
            world.insert_chunk(key.x, key.y, chunk);
        }
    }

    // Спавним меши для чанков в радиусе рендера.
    for dz in -rd..=rd {
        for dx in -rd..=rd {
            let cx = pcx + dx;
            let cz = pcz + dz;
            let key = IVec2::new(cx, cz);
            if rendered.contains(&key) { continue; }
            if !world.chunks.contains_key(&key) { continue; }

            let mesh = build_chunk_mesh(&world, cx, cz);
            let handle = meshes.add(mesh);
            commands.spawn((
                Mesh3d(handle),
                MeshMaterial3d(assets.material.clone()),
                Transform::default(),
                ChunkMesh { coord: key },
                Name::new(format!("Chunk({}, {})", cx, cz)),
            ));
        }
    }

    // Выгружаем меши за пределами радиуса рендера.
    for (entity, chunk) in chunk_q.iter() {
        let dx = (chunk.coord.x - pcx).abs();
        let dz = (chunk.coord.y - pcz).abs();
        if dx > rd || dz > rd {
            commands.entity(entity).despawn();
        }
    }

    // Выгружаем из памяти чанки за пределами кэша.
    let mem_rd = rd + 4;
    let to_remove: Vec<IVec2> = world
        .chunks
        .keys()
        .filter(|k| (k.x - pcx).abs() > mem_rd || (k.y - pcz).abs() > mem_rd)
        .copied()
        .collect();
    for k in to_remove {
        world.chunks.remove(&k);
    }
}

fn rebuild_dirty_chunks(
    world: Res<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_q: Query<(&ChunkMesh, &mut Mesh3d)>,
) {
    if dirty.0.is_empty() { return; }

    dirty.0.sort_by_key(|c| (c.x, c.y));
    dirty.0.dedup();

    // `std::mem::take` вместо `drain(..).collect()` — не аллоцирует
    // новый вектор, а забирает старый.
    let coords: Vec<IVec2> = std::mem::take(&mut dirty.0);

    let t0 = std::time::Instant::now();
    let mut rebuilt = 0;
    let mut total_verts = 0usize;

    for coord in coords {
        if !world.has_chunk(coord.x, coord.y) { continue; }

        let new_mesh = build_chunk_mesh(&world, coord.x, coord.y);
        let verts = new_mesh.count_vertices();
        total_verts += verts;
        let new_handle = meshes.add(new_mesh);

        for (chunk, mut mesh3d) in chunk_q.iter_mut() {
            if chunk.coord == coord {
                mesh3d.0 = new_handle.clone();
                rebuilt += 1;
                break;
            }
        }
    }

    if rebuilt > 0 {
        info!(
            "Rebuilt {} chunks in {:?}  ({} verts total)",
            rebuilt,
            t0.elapsed(),
            total_verts
        );
    }
}