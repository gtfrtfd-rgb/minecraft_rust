use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::world::chunk::BlockType;

pub const SX: i32 = 256;
pub const SY: i32 = 48;
pub const SZ: i32 = 256;

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

/// 9 блоков в хотбаре
pub const HOTBAR: [BlockType; 9] = [
    BlockType::Grass,
    BlockType::Dirt,
    BlockType::Stone,
    BlockType::Cobblestone,
    BlockType::Sand,
    BlockType::Log,
    BlockType::Leaves,
    BlockType::Planks,
    BlockType::Brick,
];

#[derive(Resource, Default, Clone, Copy)]
pub struct SelectedSlot(pub usize);

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<PlayerLook>()
            .init_resource::<SelectedSlot>()
            .insert_resource(WorldSeed(rand::random()))
            .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
            .add_systems(Update, handle_slot_input);
        info!("Core GameStatePlugin loaded.");
    }
}

fn handle_slot_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: EventReader<MouseWheel>,
    mut selected: ResMut<SelectedSlot>,
) {
    use KeyCode::*;
    let key_map = [Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9];
    for (i, &key) in key_map.iter().enumerate() {
        if keys.just_pressed(key) {
            selected.0 = i;
        }
    }

    for ev in wheel.read() {
        if ev.y > 0.0 {
            // Колесо вверх — назад по слотам
            selected.0 = (selected.0 + HOTBAR.len() - 1) % HOTBAR.len();
        } else if ev.y < 0.0 {
            // Колесо вниз — вперёд
            selected.0 = (selected.0 + 1) % HOTBAR.len();
        }
    }
}