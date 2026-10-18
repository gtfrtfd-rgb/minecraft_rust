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

    /// Добавляет квад. `uv_max` — во сколько раз текстура тайлится
    /// по осям U/V. Для inline-квада [1,1], для greedy [W, H].
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

    // ---------- X-грани ----------
    for x in 0..CHUNK_SIZE {
        let mut mask_pos = vec![None; (SY * CHUNK_SIZE) as usize];
        let mut mask_neg = vec![None; (SY * CHUNK_SIZE) as usize];
        for y in 0..SY {
            for z in 0..CHUNK_SIZE {
                let block = snap.get(x, y, z);
                if !block.is_solid() { continue; }
                let ft = face_type_for(block, false, false);
                let i = (y * CHUNK_SIZE + z) as usize;
                if !snap.get(x + 1, y, z).is_solid() { mask_pos[i] = Some(ft); }
                if !snap.get(x - 1, y, z).is_solid() { mask_neg[i] = Some(ft); }
            }
        }
        merge_x(snap, x, 1, x0, z0, &mask_pos, &mut builders);
        merge_x(snap, x, -1, x0, z0, &mask_neg, &mut builders);
    }

    // ---------- Z-грани ----------
    for z in 0..CHUNK_SIZE {
        let mut mask_pos = vec![None; (SY * CHUNK_SIZE) as usize];
        let mut mask_neg = vec![None; (SY * CHUNK_SIZE) as usize];
        for y in 0..SY {
            for x in 0..CHUNK_SIZE {
                let block = snap.get(x, y, z);
                if !block.is_solid() { continue; }
                let ft = face_type_for(block, false, false);
                let i = (y * CHUNK_SIZE + x) as usize;
                if !snap.get(x, y, z + 1).is_solid() { mask_pos[i] = Some(ft); }
                if !snap.get(x, y, z - 1).is_solid() { mask_neg[i] = Some(ft); }
            }
        }
        merge_z(snap, z, 1, x0, z0, &mask_pos, &mut builders);
        merge_z(snap, z, -1, x0, z0, &mask_neg, &mut builders);
    }

    // ---------- Y-грани (top/bottom) ----------
    for y in 0..SY {
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

fn merge_x(
    _snap: &ChunkSnapshot,
    x: i32,
    sign: i32,
    x0: f32,
    z0: f32,
    mask: &[Option<FaceType>],
    builders: &mut HashMap<FaceType, MeshBuilder>,
) {
    let mut visited = vec![false; mask.len()];
    let face_x = if sign > 0 {
        x0 + (x + 1) as f32
    } else {
        x0 + x as f32
    };

    for y in 0..SY {
        for z in 0..CHUNK_SIZE {
            let i = (y * CHUNK_SIZE + z) as usize;
            if visited[i] { continue; }
            let Some(ft) = mask[i] else { continue; };

            // Расширяем по Z (горизонталь) — всегда можно.
            let mut w = 1;
            while z + w < CHUNK_SIZE {
                let ni = (y * CHUNK_SIZE + z + w) as usize;
                if visited[ni] || mask[ni] != Some(ft) { break; }
                w += 1;
            }

            // Расширяем по Y (вертикаль) — только для uniform текстур.
            let allow_v = ft.merge_vertical_ok();
            let mut h = 1;
            if allow_v {
                'outer: while y + h < SY {
                    for dz in 0..w {
                        let ni = ((y + h) * CHUNK_SIZE + z + dz) as usize;
                        if visited[ni] || mask[ni] != Some(ft) { break 'outer; }
                    }
                    h += 1;
                }
            }

            for dy in 0..h {
                for dz in 0..w {
                    visited[((y + dy) * CHUNK_SIZE + z + dz) as usize] = true;
                }
            }

            let yf = y as f32;
            let zf = z0 + z as f32;
            let (wz, wy) = (w as f32, h as f32);

            // Углы в том же порядке, что inline-версия.
            // Inline +X: [[x+1, y, z+1], [x+1, y, z], [x+1, y+1, z], [x+1, y+1, z+1]]
            // Inline -X: [[x, y, z], [x, y, z+1], [x, y+1, z+1], [x, y+1, z]]
            let corners = if sign > 0 {
                [[face_x, yf, zf + wz], [face_x, yf, zf],
                 [face_x, yf + wy, zf], [face_x, yf + wy, zf + wz]]
            } else {
                [[face_x, yf, zf], [face_x, yf, zf + wz],
                 [face_x, yf + wy, zf + wz], [face_x, yf + wy, zf]]
            };
            let normal = [sign as f32, 0.0, 0.0];

            builders.entry(ft).or_insert_with(MeshBuilder::new)
                .add_quad(corners, normal, [wz, wy]);
        }
    }
}

fn merge_z(
    _snap: &ChunkSnapshot,
    z: i32,
    sign: i32,
    x0: f32,
    z0: f32,
    mask: &[Option<FaceType>],
    builders: &mut HashMap<FaceType, MeshBuilder>,
) {
    let mut visited = vec![false; mask.len()];
    let face_z = if sign > 0 {
        z0 + (z + 1) as f32
    } else {
        z0 + z as f32
    };

    for y in 0..SY {
        for x in 0..CHUNK_SIZE {
            let i = (y * CHUNK_SIZE + x) as usize;
            if visited[i] { continue; }
            let Some(ft) = mask[i] else { continue; };

            // Расширяем по X (горизонталь).
            let mut w = 1;
            while x + w < CHUNK_SIZE {
                let ni = (y * CHUNK_SIZE + x + w) as usize;
                if visited[ni] || mask[ni] != Some(ft) { break; }
                w += 1;
            }

            // Расширяем по Y (вертикаль) — только uniform.
            let allow_v = ft.merge_vertical_ok();
            let mut h = 1;
            if allow_v {
                'outer: while y + h < SY {
                    for dx in 0..w {
                        let ni = ((y + h) * CHUNK_SIZE + x + dx) as usize;
                        if visited[ni] || mask[ni] != Some(ft) { break 'outer; }
                    }
                    h += 1;
                }
            }

            for dy in 0..h {
                for dx in 0..w {
                    visited[((y + dy) * CHUNK_SIZE + x + dx) as usize] = true;
                }
            }

            let xf = x0 + x as f32;
            let yf = y as f32;
            let (wx, wy) = (w as f32, h as f32);

            // Inline +Z: [[x, y, z+1], [x+1, y, z+1], [x+1, y+1, z+1], [x, y+1, z+1]]
            // Inline -Z: [[x+1, y, z], [x, y, z], [x, y+1, z], [x+1, y+1, z]]
            let corners = if sign > 0 {
                [[xf, yf, face_z], [xf + wx, yf, face_z],
                 [xf + wx, yf + wy, face_z], [xf, yf + wy, face_z]]
            } else {
                [[xf + wx, yf, face_z], [xf, yf, face_z],
                 [xf, yf + wy, face_z], [xf + wx, yf + wy, face_z]]
            };
            let normal = [0.0, 0.0, sign as f32];

            builders.entry(ft).or_insert_with(MeshBuilder::new)
                .add_quad(corners, normal, [wx, wy]);
        }
    }
}

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

            let mut w = 1;
            while x + w < CHUNK_SIZE {
                let ni = ((x + w) * CHUNK_SIZE + z) as usize;
                if visited[ni] || mask[ni] != Some(ft) { break; }
                w += 1;
            }
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
                (
                    [[bx, face_y, bz + wz], [bx + wx, face_y, bz + wz],
                     [bx + wx, face_y, bz], [bx, face_y, bz]],
                    [0.0, 1.0, 0.0],
                )
            } else {
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