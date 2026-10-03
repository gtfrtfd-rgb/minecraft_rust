use bevy::prelude::*;
use rand::Rng;

use crate::core::state::AppState;
use crate::world::chunk::WorldData;

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
            MobType::Pig => 1.4,
            MobType::Sheep => 1.2,
            MobType::Cow => 1.2,
            MobType::Chicken => 2.0,
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

    pub fn body_color(self) -> Color {
        match self {
            MobType::Pig => Color::srgb(0.93, 0.60, 0.60),
            MobType::Sheep => Color::srgb(0.93, 0.93, 0.93),
            MobType::Cow => Color::srgb(0.40, 0.27, 0.13),
            MobType::Chicken => Color::srgb(0.98, 0.98, 0.98),
        }
    }

    pub fn leg_color(self) -> Color {
        match self {
            MobType::Pig => Color::srgb(0.78, 0.50, 0.50),
            MobType::Sheep => Color::srgb(0.28, 0.26, 0.24),
            MobType::Cow => Color::srgb(0.27, 0.17, 0.08),
            MobType::Chicken => Color::srgb(0.91, 0.62, 0.12),
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
// КОМПОНЕНТ
// ============================================================
#[derive(Component)]
pub struct Mob {
    pub mob_type: MobType,
    pub hp: i32,
    pub max_hp: i32,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub walking: bool,
    pub wander_timer: f32,
    pub target_yaw: f32,
    pub panic_timer: f32,
    pub hurt_timer: f32,
}

// ============================================================
// ПЛАГИН
// ============================================================
pub struct MobAiPlugin;

impl Plugin for MobAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (mob_movement_system, despawn_dead_mobs)
                .run_if(in_state(AppState::InGame)),
        );
        info!("MobAiPlugin loaded.");
    }
}

// ============================================================
// КОЛЛИЗИИ
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

// ============================================================
// ДВИЖЕНИЕ
// ============================================================
fn mob_movement_system(
    time: Res<Time>,
    world: Res<WorldData>,
    mut q: Query<(&mut Transform, &mut Mob)>,
) {
    let dt = time.delta_secs().min(0.05);
    let mut rng = rand::thread_rng();

    for (mut tf, mut mob) in q.iter_mut() {
        let r = mob.mob_type.radius();
        let h = mob.mob_type.height();

        if mob.hurt_timer > 0.0 { mob.hurt_timer -= dt; }
        if mob.panic_timer > 0.0 { mob.panic_timer -= dt; }

        // Выбор цели
        if mob.panic_timer > 0.0 {
            mob.walking = true;
            if rng.gen_bool(0.02) {
                mob.target_yaw = rng.gen_range(0.0..std::f32::consts::TAU);
            }
        } else {
            mob.wander_timer -= dt;
            if mob.wander_timer <= 0.0 {
                if mob.walking && rng.gen_bool(0.45) {
                    mob.walking = false;
                    mob.wander_timer = rng.gen_range(1.5..4.5);
                } else {
                    mob.walking = true;
                    mob.target_yaw = rng.gen_range(0.0..std::f32::consts::TAU);
                    mob.wander_timer = rng.gen_range(2.0..5.0);
                }
            }
        }

        // Плавный поворот
        let mut dyaw = mob.target_yaw - tf.rotation.y;
        while dyaw > std::f32::consts::PI { dyaw -= std::f32::consts::TAU; }
        while dyaw < -std::f32::consts::PI { dyaw += std::f32::consts::TAU; }
        tf.rotation.y += dyaw * (5.0 * dt).min(1.0);

        // Скорость
        let speed_mult = if mob.panic_timer > 0.0 { 1.6 } else { 1.0 };
        let speed = if mob.walking { mob.mob_type.speed() * speed_mult } else { 0.0 };

        let yaw = tf.rotation.y;
        let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        let horiz = forward * speed;

        mob.velocity.y -= 28.0 * dt;
        if mob.velocity.y < -30.0 { mob.velocity.y = -30.0; }

        let pos = tf.translation;

        // X
        let new_x = pos.x + horiz.x * dt;
        if !mob_collides(&world, new_x, pos.y, pos.z, r, h) {
            tf.translation.x = new_x;
        } else {
            mob.target_yaw += rng.gen_range(-1.5..1.5);
            mob.wander_timer = mob.wander_timer.max(0.8);
        }

        // Z
        let new_z = pos.z + horiz.z * dt;
        if !mob_collides(&world, tf.translation.x, pos.y, new_z, r, h) {
            tf.translation.z = new_z;
        } else {
            mob.target_yaw += rng.gen_range(-1.5..1.5);
            mob.wander_timer = mob.wander_timer.max(0.8);
        }

        // Y
        let new_y = pos.y + mob.velocity.y * dt;
        if !mob_collides(&world, tf.translation.x, new_y, tf.translation.z, r, h) {
            tf.translation.y = new_y;
            mob.on_ground = false;
        } else {
            if mob.velocity.y < 0.0 { mob.on_ground = true; }
            mob.velocity.y = 0.0;
        }

        if tf.translation.y < 2.0 {
            tf.translation.y = 30.0;
            mob.velocity = Vec3::ZERO;
        }

        // НИКАКИХ манипуляций с масштабом — масштаб всегда (1,1,1)!
        tf.scale = Vec3::ONE;
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
            commands.entity(entity).despawn();
        }
    }
}