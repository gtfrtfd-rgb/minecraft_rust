use bevy::prelude::*;
use rand::Rng;

use crate::core::state::AppState;
use crate::world::chunk::WorldData;
use crate::player::controller::Player;

// ============================================================
// ТИПЫ МОБОВ
// ============================================================
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
            MobType::Sheep => 1.25,
            MobType::Cow => 1.45,
            MobType::Chicken => 0.7,
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
            MobType::Sheep => Vec3::new(0.75, 0.75, 1.0),
            MobType::Cow => Vec3::new(0.75, 0.625, 1.125),
            MobType::Chicken => Vec3::new(0.3, 0.4, 0.4),
        }
    }

    pub fn head_size(self) -> Vec3 {
        match self {
            MobType::Pig => Vec3::new(0.5, 0.5, 0.5),
            MobType::Sheep => Vec3::new(0.4, 0.5, 0.4),
            MobType::Cow => Vec3::new(0.5, 0.5, 0.5),
            MobType::Chicken => Vec3::new(0.2, 0.3, 0.2),
        }
    }

    pub fn leg_size(self) -> Vec3 {
        match self {
            MobType::Pig => Vec3::new(0.22, 0.375, 0.22),
            MobType::Sheep => Vec3::new(0.22, 0.5, 0.22),
            MobType::Cow => Vec3::new(0.22, 0.75, 0.22),
            MobType::Chicken => Vec3::new(0.08, 0.25, 0.08),
        }
    }
}

// ============================================================
// СОСТОЯНИЯ AI
// ============================================================
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobState {
    Idle,
    Wander,
    Alert,
    Flee,
}

// ============================================================
// КОМПОНЕНТ
// ============================================================
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

    /// Alert: смотрит ли сейчас на игрока
    pub alert_looking: bool,
    /// Alert: идёт ли сейчас (в сторону, без разворота к игроку)
    pub alert_walking: bool,
    /// Alert: таймер текущего подсостояния
    pub alert_look_timer: f32,

    pub legs: Vec<Entity>,
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
        }
    }
}

// ============================================================
// ПЛАГИН
// ============================================================
pub struct MobAiPlugin;

impl Plugin for MobAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                mob_ai_think,
                mob_movement_and_separation,
                unstuck_mobs,
                mob_leg_animation_system,
                despawn_dead_mobs,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
        info!("MobAiPlugin loaded.");
    }
}

// ============================================================
// ХЕЛПЕРЫ
// ============================================================
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
    for y in (1..SY - 1).rev() {
        if world.get(x, y, z).is_solid() {
            return Some(y);
        }
    }
    None
}

// ============================================================
// AI: ПРИНЯТИЕ РЕШЕНИЙ
// ============================================================
fn mob_ai_think(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut q: Query<(&Transform, &mut Mob), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.05);
    let mut rng = rand::thread_rng();

    let player_pos = player_q.get_single().ok().map(|t| t.translation);

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

        // === ПРИОРИТЕТ 1: Паника ===
        if mob.panic_timer > 0.0 {
            mob.state = MobState::Flee;
            mob.state_timer = mob.panic_timer;
            mob.alert_looking = false;
            mob.alert_walking = false;
            continue;
        }

        // === ПРИОРИТЕТ 2: Игрок рядом ===
        if let Some(ppos) = player_pos {
            if dist_to_player < mob.mob_type.notice_range() {
                let dx = ppos.x - my_pos.x;
                let dz = ppos.z - my_pos.z;
                let yaw_to_player = (-dx).atan2(-dz);

                // Первое обнаружение
                if mob.state != MobState::Alert {
                    mob.state = MobState::Alert;
                    mob.state_timer = rng.gen_range(4.0..9.0);
                    mob.alert_looking = true;
                    mob.alert_walking = false;
                    mob.alert_look_timer = rng.gen_range(1.0..2.5);
                }

                // === ЧЕРЕДОВАНИЕ: смотрит / отводит / уходит ===
                if mob.alert_look_timer <= 0.0 {
                    if mob.alert_looking {
                        // Был смотрящим → переключаемся
                        mob.alert_looking = false;

                        // 60% идём, 40% просто отводим взгляд
                        if rng.gen_bool(0.6) {
                            mob.alert_walking = true;
                            // Идём ОТ игрока или вбок
                            let away_yaw = yaw_to_player
                                + std::f32::consts::PI
                                + rng.gen_range(-0.8..0.8);
                            mob.target_yaw = away_yaw;
                            mob.alert_look_timer = rng.gen_range(1.5..3.5);
                        } else {
                            mob.alert_walking = false;
                            // Взгляд в сторону
                            let offset = rng.gen_range(-1.8..1.8);
                            mob.target_yaw = yaw_to_player + offset;
                            mob.alert_look_timer = rng.gen_range(0.8..2.0);
                        }
                    } else {
                        // Был НЕ смотрящим → решаем дальше
                        if mob.alert_walking {
                            // Ходил → либо продолжаем, либо останавливаемся и смотрим
                            if rng.gen_bool(0.5) {
                                mob.alert_looking = true;
                                mob.alert_walking = false;
                                mob.alert_look_timer = rng.gen_range(0.8..2.0);
                            } else {
                                // Новое направление "куда-нибудь"
                                let new_yaw = yaw_to_player
                                    + std::f32::consts::PI
                                    + rng.gen_range(-1.2..1.2);
                                mob.target_yaw = new_yaw;
                                mob.alert_look_timer = rng.gen_range(1.0..2.5);
                            }
                        } else {
                            // Отводил взгляд → смотрим снова (с вероятностью 70%)
                            if rng.gen_bool(0.7) {
                                mob.alert_looking = true;
                                mob.alert_look_timer = rng.gen_range(0.8..2.0);
                            } else {
                                // Или начинаем уходить
                                mob.alert_walking = true;
                                let away_yaw = yaw_to_player
                                    + std::f32::consts::PI
                                    + rng.gen_range(-0.8..0.8);
                                mob.target_yaw = away_yaw;
                                mob.alert_look_timer = rng.gen_range(1.5..3.5);
                            }
                        }
                    }
                }

                // Пока смотрит — плавно подводим yaw к игроку
                if mob.alert_looking {
                    let current_target = mob.target_yaw;
                    let mut diff = yaw_to_player - current_target;
                    while diff > std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                    while diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                    mob.target_yaw = current_target + diff * (3.0 * dt).min(1.0);
                }
                // Пока идёт — target_yaw не меняем (идёт куда решил)

                mob.state_timer -= dt;

                // Пора сбросить Alert
                if mob.state_timer <= 0.0 {
                    mob.state = MobState::Wander;
                    mob.target_yaw = rng.gen_range(0.0..std::f32::consts::TAU);
                    mob.state_timer = rng.gen_range(3.0..6.0);
                    mob.alert_looking = false;
                    mob.alert_walking = false;
                }
                continue;
            }
        }

        // Игрок далеко — сбрасываем Alert-подсостояния
        mob.alert_looking = false;
        mob.alert_walking = false;

        // === ПРИОРИТЕТ 3: Блуждание ===
        mob.state_timer -= dt;
        if mob.state_timer <= 0.0 {
            match mob.state {
                MobState::Idle => {
                    mob.state = MobState::Wander;
                    mob.target_yaw = rng.gen_range(0.0..std::f32::consts::TAU);
                    mob.state_timer = rng.gen_range(3.0..6.0);
                }
                MobState::Wander => {
                    if rng.gen_bool(0.4) {
                        mob.state = MobState::Idle;
                        mob.state_timer = rng.gen_range(1.5..3.5);
                    } else {
                        mob.target_yaw = rng.gen_range(0.0..std::f32::consts::TAU);
                        mob.state_timer = rng.gen_range(3.0..6.0);
                    }
                }
                _ => {
                    mob.state = MobState::Idle;
                    mob.state_timer = rng.gen_range(1.0..2.0);
                }
            }
        }
    }
}

// ============================================================
// ДВИЖЕНИЕ + РАСТАЛКИВАНИЕ
// ============================================================
fn mob_movement_and_separation(
    time: Res<Time>,
    world: Res<WorldData>,
    player_q: Query<&Transform, With<Player>>,
    mut q: Query<(Entity, &mut Transform, &mut Mob), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.05);
    let mut rng = rand::thread_rng();
    let player_pos = player_q.get_single().ok().map(|t| t.translation);

    let snapshots: Vec<(Entity, Vec3, f32)> = q
        .iter()
        .map(|(e, tf, mob)| (e, tf.translation, mob.mob_type.radius()))
        .collect();

    for (entity, mut tf, mut mob) in q.iter_mut() {
        let r = mob.mob_type.radius();
        let h = mob.mob_type.height();

        let (speed, allow_move) = match mob.state {
            MobState::Idle => (0.0, false),
            MobState::Alert => {
                if mob.alert_walking {
                    // Идёт медленно, оглядываясь
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

        // Плавный поворот (в Alert чуть медленнее)
        let turn_speed = if mob.state == MobState::Alert { 6.0 } else { 5.0 };
        mob.yaw += dyaw * (turn_speed * dt).min(1.0);

        while mob.yaw > std::f32::consts::TAU { mob.yaw -= std::f32::consts::TAU; }
        while mob.yaw < 0.0 { mob.yaw += std::f32::consts::TAU; }

        tf.rotation = Quat::from_rotation_y(mob.yaw);

        // Обход препятствий
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
                    let turn_choice = rng.gen_range(0..3);
                    let turn_angle: f32 = match turn_choice {
                        0 => std::f32::consts::FRAC_PI_2,
                        1 => -std::f32::consts::FRAC_PI_2,
                        _ => std::f32::consts::PI,
                    };
                    mob.target_yaw = mob.yaw + turn_angle + rng.gen_range(-0.3..0.3);
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

        // Расталкивание
        let mut push_x = 0.0_f32;
        let mut push_z = 0.0_f32;
        let my_pos = tf.translation;

        for (other_entity, other_pos, other_r) in snapshots.iter() {
            if *other_entity == entity { continue; }

            let dx = my_pos.x - other_pos.x;
            let dz = my_pos.z - other_pos.z;
            let dist_sq = dx * dx + dz * dz;
            let min_dist = r + other_r + 0.15;

            if dist_sq < min_dist * min_dist && dist_sq > 1e-6 {
                let dist = dist_sq.sqrt();
                let strength = (min_dist - dist) * 0.5;
                push_x += (dx / dist) * strength;
                push_z += (dz / dist) * strength;
            }
        }

        tf.translation.x += push_x;
        tf.translation.z += push_z;

        // Разворот если застряли
        if allow_move && mob.on_ground {
            let dx = tf.translation.x - pos_before.x;
            let dz = tf.translation.z - pos_before.z;
            let moved = (dx * dx + dz * dz).sqrt();
            let expected = speed * dt;

            if expected > 0.001 && moved < expected * 0.3 {
                let turn = rng.gen_range(1.5..3.0)
                    * if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                mob.target_yaw = mob.yaw + turn;
                mob.state_timer = mob.state_timer.max(0.8);
            }
        }

        // === АНИМАЦИЯ НОГ ===
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

// ============================================================
// UNSTUCK
// ============================================================
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

// ============================================================
// АНИМАЦИЯ НОГ
// ============================================================
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

// ============================================================
// УДАЛЕНИЕ МЁРТВЫХ
// ============================================================
fn despawn_dead_mobs(
    mut commands: Commands,
    q: Query<(Entity, &Mob)>,
) {
    for (entity, mob) in q.iter() {
        if mob.hp <= 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}