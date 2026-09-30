use bevy::prelude::*;
use crate::core::state::AppState;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement_system.run_if(in_state(AppState::InGame)));
    }
}

fn player_movement_system(
    mut query: Query<(&mut Transform, &mut Player)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    // Упрощённая логика: WASD двигает игрока, гравитация тянет вниз
    for (mut transform, mut player) in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) { direction.z -= 1.0; }
        if keys.pressed(KeyCode::KeyS) { direction.z += 1.0; }
        if keys.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if keys.pressed(KeyCode::KeyD) { direction.x += 1.0; }

        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
            transform.translation += direction * 4.5 * time.delta_secs();
        }

        // Гравитация и коллизии (проверка на землю — упрощена)
        player.velocity.y -= 28.0 * time.delta_secs();
        transform.translation.y += player.velocity.y * time.delta_secs();
        if transform.translation.y < 10.0 {
            transform.translation.y = 10.0;
            player.velocity.y = 0.0;
            player.on_ground = true;
        }
    }
}