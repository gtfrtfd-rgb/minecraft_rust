use bevy::prelude::*;

pub const SX: i32 = 256;
pub const SY: i32 = 48;
pub const SZ: i32 = 256;
pub const CHUNK_SIZE: i32 = 16;

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Menu,
    InGame,
}

#[derive(Resource)]
pub struct WorldSeed(pub u32);

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
           .insert_resource(WorldSeed(rand::random()));
        info!("Core GameStatePlugin loaded.");
    }
}