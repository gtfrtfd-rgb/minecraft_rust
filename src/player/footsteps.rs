use bevy::prelude::*;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};

use crate::core::state::{AppState, Settings};
use crate::world::chunk::WorldData;
use crate::audio::SoundHandles;
use super::controller::Player;

const STEP_VOLUME: f32 = 0.10;
const SNEAK_VOLUME: f32 = 0.035;
const WALK_PITCH: f32 = 1.0;
const SNEAK_PITCH: f32 = 1.22;

const LANDING_VOLUME: f32 = 0.28;
const LANDING_PITCH: f32 = 0.72;

const MIN_SPEED: f32 = 0.5;
const MIN_AIRBORNE_TIME: f32 = 0.15;
const HARD_LANDING_TIME: f32 = 0.45;

pub struct FootstepsPlugin;

impl Plugin for FootstepsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            footsteps_system.run_if(in_state(AppState::InGame)),
        );
        info!("FootstepsPlugin loaded.");
    }
}

#[inline]
fn lcg_rand(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 16) & 0x7FFF
}

fn footsteps_system(
    time: Res<Time>,
    world: Res<WorldData>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    player_q: Query<(&Transform, &Player)>,
    sounds: Option<Res<SoundHandles>>,
    mut commands: Commands,
    mut last_pos: Local<Vec3>,
    mut step_timer: Local<f32>,
    mut airborne_time: Local<f32>,
    mut rng_state: Local<u32>,
) {
    let dt = time.delta_secs().max(0.0001);
    let Ok((tf, player)) = player_q.single() else { return; };

    let pos = tf.translation;

    if player.fly || !player.on_ground {
        *airborne_time += dt;
        *last_pos = pos;
        return;
    }

    let just_landed = *airborne_time > MIN_AIRBORNE_TIME;
    let master = settings.master_volume;

    if just_landed {
        *step_timer = 0.0;

        if let Some(s) = sounds.as_ref() {
            let bx = pos.x.floor() as i32;
            let by = (pos.y - 0.5).floor() as i32;
            let bz = pos.z.floor() as i32;
            let block = world.get(bx, by, bz);

            if block.is_solid() {
                let variants = s.step_sounds_for(block);
                if !variants.is_empty() {
                    let idx = (lcg_rand(&mut rng_state) as usize) % variants.len();
                    let handle = variants[idx].clone();

                    let fall_t = (*airborne_time / HARD_LANDING_TIME).clamp(0.0, 1.0);
                    let vol_scale = 1.0 + fall_t * 0.5;
                    let pitch_jitter =
                        (lcg_rand(&mut rng_state) as f32 / 0x7FFF as f32 - 0.5) * 0.06;

                    commands.spawn((
                        AudioPlayer(handle),
                        PlaybackSettings::DESPAWN
                            .with_volume(Volume::Linear(LANDING_VOLUME * vol_scale * master))
                            .with_speed(LANDING_PITCH + pitch_jitter),
                    ));
                }
            }
        }
    }
    *airborne_time = 0.0;

    let dx = pos.x - last_pos.x;
    let dz = pos.z - last_pos.z;
    let horiz_speed = ((dx * dx + dz * dz).sqrt()) / dt;
    *last_pos = pos;

    if horiz_speed < MIN_SPEED {
        *step_timer = 0.0;
        return;
    }

    let crouching = keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ShiftRight);

    let base_interval = (2.4 / horiz_speed).clamp(0.18, 0.40);
    let interval = if crouching {
        base_interval * 1.65
    } else {
        base_interval
    };

    *step_timer -= dt;
    if *step_timer > 0.0 {
        return;
    }
    *step_timer = interval;

    let bx = pos.x.floor() as i32;
    let by = (pos.y - 0.5).floor() as i32;
    let bz = pos.z.floor() as i32;

    let block = world.get(bx, by, bz);
    if !block.is_solid() {
        return;
    }

    let Some(s) = sounds.as_ref() else { return; };
    let variants = s.step_sounds_for(block);
    if variants.is_empty() {
        return;
    }

    let idx = (lcg_rand(&mut rng_state) as usize) % variants.len();
    let handle = variants[idx].clone();

    let vol_jitter =
        0.90 + (lcg_rand(&mut rng_state) as f32 / 0x7FFF as f32) * 0.20;
    let pitch_jitter =
        (lcg_rand(&mut rng_state) as f32 / 0x7FFF as f32 - 0.5) * 0.04;

    let (base_vol, base_pitch) = if crouching {
        (SNEAK_VOLUME, SNEAK_PITCH)
    } else {
        (STEP_VOLUME, WALK_PITCH)
    };

    commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(base_vol * vol_jitter * master))
            .with_speed(base_pitch + pitch_jitter),
    ));
}