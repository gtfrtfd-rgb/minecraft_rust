use bevy::prelude::*;
use virtual_joystick::VirtualJoystickMessage;
use crate::core::state::{is_mobile, AppState, PlayerLook};
use crate::world::chunk::WorldData;
use crate::save::persistence::LoadedSave;

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

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fly: bool,
    pub jump_buffer: f32,
    pub coyote_timer: f32,
}

#[derive(Component)]
pub struct MoveJoystick;

#[derive(Component)]
pub struct JumpButton;

#[derive(Component)]
pub struct FlyButton;

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Startup, apply_loaded_state.after(spawn_player))
            .add_systems(
                Update,
                (player_movement, toggle_fly, handle_mobile_buttons)
                    .run_if(in_state(AppState::InGame)),
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
        Transform::from_xyz(128.0, 40.0, 128.0),
        Visibility::default(),
        Name::new("Player"),
    ));
}

fn apply_loaded_state(
    loaded: Res<LoadedSave>,
    mut player_q: Query<(&mut Transform, &mut Player)>,
    mut look: ResMut<PlayerLook>,
) {
    let Some(meta) = &loaded.meta else { return; };

    if let Ok((mut tf, mut player)) = player_q.single_mut() {
        tf.translation = Vec3::new(meta.player.x, meta.player.y, meta.player.z);
        player.fly = meta.player.fly;
    }
    look.yaw = meta.player.yaw;
    look.pitch = meta.player.pitch;

    info!(
        "Player position restored: ({:.1}, {:.1}, {:.1}) fly={}",
        meta.player.x, meta.player.y, meta.player.z, meta.player.fly
    );
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

fn handle_mobile_buttons(
    mut jump_q: Query<&Interaction, (Changed<Interaction>, With<JumpButton>)>,
    mut fly_q: Query<&Interaction, (Changed<Interaction>, With<FlyButton>)>,
    mut player_q: Query<&mut Player>,
) {
    for interaction in jump_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            for mut p in player_q.iter_mut() {
                p.jump_buffer = JUMP_BUFFER;
            }
        }
    }

    for interaction in fly_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            for mut p in player_q.iter_mut() {
                p.fly = !p.fly;
                p.velocity.y = 0.0;
            }
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
    mut joystick_reader: MessageReader<VirtualJoystickMessage<()>>,
) {
    let dt = time.delta_secs().min(0.05);

    let yaw = look.yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

    let space_held = keys.pressed(KeyCode::Space);

    // Считываем значение джойстика (только на мобильных).
    // msg.axis() возвращает &Vec2, поэтому разыменовываем.
    let mut joystick_dir = Vec2::ZERO;
    if is_mobile() {
        for msg in joystick_reader.read() {
            joystick_dir = *msg.axis();
        }
    }

    for (mut transform, mut player) in player_q.iter_mut() {
        let mut dir = Vec3::ZERO;

        if is_mobile() {
            if joystick_dir.length_squared() > 0.01 {
                dir = forward * (-joystick_dir.y) + right * joystick_dir.x;
                dir = dir.normalize_or_zero();
            }
        } else {
            if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
                dir += forward;
            }
            if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
                dir -= forward;
            }
            if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
                dir += right;
            }
            if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
                dir -= right;
            }

            if dir.length_squared() > 0.0 {
                dir = dir.normalize();
            }
        }

        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

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
            let speed = if ctrl { FLY_SPEED_FAST } else { FLY_SPEED };
            let mut vel = dir * speed;
            if space_held {
                vel.y += speed;
            }
            if shift {
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

        let speed = if ctrl { SPRINT_SPEED } else { WALK_SPEED };
        let horiz_vel = dir * speed;
        let horiz_vel = if shift { horiz_vel * 0.5 } else { horiz_vel };

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
    }
}