use bevy::prelude::*;
use crate::core::state::AppState;

pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, raycast_system.run_if(in_state(AppState::InGame)));
    }
}

fn raycast_system(
    camera_query: Query<&GlobalTransform, With<crate::player::camera::PlayerCamera>>,
    // ... запрос к чанкам для проверки блоков
) {
    // Здесь будет реализация DDA-рейкаста
    // Пока что заглушка
}