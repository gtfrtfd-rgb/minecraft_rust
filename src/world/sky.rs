use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
};
use bevy::math::Affine2;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::core::state::AppState;
use crate::player::controller::Player;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_sun, spawn_clouds))
            .add_systems(Update, (
                sun_follow_player,
                update_clouds.run_if(in_state(AppState::InGame)),
            ));
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
const SUN_EMISSIVE: f32 = 50.0;

const CLOUD_Y: f32 = 60.0;
const CLOUD_SIZE: f32 = 100_000.0;
const CLOUD_TEX: u32 = 256;
const CLOUD_UV_TILES: f32 = 400.0;
const CLOUD_SUBDIVISIONS: u32 = 16;
const CLOUD_EMISSIVE: f32 = 3.0;
const CLOUD_DRIFT_SPEED: f32 = 0.002;

/// Мировой размер одного UV-тайла текстуры облаков.
const CLOUD_TILE_WORLD: f32 = CLOUD_SIZE / CLOUD_UV_TILES;

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

fn sun_follow_player(
    player_q: Query<&Transform, (With<Player>, Without<Sun>)>,
    mut sun_q: Query<&mut Transform, (With<Sun>, Without<Player>)>,
) {
    let Ok(pt) = player_q.single() else { return; };
    let pos = pt.translation + SUN_DIR.normalize() * SUN_DISTANCE;
    for mut st in sun_q.iter_mut() {
        st.translation = pos;
    }
}

// ============================================================
// ОБЛАКА
// ============================================================

fn build_cloud_mesh(size: f32, subdivisions: u32) -> Mesh {
    let step = size / subdivisions as f32;
    let half = size * 0.5;
    let row = subdivisions + 1;
    let vertex_count = (row * row) as usize;
    let index_count = (subdivisions * subdivisions * 6) as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vertex_count);
    let mut indices:   Vec<u32>      = Vec::with_capacity(index_count);

    for j in 0..=subdivisions {
        for i in 0..=subdivisions {
            let x = -half + i as f32 * step;
            let z = -half + j as f32 * step;
            positions.push([x, 0.0, z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([
                i as f32 / subdivisions as f32,
                j as f32 / subdivisions as f32,
            ]);
        }
    }

    for j in 0..subdivisions {
        for i in 0..subdivisions {
            let a = j * row + i;
            let b = a + 1;
            let c = (j + 1) * row + i;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn spawn_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let texture = images.add(make_cloud_texture());

    commands.spawn((
        Mesh3d(meshes.add(build_cloud_mesh(CLOUD_SIZE, CLOUD_SUBDIVISIONS))),
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
        "Cloud layer spawned at y={} (size {}, tiles {}×{}, subdivisions {})",
        CLOUD_Y, CLOUD_SIZE, CLOUD_UV_TILES, CLOUD_UV_TILES, CLOUD_SUBDIVISIONS
    );
}

/// Одна система: двигает плоскость за игроком И компенсирует
/// движение через UV-сдвиг, чтобы облака оставались привязанными
/// к миру, а не к игроку. Плюс медленный дрейф.
fn update_clouds(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<CloudLayer>)>,
    mut cloud_q: Query<
        (&mut Transform, &MeshMaterial3d<StandardMaterial>),
        (With<CloudLayer>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut last_player_xz: Local<Vec2>,
    mut uv_offset: Local<Vec2>,
) {
    let Ok(pt) = player_q.single() else { return; };

    let player_xz = Vec2::new(pt.translation.x, pt.translation.z);
    let delta = player_xz - *last_player_xz;
    *last_player_xz = player_xz;

    // Дрейф по времени (ветер).
    let drift_dir = Vec2::new(1.0, 0.35).normalize();
    *uv_offset += drift_dir * time.delta_secs() * CLOUD_DRIFT_SPEED;

    // Компенсация движения игрока.
    //
    // Плоскость сдвигается на +delta (следует за игроком). Значит
    // UV-текстура на ней «уезжает» вместе с ней. Чтобы облака
    // остались на месте в мире, надо сдвинуть UV в ту же сторону
    // на delta / tile_size.
    //
    // Знак ВАЖЕН: если здесь минус, облака будут двигаться в
    // 2 раза быстрее в сторону игрока (delta от плоскости + delta
    // от UV-сдвига вместо компенсации).
    *uv_offset += delta / CLOUD_TILE_WORLD;

    // Wrap: держим offset в [0, 1), иначе f32 теряет точность
    // на больших числах — это даёт дёргание.
    let wrapped = Vec2::new(
        uv_offset.x.rem_euclid(1.0),
        uv_offset.y.rem_euclid(1.0),
    );

    for (mut ct, mat_handle) in cloud_q.iter_mut() {
        ct.translation.x = pt.translation.x;
        ct.translation.z = pt.translation.z;

        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.uv_transform = Affine2::from_scale_angle_translation(
                Vec2::splat(CLOUD_UV_TILES),
                0.0,
                wrapped,
            );
        }
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
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
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