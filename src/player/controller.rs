use bevy::prelude::*;
use crate::core::state::PlayerLook;
use crate::world::chunk::WorldData;

const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_RADIUS: f32 = 0.3;
const GRAVITY: f32 = 28.0;
const JUMP_VELOCITY: f32 = 9.0;
const WALK_SPEED: f32 = 4.6;
const SPRINT_SPEED: f32 = 7.4;
const FLY_SPEED: f32 = 12.0;
const FLY_SPEED_FAST: f32 = 26.0;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fly: bool,
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
           .add_systems(Update, (player_movement, toggle_fly));
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player {
            velocity: Vec3::ZERO,
            on_ground: false,
            fly: false,
        },
        Transform::from_xyz(128.0, 60.0, 128.0),
        Visibility::default(),
        Name::new("Player"),
    ));
}

fn toggle_fly(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<&mut Player>,
) {
    if keys.just_pressed(KeyCode::KeyF) {
        for mut p in player_q.iter_mut() {
            p.fly = !p.fly;
            p.velocity.y = 0.0;
        }
    }
}

fn collides(world: &WorldData, px: f32, py: f32, pz: f32) -> bool {
    let x0 = (px - PLAYER_RADIUS).floor() as i32;
    let x1 = (px + PLAYER_RADIUS).floor() as i32;
    let y0 = py.floor() as i32;
    let y1 = (py + PLAYER_HEIGHT - 0.001).floor() as i32;
    let z0 = (pz - PLAYER_RADIUS).floor() as i32;
    let z1 = (pz + PLAYER_RADIUS).floor() as i32;

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

fn player_movement(
    mut player_q: Query<(&mut Transform, &mut Player)>,
    world: Res<WorldData>,
    look: Res<PlayerLook>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs().min(0.05);

    // ----- Направления относительно камеры -----
    // yaw = 0   → игрок смотрит в сторону -Z
    // yaw = π/2 → игрок смотрит в сторону -X
    let yaw = look.yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right   = Vec3::new( yaw.cos(), 0.0, -yaw.sin());

    for (mut transform, mut player) in player_q.iter_mut() {
        // ----- Ввод: WASD + стрелки -----
        let mut dir = Vec3::ZERO;

        // Вперёд
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            dir += forward;
        }
        // Назад
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            dir -= forward;
        }
        // Вправо
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            dir += right;
        }
        // Влево
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            dir -= right;
        }

        // Нормализуем диагонали, чтобы не двигаться быстрее
        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }

        // Модификаторы
        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

        // ----- Полёт / Ходьба -----
        if player.fly {
            let speed = if ctrl { FLY_SPEED_FAST } else { FLY_SPEED };

            let mut vel = dir * speed;
            if keys.pressed(KeyCode::Space) { vel.y += speed; }
            if shift { vel.y -= speed; }

            player.velocity = vel;

            // Пробуем двигаться по осям отдельно — чтобы не застревать
            let pos = transform.translation;

            let new_x = pos.x + player.velocity.x * dt;
            if !collides(&world, new_x, pos.y, pos.z) {
                transform.translation.x = new_x;
            }
            let new_z = pos.z + player.velocity.z * dt;
            if !collides(&world, transform.translation.x, pos.y, new_z) {
                transform.translation.z = new_z;
            }
            let new_y = pos.y + player.velocity.y * dt;
            if !collides(&world, transform.translation.x, new_y, transform.translation.z) {
                transform.translation.y = new_y;
            } else {
                player.velocity.y = 0.0;
            }
            continue;
        }

        // Ходьба / бег
        let speed = if ctrl { SPRINT_SPEED } else { WALK_SPEED };
        let horiz_vel = dir * speed;

        // Гравитация
        player.velocity.y -= GRAVITY * dt;
        if player.velocity.y < -55.0 {
            player.velocity.y = -55.0;
        }

        // Присесть (замедляет, не критично)
        let horiz_vel = if shift { horiz_vel * 0.4 } else { horiz_vel };

        // Пошаговое движение по осям
        let pos = transform.translation;

        let new_x = pos.x + horiz_vel.x * dt;
        if !collides(&world, new_x, pos.y, pos.z) {
            transform.translation.x = new_x;
        }

        let new_z = pos.z + horiz_vel.z * dt;
        if !collides(&world, transform.translation.x, pos.y, new_z) {
            transform.translation.z = new_z;
        }

        let new_y = pos.y + player.velocity.y * dt;
        if !collides(&world, transform.translation.x, new_y, transform.translation.z) {
            transform.translation.y = new_y;
            player.on_ground = false;
        } else {
            if player.velocity.y < 0.0 {
                player.on_ground = true;
            }
            player.velocity.y = 0.0;
        }

        // Прыжок
        if keys.just_pressed(KeyCode::Space) && player.on_ground {
            player.velocity.y = JUMP_VELOCITY;
            player.on_ground = false;
        }
    }
}