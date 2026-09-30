use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::ImageSampler;
use bevy::render::render_resource::Face;
use crate::core::state::{SX, SY, SZ, CHUNK_SIZE};

// ============================================================
// БЛОКИ
// ============================================================
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockType {
    Air,
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
    pub fn top_tile(self) -> u32 {
        match self {
            BlockType::Air | BlockType::Stone | BlockType::Obsidian => 3,
            BlockType::Grass => 0,
            BlockType::Dirt => 2,
            BlockType::Cobblestone => 4,
            BlockType::Sand => 5,
            BlockType::Log | BlockType::Planks => 7,
            BlockType::Leaves => 8,
            BlockType::Brick => 10,
            BlockType::Glass => 11,
            BlockType::Snow => 12,
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
// ДАННЫЕ МИРА
// ============================================================
#[derive(Resource)]
pub struct WorldData {
    pub blocks: Vec<BlockType>,
}

impl WorldData {
    pub fn new() -> Self {
        Self {
            blocks: vec![BlockType::Air; (SX * SY * SZ) as usize],
        }
    }

    #[inline]
    fn idx(x: i32, y: i32, z: i32) -> usize {
        (y as usize) * (SX as usize) * (SZ as usize)
            + (z as usize) * (SX as usize)
            + (x as usize)
    }

    #[inline]
    pub fn in_bounds(x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < SX && y >= 0 && y < SY && z >= 0 && z < SZ
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if !Self::in_bounds(x, y, z) {
            return BlockType::Air;
        }
        self.blocks[Self::idx(x, y, z)]
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if Self::in_bounds(x, y, z) {
            self.blocks[Self::idx(x, y, z)] = block;
        }
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if y < 0 { return true; }
        if y >= SY { return false; }
        if !Self::in_bounds(x, y, z) { return false; }
        self.get(x, y, z).is_solid()
    }

    #[allow(dead_code)]
    pub fn highest_at(&self, x: i32, z: i32) -> i32 {
        for y in (0..SY).rev() {
            if self.get(x, y, z).is_solid() {
                return y;
            }
        }
        -1
    }
}

// ============================================================
// СПИСОК ЧАНКОВ НА ПЕРЕСТРОЙКУ
// ============================================================
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
// АТЛАС
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
        let base = (0.30 + n * 0.15) * 255.0;
        [(base * 0.55) as u8, base as u8, (base * 0.30) as u8, 255]
    });
    draw_tile(&mut data, w, 1, 2, |x, y, s| {
        let n = hash(x, y, s);
        let edge = 3 + (hash(x, 0, s + 7) * 2.0) as u32;
        if y < edge {
            let base = (0.30 + n * 0.15) * 255.0;
            [(base * 0.55) as u8, base as u8, (base * 0.3) as u8, 255]
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
        let dark = if n < 0.2 { -0.15 } else { 0.0 };
        let base = (0.22 + n * 0.15 + dark).clamp(0.0, 1.0) * 255.0;
        let base = base as u8;
        [base, ((base as f32) * 2.2).min(255.0) as u8, base, 255]
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
    let u0 = tc / cols;
    let u1 = (tc + 1.0) / cols;
    let v_top = tr / rows;
    let v_bot = (tr + 1.0) / rows;
    [[u0, v_bot], [u1, v_bot], [u1, v_top], [u0, v_top]]
}

// ============================================================
// МЕШ
// ============================================================
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
        for z in z_start..(z_start + CHUNK_SIZE) {
            for x in x_start..(x_start + CHUNK_SIZE) {
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

// ============================================================
// КОМПОНЕНТ
// ============================================================
#[derive(Component)]
pub struct ChunkMesh {
    pub coord: IVec2,
}

// ============================================================
// ПЛАГИН
// ============================================================
pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1200.0,  // было 350 — теперь стены не чёрные
        })
        .insert_resource(WorldData::new())
        .init_resource::<DirtyChunks>()
        .add_systems(PostStartup, spawn_all_chunks)
        .add_systems(PostUpdate, rebuild_dirty_chunks);
        info!("ChunkPlugin loaded.");
    }
}

fn spawn_all_chunks(
    mut commands: Commands,
    world: Res<WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
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
        base_color_texture: Some(atlas_handle),
        unlit: false,
        cull_mode: Some(Face::Back),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    info!("Building chunk meshes...");
    let t0 = std::time::Instant::now();
    let chunks_x = SX / CHUNK_SIZE;
    let chunks_z = SZ / CHUNK_SIZE;
    let mut total_vertices = 0usize;

    for cx in 0..chunks_x {
        for cz in 0..chunks_z {
            let mesh = build_chunk_mesh(&world, cx, cz);
            total_vertices += mesh.count_vertices();
            let handle = meshes.add(mesh);
            commands.spawn((
                Mesh3d(handle),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                ChunkMesh { coord: IVec2::new(cx, cz) },
                Name::new(format!("Chunk({}, {})", cx, cz)),
            ));
        }
    }

    info!(
        "All {} chunks built in {:?}: {} vertices total",
        chunks_x * chunks_z,
        t0.elapsed(),
        total_vertices
    );

    // --- Основной свет (солнце) ---
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.9, -0.5, 0.0)),
    ));

    // --- Дополнительный свет "от неба" с другой стороны ---
    // Убирает чёрные провалы на вертикальных гранях в ямах
    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.8, 2.5, 0.0)),
    ));
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

    let coords: Vec<IVec2> = dirty.0.drain(..).collect();
    let t0 = std::time::Instant::now();
    let mut rebuilt = 0;
    let mut total_verts = 0usize;

    for coord in coords {
        if coord.x < 0 || coord.x >= (SX / CHUNK_SIZE) { continue; }
        if coord.y < 0 || coord.y >= (SZ / CHUNK_SIZE) { continue; }

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