use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::RngExt;
use rand::rngs::SmallRng;
use rand::SeedableRng;

use crate::core::state::{SY, AppState, CHUNK_SIZE};
use crate::world::chunk::WorldData;
use crate::player::controller::Player;
use super::ai::{Mob, MobType};

pub struct MobSpawnPlugin;

impl Plugin for MobSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_mob_assets)
            .add_systems(
                Update,
                spawn_cycle.run_if(in_state(AppState::InGame)),
            );
        info!("MobSpawnPlugin loaded.");
    }
}

// ============================================================
// ПАРАМЕТРЫ СПАВНА
// ============================================================

const MIN_SPAWN_DISTANCE: f32 = 24.0;
const MAX_SPAWN_DISTANCE: f32 = 128.0;
const PASSIVE_MOB_CAP: usize = 30;
const SPAWN_CYCLE_INTERVAL: f32 = 4.0;
const SPAWN_ATTEMPTS_PER_CYCLE: usize = 16;
const PACK_SIZE_MIN: usize = 2;
const PACK_SIZE_MAX: usize = 4;
const PACK_RADIUS: f32 = 4.0;

// ============================================================
// ОБЩИЙ АТЛАС МОБОВ
// ============================================================
//
// Один атлас 8×4 тайла (128×64 пикселей) вместо четырёх отдельных
// 4×4. Раскладка:
//   Row 0: [pig_body, pig_head, pig_leg, pig_detail, pig_eye, pig_extra, WHITE, —]
//   Row 1: [sheep_*, ..., WHITE-ссылка, —]
//   Row 2: [cow_*, ..., WHITE-ссылка, —]
//   Row 3: [chicken_*, ..., WHITE-ссылка, —]
//
// WHITE-тайл один на всех (индекс 7), потому что его цвет
// одинаков для всех мобов.

const TILE: u32 = 16;
const ATLAS_COLS: u32 = 8;
const ATLAS_ROWS: u32 = 4;
const ATLAS_W: u32 = TILE * ATLAS_COLS; // 128
const ATLAS_H: u32 = TILE * ATLAS_ROWS; // 64

/// Смещение внутри ряда для каждого моба.
const T_BODY:   u32 = 0;
const T_HEAD:   u32 = 1;
const T_LEG:    u32 = 2;
const T_DETAIL: u32 = 3;
const T_EYE:    u32 = 4;
const T_EXTRA:  u32 = 5;

/// Общий белый тайл (один на всех мобов).
const T_WHITE:  u32 = 7;

#[inline]
fn mob_tile_base(mt: MobType) -> u32 {
    match mt {
        MobType::Pig     => 0,
        MobType::Sheep   => 8,
        MobType::Cow     => 16,
        MobType::Chicken => 24,
    }
}

const CUBE_FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
    ([ 1.0, 0.0, 0.0], [[ 1.0,-1.0, 1.0],[ 1.0,-1.0,-1.0],[ 1.0, 1.0,-1.0],[ 1.0, 1.0, 1.0]]),
    ([-1.0, 0.0, 0.0], [[-1.0,-1.0,-1.0],[-1.0,-1.0, 1.0],[-1.0, 1.0, 1.0],[-1.0, 1.0,-1.0]]),
    ([0.0,  1.0, 0.0], [[-1.0, 1.0, 1.0],[ 1.0, 1.0, 1.0],[ 1.0, 1.0,-1.0],[-1.0, 1.0,-1.0]]),
    ([0.0, -1.0, 0.0], [[-1.0,-1.0,-1.0],[ 1.0,-1.0,-1.0],[ 1.0,-1.0, 1.0],[-1.0,-1.0, 1.0]]),
    ([0.0, 0.0,  1.0], [[-1.0,-1.0, 1.0],[ 1.0,-1.0, 1.0],[ 1.0, 1.0, 1.0],[-1.0, 1.0, 1.0]]),
    ([0.0, 0.0, -1.0], [[ 1.0,-1.0,-1.0],[-1.0,-1.0,-1.0],[-1.0, 1.0,-1.0],[ 1.0, 1.0,-1.0]]),
];

fn tile_uv(tile: u32) -> [[f32; 2]; 4] {
    let cols = ATLAS_COLS as f32;
    let rows = ATLAS_ROWS as f32;
    let inset = 0.5 / ATLAS_W as f32;
    let tc = (tile % ATLAS_COLS) as f32;
    let tr = (tile / ATLAS_COLS) as f32;
    [
        [(tc + 0.0) / cols + inset, (tr + 1.0) / rows - inset],
        [(tc + 1.0) / cols - inset, (tr + 1.0) / rows - inset],
        [(tc + 1.0) / cols - inset, (tr + 0.0) / rows + inset],
        [(tc + 0.0) / cols + inset, (tr + 0.0) / rows + inset],
    ]
}

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

    fn add_cube(&mut self, center: Vec3, size: Vec3, tile: u32) {
        let half = size * 0.5;
        let tuv = tile_uv(tile);
        for (normal, corners) in CUBE_FACES.iter() {
            let base = self.positions.len() as u32;
            for (j, c) in corners.iter().enumerate() {
                self.positions.push([
                    center.x + c[0] * half.x,
                    center.y + c[1] * half.y,
                    center.z + c[2] * half.z,
                ]);
                self.normals.push(*normal);
                self.uvs.push(tuv[j]);
            }
            self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
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

struct MobColors { body: Color, head: Color, leg: Color, detail: Color, extra: Color, eye: Color }

fn colors_for(mt: MobType) -> MobColors {
    match mt {
        MobType::Pig => MobColors {
            body:   Color::srgb(0.95, 0.62, 0.62),
            head:   Color::srgb(0.95, 0.62, 0.62),
            leg:    Color::srgb(0.80, 0.50, 0.50),
            detail: Color::srgb(0.85, 0.45, 0.45),
            extra:  Color::srgb(0.95, 0.62, 0.62),
            eye:    Color::srgb(0.05, 0.03, 0.05),
        },
        MobType::Sheep => MobColors {
            body:   Color::srgb(0.94, 0.94, 0.91),
            head:   Color::srgb(0.62, 0.45, 0.33),
            leg:    Color::srgb(0.55, 0.40, 0.29),
            detail: Color::srgb(0.42, 0.30, 0.22),
            extra:  Color::srgb(0.80, 0.74, 0.66),
            eye:    Color::srgb(0.05, 0.03, 0.05),
        },
        MobType::Cow => MobColors {
            body:   Color::srgb(0.24, 0.15, 0.08),
            head:   Color::srgb(0.20, 0.12, 0.06),
            leg:    Color::srgb(0.22, 0.14, 0.07),
            detail: Color::srgb(0.92, 0.88, 0.80),
            extra:  Color::srgb(0.10, 0.06, 0.03),
            eye:    Color::srgb(0.04, 0.02, 0.03),
        },
        MobType::Chicken => MobColors {
            body:   Color::srgb(0.98, 0.98, 0.98),
            head:   Color::srgb(0.98, 0.98, 0.98),
            leg:    Color::srgb(0.92, 0.62, 0.12),
            detail: Color::srgb(0.98, 0.68, 0.10),
            extra:  Color::srgb(0.82, 0.14, 0.14),
            eye:    Color::srgb(0.05, 0.03, 0.05),
        },
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

#[inline]
fn rgb(c: Color) -> [u8; 3] {
    let s = c.to_srgba();
    [
        (s.red   * 255.0).clamp(0.0, 255.0).round() as u8,
        (s.green * 255.0).clamp(0.0, 255.0).round() as u8,
        (s.blue  * 255.0).clamp(0.0, 255.0).round() as u8,
    ]
}

fn shade_pixel(base: [u8; 3], x: u32, y: u32, seed: u32, intensity: i32) -> [u8; 4] {
    let n = hash(x, y, seed);
    let delta = if n < 0.12 { -intensity }
        else if n > 0.88 { intensity }
        else if n < 0.30 { -(intensity / 2) }
        else if n > 0.70 { intensity / 2 }
        else { 0 };
    [
        (base[0] as i32 + delta).clamp(0, 255) as u8,
        (base[1] as i32 + delta).clamp(0, 255) as u8,
        (base[2] as i32 + delta).clamp(0, 255) as u8,
        255,
    ]
}

fn draw_tile<F>(data: &mut [u8], tile_idx: u32, f: F)
where F: Fn(u32, u32) -> [u8; 4]
{
    let tx = (tile_idx % ATLAS_COLS) * TILE;
    let ty = (tile_idx / ATLAS_COLS) * TILE;
    for py in 0..TILE {
        for px in 0..TILE {
            let c = f(px, py);
            let o = (((ty + py) * ATLAS_W + (tx + px)) * 4) as usize;
            data[o] = c[0]; data[o + 1] = c[1]; data[o + 2] = c[2]; data[o + 3] = c[3];
        }
    }
}

/// Рисует 6 тайлов одного моба (body/head/leg/detail/eye/extra).
/// WHITE не рисует — он общий.
fn draw_mob_tiles(data: &mut [u8], mt: MobType) {
    let c = colors_for(mt);
    let base = mob_tile_base(mt);
    let body   = rgb(c.body);
    let head   = rgb(c.head);
    let leg    = rgb(c.leg);
    let detail = rgb(c.detail);
    let extra  = rgb(c.extra);
    let eye    = rgb(c.eye);

    draw_tile(data, base + T_BODY,   |x, y| shade_pixel(body,   x, y, 1, 10));
    draw_tile(data, base + T_HEAD,   |x, y| shade_pixel(head,   x, y, 2, 10));
    draw_tile(data, base + T_LEG,    |x, y| shade_pixel(leg,    x, y, 3, 12));
    draw_tile(data, base + T_DETAIL, |x, y| shade_pixel(detail, x, y, 4,  8));
    draw_tile(data, base + T_EXTRA,  |x, y| shade_pixel(extra,  x, y, 6, 10));
    draw_tile(data, base + T_EYE, |x, y| {
        let n = hash(x, y, 5);
        let v = if n > 0.75 { 22 } else { 0 };
        [
            (eye[0] as i32 + v).min(255) as u8,
            (eye[1] as i32 + v).min(255) as u8,
            (eye[2] as i32 + v).min(255) as u8,
            255,
        ]
    });
}

/// Общий белый тайл — используется всеми мобами.
fn draw_shared_white(data: &mut [u8]) {
    draw_tile(data, T_WHITE, |x, y| {
        let n = hash(x, y, 7);
        let v: u8 = if n < 0.20 { 208 } else if n > 0.80 { 222 } else { 215 };
        [(v as f32 * 0.98) as u8, v, (v as f32 * 0.90) as u8, 255]
    });
}

/// Создаёт единый атлас 128×64 со всеми мобами и общим белым тайлом.
fn make_mob_atlas() -> Image {
    let mut data = vec![0u8; (ATLAS_W * ATLAS_H * 4) as usize];

    draw_mob_tiles(&mut data, MobType::Pig);
    draw_mob_tiles(&mut data, MobType::Sheep);
    draw_mob_tiles(&mut data, MobType::Cow);
    draw_mob_tiles(&mut data, MobType::Chicken);
    draw_shared_white(&mut data);

    let mut img = Image::new(
        Extent3d { width: ATLAS_W, height: ATLAS_H, depth_or_array_layers: 1 },
        TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::nearest();
    img
}

fn build_body_mesh(mt: MobType) -> Mesh {
    let mut b = MeshBuilder::new();
    let body_size = mt.body_size();
    let head_size = mt.head_size();
    let leg_size  = mt.leg_size();

    let base = mob_tile_base(mt);
    let t_body   = base + T_BODY;
    let t_head   = base + T_HEAD;
    let t_detail = base + T_DETAIL;
    let t_extra  = base + T_EXTRA;
    let t_white  = T_WHITE;

    let body_bottom = leg_size.y;
    let body_top    = body_bottom + body_size.y;
    let body_center_y = (body_bottom + body_top) * 0.5;

    let head_center_y = body_top - head_size.y * 0.5;
    let head_center_z = -(body_size.z * 0.5 + head_size.z * 0.5);
    let head_front_z  = head_center_z - head_size.z * 0.5;

    b.add_cube(Vec3::new(0.0, body_center_y, 0.0), body_size, t_body);

    let head_tile = if matches!(mt, MobType::Sheep) { t_body } else { t_head };
    b.add_cube(Vec3::new(0.0, head_center_y, head_center_z), head_size, head_tile);

    match mt {
        MobType::Pig => {
            let snout_size = Vec3::new(head_size.x * 0.55, head_size.y * 0.42, 0.06);
            let snout_y = head_center_y - head_size.y * 0.15;
            b.add_cube(Vec3::new(0.0, snout_y, head_front_z - snout_size.z * 0.5), snout_size, t_body);
            for sx in [-1.0_f32, 1.0] {
                b.add_cube(
                    Vec3::new(sx * snout_size.x * 0.28, snout_y, head_front_z - snout_size.z - 0.004),
                    Vec3::new(snout_size.x * 0.18, snout_size.y * 0.35, 0.008), t_body);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.30, head_center_y + head_size.y * 0.12, head_front_z - 0.004),
                    Vec3::new(0.13, 0.13, 0.008), t_white);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.30, head_center_y + head_size.y * 0.12, head_front_z - 0.011),
                    Vec3::new(0.11, 0.11, 0.008), base + T_EYE);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.36, head_center_y + head_size.y * 0.5 + 0.035, head_center_z + head_size.z * 0.15),
                    Vec3::new(0.11, 0.10, 0.06), t_body);
            }
        }
        MobType::Sheep => {
            let muzzle = Vec3::new(head_size.x * 0.85, head_size.y * 0.60, 0.04);
            let muzzle_y = head_center_y - head_size.y * 0.08;
            b.add_cube(Vec3::new(0.0, muzzle_y, head_front_z - muzzle.z * 0.5), muzzle, t_head);
            for sx in [-1.0_f32, 1.0] {
                b.add_cube(
                    Vec3::new(sx * muzzle.x * 0.20, muzzle_y - muzzle.y * 0.15, head_front_z - muzzle.z - 0.003),
                    Vec3::new(0.035, 0.035, 0.008), base + T_EYE);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.30, head_center_y + head_size.y * 0.10, head_front_z - muzzle.z - 0.006),
                    Vec3::new(0.13, 0.13, 0.008), t_white);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.30, head_center_y + head_size.y * 0.10, head_front_z - muzzle.z - 0.013),
                    Vec3::new(0.11, 0.11, 0.008), base + T_EYE);
                b.add_cube(
                    Vec3::new(sx * (head_size.x * 0.5 + 0.035), head_center_y + head_size.y * 0.30, head_center_z + head_size.z * 0.10),
                    Vec3::new(0.06, 0.10, 0.10), t_head);
            }
        }
        MobType::Cow => {
            let muzzle = Vec3::new(head_size.x * 0.7, head_size.y * 0.42, 0.05);
            let muzzle_y = head_center_y - head_size.y * 0.15;
            b.add_cube(Vec3::new(0.0, muzzle_y, head_front_z - muzzle.z * 0.5), muzzle, t_detail);
            for sx in [-1.0_f32, 1.0] {
                b.add_cube(
                    Vec3::new(sx * muzzle.x * 0.22, muzzle_y, head_front_z - muzzle.z - 0.003),
                    Vec3::new(0.05, 0.05, 0.008), base + T_EYE);
            }
            b.add_cube(
                Vec3::new(0.0, head_center_y + head_size.y * 0.18, head_front_z - 0.005),
                Vec3::new(head_size.x * 0.22, head_size.y * 0.42, 0.008), t_detail);
            for sx in [-1.0_f32, 1.0] {
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.30, head_center_y + head_size.y * 0.22, head_front_z - 0.008),
                    Vec3::new(0.13, 0.13, 0.008), t_white);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.30, head_center_y + head_size.y * 0.22, head_front_z - 0.015),
                    Vec3::new(0.11, 0.11, 0.008), base + T_EYE);
                b.add_cube(
                    Vec3::new(sx * head_size.x * 0.34, head_center_y + head_size.y * 0.5 + 0.045, head_center_z + 0.02),
                    Vec3::new(0.08, 0.11, 0.08), t_detail);
                b.add_cube(
                    Vec3::new(sx * (head_size.x * 0.5 + 0.04), head_center_y + head_size.y * 0.02, head_center_z + 0.05),
                    Vec3::new(0.10, 0.14, 0.06), t_head);
            }

            let eps = 0.008;
            let spot_th = 0.016;
            let body_hx = body_size.x * 0.5;
            let body_hy = body_size.y * 0.5;
            let body_hz = body_size.z * 0.5;
            for sx in [-1.0_f32, 1.0] {
                b.add_cube(
                    Vec3::new(sx * (body_hx + eps), body_center_y + body_hy * 0.25, body_hz * 0.15),
                    Vec3::new(spot_th, body_size.y * 0.55, body_size.z * 0.38), t_detail);
                b.add_cube(
                    Vec3::new(sx * (body_hx + eps), body_center_y - body_hy * 0.30, -body_hz * 0.30),
                    Vec3::new(spot_th, body_size.y * 0.28, body_size.z * 0.22), t_detail);
            }
            b.add_cube(
                Vec3::new(-body_hx * 0.25, body_center_y + body_hy + eps, -body_hz * 0.20),
                Vec3::new(body_size.x * 0.42, spot_th, body_size.z * 0.34), t_detail);
            b.add_cube(
                Vec3::new(body_hx * 0.22, body_center_y + body_hy * 0.05, body_hz + eps),
                Vec3::new(body_size.x * 0.42, body_size.y * 0.45, spot_th), t_detail);
            b.add_cube(
                Vec3::new(-body_hx * 0.30, body_center_y - body_hy * 0.20, -body_hz - eps),
                Vec3::new(body_size.x * 0.30, body_size.y * 0.30, spot_th), t_detail);
        }
        MobType::Chicken => {
            let body_hx = body_size.x * 0.5;
            let body_hz = body_size.z * 0.5;
            let head_hx = head_size.x * 0.5;

            b.add_cube(
                Vec3::new(0.0, head_center_y - head_size.y * 0.10, head_front_z - 0.035),
                Vec3::new(0.09, 0.06, 0.07), t_detail);
            b.add_cube(
                Vec3::new(0.0, head_center_y + head_size.y * 0.5 + 0.04, head_center_z),
                Vec3::new(0.08, 0.08, 0.14), t_extra);
            b.add_cube(
                Vec3::new(0.0, head_center_y - head_size.y * 0.10 - 0.06, head_front_z - 0.02),
                Vec3::new(0.05, 0.06, 0.04), t_extra);
            for sx in [-1.0_f32, 1.0] {
                b.add_cube(
                    Vec3::new(sx * head_hx, head_center_y + head_size.y * 0.10, head_front_z - 0.006),
                    Vec3::new(0.05, 0.07, 0.02), base + T_EYE);
                b.add_cube(
                    Vec3::new(sx * (body_hx + 0.008), body_center_y, 0.0),
                    Vec3::new(0.03, body_size.y * 0.85, body_size.z * 0.85), t_body);
            }
            b.add_cube(
                Vec3::new(0.0, body_center_y + body_size.y * 0.15, body_hz + 0.03),
                Vec3::new(0.10, 0.14, 0.06), t_body);
        }
    }

    b.build()
}

fn build_leg_mesh(mt: MobType) -> Mesh {
    let mut b = MeshBuilder::new();
    let leg_size = mt.leg_size();
    let t_leg = mob_tile_base(mt) + T_LEG;
    b.add_cube(Vec3::new(0.0, -leg_size.y * 0.5, 0.0), leg_size, t_leg);
    b.build()
}

// ============================================================
// РЕСУРС АССЕТОВ
// ============================================================

/// Единый материал и один атлас на всех мобов.
#[derive(Resource)]
struct MobAssets {
    pig_body: Handle<Mesh>,
    sheep_body: Handle<Mesh>,
    cow_body: Handle<Mesh>,
    chicken_body: Handle<Mesh>,
    pig_leg: Handle<Mesh>,
    sheep_leg: Handle<Mesh>,
    cow_leg: Handle<Mesh>,
    chicken_leg: Handle<Mesh>,
    /// Один материал для всех мобов — общий атлас.
    material: Handle<StandardMaterial>,
}

fn setup_mob_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Один атлас и один материал.
    let atlas = images.add(make_mob_atlas());
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(atlas),
        unlit: false,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    commands.insert_resource(MobAssets {
        pig_body:     meshes.add(build_body_mesh(MobType::Pig)),
        sheep_body:   meshes.add(build_body_mesh(MobType::Sheep)),
        cow_body:     meshes.add(build_body_mesh(MobType::Cow)),
        chicken_body: meshes.add(build_body_mesh(MobType::Chicken)),
        pig_leg:      meshes.add(build_leg_mesh(MobType::Pig)),
        sheep_leg:    meshes.add(build_leg_mesh(MobType::Sheep)),
        cow_leg:      meshes.add(build_leg_mesh(MobType::Cow)),
        chicken_leg:  meshes.add(build_leg_mesh(MobType::Chicken)),
        material,
    });
    info!("MobSpawn: assets built (1 atlas 128×64, 1 material, 8 meshes)");
}

// ============================================================
// СИСТЕМА СПАВНА
// ============================================================

fn spawn_cycle(
    mut commands: Commands,
    assets: Option<Res<MobAssets>>,
    world: Res<WorldData>,
    player_q: Query<&Transform, With<Player>>,
    mob_q: Query<&Transform, With<Mob>>,
    mut cycle_timer: Local<f32>,
    time: Res<Time>,
    mut rng_local: Local<Option<SmallRng>>,
) {
    let Some(assets) = assets else { return; };
    let Ok(pt) = player_q.single() else { return; };
    let player_pos = pt.translation;

    *cycle_timer += time.delta_secs();
    if *cycle_timer < SPAWN_CYCLE_INTERVAL { return; }
    *cycle_timer = 0.0;

    let nearby = mob_q
        .iter()
        .filter(|tf| tf.translation.distance(player_pos) <= MAX_SPAWN_DISTANCE)
        .count();

    if nearby >= PASSIVE_MOB_CAP { return; }

    let rng = rng_local.get_or_insert_with(|| SmallRng::seed_from_u64(0x5EED_5EED_5EED_5EED));
    let types = [MobType::Pig, MobType::Sheep, MobType::Cow, MobType::Chicken];

    for _ in 0..SPAWN_ATTEMPTS_PER_CYCLE {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let dist = rng.random_range(MIN_SPAWN_DISTANCE..MAX_SPAWN_DISTANCE);
        let wx = (player_pos.x + angle.cos() * dist).floor() as i32;
        let wz = (player_pos.z + angle.sin() * dist).floor() as i32;

        let cx = wx.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        if !world.has_chunk(cx, cz) { continue; }

        let mut top_y: Option<i32> = None;
        for y in (1..SY - 1).rev() {
            if world.get(wx, y, wz).is_solid() {
                top_y = Some(y);
                break;
            }
        }
        let Some(ground_y) = top_y else { continue };
        let spawn_y = ground_y + 1;

        let mut sky_clear = true;
        for dy in 0..3 {
            if world.get(wx, spawn_y + dy, wz).is_solid() {
                sky_clear = false;
                break;
            }
        }
        if !sky_clear { continue; }

        let mob_type = types[rng.random_range(0..types.len())];
        let pack_size = rng.random_range(PACK_SIZE_MIN..=PACK_SIZE_MAX);

        let (body_mesh, leg_mesh) = match mob_type {
            MobType::Pig     => (assets.pig_body.clone(),     assets.pig_leg.clone()),
            MobType::Sheep   => (assets.sheep_body.clone(),   assets.sheep_leg.clone()),
            MobType::Cow     => (assets.cow_body.clone(),     assets.cow_leg.clone()),
            MobType::Chicken => (assets.chicken_body.clone(), assets.chicken_leg.clone()),
        };

        let mut spawned_in_pack = 0usize;
        for _ in 0..pack_size * 3 {
            if spawned_in_pack >= pack_size { break; }

            let ox = rng.random_range(-PACK_RADIUS..PACK_RADIUS);
            let oz = rng.random_range(-PACK_RADIUS..PACK_RADIUS);
            let mx = wx + ox.floor() as i32;
            let mz = wz + oz.floor() as i32;

            let mcx = mx.div_euclid(CHUNK_SIZE);
            let mcz = mz.div_euclid(CHUNK_SIZE);
            if !world.has_chunk(mcx, mcz) { continue; }

            let mut my: Option<i32> = None;
            for y in (1..SY - 1).rev() {
                if world.get(mx, y, mz).is_solid() {
                    my = Some(y);
                    break;
                }
            }
            let Some(gy) = my else { continue };
            let sy = gy + 1;

            let mut ok = true;
            for dy in 0..3 {
                if world.get(mx, sy + dy, mz).is_solid() {
                    ok = false;
                    break;
                }
            }
            if !ok { continue; }

            let yaw = rng.random_range(0.0..std::f32::consts::TAU);

            spawn_mob(
                &mut commands,
                body_mesh.clone(),
                leg_mesh.clone(),
                assets.material.clone(),
                mob_type,
                Vec3::new(mx as f32 + 0.5, sy as f32, mz as f32 + 0.5),
                yaw,
            );

            spawned_in_pack += 1;
        }

        if spawned_in_pack > 0 {
            break;
        }
    }
}

fn spawn_mob(
    commands: &mut Commands,
    body_mesh: Handle<Mesh>,
    leg_mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    mob_type: MobType,
    position: Vec3,
    yaw: f32,
) {
    let body_size = mob_type.body_size();
    let leg_size  = mob_type.leg_size();

    let leg_offset_x = (body_size.x - leg_size.x) * 0.5;
    let leg_offset_z = (body_size.z - leg_size.z) * 0.5;
    let leg_top_y = leg_size.y;

    let leg_positions = [
        Vec3::new( leg_offset_x, leg_top_y, -leg_offset_z),
        Vec3::new(-leg_offset_x, leg_top_y, -leg_offset_z),
        Vec3::new( leg_offset_x, leg_top_y,  leg_offset_z),
        Vec3::new(-leg_offset_x, leg_top_y,  leg_offset_z),
    ];

    let mut leg_pivots: Vec<Entity> = Vec::with_capacity(4);

    for (i, pos) in leg_positions.iter().enumerate() {
        let leg_mesh_entity = commands.spawn((
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Name::new(format!("LegMesh{}", i)),
        )).id();

        let pivot_entity = commands.spawn((
            Transform::from_translation(*pos),
            Visibility::default(),
            Name::new(format!("LegPivot{}", i)),
        )).id();

        commands.entity(pivot_entity).add_children(&[leg_mesh_entity]);
        leg_pivots.push(pivot_entity);
    }

    let body_entity = commands.spawn((
        Mesh3d(body_mesh.clone()),
        MeshMaterial3d(material.clone()),
        Name::new("BodyHead"),
    )).id();

    let mut mob = Mob::new(mob_type, yaw);
    mob.legs = leg_pivots.clone();

    let root_entity = commands.spawn((
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        mob,
        Name::new(format!("{:?}", mob_type)),
    )).id();

    let mut all_children = vec![body_entity];
    all_children.extend(leg_pivots.iter().copied());
    commands.entity(root_entity).add_children(&all_children);
}