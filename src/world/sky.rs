use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
};
use bevy::math::Affine2;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::player::controller::Player;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_sun, spawn_clouds))
            .add_systems(Update, clouds_follow_player);
        info!("SkyPlugin loaded.");
    }
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct CloudLayer;

const SUN_SIZE: f32 = 250.0;
const SUN_DIR: Vec3 = Vec3::new(0.5, 1.0, 0.35);
const SUN_DISTANCE: f32 = 1500.0;

/// HDR-яркость солнца в линейном пространстве.
/// Работает ТОЛЬКО если unlit выключен — иначе Bevy игнорирует
/// emissive и просто рисует base_color, что после tone mapping
/// даёт серое пятно.
const SUN_EMISSIVE: f32 = 50.0;

const CLOUD_Y: f32 = 80.0;
const CLOUD_SIZE: f32 = 20000.0;
const CLOUD_TEX: u32 = 128;
const CLOUD_UV_TILES: f32 = 80.0;

/// HDR-яркость облака. 3.0 в linear space после tone mapping
/// даёт чистый белый, не в тени directional light.
const CLOUD_EMISSIVE: f32 = 3.0;

// ============================================================
// СОЛНЦЕ
// ============================================================

fn spawn_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let position = SUN_DIR.normalize() * SUN_DISTANCE;
    let facing = (-position).normalize();
    let rotation = Quat::from_rotation_arc(Vec3::Z, facing);

    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(SUN_SIZE, SUN_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            // emissive без unlit — Bevy пропустит его через HDR
            // pipeline, tone mapping выжжет в чистый белый.
            emissive: LinearRgba::rgb(SUN_EMISSIVE, SUN_EMISSIVE, SUN_EMISSIVE * 0.95),
            fog_enabled: false,
            cull_mode: None,
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(position).with_rotation(rotation),
        Sun,
        Name::new("Sun"),
    ));

    info!("Sun spawned at {:?} (size {}, distance {})", position, SUN_SIZE, SUN_DISTANCE);
}

// ============================================================
// ОБЛАКА
// ============================================================

fn spawn_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let texture = images.add(make_cloud_texture());

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(CLOUD_SIZE * 0.5)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture),
            emissive: LinearRgba::rgb(CLOUD_EMISSIVE, CLOUD_EMISSIVE, CLOUD_EMISSIVE),
            fog_enabled: false,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            uv_transform: Affine2::from_scale(Vec2::splat(CLOUD_UV_TILES)),
            ..default()
        })),
        Transform::from_xyz(0.0, CLOUD_Y, 0.0),
        CloudLayer,
        Name::new("CloudLayer"),
    ));

    info!(
        "Cloud layer spawned at y={} (size {}, tiles {}×{})",
        CLOUD_Y, CLOUD_SIZE, CLOUD_UV_TILES, CLOUD_UV_TILES
    );
}

fn clouds_follow_player(
    player_q: Query<&Transform, With<Player>>,
    mut cloud_q: Query<&mut Transform, (With<CloudLayer>, Without<Player>)>,
) {
    let Ok(pt) = player_q.single() else { return; };
    for mut ct in cloud_q.iter_mut() {
        ct.translation.x = pt.translation.x;
        ct.translation.z = pt.translation.z;
    }
}

// ============================================================
// ПРОЦЕДУРНАЯ ТЕКСТУРА ОБЛАКОВ
// ============================================================

fn make_cloud_texture() -> Image {
    let size = CLOUD_TEX;
    let mut data = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let n = cloud_noise(
                x as f32 / size as f32,
                y as f32 / size as f32,
            );

            // Порог 0.70 — облака занимают ~20–25% площади,
            // между ними чистое небо.
            let alpha: f32 = if n > 0.72 {
                1.0
            } else if n > 0.68 {
                (n - 0.68) / 0.04
            } else {
                0.0
            };

            let bright = 255u8;
            let a = (alpha * 255.0) as u8;

            let o = ((y * size + x) * 4) as usize;
            data[o] = bright;
            data[o + 1] = bright;
            data[o + 2] = bright;
            data[o + 3] = a;
        }
    }

    let mut img = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    });
    img
}

fn cloud_noise(x: f32, y: f32) -> f32 {
    let mut total = 0.0;
    let mut amp = 1.0;
    let mut freq = 6.0;
    let mut sum = 0.0;
    for _ in 0..7 {
        total += value_noise_2d(x * freq, y * freq) * amp;
        sum += amp;
        amp *= 0.55;
        freq *= 2.0;
    }
    total / sum
}

fn value_noise_2d(x: f32, y: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let h = |ix: i32, iy: i32| -> f32 {
        let mut n = (ix as u32).wrapping_mul(374761393)
            ^ (iy as u32).wrapping_mul(668265263);
        n = n.wrapping_add(1013904223);
        n ^= n >> 13;
        n = n.wrapping_mul(1274126177);
        n ^= n >> 16;
        (n & 0xFF) as f32 / 255.0
    };

    let v00 = h(x0, y0);
    let v10 = h(x0 + 1, y0);
    let v01 = h(x0, y0 + 1);
    let v11 = h(x0 + 1, y0 + 1);

    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);

    let a = v00 * (1.0 - sx) + v10 * sx;
    let b = v01 * (1.0 - sx) + v11 * sx;
    a * (1.0 - sy) + b * sy
}