use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use std::collections::HashMap;

use crate::core::state::{SY, CHUNK_SIZE};
use super::chunk::ChunkSnapshot;
use super::face_textures::{face_type_for, FaceType};

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals:   Vec<[f32; 3]>,
    uvs:       Vec<[f32; 2]>,
    indices:   Vec<u32>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self { positions: Vec::new(), normals: Vec::new(), uvs: Vec::new(), indices: Vec::new() }
    }

    fn add_quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3]) {
        let base = self.positions.len() as u32;
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for j in 0..4 {
            self.positions.push(corners[j]);
            self.normals.push(normal);
            self.uvs.push(uv[j]);
        }
        // ВАЖНО: winding CCW (против часовой) — иначе cull_mode: Back
        // отсекает видимые грани, и мир выглядит «вывернутым».
        self.indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    fn build(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Строит меш чанка — по одному кваду на видимую грань блока
/// (без greedy merge). Возвращает по одному мешу на каждый `FaceType`.
pub fn build_chunk_meshes(snap: &ChunkSnapshot) -> HashMap<FaceType, Mesh> {
    let mut builders: HashMap<FaceType, MeshBuilder> = HashMap::new();

    let x0 = (snap.coord.x * CHUNK_SIZE) as f32;
    let z0 = (snap.coord.y * CHUNK_SIZE) as f32;

    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let block = snap.get(lx, y, lz);
                if !block.is_solid() { continue; }

                let x = x0 + lx as f32;
                let z = z0 + lz as f32;

                // +X
                if !snap.get(lx + 1, y, lz).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[x + 1.0, y as f32, z], [x + 1.0, y as f32, z + 1.0],
                         [x + 1.0, y as f32 + 1.0, z + 1.0], [x + 1.0, y as f32 + 1.0, z]],
                        [1.0, 0.0, 0.0],
                    );
                }

                // -X
                if !snap.get(lx - 1, y, lz).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[x, y as f32, z + 1.0], [x, y as f32, z],
                         [x, y as f32 + 1.0, z], [x, y as f32 + 1.0, z + 1.0]],
                        [-1.0, 0.0, 0.0],
                    );
                }

                // +Y
                if !snap.get(lx, y + 1, lz).is_solid() {
                    let ft = face_type_for(block, true, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[x, y as f32 + 1.0, z], [x + 1.0, y as f32 + 1.0, z],
                         [x + 1.0, y as f32 + 1.0, z + 1.0], [x, y as f32 + 1.0, z + 1.0]],
                        [0.0, 1.0, 0.0],
                    );
                }

                // -Y
                if y > 0 && !snap.get(lx, y - 1, lz).is_solid() {
                    let ft = face_type_for(block, false, true);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[x, y as f32, z + 1.0], [x + 1.0, y as f32, z + 1.0],
                         [x + 1.0, y as f32, z], [x, y as f32, z]],
                        [0.0, -1.0, 0.0],
                    );
                }

                // +Z
                if !snap.get(lx, y, lz + 1).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[x + 1.0, y as f32, z + 1.0], [x, y as f32, z + 1.0],
                         [x, y as f32 + 1.0, z + 1.0], [x + 1.0, y as f32 + 1.0, z + 1.0]],
                        [0.0, 0.0, 1.0],
                    );
                }

                // -Z
                if !snap.get(lx, y, lz - 1).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[x, y as f32, z], [x + 1.0, y as f32, z],
                         [x + 1.0, y as f32 + 1.0, z], [x, y as f32 + 1.0, z]],
                        [0.0, 0.0, -1.0],
                    );
                }
            }
        }
    }

    builders.into_iter().map(|(k, v)| (k, v.build())).collect()
}