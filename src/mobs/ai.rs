use bevy::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rand::RngExt;
use std::collections::HashMap;

use crate::core::state::AppState;
use crate::world::chunk::WorldData;
use crate::player::controller::Player;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobType {
    Pig,
    Sheep,
    Cow,
    Chicken,
}

impl MobType {
    pub fn height(self) -> f32 {
        match self {
            MobType::Pig => 0.9,
            MobType::Sheep => 1.15,
            MobType::Cow => 1.45,
            MobType::Chicken => 0.66,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            MobType::Pig => 0.4,
            MobType::Sheep => 0.4,
            MobType::Cow => 0.45,
            MobType::Chicken => 0.22,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            MobType::Pig => 2.2,
            MobType::Sheep => 2.0,
            MobType::Cow => 2.0,
            MobType::Chicken => 3.0,
        }
    }

    pub fn max_hp(self) -> i32 {
        match self {
            MobType::Pig => 10,
            MobType::Sheep => 8,
            MobType::Cow => 10,
            MobType::Chicken => 4,
        }
    }

    pub fn notice_range(self) -> f32 {
        match self {
            MobType::Pig => 8.0,
            MobType::Sheep => 9.0,
            MobType::Cow => 8.0,
            MobType::Chicken => 6.0,
        }
    }

    pub fn body_size(self) -> Vec3 {
        match self {
            MobType::Pig => Vec3::new(0.6, 0.5, 1.0),
            MobType::Sheep => Vec3::new(0.8, 0.55, 1.1),
            MobType::Cow => Vec3::new(0.75, 0.625, 1.125),
            MobType::Chicken => Vec3::new(0.34, 0.34, 0.42),
        }
    }

    pub fn head_size(self) -> Vec3 {
        match self {
            MobType::Pig => Vec3::new(0.5, 0.5, 0.5),
            MobType::Sheep => Vec3::new(0.5, 0.5, 0.5),
            MobType::Cow => Vec3::new(0.5, 0.5, 0.5),
            MobType::Chicken => Vec3::new(0.30, 0.32, 0.28),
        }
    }

    pub fn leg_size(self) -> Vec3 {
        match self {
            MobType::Pig => Vec3::new(0.22, 0.375, 0.22),
            MobType::Sheep => Vec3::new(0.22, 0.5, 0.22),
            MobType::Cow => Vec3::new(0.22, 0.75, 0.22),
            MobType::Chicken => Vec3::new(0.08, 0.16, 0.08),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobState {
    Idle,
    Wander,
    Alert,
    Flee,
}

#[derive(Component)]
pub struct Mob {
    pub mob_type: MobType,
    pub hp: i32,
    pub max_hp: i32,
    pub velocity: Vec3,
    pub on_ground: bool,

    pub state: MobState,
    pub state_timer: f32,
    pub yaw: f32,
    pub target_yaw: f32,
    pub panic_timer: f32,
    pub hurt_timer: f32,

    pub walk_phase: f32,
    pub walk_amplitude: f32,

    pub stuck_timer: f32,
    pub jump_cooldown: f32,

    pub alert_looking: bool,
    pub alert_walking: bool,
    pub alert_look_timer: f32,

    pub legs: Vec<Entity>,

    pub despawn_timer: f32,
}

impl Mob {
    pub fn new(mob_type: MobType, yaw: f32) -> Self {
        Self {
            mob_type,
            hp: mob_type.max_hp(),
            max_hp: mob_type.max_hp(),
            velocity: Vec3::ZERO,
            on_ground: false,
            state: MobState::Idle,
            state_timer: 1.0,
            yaw,
            target_yaw: yaw,
            panic_timer: 0.0,
            hurt_timer: 0.0,
            walk_phase: 0.0,
            walk_amplitude: 0.0,
            stuck_timer: 0.0,
            jump_cooldown: 0.0,
            alert_looking: false,
            alert_walking: false,
            alert_look_timer: 0.0,
            legs: Vec::new(),
            despawn_timer: 0.0,
        }
    }
}

// ============================================================
// SPATIAL HASH ДЛЯ МОБОВ
// ============================================================
//
// Разбивает мир на квадратную сетку (XZ). Для каждой ячейки
// хранит список мобов, попавших в неё. При расталкивании моб
// проверяет только 9 ячеек вокруг себя (свой + 8 соседних),
// вместо того чтобы бегать по всему списку.
//
// При 30 мобах O(N²) — это 900 итераций в кадр, что не проблема.
// Но при 200+ мобах это уже 40 000 итераций — spatial hash
// снижает до ~O(N) за счёт локальности.
//
// cell_size = 4 блока достаточно: max радиус расталкивания
// = r1 + r2 + 0.15 ≤ 1.05, с большим запасом.

/// Один моб в spatial-хеше: entity, позиция, радиус.
#[derive(Clone, Copy)]
pub struct MobEntry {
    pub entity: Entity,
    pub pos: Vec3,
    pub radius: f32,
}

#[derive(Resource)]
pub struct MobSpatialGrid {
    pub cells: HashMap<IVec2, Vec<MobEntry>>,
    pub cell_size: f32,
}

impl Default for MobSpatialGrid {
    fn default() -> Self {
        Self {
            cells: HashMap::new(),
            cell_size: 4.0,
        }
    }
}

impl MobSpatialGrid {
    /// Сбрасывает сетку перед заполнением в новом кадре.
    /// HashMap::clear() сохраняет capacity, но удаляет все ключи —
    /// это ключевое отличие от простого перебора values_mut().clear(),
    /// который оставлял пустые ячейки и приводил к утечке памяти.
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    #[inline]
    pub fn key_for(&self, pos: Vec3) -> IVec2 {
        IVec2::new(
            (pos.x / self.cell_size).floor() as i32,
            (pos.z / self.cell_size).floor() as i32,
        )
    }

    pub fn insert(&mut self, entry: MobEntry) {
        let key = self.key_for(entry.pos);
        self.cells.entry(key).or_default().push(entry);
    }

    /// Возвращает всех мобов в 9 ячейках вокруг `pos` (свой + 8 соседей).
    /// Записывает их в переданный `out` (переиспользуется между вызовами).
    pub fn collect_neighbors(&self, pos: Vec3, out: &mut Vec<MobEntry>) {
        out.clear();
        let key = self.key_for(pos);
        for dz in -1..=1 {
            for dx in -1..=1 {
                if let Some(list) = self.cells.get(&IVec2::new(key.x + dx, key.y + dz)) {
                    out.extend_from_slice(list);
                }
            }
        }
    }
}

pub struct MobAiPlugin;

impl Plugin for MobAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MobSpatialGrid>()
            .add_systems(
                Update,
                (
                    mob_ai_think,
                    build_mob_spatial_grid,
                    mob_movement_and_separation,
                    unstuck_mobs,
                    mob_leg_animation_system,
                    despawn_dead_mobs,
                    despawn_distant_mobs,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
        info!("MobAiPlugin loaded.");
    }
}

const INSTANT_DESPAWN_DISTANCE: f32 = 128.0;
const DESPAWN_SAFE_DISTANCE: f32 = 32.0;
const RANDOM_DESPAWN_TIME: f32 = 30.0;
const RANDOM_DESPAWN_CHANCE_PER_TICK: f32 = 1.0 / 800.0;

fn despawn_distant_mobs(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut mob_q: Query<(Entity, &Transform, &mut Mob)>,
    mut commands: Commands,
    mut rng_state: Local<u32>,
) {
    let Ok(pt) = player_q.single() else { return; };
    let player_pos = pt.translation;
    let dt = time.delta_secs();

    let ticks_elapsed = dt * 20.0;

    for (entity, tf, mut mob) in mob_q.iter_mut() {
        let dist = tf.translation.distance(player_pos);

        if dist > INSTANT_DESPAWN_DISTANCE {
            commands.entity(entity).despawn();
            continue;
        }

        if dist > DESPAWN_SAFE_DISTANCE {
            mob.despawn_timer += dt;
        } else {
            mob.despawn_timer = 0.0;
        }

        if mob.despawn_timer > RANDOM_DESPAWN_TIME {
            let chance = RANDOM_DESPAWN_CHANCE_PER_TICK * ticks_elapsed;
            if lcg_f32(&mut rng_state) < chance {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[inline]
fn lcg_f32(state: &mut u32) -> f32 {
    if *state == 0 { *state = 0x9E37_79B9; }
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*state >> 16) as u16 as f32) / 65535.0
}

fn mob_collides(world: &WorldData, px: f32, py: f32, pz: f32, r: f32, h: f32) -> bool {
    let x0 = (px - r).floor() as i32;
    let x1 = (px + r).floor() as i32;
    let y0 = py.floor() as i32;
    let y1 = (py + h - 0.001).floor() as i32;
    let z0 = (pz - r).floor() as i32;
    let z1 = (pz + r).floor() as i32;

    for y in y0..=y1 {
        for z in z0..=z1 {
            for x in x0..=x1 {
                if world.is_solid(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

fn highest_solid(world: &WorldData, x: i32, z: i32) -> Option<i32> {
    use crate::core::state::SY;
    (1..SY - 1).rev().find(|&y| world.get(x, y, z).is_solid())
}

fn mob_ai_think(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut q: Query<(&Transform, &mut Mob), Without<Player>>,
    mut rng_local: Local<Option<SmallRng>>,
) {
    let dt = time.delta_secs().min(0.05);
    let rng = rng_local.get_or_insert_with(|| SmallRng::seed_from_u64(0xC0FF_EE00_DEAD_BEEF));

    let player_pos = player_q.single().ok().map(|t| t.translation);

    for (tf, mut mob) in q.iter_mut() {
        let my_pos = tf.translation;

        if mob.hurt_timer > 0.0 { mob.hurt_timer -= dt; }
        if mob.panic_timer > 0.0 { mob.panic_timer -= dt; }
        if mob.jump_cooldown > 0.0 { mob.jump_cooldown -= dt; }
        if mob.alert_look_timer > 0.0 { mob.alert_look_timer -= dt; }

        let dist_to_player = player_pos
            .map(|p| {
                let dx = p.x - my_pos.x;
                let dz = p.z - my_pos.z;
                (dx * dx + dz * dz).sqrt()
            })
            .unwrap_or(f32::MAX);

        if mob.panic_timer > 0.0 {
            mob.state = MobState::Flee;
            mob.state_timer = mob.panic_timer;
            mob.alert_looking = false;
            mob.alert_walking = false;
            continue;
        }

        if let Some(ppos) = player_pos {
            if dist_to_player < mob.mob_type.notice_range() {
                let dx = ppos.x - my_pos.x;
                let dz = ppos.z - my_pos.z;
                let yaw_to_player = (-dx).atan2(-dz);

                if mob.state != MobState::Alert {
                    mob.state = MobState::Alert;
                    mob.state_timer = rng.random_range(4.0..9.0);
                    mob.alert_looking = true;
                    mob.alert_walking = false;
                    mob.alert_look_timer = rng.random_range(1.0..2.5);
                }

                if mob.alert_look_timer <= 0.0 {
                    if mob.alert_looking {
                        mob.alert_looking = false;

                        if rng.random_bool(0.6) {
                            mob.alert_walking = true;
                            let away_yaw = yaw_to_player
                                + std::f32::consts::PI
                                + rng.random_range(-0.8..0.8);
                            mob.target_yaw = away_yaw;
                            mob.alert_look_timer = rng.random_range(1.5..3.5);
                        } else {
                            mob.alert_walking = false;
                            let offset = rng.random_range(-1.8..1.8);
                            mob.target_yaw = yaw_to_player + offset;
                            mob.alert_look_timer = rng.random_range(0.8..2.0);
                        }
                    } else {
                        if mob.alert_walking {
                            if rng.random_bool(0.5) {
                                mob.alert_looking = true;
                                mob.alert_walking = false;
                                mob.alert_look_timer = rng.random_range(0.8..2.0);
                            } else {
                                let new_yaw = yaw_to_player
                                    + std::f32::consts::PI
                                    + rng.random_range(-1.2..1.2);
                                mob.target_yaw = new_yaw;
                                mob.alert_look_timer = rng.random_range(1.0..2.5);
                            }
                        } else {
                            if rng.random_bool(0.7) {
                                mob.alert_looking = true;
                                mob.alert_look_timer = rng.random_range(0.8..2.0);
                            } else {
                                mob.alert_walking = true;
                                let away_yaw = yaw_to_player
                                    + std::f32::consts::PI
                                    + rng.random_range(-0.8..0.8);
                                mob.target_yaw = away_yaw;
                                mob.alert_look_timer = rng.random_range(1.5..3.5);
                            }
                        }
                    }
                }

                if mob.alert_looking {
                    let current_target = mob.target_yaw;
                    let mut diff = yaw_to_player - current_target;
                    while diff > std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                    while diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                    mob.target_yaw = current_target + diff * (3.0 * dt).min(1.0);
                }

                mob.state_timer -= dt;

                if mob.state_timer <= 0.0 {
                    mob.state = MobState::Wander;
                    mob.target_yaw = rng.random_range(0.0..std::f32::consts::TAU);
                    mob.state_timer = rng.random_range(3.0..6.0);
                    mob.alert_looking = false;
                    mob.alert_walking = false;
                }
                continue;
            }
        }

        mob.alert_looking = false;
        mob.alert_walking = false;

        mob.state_timer -= dt;
        if mob.state_timer <= 0.0 {
            match mob.state {
                MobState::Idle => {
                    mob.state = MobState::Wander;
                    mob.target_yaw = rng.random_range(0.0..std::f32::consts::TAU);
                    mob.state_timer = rng.random_range(3.0..6.0);
                }
                MobState::Wander => {
                    if rng.random_bool(0.4) {
                        mob.state = MobState::Idle;
                        mob.state_timer = rng.random_range(1.5..3.5);
                    } else {
                        mob.target_yaw = rng.random_range(0.0..std::f32::consts::TAU);
                        mob.state_timer = rng.random_range(3.0..6.0);
                    }
                }
                _ => {
                    mob.state = MobState::Idle;
                    mob.state_timer = rng.random_range(1.0..2.0);
                }
            }
        }
    }
}

/// Строит spatial hash мобов. Вызывается каждый кадр перед
/// `mob_movement_and_separation`, чтобы система расталкивания
/// могла искать соседей быстро.
fn build_mob_spatial_grid(
    mut grid: ResMut<MobSpatialGrid>,
    mob_q: Query<(Entity, &Transform, &Mob)>,
) {
    grid.clear();
    for (entity, tf, mob) in mob_q.iter() {
        grid.insert(MobEntry {
            entity,
            pos: tf.translation,
            radius: mob.mob_type.radius(),
        });
    }
}

fn mob_movement_and_separation(
    time: Res<Time>,
    world: Res<WorldData>,
    player_q: Query<&Transform, With<Player>>,
    mut q: Query<(Entity, &mut Transform, &mut Mob), Without<Player>>,
    grid: Res<MobSpatialGrid>,
    mut rng_local: Local<Option<SmallRng>>,
    mut neighbor_buf: Local<Vec<MobEntry>>,
) {
    let dt = time.delta_secs().min(0.05);
    let rng = rng_local.get_or_insert_with(|| SmallRng::seed_from_u64(0xABCD_1234_5678_9F01));
    let player_pos = player_q.single().ok().map(|t| t.translation);

    for (entity, mut tf, mut mob) in q.iter_mut() {
        let r = mob.mob_type.radius();
        let h = mob.mob_type.height();

        let (speed, allow_move) = match mob.state {
            MobState::Idle => (0.0, false),
            MobState::Alert => {
                if mob.alert_walking {
                    (mob.mob_type.speed() * 0.55, true)
                } else {
                    (0.0, false)
                }
            }
            MobState::Wander => (mob.mob_type.speed() * 0.75, true),
            MobState::Flee => {
                if let Some(ppos) = player_pos {
                    let dx = tf.translation.x - ppos.x;
                    let dz = tf.translation.z - ppos.z;
                    mob.target_yaw = (-dx).atan2(-dz);
                }
                (mob.mob_type.speed() * 1.6, true)
            }
        };

        let mut dyaw = mob.target_yaw - mob.yaw;
        while dyaw > std::f32::consts::PI { dyaw -= std::f32::consts::TAU; }
        while dyaw < -std::f32::consts::PI { dyaw += std::f32::consts::TAU; }

        let turn_speed = if mob.state == MobState::Alert { 6.0 } else { 5.0 };
        mob.yaw += dyaw * (turn_speed * dt).min(1.0);

        while mob.yaw > std::f32::consts::TAU { mob.yaw -= std::f32::consts::TAU; }
        while mob.yaw < 0.0 { mob.yaw += std::f32::consts::TAU; }

        tf.rotation = Quat::from_rotation_y(mob.yaw);

        if allow_move && mob.on_ground {
            let forward = Vec3::new(-mob.yaw.sin(), 0.0, -mob.yaw.cos());
            let test_dist = r + 0.15;
            let test_x = tf.translation.x + forward.x * test_dist;
            let test_z = tf.translation.z + forward.z * test_dist;

            let blocked = mob_collides(&world, test_x, tf.translation.y, test_z, r, h);

            if blocked && mob.jump_cooldown <= 0.0 {
                let above_y = tf.translation.y + 1.0;
                let above_blocked = mob_collides(&world, test_x, above_y, test_z, r, h);

                if !above_blocked {
                    mob.velocity.y = 8.5;
                    mob.jump_cooldown = 0.6;
                } else {
                    let turn_choice = rng.random_range(0..3);
                    let turn_angle: f32 = match turn_choice {
                        0 => std::f32::consts::FRAC_PI_2,
                        1 => -std::f32::consts::FRAC_PI_2,
                        _ => std::f32::consts::PI,
                    };
                    mob.target_yaw = mob.yaw + turn_angle + rng.random_range(-0.3..0.3);
                    mob.state_timer = mob.state_timer.max(1.0);
                }
            }
        }

        let pos_before = tf.translation;

        let forward = Vec3::new(-mob.yaw.sin(), 0.0, -mob.yaw.cos());
        let horiz = forward * speed;

        mob.velocity.y -= 28.0 * dt;
        if mob.velocity.y < -30.0 { mob.velocity.y = -30.0; }

        let new_x = pos_before.x + horiz.x * dt;
        if !mob_collides(&world, new_x, pos_before.y, pos_before.z, r, h) {
            tf.translation.x = new_x;
        }

        let new_z = pos_before.z + horiz.z * dt;
        if !mob_collides(&world, tf.translation.x, pos_before.y, new_z, r, h) {
            tf.translation.z = new_z;
        }

        let new_y = pos_before.y + mob.velocity.y * dt;
        if !mob_collides(&world, tf.translation.x, new_y, tf.translation.z, r, h) {
            tf.translation.y = new_y;
            mob.on_ground = false;
        } else {
            if mob.velocity.y < 0.0 { mob.on_ground = true; }
            mob.velocity.y = 0.0;
        }

        if tf.translation.y < 2.0 {
            let ix = tf.translation.x.floor() as i32;
            let iz = tf.translation.z.floor() as i32;
            if let Some(top_y) = highest_solid(&world, ix, iz) {
                tf.translation.y = (top_y + 1) as f32;
            } else {
                tf.translation.y = 30.0;
            }
            mob.velocity = Vec3::ZERO;
        }

        tf.scale = Vec3::ONE;

        // Расталкивание через spatial hash: берём только мобов
        // в 9 ячейках вокруг текущей позиции.
        if allow_move {
            grid.collect_neighbors(tf.translation, &mut neighbor_buf);

            let mut push_x = 0.0_f32;
            let mut push_z = 0.0_f32;
            let my_pos = tf.translation;

            for other in neighbor_buf.iter() {
                if other.entity == entity { continue; }

                let dx = my_pos.x - other.pos.x;
                let dz = my_pos.z - other.pos.z;
                let dist_sq = dx * dx + dz * dz;
                let min_dist = r + other.radius + 0.15;

                if dist_sq < min_dist * min_dist && dist_sq > 1e-6 {
                    let dist = dist_sq.sqrt();
                    let strength = (min_dist - dist) * 0.5;
                    push_x += (dx / dist) * strength;
                    push_z += (dz / dist) * strength;
                }
            }

            tf.translation.x += push_x;
            tf.translation.z += push_z;
        }

        if allow_move && mob.on_ground {
            let dx = tf.translation.x - pos_before.x;
            let dz = tf.translation.z - pos_before.z;
            let moved = (dx * dx + dz * dz).sqrt();
            let expected = speed * dt;

            if expected > 0.001 && moved < expected * 0.3 {
                let turn = rng.random_range(1.5..3.0)
                    * if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                mob.target_yaw = mob.yaw + turn;
                mob.state_timer = mob.state_timer.max(0.8);
            }
        }

        let walking_state = matches!(mob.state, MobState::Wander | MobState::Flee)
            || (mob.state == MobState::Alert && mob.alert_walking);
        let want_walk = walking_state && mob.on_ground;

        if want_walk {
            mob.walk_phase += dt * 10.0;
            while mob.walk_phase > std::f32::consts::TAU {
                mob.walk_phase -= std::f32::consts::TAU;
            }
            mob.walk_amplitude = (mob.walk_amplitude + dt * 5.0).min(1.0);
        } else {
            mob.walk_amplitude = (mob.walk_amplitude - dt * 4.0).max(0.0);
        }
    }
}

fn unstuck_mobs(
    time: Res<Time>,
    world: Res<WorldData>,
    mut q: Query<(&mut Transform, &mut Mob), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.05);

    for (mut tf, mut mob) in q.iter_mut() {
        let r = mob.mob_type.radius();
        let h = mob.mob_type.height();
        let pos = tf.translation;

        if mob_collides(&world, pos.x, pos.y, pos.z, r, h) {
            mob.stuck_timer += dt;

            let mut test_y = pos.y;
            let mut freed = false;
            for _ in 0..10 {
                test_y += 1.0;
                if !mob_collides(&world, pos.x, test_y, pos.z, r, h) {
                    freed = true;
                    break;
                }
            }
            if freed {
                tf.translation.y = test_y;
                mob.velocity.y = 0.0;
            }
        } else {
            mob.stuck_timer = (mob.stuck_timer - dt).max(0.0);
        }

        if mob.stuck_timer > 3.0 {
            let ix = pos.x.floor() as i32;
            let iz = pos.z.floor() as i32;
            if let Some(top_y) = highest_solid(&world, ix, iz) {
                tf.translation.y = (top_y + 1) as f32;
                mob.velocity = Vec3::ZERO;
                mob.stuck_timer = 0.0;
            }
        }
    }
}

fn mob_leg_animation_system(
    mob_q: Query<&Mob>,
    mut pivot_q: Query<&mut Transform>,
) {
    for mob in mob_q.iter() {
        let swing = mob.walk_phase.sin() * 0.55 * mob.walk_amplitude;

        for (i, &pivot_entity) in mob.legs.iter().enumerate() {
            if let Ok(mut tf) = pivot_q.get_mut(pivot_entity) {
                let phase: f32 = if i < 2 { 1.0 } else { -1.0 };
                tf.rotation = Quat::from_rotation_x(swing * phase);
            }
        }
    }
}

fn despawn_dead_mobs(
    mut commands: Commands,
    q: Query<(Entity, &Mob)>,
) {
    for (entity, mob) in q.iter() {
        if mob.hp <= 0 {
            commands.entity(entity).despawn();
        }
    }
}