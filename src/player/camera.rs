use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::input::touch::TouchInput;
use bevy::pbr::{DistanceFog, FogFalloff};
use crate::core::state::{is_mobile, AppState, PlayerLook};
use super::controller::Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Resource, Default)]
pub struct TouchLookState {
    pub active_touch_id: Option<u64>,
    pub last_position: Vec2,
}

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchLookState>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (
                    camera_follow_and_look,
                    touch_look_system.run_if(is_mobile),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(128.0, 42.0, 128.0)
            .looking_at(Vec3::new(128.0, 20.0, 100.0), Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.53, 0.81, 0.92),
            falloff: FogFalloff::Linear {
                start: 40.0,
                end: 90.0,
            },
            ..default()
        },
        PlayerCamera,
    ));
}

fn camera_follow_and_look(
    mut motion: MessageReader<MouseMotion>,
    mut look: ResMut<PlayerLook>,
    mut camera_q: Query<&mut Transform, With<PlayerCamera>>,
    player_q: Query<&Transform, (With<Player>, Without<PlayerCamera>)>,
) {
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta != Vec2::ZERO {
        look.yaw -= delta.x * 0.0022;
        look.pitch = (look.pitch - delta.y * 0.0022).clamp(-1.5, 1.5);
    }

    let rot = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
    if let Ok(p_tf) = player_q.single() {
        if let Ok(mut c_tf) = camera_q.single_mut() {
            c_tf.translation = p_tf.translation + Vec3::new(0.0, 1.62, 0.0);
            c_tf.rotation = rot;
        }
    }
}

fn touch_look_system(
    mut touch_events: MessageReader<TouchInput>,
    mut look: ResMut<PlayerLook>,
    mut state: ResMut<TouchLookState>,
) {
    for ev in touch_events.read() {
        match ev.phase {
            bevy::input::touch::TouchPhase::Started => {
                if state.active_touch_id.is_none() {
                    state.active_touch_id = Some(ev.id);
                    state.last_position = ev.position;
                }
            }
            bevy::input::touch::TouchPhase::Moved => {
                if state.active_touch_id == Some(ev.id) {
                    let delta = ev.position - state.last_position;
                    look.yaw -= delta.x * 0.005;
                    look.pitch = (look.pitch - delta.y * 0.005).clamp(-1.5, 1.5);
                    state.last_position = ev.position;
                }
            }
            bevy::input::touch::TouchPhase::Ended
            | bevy::input::touch::TouchPhase::Canceled => {
                if state.active_touch_id == Some(ev.id) {
                    state.active_touch_id = None;
                }
            }
        }
    }
}