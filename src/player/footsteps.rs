use bevy::prelude::*;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};

use crate::core::state::AppState;
use crate::world::chunk::WorldData;
use crate::audio::SoundHandles;
use super::controller::Player;

/// Базовая громкость шагов. Тише, чем ломание/установка.
const STEP_VOLUME: f32 = 0.35;

/// Ниже этой горизонтальной скорости считаем, что игрок стоит.
const MIN_SPEED: f32 = 0.5;

/// Сколько секунд игрок должен непрерывно быть в воздухе, чтобы
/// его отрыв от земли считался настоящим (прыжок/падение), а не
/// дрожанием флага on_ground при контакте с поверхностью.
const MIN_AIRBORNE_TIME: f32 = 0.15;

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

fn footsteps_system(
    time: Res<Time>,
    world: Res<WorldData>,
    player_q: Query<(&Transform, &Player)>,
    sounds: Option<Res<SoundHandles>>,
    mut commands: Commands,
    mut last_pos: Local<Vec3>,
    mut step_timer: Local<f32>,
    mut airborne_time: Local<f32>,
) {
    let dt = time.delta_secs().max(0.0001);
    let Ok((tf, player)) = player_q.single() else { return; };

    let pos = tf.translation;

    // В полёте или в воздухе — копим время отрыва.
    // on_ground может дёргаться каждый кадр (см. controller.rs),
    // поэтому настоящий прыжок — это > MIN_AIRBORNE_TIME секунд.
    if player.fly || !player.on_ground {
        *airborne_time += dt;
        *last_pos = pos;
        return;
    }

    // Игрок на земле. Если до этого он реально висел в воздухе —
    // сбрасываем таймер, чтобы первый шаг после приземления сработал
    // сразу. Если же это был короткий «дребезг» on_ground — таймер
    // сохраняется и второй звук не играет.
    if *airborne_time > MIN_AIRBORNE_TIME {
        *step_timer = 0.0;
    }
    *airborne_time = 0.0;

    // Горизонтальная скорость по разнице позиций.
    let dx = pos.x - last_pos.x;
    let dz = pos.z - last_pos.z;
    let horiz_speed = ((dx * dx + dz * dz).sqrt()) / dt;
    *last_pos = pos;

    if horiz_speed < MIN_SPEED {
        // Стоим на месте — таймер в 0, чтобы первый шаг
        // после начала движения сработал сразу.
        *step_timer = 0.0;
        return;
    }

    // Интервал между шагами зависит от скорости:
    //   ходьба  ~6.0  → интервал 0.50 сек
    //   бег     ~9.5  → интервал 0.32 сек
    //   спринт  ~15   → интервал 0.20 сек
    let interval = (3.0 / horiz_speed).clamp(0.20, 0.55);

    *step_timer -= dt;
    if *step_timer > 0.0 {
        return;
    }
    *step_timer = interval;

    // Блок под ногами (Player.translation — это позиция ног).
    let bx = pos.x.floor() as i32;
    let by = (pos.y - 0.5).floor() as i32;
    let bz = pos.z.floor() as i32;

    let block = world.get(bx, by, bz);
    if !block.is_solid() {
        return;
    }

    if let Some(s) = sounds.as_ref() {
        commands.spawn((
            AudioPlayer(s.step_sound_for(block)),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(STEP_VOLUME)),
        ));
    }
}