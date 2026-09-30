use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use crate::core::state::PlayerLook;
use super::controller::Player;

#[derive(Component)]
pub struct PlayerCamera;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
           .add_systems(Update, camera_follow_and_look);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(128.0, 62.0, 128.0)
            .looking_at(Vec3::new(128.0, 30.0, 100.0), Vec3::Y),
        PlayerCamera,
    ));
}

fn camera_follow_and_look(
    mut motion: EventReader<MouseMotion>,
    mut look: ResMut<PlayerLook>,
    mut camera_q: Query<&mut Transform, With<PlayerCamera>>,
    player_q: Query<&Transform, (With<Player>, Without<PlayerCamera>)>,
) {
    // Мышь вращает камеру
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta != Vec2::ZERO {
        look.yaw -= delta.x * 0.0022;
        look.pitch = (look.pitch - delta.y * 0.0022).clamp(-1.5, 1.5);
    }

    // Камера следует за игроком и принимает нужный поворот
    let rot = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
    if let Ok(p_tf) = player_q.get_single() {
        if let Ok(mut c_tf) = camera_q.get_single_mut() {
            c_tf.translation = p_tf.translation + Vec3::new(0.0, 1.62, 0.0);
            c_tf.rotation = rot;
        }
    }
}