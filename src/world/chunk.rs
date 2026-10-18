use bevy::prelude::*;
use bevy::light::GlobalAmbientLight;
use std::collections::HashMap;
use std::collections::HashSet;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};

use crate::core::state::{
    SY, CHUNK_SIZE, CHUNK_AREA, CHUNK_VOLUME, Settings, AppState,
};
use crate::player::controller::Player;
use super::generator::{self, PerlinCacheRes};
use super::face_textures::{setup_face_materials, FaceMaterials};
use super::greedy::build_chunk_meshes;

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
            Grass => Color::srgb(0.30, 0.60, 0.15),
            Dirt => Color::srgb(0.45, 0.32, 0.20),
            Stone => Color::srgb(0.45, 0.45, 0.45),
            Cobblestone => Color::srgb(0.38, 0.38, 0.38),
            Sand => Color::srgb(0.80, 0.75, 0.58),
            Log => Color::srgb(0.42, 0.30, 0.18),
            Leaves => Color::srgb(0.25, 0.50, 0.25),
            Planks => Color::srgb(0.65, 0.52, 0.32),
            Brick => Color::srgb(0.59, 0.26, 0.20),
            Glass => Color::srgb(0.78, 0.88, 0.94),
            Snow => Color::srgb(0.92, 0.96, 0.99),
            Obsidian => Color::srgb(0.15, 0.11, 0.23),
        }
    }
}

#[derive(Clone)]
pub struct ChunkData { pub blocks: Vec<BlockType> }

impl ChunkData {
    pub fn new() -> Self { Self { blocks: vec![BlockType::Air; CHUNK_VOLUME] } }
    #[inline] fn idx(lx: i32, y: i32, lz: i32) -> usize {
        (y as usize) * CHUNK_AREA + (lz as usize) * (CHUNK_SIZE as usize) + (lx as usize)
    }
    #[inline]
    pub fn get_local(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        if y < 0 || y >= SY || lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE {
            return BlockType::Air;
        }
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
pub struct ChunkSnapshot {
    pub coord: IVec2,
    pub blocks: Vec<BlockType>,
}

impl ChunkSnapshot {
    pub fn new(coord: IVec2) -> Self {
        Self { coord, blocks: vec![BlockType::Air; PAD_AREA * SY as usize] }
    }
    #[inline(always)]
    fn idx(lx: i32, y: i32, lz: i32) -> usize {
        let px = (lx + 1) as usize;
        let pz = (lz + 1) as usize;
        (y as usize) * PAD_AREA + pz * (PAD_SIZE as usize) + px
    }
    #[inline(always)]
    pub fn get(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        if y < 0 || y >= SY { return BlockType::Air; }
        if lx < -1 || lx > CHUNK_SIZE || lz < -1 || lz > CHUNK_SIZE { return BlockType::Air; }
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
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < 0 || y >= SY { return BlockType::Air; }
        let cx = x.div_euclid(CHUNK_SIZE);
        let cz = z.div_euclid(CHUNK_SIZE);
        let Some(chunk) = self.chunks.get(&IVec2::new(cx, cz)) else { return BlockType::Air; };
        chunk.get_local(x.rem_euclid(CHUNK_SIZE), y, z.rem_euclid(CHUNK_SIZE))
    }
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if y < 0 || y >= SY { return; }
        let key = IVec2::new(x.div_euclid(CHUNK_SIZE), z.div_euclid(CHUNK_SIZE));
        self.edits.entry(key).or_default().insert(IVec3::new(x, y, z), block);
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.set_local(x.rem_euclid(CHUNK_SIZE), y, z.rem_euclid(CHUNK_SIZE), block);
        }
    }
    pub fn set_natural(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if y < 0 || y >= SY { return; }
        let key = IVec2::new(x.div_euclid(CHUNK_SIZE), z.div_euclid(CHUNK_SIZE));
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.set_local(x.rem_euclid(CHUNK_SIZE), y, z.rem_euclid(CHUNK_SIZE), block);
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
                chunk.set_local(pos.x.rem_euclid(CHUNK_SIZE), pos.y, pos.z.rem_euclid(CHUNK_SIZE), block);
            }
        }
        self.chunks.insert(key, chunk);
    }
    pub fn has_chunk(&self, cx: i32, cz: i32) -> bool { self.chunks.contains_key(&IVec2::new(cx, cz)) }
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
        if lx == 0 { self.0.push(IVec2::new(cx - 1, cz)); }
        if lx == CHUNK_SIZE - 1 { self.0.push(IVec2::new(cx + 1, cz)); }
        if lz == 0 { self.0.push(IVec2::new(cx, cz - 1)); }
        if lz == CHUNK_SIZE - 1 { self.0.push(IVec2::new(cx, cz + 1)); }
    }
}

pub struct PendingChunkTask { pub coord: IVec2, pub task: Task<ChunkData> }
#[derive(Resource, Default)]
pub struct PendingChunks { pub tasks: Vec<PendingChunkTask> }

pub struct PendingMeshTask { pub coord: IVec2, pub task: Task<HashMap<super::face_textures::FaceType, Mesh>> }
#[derive(Resource, Default)]
pub struct PendingMeshes { pub tasks: Vec<PendingMeshTask> }

const MAX_GEN_IN_FLIGHT: usize = 32;
const MAX_MESH_IN_FLIGHT: usize = 16;
const TASKS_SPAWN_PER_FRAME: usize = 6;
const MESH_TASKS_PER_FRAME: usize = 8;
const SYNC_STARTUP_RADIUS: i32 = 1;

#[derive(Component)]
pub struct ChunkMesh { pub coord: IVec2 }

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
        .init_resource::<PendingChunks>()
        .init_resource::<PendingMeshes>()
        .add_systems(Startup, (setup_face_materials, setup_lights))
        .add_systems(PostUpdate, (
            stream_chunks.run_if(in_state(AppState::InGame)),
            rebuild_dirty_chunks.run_if(in_state(AppState::InGame)),
        ).chain());
        info!("ChunkPlugin loaded.");
    }
}

fn setup_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight { illuminance: 8000.0, shadow_maps_enabled: false, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.9, -0.5, 0.0)),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 4000.0, shadow_maps_enabled: false, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.8, 2.5, 0.0)),
    ));
}

fn spawn_chunk_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &FaceMaterials,
    coord: IVec2,
    face_meshes: HashMap<super::face_textures::FaceType, Mesh>,
) {
    commands.spawn((
        Transform::default(),
        Visibility::default(),
        ChunkMesh { coord },
        Name::new(format!("Chunk({}, {})", coord.x, coord.y)),
    )).with_children(|parent| {
        for (ft, mesh) in face_meshes {
            parent.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.get(ft)),
            ));
        }
    });
}

fn stream_chunks(
    settings: Res<Settings>,
    perlin: Res<PerlinCacheRes>,
    mut world: ResMut<WorldData>,
    mut pending_gen: ResMut<PendingChunks>,
    mut pending_mesh: ResMut<PendingMeshes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<Res<FaceMaterials>>,
    chunk_q: Query<(Entity, &ChunkMesh)>,
    player_q: Query<&Transform, With<Player>>,
) {
    let Some(materials) = materials else { return; };
    let Ok(pt) = player_q.single() else { return; };

    let pcx = (pt.translation.x.floor() as i32).div_euclid(CHUNK_SIZE);
    let pcz = (pt.translation.z.floor() as i32).div_euclid(CHUNK_SIZE);
    let rd = settings.render_distance_chunks;
    let load_rd = rd + 2;

    // Первый кадр — синхронная генерация 3×3.
    if world.chunks.is_empty() {
        pending_gen.tasks.clear();
        pending_mesh.tasks.clear();
        for dz in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
            for dx in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
                let cx = pcx + dx;
                let cz = pcz + dz;
                if world.has_chunk(cx, cz) { continue; }
                world.insert_chunk(cx, cz, generator::generate_chunk(&perlin.0, cx, cz));
            }
        }
        for dz in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
            for dx in -SYNC_STARTUP_RADIUS..=SYNC_STARTUP_RADIUS {
                let key = IVec2::new(pcx + dx, pcz + dz);
                let Some(snap) = build_snapshot(&world, key.x, key.y) else { continue; };
                let face_meshes = build_chunk_meshes(&snap);
                spawn_chunk_entity(&mut commands, &mut meshes, &materials, key, face_meshes);
            }
        }
        info!("Sync startup: 3×3 chunks");
    }

    // Поллинг генерации.
    let mut done_gen = Vec::new();
    pending_gen.tasks.retain_mut(|p| {
        if let Some(chunk) = block_on(poll_once(&mut p.task)) {
            done_gen.push((p.coord, chunk));
            false
        } else { true }
    });
    for (coord, chunk) in done_gen {
        if world.chunks.contains_key(&coord) { continue; }
        world.insert_chunk(coord.x, coord.y, chunk);
    }

    // Поллинг мешей.
    let rendered: HashSet<IVec2> = chunk_q.iter().map(|(_, c)| c.coord).collect();
    let mut done_mesh = Vec::new();
    pending_mesh.tasks.retain_mut(|p| {
        if let Some(face_meshes) = block_on(poll_once(&mut p.task)) {
            done_mesh.push((p.coord, face_meshes));
            false
        } else { true }
    });
    for (coord, face_meshes) in done_mesh {
        if !world.has_chunk(coord.x, coord.y) { continue; }
        if rendered.contains(&coord) { continue; }
        spawn_chunk_entity(&mut commands, &mut meshes, &materials, coord, face_meshes);
    }

    // Планирование мешей.
    let pending_mesh_coords: HashSet<IVec2> = pending_mesh.tasks.iter().map(|p| p.coord).collect();
    let mut mesh_spawned = 0usize;
    'mesh_sched: for dz in -rd..=rd {
        for dx in -rd..=rd {
            if mesh_spawned >= MESH_TASKS_PER_FRAME { break 'mesh_sched; }
            if pending_mesh.tasks.len() >= MAX_MESH_IN_FLIGHT { break 'mesh_sched; }
            let key = IVec2::new(pcx + dx, pcz + dz);
            if rendered.contains(&key) || pending_mesh_coords.contains(&key) { continue; }
            if !world.has_chunk(key.x, key.y) { continue; }
            let Some(snap) = build_snapshot(&world, key.x, key.y) else { continue; };
            let pool = AsyncComputeTaskPool::get();
            let task = pool.spawn(async move { build_chunk_meshes(&snap) });
            pending_mesh.tasks.push(PendingMeshTask { coord: key, task });
            mesh_spawned += 1;
        }
    }

    // Выгрузка сущностей.
    for (entity, chunk) in chunk_q.iter() {
        let dx = (chunk.coord.x - pcx).abs();
        let dz = (chunk.coord.y - pcz).abs();
        if dx > rd || dz > rd {
            commands.entity(entity).despawn();
        }
    }

    // Выгрузка памяти.
    let mem_rd = rd + 4;
    let to_remove: Vec<IVec2> = world.chunks.keys()
        .filter(|k| (k.x - pcx).abs() > mem_rd || (k.y - pcz).abs() > mem_rd)
        .copied().collect();
    for k in to_remove { world.chunks.remove(&k); }

    // Планирование генерации.
    if pending_gen.tasks.len() >= MAX_GEN_IN_FLIGHT { return; }
    let in_flight: HashSet<IVec2> = pending_gen.tasks.iter().map(|p| p.coord).collect();
    let fwd = pt.forward().as_vec3();
    let mut to_load: Vec<(f32, IVec2)> = Vec::with_capacity(64);
    for dz in -load_rd..=load_rd {
        for dx in -load_rd..=load_rd {
            let key = IVec2::new(pcx + dx, pcz + dz);
            if world.chunks.contains_key(&key) || in_flight.contains(&key) { continue; }
            let fdx = dx as f32; let fdz = dz as f32;
            let dist_sq = fdx * fdx + fdz * fdz;
            let dot = fdx * fwd.x + fdz * fwd.z;
            let score = if dot > 0.0 { dist_sq } else { dist_sq * 3.0 };
            to_load.push((score, key));
        }
    }
    to_load.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let to_spawn = MAX_GEN_IN_FLIGHT.saturating_sub(pending_gen.tasks.len()).min(TASKS_SPAWN_PER_FRAME);
    let pool = AsyncComputeTaskPool::get();
    let cache_snapshot = perlin.0;
    for (_, key) in to_load.into_iter().take(to_spawn) {
        let (cx, cz) = (key.x, key.y);
        let cache = cache_snapshot;
        let task = pool.spawn(async move { generator::generate_chunk(&cache, cx, cz) });
        pending_gen.tasks.push(PendingChunkTask { coord: key, task });
    }
}

fn rebuild_dirty_chunks(
    world: Res<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<Res<FaceMaterials>>,
    mut chunk_q: Query<(Entity, &ChunkMesh, Option<&Children>)>,
    mut pending_mesh: ResMut<PendingMeshes>,
    mut commands: Commands,
) {
    let Some(materials) = materials else { return; };
    if dirty.0.is_empty() { return; }

    dirty.0.sort_by_key(|c| (c.x, c.y));
    dirty.0.dedup();
    let coords: Vec<IVec2> = std::mem::take(&mut dirty.0);

    for coord in coords {
        if !world.has_chunk(coord.x, coord.y) { continue; }

        // Убираем pending-задачу для этого чанка.
        pending_mesh.tasks.retain(|p| p.coord != coord);

        let Some((root, _, children)) = chunk_q.iter_mut().find(|(_, c, _)| c.coord == coord) else { continue; };

        // Удаляем старых детей.
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        let Some(snap) = build_snapshot(&world, coord.x, coord.y) else { continue; };
        let face_meshes = build_chunk_meshes(&snap);

        commands.entity(root).with_children(|parent| {
            for (ft, mesh) in face_meshes {
                parent.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.get(ft)),
                ));
            }
        });
    }
}