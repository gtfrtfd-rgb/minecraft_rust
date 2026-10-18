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

    /// Добавляет квад. `uv_max` — на сколько раз текстура тайлится
    /// по X и Y. Для inline-квада (1×1 блок) это [1, 1].
    /// Для greedy-квада (W×H блоков) — [W, H], и текстура с
    /// `address_mode: Repeat` тайлится ровно W×H раз — визуально
    /// идентично W×H отдельным блокам.
    fn add_quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3], uv_max: [f32; 2]) {
        let base = self.positions.len() as u32;
        let uv = [
            [0.0, 0.0],
            [uv_max[0], 0.0],
            [uv_max[0], uv_max[1]],
            [0.0, uv_max[1]],
        ];
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

pub fn build_chunk_meshes(snap: &ChunkSnapshot) -> HashMap<FaceType, Mesh> {
    let mut builders: HashMap<FaceType, MeshBuilder> = HashMap::new();

    let x0 = (snap.coord.x * CHUNK_SIZE) as f32;
    let z0 = (snap.coord.y * CHUNK_SIZE) as f32;

    // ---------- X и Z грани: inline, как в рабочей версии ----------
    for y in 0..SY {
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let block = snap.get(lx, y, lz);
                if !block.is_solid() { continue; }

                let bx = x0 + lx as f32;
                let by = y as f32;
                let bz = z0 + lz as f32;

                // +X
                if !snap.get(lx + 1, y, lz).is_solid() {
                    let ft = face_type_for(block, false, false);
                    builders.entry(ft).or_insert_with(MeshBuilder::new).add_quad(
                        [[bx + 1.0, by, bz + 1.0], [bx + 1.0, by, bz],
                         [bx + 1.0, by + 1.0, bz], [bx + 1.0, by + 1.0, bz + 1.0]],
                        [1.0, 0.0, 0.0], [1.0, 1.0],
                    );
                }

                // -X
                if !snap.get(lx - 1, y, lz).is_solid() {
                    let ft = face_type_for(block, false, false);
                    builders.entry(ft).or_insert_with(MeshBuilder::new).add_quad(
                        [[bx, by, bz], [bx, by, bz + 1.0],
                         [bx, by + 1.0, bz + 1.0], [bx, by + 1.0, bz]],
                        [-1.0, 0.0, 0.0], [1.0, 1.0],
                    );
                }

                // +Z
                if !snap.get(lx, y, lz + 1).is_solid() {
                    let ft = face_type_for(block, false, false);
                    builders.entry(ft).or_insert_with(MeshBuilder::new).add_quad(
                        [[bx, by, bz + 1.0], [bx + 1.0, by, bz + 1.0],
                         [bx + 1.0, by + 1.0, bz + 1.0], [bx, by + 1.0, bz + 1.0]],
                        [0.0, 0.0, 1.0], [1.0, 1.0],
                    );
                }

                // -Z
                if !snap.get(lx, y, lz - 1).is_solid() {
                    let ft = face_type_for(block, false, false);
                    builders.entry(ft).or_insert_with(MeshBuilder::new).add_quad(
                        [[bx + 1.0, by, bz], [bx, by, bz],
                         [bx, by + 1.0, bz], [bx + 1.0, by + 1.0, bz]],
                        [0.0, 0.0, -1.0], [1.0, 1.0],
                    );
                }
            }
        }
    }

    // ---------- Y грани: greedy merge по X и Z ----------
    for y in 0..SY {
        // Top (+Y)
        let mut mask_top = vec![None; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let block = snap.get(x, y, z);
                if !block.is_solid() { continue; }
                if !snap.get(x, y + 1, z).is_solid() {
                    mask_top[(x * CHUNK_SIZE + z) as usize] =
                        Some(face_type_for(block, true, false));
                }
            }
        }
        greedy_y(y, 1, x0, z0, &mask_top, &mut builders);

        // Bottom (-Y)
        if y > 0 {
            let mut mask_bot = vec![None; (CHUNK_SIZE * CHUNK_SIZE) as usize];
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let block = snap.get(x, y, z);
                    if !block.is_solid() { continue; }
                    if !snap.get(x, y - 1, z).is_solid() {
                        mask_bot[(x * CHUNK_SIZE + z) as usize] =
                            Some(face_type_for(block, false, true));
                    }
                }
            }
            greedy_y(y, -1, x0, z0, &mask_bot, &mut builders);
        }
    }

    builders.into_iter().map(|(k, v)| (k, v.build())).collect()
}

/// Greedy merge Y-граней (top/bottom) по осям X и Z.
/// Углы и winding совпадают с inline-версией — размеры W×H
/// просто подставлены вместо 1×1.
fn greedy_y(
    y: i32,
    sign: i32,
    x0: f32,
    z0: f32,
    mask: &[Option<FaceType>],
    builders: &mut HashMap<FaceType, MeshBuilder>,
) {
    let mut visited = vec![false; mask.len()];
    let face_y = y as f32 + if sign > 0 { 1.0 } else { 0.0 };

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let i = (x * CHUNK_SIZE + z) as usize;
            if visited[i] { continue; }
            let Some(ft) = mask[i] else { continue; };

            // Расширяем по X
            let mut w = 1;
            while x + w < CHUNK_SIZE {
                let ni = ((x + w) * CHUNK_SIZE + z) as usize;
                if visited[ni] || mask[ni] != Some(ft) { break; }
                w += 1;
            }

            // Расширяем по Z
            let mut h = 1;
            'outer: while z + h < CHUNK_SIZE {
                for dx in 0..w {
                    let ni = ((x + dx) * CHUNK_SIZE + z + h) as usize;
                    if visited[ni] || mask[ni] != Some(ft) { break 'outer; }
                }
                h += 1;
            }

            for dz in 0..h {
                for dx in 0..w {
                    visited[((x + dx) * CHUNK_SIZE + z + dz) as usize] = true;
                }
            }

            let bx = x0 + x as f32;
            let bz = z0 + z as f32;
            let (wx, wz) = (w as f32, h as f32);

            let (corners, normal) = if sign > 0 {
                // Top: порядок из inline: (x, z+H), (x+W, z+H), (x+W, z), (x, z)
                (
                    [[bx, face_y, bz + wz], [bx + wx, face_y, bz + wz],
                     [bx + wx, face_y, bz], [bx, face_y, bz]],
                    [0.0, 1.0, 0.0],
                )
            } else {
                // Bottom: порядок из inline: (x, z), (x+W, z), (x+W, z+H), (x, z+H)
                (
                    [[bx, face_y, bz], [bx + wx, face_y, bz],
                     [bx + wx, face_y, bz + wz], [bx, face_y, bz + wz]],
                    [0.0, -1.0, 0.0],
                )
            };

            builders.entry(ft).or_insert_with(MeshBuilder::new)
                .add_quad(corners, normal, [wx, wz]);
        }
    }
}