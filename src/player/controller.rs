use bevy::prelude::*;
use crate::core::state::{AppState, PlayerLook, SY, KeyBinds};
use crate::world::chunk::WorldData;

const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_RADIUS: f32 = 0.3;
const GRAVITY: f32 = 28.0;
const JUMP_VELOCITY: f32 = 9.0;
const WALK_SPEED: f32 = 6.0;
const SPRINT_SPEED: f32 = 9.5;
const FLY_SPEED: f32 = 15.0;
const FLY_SPEED_FAST: f32 = 30.0;

const JUMP_BUFFER: f32 = 0.15;
const COYOTE_TIME: f32 = 0.10;

const GROUND_SNAP: f32 = 0.5;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fly: bool,
    pub jump_buffer: f32,
    pub coyote_timer: f32,
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (player_movement, toggle_fly).run_if(in_state(AppState::InGame)),
            );
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player {
            velocity: Vec3::ZERO,
            on_ground: false,
            fly: false,
            jump_buffer: 0.0,
            coyote_timer: 0.0,
        },
        Transform::from_xyz(0.5, 40.0, 0.5),
        Visibility::default(),
        Name::new("Player"),
    ));
}

fn toggle_fly(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<KeyBinds>,
    mut player_q: Query<&mut Player>,
) {
    if keys.just_pressed(binds.fly) {
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

fn ground_surface_below(world: &WorldData, px: f32, py: f32, pz: f32) -> Option<f32> {
    let x0 = (px - PLAYER_RADIUS).floor() as i32;
    let x1 = (px + PLAYER_RADIUS).floor() as i32;
    let z0 = (pz - PLAYER_RADIUS).floor() as i32;
    let z1 = (pz + PLAYER_RADIUS).floor() as i32;

    let mut best: Option<f32> = None;
    let y_min = (py - GROUND_SNAP).floor() as i32;
    let y_max = py.ceil() as i32;

    for y in y_min..y_max {
        for z in z0..=z1 {
            for x in x0..=x1 {
                if world.is_solid(x, y, z) {
                    let top = (y + 1) as f32;
                    if top <= py + 0.001 && top >= py - GROUND_SNAP {
                        best = Some(best.map_or(top, |b: f32| b.max(top)));
                    }
                }
            }
        }
    }
    best
}

fn player_movement(
    mut player_q: Query<(&mut Transform, &mut Player)>,
    world: Res<WorldData>,
    look: Res<PlayerLook>,
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<KeyBinds>,
    time: Res<Time>,
) {
    let dt = time.delta_secs().min(0.05);

    let yaw = look.yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

    let space_held = keys.pressed(binds.jump);
    let sprint_held = keys.pressed(binds.sprint);
    let sneak_held = keys.pressed(binds.sneak);

    for (mut transform, mut player) in player_q.iter_mut() {
        let mut dir = Vec3::ZERO;

        if keys.pressed(binds.forward) {
            dir += forward;
        }
        if keys.pressed(binds.backward) {
            dir -= forward;
        }
        if keys.pressed(binds.right) {
            dir += right;
        }
        if keys.pressed(binds.left) {
            dir -= right;
        }

        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }

        if space_held {
            player.jump_buffer = JUMP_BUFFER;
        }
        if player.jump_buffer > 0.0 {
            player.jump_buffer -= dt;
            if player.jump_buffer < 0.0 {
                player.jump_buffer = 0.0;
            }
        }

        if player.fly {
            let speed = if sprint_held { FLY_SPEED_FAST } else { FLY_SPEED };
            let mut vel = dir * speed;
            if space_held {
                vel.y += speed;
            }
            if sneak_held {
                vel.y -= speed;
            }
            player.velocity = vel;

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

        let speed = if sprint_held { SPRINT_SPEED } else { WALK_SPEED };
        let horiz_vel = dir * speed;
        let horiz_vel = if sneak_held { horiz_vel * 0.5 } else { horiz_vel };

        player.velocity.y -= GRAVITY * dt;
        if player.velocity.y < -55.0 {
            player.velocity.y = -55.0;
        }

        let pos = transform.translation;

        let new_x = pos.x + horiz_vel.x * dt;
        if !collides(&world, new_x, pos.y, pos.z) {
            transform.translation.x = new_x;
        }

        let new_z = pos.z + horiz_vel.z * dt;
        if !collides(&world, transform.translation.x, pos.y, new_z) {
            transform.translation.z = new_z;
        }

        let was_on_ground = player.on_ground;
        let new_y = pos.y + player.velocity.y * dt;

        if !collides(&world, transform.translation.x, new_y, transform.translation.z) {
            transform.translation.y = new_y;
            player.on_ground = false;
        } else {
            if player.velocity.y < 0.0 {
                let here = transform.translation;
                if let Some(top) = ground_surface_below(&world, here.x, here.y, here.z) {
                    transform.translation.y = top;
                }
                player.on_ground = true;
            }
            player.velocity.y = 0.0;
        }

        if player.on_ground {
            player.coyote_timer = COYOTE_TIME;
        } else if !was_on_ground {
            player.coyote_timer = (player.coyote_timer - dt).max(0.0);
        }

        let can_jump = player.on_ground || player.coyote_timer > 0.0;
        if player.jump_buffer > 0.0 && can_jump {
            player.velocity.y = JUMP_VELOCITY;
            player.on_ground = false;
            player.jump_buffer = 0.0;
            player.coyote_timer = 0.0;
        }

        if transform.translation.y < -10.0 {
            transform.translation.y = (SY - 10) as f32;
            player.velocity = Vec3::ZERO;
        }
    }
}