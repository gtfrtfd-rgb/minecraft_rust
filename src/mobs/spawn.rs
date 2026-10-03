use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use rand::Rng;

use crate::core::state::{SX, SY, SZ};
use crate::world::chunk::WorldData;
use crate::player::controller::Player;
use super::ai::{Mob, MobType};

pub struct MobSpawnPlugin;

impl Plugin for MobSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_initial_mobs);
        info!("MobSpawnPlugin loaded.");
    }
}

const INITIAL_MOB_COUNT: usize = 40;
const MIN_DIST_FROM_PLAYER: f32 = 16.0;
const SKY_CLEARANCE: i32 = 5;

const CUBE_FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
    ([ 1.0, 0.0, 0.0], [[ 1.0,-1.0, 1.0],[ 1.0,-1.0,-1.0],[ 1.0, 1.0,-1.0],[ 1.0, 1.0, 1.0]]),
    ([-1.0, 0.0, 0.0], [[-1.0,-1.0,-1.0],[-1.0,-1.0, 1.0],[-1.0, 1.0, 1.0],[-1.0, 1.0,-1.0]]),
    ([0.0,  1.0, 0.0], [[-1.0, 1.0, 1.0],[ 1.0, 1.0, 1.0],[ 1.0, 1.0,-1.0],[-1.0, 1.0,-1.0]]),
    ([0.0, -1.0, 0.0], [[-1.0,-1.0,-1.0],[ 1.0,-1.0,-1.0],[ 1.0,-1.0, 1.0],[-1.0,-1.0, 1.0]]),
    ([0.0, 0.0,  1.0], [[-1.0,-1.0, 1.0],[ 1.0,-1.0, 1.0],[ 1.0, 1.0, 1.0],[-1.0, 1.0, 1.0]]),
    ([0.0, 0.0, -1.0], [[ 1.0,-1.0,-1.0],[-1.0,-1.0,-1.0],[-1.0, 1.0,-1.0],[ 1.0, 1.0,-1.0]]),
];

fn push_cube(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
    color: [f32; 4],
) {
    let half = size * 0.5;

    for (normal, corners) in CUBE_FACES.iter() {
        let base = positions.len() as u32;
        for c in corners {
            positions.push([
                center.x + c[0] * half.x,
                center.y + c[1] * half.y,
                center.z + c[2] * half.z,
            ]);
            normals.push(*normal);
            colors.push(color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn build_mob_mesh(mob_type: MobType) -> Mesh {
    let h = mob_type.height();
    let body_size = mob_type.body_size();
    let head_size = mob_type.head_size();
    let leg_size = mob_type.leg_size();

    let body_srgba = mob_type.body_color().to_srgba();
    let leg_srgba = mob_type.leg_color().to_srgba();

    let body_color = [body_srgba.red, body_srgba.green, body_srgba.blue, 1.0];
    let leg_color = [leg_srgba.red, leg_srgba.green, leg_srgba.blue, 1.0];

    let head_offset_z = -(body_size.z * 0.5 + head_size.z * 0.5);
    let head_y = h * 0.85;

    let leg_offset_x = body_size.x * 0.35;
    let leg_offset_z = body_size.z * 0.35;
    let leg_y = leg_size.y * 0.5;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    push_cube(
        &mut positions, &mut normals, &mut colors, &mut indices,
        Vec3::new(0.0, h * 0.5, 0.0),
        body_size,
        body_color,
    );

    push_cube(
        &mut positions, &mut normals, &mut colors, &mut indices,
        Vec3::new(0.0, head_y, head_offset_z),
        head_size,
        body_color,
    );

    let leg_positions = [
        Vec3::new( leg_offset_x, leg_y, -leg_offset_z),
        Vec3::new(-leg_offset_x, leg_y, -leg_offset_z),
        Vec3::new( leg_offset_x, leg_y,  leg_offset_z),
        Vec3::new(-leg_offset_x, leg_y,  leg_offset_z),
    ];
    for pos in leg_positions {
        push_cube(
            &mut positions, &mut normals, &mut colors, &mut indices,
            pos,
            leg_size,
            leg_color,
        );
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

fn spawn_initial_mobs(
    mut commands: Commands,
    world: Res<WorldData>,
    player_q: Query<&Transform, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_pos = player_q
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::new(SX as f32 * 0.5, 30.0, SZ as f32 * 0.5));

    info!(
        "Spawning {} mobs (min {} blocks from player at {:?})...",
        INITIAL_MOB_COUNT, MIN_DIST_FROM_PLAYER, player_pos
    );
    let t0 = std::time::Instant::now();

    let pig_mesh     = meshes.add(build_mob_mesh(MobType::Pig));
    let sheep_mesh   = meshes.add(build_mob_mesh(MobType::Sheep));
    let cow_mesh     = meshes.add(build_mob_mesh(MobType::Cow));
    let chicken_mesh = meshes.add(build_mob_mesh(MobType::Chicken));

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: false,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    let mut rng = rand::thread_rng();
    let types = [MobType::Pig, MobType::Sheep, MobType::Cow, MobType::Chicken];

    let mut spawned = 0;
    let mut attempts = 0;
    let max_attempts = INITIAL_MOB_COUNT * 100;

    while spawned < INITIAL_MOB_COUNT && attempts < max_attempts {
        attempts += 1;

        let x = rng.gen_range(4..SX - 4);
        let z = rng.gen_range(4..SZ - 4);

        let dx = (x as f32 + 0.5) - player_pos.x;
        let dz = (z as f32 + 0.5) - player_pos.z;
        if dx * dx + dz * dz < MIN_DIST_FROM_PLAYER * MIN_DIST_FROM_PLAYER {
            continue;
        }

        let mut top_y: Option<i32> = None;
        for y in (1..SY - 1).rev() {
            if world.get(x, y, z).is_solid() {
                top_y = Some(y);
                break;
            }
        }
        let Some(ground_y) = top_y else { continue };
        let spawn_y = ground_y + 1;

        let mut sky_clear = true;
        for dy in 0..SKY_CLEARANCE {
            let check_y = spawn_y + dy;
            if check_y >= SY { break; }
            if world.get(x, check_y, z).is_solid() {
                sky_clear = false;
                break;
            }
        }
        if !sky_clear { continue; }

        let mut free = true;
        for dy in 0..3 {
            if world.get(x, spawn_y + dy, z).is_solid() {
                free = false;
                break;
            }
        }
        if !free { continue; }

        let mob_type = types[rng.gen_range(0..types.len())];
        let mesh = match mob_type {
            MobType::Pig => pig_mesh.clone(),
            MobType::Sheep => sheep_mesh.clone(),
            MobType::Cow => cow_mesh.clone(),
            MobType::Chicken => chicken_mesh.clone(),
        };

        let yaw = rng.gen_range(0.0..std::f32::consts::TAU);

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(
                Vec3::new(x as f32 + 0.5, spawn_y as f32, z as f32 + 0.5)
            ),
            // ВАЖНО: без .with_rotation — поворот ставится каждый кадр из mob.yaw
            Mob {
                mob_type,
                hp: mob_type.max_hp(),
                max_hp: mob_type.max_hp(),
                velocity: Vec3::ZERO,
                on_ground: false,
                walking: false,
                wander_timer: 1.0,
                yaw,
                target_yaw: yaw,
                panic_timer: 0.0,
                hurt_timer: 0.0,
            },
            Name::new(format!("{:?}", mob_type)),
        ));

        spawned += 1;
    }

    info!(
        "Spawned {} mobs in {:?} (attempts: {} / {})",
        spawned,
        t0.elapsed(),
        attempts,
        max_attempts
    );
}