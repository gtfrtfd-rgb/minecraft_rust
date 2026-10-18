use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::chunk::BlockType;

pub const TILE_SIZE: u32 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FaceType {
    GrassTop,
    GrassSide,
    Dirt,
    Stone,
    Cobblestone,
    Sand,
    LogSide,
    LogTop,
    Leaves,
    Planks,
    Brick,
    Glass,
    Snow,
    Obsidian,
}

impl FaceType {
    /// Uniform/noisy текстуры тянутся нормально по обеим осям.
    /// Directional (полосатые, клетчатые) — только по горизонтали.
    pub fn merge_vertical_ok(self) -> bool {
        matches!(
            self,
            FaceType::GrassTop
            | FaceType::Dirt
            | FaceType::Stone
            | FaceType::Sand
            | FaceType::Snow
            | FaceType::Obsidian
            | FaceType::Leaves
            | FaceType::Glass
            | FaceType::LogTop
        )
    }
}

pub fn face_type_for(block: BlockType, is_top: bool, is_bottom: bool) -> FaceType {
    match block {
        BlockType::Grass => {
            if is_top { FaceType::GrassTop }
            else if is_bottom { FaceType::Dirt }
            else { FaceType::GrassSide }
        }
        BlockType::Dirt => FaceType::Dirt,
        BlockType::Stone => FaceType::Stone,
        BlockType::Cobblestone => FaceType::Cobblestone,
        BlockType::Sand => FaceType::Sand,
        BlockType::Log => {
            if is_top || is_bottom { FaceType::LogTop } else { FaceType::LogSide }
        }
        BlockType::Leaves => FaceType::Leaves,
        BlockType::Planks => FaceType::Planks,
        BlockType::Brick => FaceType::Brick,
        BlockType::Glass => FaceType::Glass,
        BlockType::Snow => FaceType::Snow,
        BlockType::Obsidian => FaceType::Obsidian,
        BlockType::Air => FaceType::Dirt,
    }
}

#[inline]
fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    h = h.wrapping_add(seed.wrapping_mul(1013904223));
    h ^= h >> 13;
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xFF) as f32 / 255.0
}

fn fill_tile(data: &mut [u8], seed: u32, painter: impl Fn(u32, u32, u32) -> [u8; 4]) {
    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let c = painter(px, py, seed);
            let o = ((py * TILE_SIZE + px) * 4) as usize;
            data[o] = c[0]; data[o + 1] = c[1]; data[o + 2] = c[2]; data[o + 3] = c[3];
        }
    }
}

pub fn make_texture(ft: FaceType) -> Image {
    let mut data = vec![0u8; (TILE_SIZE * TILE_SIZE * 4) as usize];

    match ft {
        FaceType::GrassTop => fill_tile(&mut data, 1, |x, y, s| {
            let n = hash(x, y, s);
            let base = (0.32 + n * 0.14) * 255.0;
            [(base * 0.50) as u8, base as u8, (base * 0.28) as u8, 255]
        }),
        FaceType::GrassSide => fill_tile(&mut data, 2, |x, y, s| {
            let n = hash(x, y, s);
            let edge = 3 + (hash(x, 0, s + 7) * 2.0) as u32;
            if y < edge {
                let base = (0.32 + n * 0.14) * 255.0;
                [(base * 0.50) as u8, base as u8, (base * 0.28) as u8, 255]
            } else {
                let base = (0.45 + n * 0.12) * 255.0;
                [base as u8, (base * 0.70) as u8, (base * 0.45) as u8, 255]
            }
        }),
        FaceType::Dirt => fill_tile(&mut data, 3, |x, y, s| {
            let n = hash(x, y, s);
            let base = (0.45 + n * 0.12) * 255.0;
            [base as u8, (base * 0.70) as u8, (base * 0.45) as u8, 255]
        }),
        FaceType::Stone => fill_tile(&mut data, 4, |x, y, s| {
            let n = hash(x, y, s);
            let base = (0.42 + n * 0.12) * 255.0;
            [base as u8, base as u8, base as u8, 255]
        }),
        FaceType::Cobblestone => fill_tile(&mut data, 5, |x, y, _s| {
            let gx = x / 4; let gy = y / 4;
            let v = ((gx * 7 + gy * 13) % 5) as f32 * 0.03;
            let edge = if x % 4 == 0 || y % 4 == 0 { -0.08 } else { 0.0 };
            let b = (0.40 + v + edge).clamp(0.0, 1.0) * 255.0;
            [b as u8, b as u8, b as u8, 255]
        }),
        FaceType::Sand => fill_tile(&mut data, 6, |x, y, s| {
            let n = hash(x, y, s);
            let base = (0.80 + n * 0.08) * 255.0;
            [base as u8, (base * 0.94) as u8, (base * 0.72) as u8, 255]
        }),
        FaceType::LogSide => fill_tile(&mut data, 7, |x, y, s| {
            let n = hash(x, y, s);
            let stripe = ((x as f32 * 0.9).sin() * 0.05).abs();
            let base = (0.42 + n * 0.08 + stripe) * 255.0;
            [base as u8, (base * 0.72) as u8, (base * 0.42) as u8, 255]
        }),
        FaceType::LogTop => fill_tile(&mut data, 8, |x, y, _s| {
            let dx = x as f32 - 7.5; let dy = y as f32 - 7.5;
            let r = (dx * dx + dy * dy).sqrt();
            let ring = (r * 2.4).sin() * 0.05;
            let base = (0.55 + ring) * 255.0;
            [base as u8, (base * 0.78) as u8, (base * 0.48) as u8, 255]
        }),
        FaceType::Leaves => fill_tile(&mut data, 9, |x, y, s| {
            let n = hash(x, y, s);
            let dark = if n < 0.35 { -0.10 } else { 0.0 };
            let base = (0.18 + n * 0.18 + dark).clamp(0.0, 1.0) * 255.0;
            let b = base as u8;
            [b, ((b as f32) * 1.7).min(255.0) as u8, b, 255]
        }),
        FaceType::Planks => fill_tile(&mut data, 10, |x, y, s| {
            let n = hash(x, y, s);
            let row = y / 4; let off = (row % 2) * 4;
            let line = if y % 4 == 0 || (x + off) % 8 == 0 { -0.12 } else { 0.0 };
            let b = (0.65 + n * 0.06 + line).clamp(0.0, 1.0) * 255.0;
            let b = b as u8;
            [b, (b as f32 * 0.80) as u8, (b as f32 * 0.50) as u8, 255]
        }),
        FaceType::Brick => fill_tile(&mut data, 11, |x, y, s| {
            let n = hash(x, y, s);
            let row = y / 4; let off = (row % 2) * 4;
            if y % 4 == 0 || (x + off) % 8 == 0 {
                [196, 190, 184, 255]
            } else {
                let r = (0.59 + n * 0.06) * 255.0;
                [r as u8, 66, 51, 255]
            }
        }),
        FaceType::Glass => fill_tile(&mut data, 12, |_, _, _| [200, 226, 240, 255]),
        FaceType::Snow => fill_tile(&mut data, 13, |x, y, s| {
            let n = hash(x, y, s);
            let b = (0.92 + n * 0.05) * 255.0;
            [b as u8, (b * 0.98) as u8, (b * 0.99) as u8, 255]
        }),
        FaceType::Obsidian => fill_tile(&mut data, 14, |x, y, s| {
            let n = hash(x, y, s);
            let b = (0.15 + n * 0.10) * 255.0;
            [b as u8, (b * 0.75) as u8, (b * 1.25).min(255.0) as u8, 255]
        }),
    }

    let mut img = Image::new(
        Extent3d { width: TILE_SIZE, height: TILE_SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

#[derive(Resource)]
pub struct FaceMaterials {
    pub grass_top: Handle<StandardMaterial>,
    pub grass_side: Handle<StandardMaterial>,
    pub dirt: Handle<StandardMaterial>,
    pub stone: Handle<StandardMaterial>,
    pub cobblestone: Handle<StandardMaterial>,
    pub sand: Handle<StandardMaterial>,
    pub log_side: Handle<StandardMaterial>,
    pub log_top: Handle<StandardMaterial>,
    pub leaves: Handle<StandardMaterial>,
    pub planks: Handle<StandardMaterial>,
    pub brick: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
    pub snow: Handle<StandardMaterial>,
    pub obsidian: Handle<StandardMaterial>,
}

impl FaceMaterials {
    pub fn get(&self, ft: FaceType) -> Handle<StandardMaterial> {
        match ft {
            FaceType::GrassTop => self.grass_top.clone(),
            FaceType::GrassSide => self.grass_side.clone(),
            FaceType::Dirt => self.dirt.clone(),
            FaceType::Stone => self.stone.clone(),
            FaceType::Cobblestone => self.cobblestone.clone(),
            FaceType::Sand => self.sand.clone(),
            FaceType::LogSide => self.log_side.clone(),
            FaceType::LogTop => self.log_top.clone(),
            FaceType::Leaves => self.leaves.clone(),
            FaceType::Planks => self.planks.clone(),
            FaceType::Brick => self.brick.clone(),
            FaceType::Glass => self.glass.clone(),
            FaceType::Snow => self.snow.clone(),
            FaceType::Obsidian => self.obsidian.clone(),
        }
    }
}

pub fn setup_face_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut make_mat = |ft: FaceType| {
        let tex = images.add(make_texture(ft));
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(tex),
            unlit: false,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            ..default()
        })
    };

    commands.insert_resource(FaceMaterials {
        grass_top: make_mat(FaceType::GrassTop),
        grass_side: make_mat(FaceType::GrassSide),
        dirt: make_mat(FaceType::Dirt),
        stone: make_mat(FaceType::Stone),
        cobblestone: make_mat(FaceType::Cobblestone),
        sand: make_mat(FaceType::Sand),
        log_side: make_mat(FaceType::LogSide),
        log_top: make_mat(FaceType::LogTop),
        leaves: make_mat(FaceType::Leaves),
        planks: make_mat(FaceType::Planks),
        brick: make_mat(FaceType::Brick),
        glass: make_mat(FaceType::Glass),
        snow: make_mat(FaceType::Snow),
        obsidian: make_mat(FaceType::Obsidian),
    });

    info!("Face materials ready (14 textures)");
}