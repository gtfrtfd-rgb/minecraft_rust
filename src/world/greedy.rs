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
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
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

/// Строит меш чанка — по одному кваду на видимую грань блока.
/// Углы квадов идентичны рабочим из atlas-версии (проверено).
pub fn build_chunk_meshes(snap: &ChunkSnapshot) -> HashMap<FaceType, Mesh> {
    let mut builders: HashMap<FaceType, MeshBuilder> = HashMap::new();

    let x0 = (snap.coord.x * CHUNK_SIZE) as f32;
    let z0 = (snap.coord.y * CHUNK_SIZE) as f32;

    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let block = snap.get(lx, y, lz);
                if !block.is_solid() { continue; }

                let bx = x0 + lx as f32;
                let by = y as f32;
                let bz = z0 + lz as f32;

                // +X (углы из atlas-версии, работают)
                if !snap.get(lx + 1, y, lz).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[bx + 1.0, by, bz + 1.0], [bx + 1.0, by, bz],
                         [bx + 1.0, by + 1.0, bz], [bx + 1.0, by + 1.0, bz + 1.0]],
                        [1.0, 0.0, 0.0],
                    );
                }

                // -X
                if !snap.get(lx - 1, y, lz).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[bx, by, bz], [bx, by, bz + 1.0],
                         [bx, by + 1.0, bz + 1.0], [bx, by + 1.0, bz]],
                        [-1.0, 0.0, 0.0],
                    );
                }

                // +Y (top)
                if !snap.get(lx, y + 1, lz).is_solid() {
                    let ft = face_type_for(block, true, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[bx, by + 1.0, bz + 1.0], [bx + 1.0, by + 1.0, bz + 1.0],
                         [bx + 1.0, by + 1.0, bz], [bx, by + 1.0, bz]],
                        [0.0, 1.0, 0.0],
                    );
                }

                // -Y (bottom)
                if y > 0 && !snap.get(lx, y - 1, lz).is_solid() {
                    let ft = face_type_for(block, false, true);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[bx, by, bz], [bx + 1.0, by, bz],
                         [bx + 1.0, by, bz + 1.0], [bx, by, bz + 1.0]],
                        [0.0, -1.0, 0.0],
                    );
                }

                // +Z (front)
                if !snap.get(lx, y, lz + 1).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[bx, by, bz + 1.0], [bx + 1.0, by, bz + 1.0],
                         [bx + 1.0, by + 1.0, bz + 1.0], [bx, by + 1.0, bz + 1.0]],
                        [0.0, 0.0, 1.0],
                    );
                }

                // -Z (back)
                if !snap.get(lx, y, lz - 1).is_solid() {
                    let ft = face_type_for(block, false, false);
                    let b = builders.entry(ft).or_insert_with(MeshBuilder::new);
                    b.add_quad(
                        [[bx + 1.0, by, bz], [bx, by, bz],
                         [bx, by + 1.0, bz], [bx + 1.0, by + 1.0, bz]],
                        [0.0, 0.0, -1.0],
                    );
                }
            }
        }
    }

    builders.into_iter().map(|(k, v)| (k, v.build())).collect()
}