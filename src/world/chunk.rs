use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use crate::core::state::{SX, SY, SZ};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockType {
    Air,
    Grass,
    Dirt,
    Stone,
}

impl BlockType {
    #[inline]
    pub fn is_solid(self) -> bool {
        !matches!(self, BlockType::Air)
    }

    /// Returns (top_color, side_color, bottom_color)
    #[inline]
    pub fn colors(self) -> ([f32; 4], [f32; 4], [f32; 4]) {
        match self {
            BlockType::Air => ([0.0; 4], [0.0; 4], [0.0; 4]),
            BlockType::Grass => (
                [0.30, 0.60, 0.15, 1.0],
                [0.45, 0.32, 0.20, 1.0],
                [0.45, 0.32, 0.20, 1.0],
            ),
            BlockType::Dirt => (
                [0.45, 0.32, 0.20, 1.0],
                [0.45, 0.32, 0.20, 1.0],
                [0.45, 0.32, 0.20, 1.0],
            ),
            BlockType::Stone => (
                [0.45, 0.45, 0.45, 1.0],
                [0.45, 0.45, 0.45, 1.0],
                [0.45, 0.45, 0.45, 1.0],
            ),
        }
    }
}

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

    /// Пока не используется — понадобится для спавна мобов и игрока.
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

const FACES: [([i32; 3], [[f32; 3]; 4], f32); 6] = [
    ([1, 0, 0],  [[1.0,0.0,1.0],[1.0,0.0,0.0],[1.0,1.0,0.0],[1.0,1.0,1.0]], 0.75),
    ([-1, 0, 0], [[0.0,0.0,0.0],[0.0,0.0,1.0],[0.0,1.0,1.0],[0.0,1.0,0.0]], 0.75),
    ([0, 1, 0],  [[0.0,1.0,1.0],[1.0,1.0,1.0],[1.0,1.0,0.0],[0.0,1.0,0.0]], 1.0),
    ([0, -1, 0], [[0.0,0.0,0.0],[1.0,0.0,0.0],[1.0,0.0,1.0],[0.0,0.0,1.0]], 0.5),
    ([0, 0, 1],  [[0.0,0.0,1.0],[1.0,0.0,1.0],[1.0,1.0,1.0],[0.0,1.0,1.0]], 0.85),
    ([0, 0, -1], [[1.0,0.0,0.0],[0.0,0.0,0.0],[0.0,1.0,0.0],[1.0,1.0,0.0]], 0.85),
];

pub fn build_world_mesh(world: &WorldData) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut colors:    Vec<[f32; 4]> = Vec::new();
    let mut indices:   Vec<u32>      = Vec::new();

    for y in 0..SY {
        for z in 0..SZ {
            for x in 0..SX {
                let block = world.get(x, y, z);
                if !block.is_solid() { continue; }

                let (c_top, c_side, c_bot) = block.colors();

                for (i, (dir, corners, shade)) in FACES.iter().enumerate() {
                    if world.is_solid(x + dir[0], y + dir[1], z + dir[2]) {
                        continue;
                    }

                    let base_color = match i {
                        2 => c_top,
                        3 => c_bot,
                        _ => c_side,
                    };

                    let base = positions.len() as u32;
                    for corner in corners {
                        positions.push([
                            x as f32 + corner[0],
                            y as f32 + corner[1],
                            z as f32 + corner[2],
                        ]);
                        normals.push([dir[0] as f32, dir[1] as f32, dir[2] as f32]);
                        colors.push([
                            base_color[0] * shade,
                            base_color[1] * shade,
                            base_color[2] * shade,
                            1.0,
                        ]);
                    }
                    indices.extend_from_slice(&[
                        base, base + 1, base + 2,
                        base, base + 2, base + 3,
                    ]);
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldData::new())
           .add_systems(PostStartup, spawn_world_mesh);
        info!("ChunkPlugin loaded.");
    }
}

fn spawn_world_mesh(
    mut commands: Commands,
    world: Res<WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Building world mesh...");
    let t0 = std::time::Instant::now();

    let mesh = build_world_mesh(&world);
    let vertex_count = mesh.count_vertices();
    let handle = meshes.add(mesh);

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(handle),
        MeshMaterial3d(material),
        Transform::default(),
        Name::new("WorldMesh"),
    ));

    info!(
        "World mesh built in {:?}: {} vertices",
        t0.elapsed(),
        vertex_count
    );
}