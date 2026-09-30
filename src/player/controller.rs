use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fly: bool,
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, player_movement);
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player { velocity: Vec3::ZERO, on_ground: false, fly: false },
        Transform::from_xyz(128.0, 30.0, 128.0),
        Visibility::default(),
        Name::new("Player"),
    ));
}

fn player_movement(
    mut q: Query<(&mut Transform, &mut Player)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut transform, mut player) in q.iter_mut() {
        let mut dir = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) { dir.z -= 1.0; }
        if keys.pressed(KeyCode::KeyS) { dir.z += 1.0; }
        if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
        if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
            transform.translation += dir * 4.6 * time.delta_secs();
        }

        player.velocity.y -= 28.0 * time.delta_secs();
        transform.translation.y += player.velocity.y * time.delta_secs();
        if transform.translation.y < 20.0 {
            transform.translation.y = 20.0;
            player.velocity.y = 0.0;
            player.on_ground = true;
        } else {
            player.on_ground = false;
        }
    }
}