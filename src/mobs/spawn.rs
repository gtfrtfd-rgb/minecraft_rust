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

// ============================================================
// КУБ (6 граней × 4 вершины)
// ============================================================
const CUBE_FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
    ([ 1.0, 0.0, 0.0], [[ 1.0,-1.0, 1.0],[ 1.0,-1.0,-1.0],[ 1.0, 1.0,-1.0],[ 1.0, 1.0, 1.0]]),
    ([-1.0, 0.0, 0.0], [[-1.0,-1.0,-1.0],[-1.0,-1.0, 1.0],[-1.0, 1.0, 1.0],[-1.0, 1.0,-1.0]]),
    ([0.0,  1.0, 0.0], [[-1.0, 1.0, 1.0],[ 1.0, 1.0, 1.0],[ 1.0, 1.0,-1.0],[-1.0, 1.0,-1.0]]),
    ([0.0, -1.0, 0.0], [[-1.0,-1.0,-1.0],[ 1.0,-1.0,-1.0],[ 1.0,-1.0, 1.0],[-1.0,-1.0, 1.0]]),
    ([0.0, 0.0,  1.0], [[-1.0,-1.0, 1.0],[ 1.0,-1.0, 1.0],[ 1.0, 1.0, 1.0],[-1.0, 1.0, 1.0]]),
    ([0.0, 0.0, -1.0], [[ 1.0,-1.0,-1.0],[-1.0,-1.0,-1.0],[-1.0, 1.0,-1.0],[ 1.0, 1.0,-1.0]]),
];

const FACE_UV: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals:   Vec<[f32; 3]>,
    uvs:       Vec<[f32; 2]>,
    colors:    Vec<[f32; 4]>,
    indices:   Vec<u32>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn add_cube(&mut self, center: Vec3, size: Vec3, color: [f32; 4]) {
        let half = size * 0.5;

        for (normal, corners) in CUBE_FACES.iter() {
            let base = self.positions.len() as u32;
            for (j, c) in corners.iter().enumerate() {
                self.positions.push([
                    center.x + c[0] * half.x,
                    center.y + c[1] * half.y,
                    center.z + c[2] * half.z,
                ]);
                self.normals.push(*normal);
                self.uvs.push(FACE_UV[j]);
                self.colors.push(color);
            }
            self.indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);
        }
    }

    fn build(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

fn rgba(c: Color) -> [f32; 4] {
    let s = c.to_srgba();
    [s.red, s.green, s.blue, 1.0]
}

// ============================================================
// ЦВЕТА МОБОВ
// ============================================================
struct MobColors {
    body: Color,
    head: Color,
    leg: Color,
    detail: Color,
    extra: Color,
    eye: Color,
}

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
            body:   Color::srgb(0.95, 0.95, 0.92),
            head:   Color::srgb(0.85, 0.80, 0.72),
            leg:    Color::srgb(0.30, 0.28, 0.26),
            detail: Color::srgb(0.20, 0.18, 0.16),
            extra:  Color::srgb(0.75, 0.70, 0.62),
            eye:    Color::srgb(0.05, 0.03, 0.05),
        },
        MobType::Cow => MobColors {
            body:   Color::srgb(0.42, 0.28, 0.14),
            head:   Color::srgb(0.42, 0.28, 0.14),
            leg:    Color::srgb(0.35, 0.22, 0.10),
            detail: Color::srgb(0.92, 0.88, 0.82),
            extra:  Color::srgb(0.20, 0.14, 0.08),
            eye:    Color::srgb(0.05, 0.03, 0.05),
        },
        MobType::Chicken => MobColors {
            body:   Color::srgb(0.98, 0.98, 0.98),
            head:   Color::srgb(0.98, 0.98, 0.98),
            leg:    Color::srgb(0.92, 0.62, 0.12),
            detail: Color::srgb(0.95, 0.65, 0.15),
            extra:  Color::srgb(0.85, 0.15, 0.15),
            eye:    Color::srgb(0.05, 0.03, 0.05),
        },
    }
}

// ============================================================
// МЕШ ТЕЛА + ГОЛОВЫ + ДЕТАЛЕЙ
// ============================================================
fn build_body_mesh(mt: MobType) -> Mesh {
    let c = colors_for(mt);
    let mut b = MeshBuilder::new();

    let h = mt.height();
    let body_size = mt.body_size();
    let head_size = mt.head_size();

    let body_y = h * 0.5;
    let head_offset_z = -(body_size.z * 0.5 + head_size.z * 0.5);
    let head_y = h * 0.85;

    // Тело
    b.add_cube(Vec3::new(0.0, body_y, 0.0), body_size, rgba(c.body));

    // Голова
    b.add_cube(Vec3::new(0.0, head_y, head_offset_z), head_size, rgba(c.head));

    // Детали
    match mt {
        MobType::Pig => {
            // Пятачок
            b.add_cube(
                Vec3::new(0.0, head_y - 0.05, head_offset_z - head_size.z * 0.5 - 0.03),
                Vec3::new(head_size.x * 0.55, head_size.y * 0.45, 0.06),
                rgba(c.detail),
            );
            // Глаза
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * head_size.x * 0.28,
                        head_y + head_size.y * 0.15,
                        head_offset_z - head_size.z * 0.5 - 0.005,
                    ),
                    Vec3::new(0.08, 0.08, 0.02),
                    rgba(c.eye),
                );
            }
            // Ушки
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * head_size.x * 0.35,
                        head_y + head_size.y * 0.5 + 0.05,
                        head_offset_z,
                    ),
                    Vec3::new(0.12, 0.15, 0.08),
                    rgba(c.body),
                );
            }
        }
        MobType::Sheep => {
            // Морда
            b.add_cube(
                Vec3::new(0.0, head_y - 0.05, head_offset_z - head_size.z * 0.5 - 0.02),
                Vec3::new(head_size.x * 0.7, head_size.y * 0.6, 0.05),
                rgba(c.detail),
            );
            // Глаза
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * head_size.x * 0.25,
                        head_y + head_size.y * 0.15,
                        head_offset_z - head_size.z * 0.5 - 0.005,
                    ),
                    Vec3::new(0.07, 0.07, 0.02),
                    rgba(c.eye),
                );
            }
            // Уши
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * (head_size.x * 0.5 + 0.04),
                        head_y + 0.05,
                        head_offset_z,
                    ),
                    Vec3::new(0.10, 0.05, 0.15),
                    rgba(c.head),
                );
            }
        }
        MobType::Cow => {
            // Белая морда
            b.add_cube(
                Vec3::new(0.0, head_y - 0.08, head_offset_z - head_size.z * 0.5 - 0.03),
                Vec3::new(head_size.x * 0.75, head_size.y * 0.55, 0.07),
                rgba(c.detail),
            );
            // Нос
            b.add_cube(
                Vec3::new(0.0, head_y - 0.12, head_offset_z - head_size.z * 0.5 - 0.06),
                Vec3::new(0.15, 0.08, 0.03),
                rgba(Color::srgb(0.15, 0.08, 0.05)),
            );
            // Глаза
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * head_size.x * 0.30,
                        head_y + head_size.y * 0.10,
                        head_offset_z - head_size.z * 0.5 - 0.005,
                    ),
                    Vec3::new(0.08, 0.08, 0.02),
                    rgba(c.eye),
                );
            }
            // Рога
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * head_size.x * 0.32,
                        head_y + head_size.y * 0.5 + 0.06,
                        head_offset_z,
                    ),
                    Vec3::new(0.10, 0.15, 0.10),
                    rgba(c.detail),
                );
            }
            // Уши
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * (head_size.x * 0.5 + 0.05),
                        head_y + head_size.y * 0.15,
                        head_offset_z,
                    ),
                    Vec3::new(0.10, 0.15, 0.08),
                    rgba(c.head),
                );
            }
        }
        MobType::Chicken => {
            // Клюв
            b.add_cube(
                Vec3::new(0.0, head_y - 0.02, head_offset_z - head_size.z * 0.5 - 0.04),
                Vec3::new(0.10, 0.06, 0.08),
                rgba(c.detail),
            );
            // Гребень
            b.add_cube(
                Vec3::new(0.0, head_y + head_size.y * 0.5 + 0.05, head_offset_z),
                Vec3::new(0.10, 0.10, 0.18),
                rgba(c.extra),
            );
            // Бородка
            b.add_cube(
                Vec3::new(0.0, head_y - 0.12, head_offset_z - head_size.z * 0.5 - 0.02),
                Vec3::new(0.06, 0.08, 0.05),
                rgba(c.extra),
            );
            // Глаза
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * head_size.x * 0.35,
                        head_y + head_size.y * 0.20,
                        head_offset_z - head_size.z * 0.5 - 0.003,
                    ),
                    Vec3::new(0.05, 0.05, 0.02),
                    rgba(c.eye),
                );
            }
            // Крылья
            for sx in [-1.0, 1.0] {
                b.add_cube(
                    Vec3::new(
                        sx * (body_size.x * 0.5 + 0.025),
                        body_y,
                        0.0,
                    ),
                    Vec3::new(0.05, body_size.y * 0.6, body_size.z * 0.7),
                    rgba(c.body),
                );
            }
        }
    }

    b.build()
}

// ============================================================
// МЕШ НОГИ
// ============================================================
fn build_leg_mesh(mt: MobType) -> Mesh {
    let c = colors_for(mt);
    let mut b = MeshBuilder::new();
    let leg_size = mt.leg_size();
    let center = Vec3::new(0.0, -leg_size.y * 0.5, 0.0);
    b.add_cube(center, leg_size, rgba(c.leg));
    b.build()
}

// ============================================================
// СПАВН
// ============================================================
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

    // Меши для каждого типа
    let pig_body     = meshes.add(build_body_mesh(MobType::Pig));
    let sheep_body   = meshes.add(build_body_mesh(MobType::Sheep));
    let cow_body     = meshes.add(build_body_mesh(MobType::Cow));
    let chicken_body = meshes.add(build_body_mesh(MobType::Chicken));

    let pig_leg     = meshes.add(build_leg_mesh(MobType::Pig));
    let sheep_leg   = meshes.add(build_leg_mesh(MobType::Sheep));
    let cow_leg     = meshes.add(build_leg_mesh(MobType::Cow));
    let chicken_leg = meshes.add(build_leg_mesh(MobType::Chicken));

    // Один материал на всех
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

        // Верхний твёрдый блок
        let mut top_y: Option<i32> = None;
        for y in (1..SY - 1).rev() {
            if world.get(x, y, z).is_solid() {
                top_y = Some(y);
                break;
            }
        }
        let Some(ground_y) = top_y else { continue };
        let spawn_y = ground_y + 1;

        // Открытое небо
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

        // Свободное место
        let mut free = true;
        for dy in 0..3 {
            if world.get(x, spawn_y + dy, z).is_solid() {
                free = false;
                break;
            }
        }
        if !free { continue; }

        let mob_type = types[rng.gen_range(0..types.len())];
        let (body_mesh, leg_mesh) = match mob_type {
            MobType::Pig => (pig_body.clone(), pig_leg.clone()),
            MobType::Sheep => (sheep_body.clone(), sheep_leg.clone()),
            MobType::Cow => (cow_body.clone(), cow_leg.clone()),
            MobType::Chicken => (chicken_body.clone(), chicken_leg.clone()),
        };

        let yaw = rng.gen_range(0.0..std::f32::consts::TAU);

        spawn_mob(
            &mut commands,
            body_mesh,
            leg_mesh,
            material.clone(),
            mob_type,
            Vec3::new(x as f32 + 0.5, spawn_y as f32, z as f32 + 0.5),
            yaw,
        );

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

// ============================================================
// СОЗДАНИЕ ОДНОГО МОБА
// ============================================================
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
    let leg_size = mob_type.leg_size();

    let leg_offset_x = body_size.x * 0.35;
    let leg_offset_z = body_size.z * 0.35;
    let leg_top_y = leg_size.y;

    // 4 позиции для ног: FL, FR, BL, BR
    let leg_positions = [
        Vec3::new( leg_offset_x, leg_top_y, -leg_offset_z), // Front-Left
        Vec3::new(-leg_offset_x, leg_top_y, -leg_offset_z), // Front-Right
        Vec3::new( leg_offset_x, leg_top_y,  leg_offset_z), // Back-Left
        Vec3::new(-leg_offset_x, leg_top_y,  leg_offset_z), // Back-Right
    ];

    let mut leg_pivots: Vec<Entity> = Vec::with_capacity(4);

    for (i, pos) in leg_positions.iter().enumerate() {
        // Меш ноги — потомок pivot, сдвинут вниз
        let leg_mesh_entity = commands
            .spawn((
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Name::new(format!("LegMesh{}", i)),
            ))
            .id();

        // Pivot — верхняя точка ноги
        let pivot_entity = commands
            .spawn((
                Transform::from_translation(*pos),
                Visibility::default(),
                Name::new(format!("LegPivot{}", i)),
            ))
            .id();

        commands.entity(pivot_entity).add_children(&[leg_mesh_entity]);
        leg_pivots.push(pivot_entity);
    }

    // Тело + голова одним мешем
    let body_entity = commands
        .spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Name::new("BodyHead"),
        ))
        .id();

    // === КОРЕНЬ МОБА — Mob::new() ===
    let mut mob = Mob::new(mob_type, yaw);
    mob.legs = leg_pivots.clone();

    let root_entity = commands
        .spawn((
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            mob,
            Name::new(format!("{:?}", mob_type)),
        ))
        .id();

    // Привязываем всё к корню
    let mut all_children = vec![body_entity];
    all_children.extend(leg_pivots.iter().copied());
    commands.entity(root_entity).add_children(&all_children);
}