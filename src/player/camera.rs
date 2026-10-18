use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::core_pipeline::tonemapping::Tonemapping;
use crate::core::state::{AppState, PlayerLook, Settings};
use super::controller::Player;

#[derive(Component)]
pub struct PlayerCamera;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                camera_follow_and_look.run_if(in_state(AppState::InGame)),
            );
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,                    // <-- отключаем MSAA
        Tonemapping::None,            // <-- отключаем tone mapping
        Transform::from_xyz(128.0, 42.0, 128.0)
            .looking_at(Vec3::new(128.0, 20.0, 100.0), Vec3::Y),
        PlayerCamera,
    ));
}

fn camera_follow_and_look(
    mut motion: MessageReader<MouseMotion>,
    mut look: ResMut<PlayerLook>,
    settings: Res<Settings>,
    mut camera_q: Query<&mut Transform, With<PlayerCamera>>,
    player_q: Query<&Transform, (With<Player>, Without<PlayerCamera>)>,
) {
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta != Vec2::ZERO {
        let sens = settings.mouse_sensitivity;
        look.yaw -= delta.x * sens;
        look.pitch = (look.pitch - delta.y * sens).clamp(-1.5, 1.5);
    }

    let rot = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
    if let Ok(p_tf) = player_q.single() {
        if let Ok(mut c_tf) = camera_q.single_mut() {
            c_tf.translation = p_tf.translation + Vec3::new(0.0, 1.62, 0.0);
            c_tf.rotation = rot;
        }
    }
}