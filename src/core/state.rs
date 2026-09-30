use bevy::prelude::*;

pub const SX: i32 = 256;
pub const SY: i32 = 48;
pub const SZ: i32 = 256;

#[allow(dead_code)]
pub const CHUNK_SIZE: i32 = 16;

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    InGame,
}

#[derive(Resource)]
pub struct WorldSeed(pub u32);

#[derive(Resource, Default)]
pub struct PlayerLook {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<PlayerLook>()
            .insert_resource(WorldSeed(rand::random()))
            .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)));
        info!("Core GameStatePlugin loaded.");
    }
}