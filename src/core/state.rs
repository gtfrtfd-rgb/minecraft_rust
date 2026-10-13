use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use serde::{Serialize, Deserialize};
use crate::world::chunk::BlockType;

pub const SY: i32 = 48;
pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_AREA: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * SY as usize;

// ============================================================
// СОСТОЯНИЯ ИГРЫ
// ============================================================
#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Menu,
    Settings,
    InGame,
    Paused,
}

// ============================================================
// НАСТРОЙКИ
// ============================================================
#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub master_volume: f32,
    pub mouse_sensitivity: f32,
    /// Дальность прорисовки в ЧАНКАХ. 1 чанк = 16 блоков.
    /// Например: 8 = 128 блоков, 12 = 192, 16 = 256, 24 = 384.
    #[serde(alias = "render_distance")]
    pub render_distance_chunks: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            mouse_sensitivity: 0.0022,
            render_distance_chunks: 12,
        }
    }
}

#[derive(Resource)]
pub struct WorldSeed(pub u32);

#[derive(Resource, Default)]
pub struct PlayerLook {
    pub yaw: f32,
    pub pitch: f32,
}

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
            .init_resource::<Settings>()
            .insert_resource(WorldSeed(rand::random()))
            .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
            .add_systems(Update, handle_slot_input.run_if(in_state(AppState::InGame)));
        info!("Core GameStatePlugin loaded.");
    }
}

fn handle_slot_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
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
            selected.0 = (selected.0 + HOTBAR.len() - 1) % HOTBAR.len();
        } else if ev.y < 0.0 {
            selected.0 = (selected.0 + 1) % HOTBAR.len();
        }
    }
}