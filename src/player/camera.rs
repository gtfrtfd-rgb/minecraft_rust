use bevy::prelude::*;
use crate::core::state::AppState;

#[derive(Component)]
pub struct PlayerCamera;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
           .add_systems(Update, camera_look_system.run_if(in_state(AppState::InGame)));
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle::default(),
        PlayerCamera,
    ));
}

fn camera_look_system(
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
    mouse_motion: Res<bevy::input::mouse::MouseMotion>,
    mut yaw: Local<f32>,
    mut pitch: Local<f32>,
) {
    for mut transform in camera_query.iter_mut() {
        let (dx, dy) = mouse_motion.delta;
        *yaw -= dx * 0.0022;
        *pitch -= dy * 0.0022;
        *pitch = pitch.clamp(-1.5, 1.5);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, *yaw, *pitch, 0.0);
    }
}