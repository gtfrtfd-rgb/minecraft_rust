use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

#[derive(Component)]
pub struct PlayerCamera;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, camera_look);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(128.0, 32.0, 128.0).looking_at(Vec3::new(0.0, 32.0, 0.0), Vec3::Y),
        PlayerCamera,
    ));
}

fn camera_look(
    mut motion: EventReader<MouseMotion>,
    mut camera_q: Query<&mut Transform, With<PlayerCamera>>,
    player_q: Query<&Transform, (With<super::controller::Player>, Without<PlayerCamera>)>,
    mut yaw: Local<f32>,
    mut pitch: Local<f32>,
) {
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO { return; }

    *yaw -= delta.x * 0.0022;
    *pitch = (*pitch - delta.y * 0.0022).clamp(-1.5, 1.5);

    let rot = Quat::from_euler(EulerRot::YXZ, *yaw, *pitch, 0.0);
    if let Ok(player) = player_q.get_single() {
        for mut cam_tf in camera_q.iter_mut() {
            cam_tf.translation = player.translation + Vec3::new(0.0, 1.62, 0.0);
            cam_tf.rotation = rot;
        }
    }
}