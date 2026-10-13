use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::window::{PrimaryWindow, Window, WindowResolution, PresentMode};
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
// СПИСКИ ВАРИАНТОВ ДЛЯ НАСТРОЕК
// ============================================================

/// Варианты ограничения FPS. 0 = Unlimited.
pub const FPS_OPTIONS: [u32; 9] = [30, 60, 90, 120, 144, 165, 180, 240, 0];

/// Варианты разрешения окна (все 16:9).
pub const RESOLUTION_OPTIONS: [(u32, u32); 5] = [
    (1280, 720),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];

// ============================================================
// НАСТРОЙКИ
// ============================================================
#[derive(Resource, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub master_volume: f32,
    pub mouse_sensitivity: f32,
    #[serde(alias = "render_distance")]
    pub render_distance_chunks: i32,
    pub fps_limit_idx: usize,
    pub resolution_idx: usize,
    /// VSync: включён / выключен.
    pub vsync: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            mouse_sensitivity: 0.0022,
            render_distance_chunks: 12,
            fps_limit_idx: 1,       // 60
            resolution_idx: 0,      // 1920×1080
            vsync: true,
        }
    }
}

impl Settings {
    pub fn fps_limit(&self) -> u32 {
        FPS_OPTIONS[self.fps_limit_idx.min(FPS_OPTIONS.len() - 1)]
    }
    pub fn resolution(&self) -> (u32, u32) {
        RESOLUTION_OPTIONS[self.resolution_idx.min(RESOLUTION_OPTIONS.len() - 1)]
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
            .add_systems(Update, handle_slot_input.run_if(in_state(AppState::InGame)))
            .add_systems(Last, (
                apply_resolution,
                apply_vsync,
                apply_fps_limit,
            ));
        info!("Core GameStatePlugin loaded.");
    }
}

/// Реактивно меняет разрешение окна.
fn apply_resolution(
    settings: Res<Settings>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }
    let (w, h) = settings.resolution();
    for mut win in window_q.iter_mut() {
        let current_w = win.resolution.physical_width();
        let current_h = win.resolution.physical_height();
        if current_w == w && current_h == h {
            continue;
        }
        win.resolution = WindowResolution::new(w, h);
        info!("Window resolution set to {}x{}", w, h);
    }
}

/// Реактивно включает / выключает VSync.
fn apply_vsync(
    settings: Res<Settings>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }
    let desired = if settings.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };

    for mut win in window_q.iter_mut() {
        if win.present_mode == desired {
            continue;
        }
        win.present_mode = desired;
        info!("VSync: {}", settings.vsync);
    }
}

/// Ограничение FPS через sleep в конце кадра.
/// 0 = без ограничения.
/// Работает независимо от VSync, но если VSync уже ограничивает,
/// sleep будет коротким или нулевым.
fn apply_fps_limit(
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let limit = settings.fps_limit();
    if limit == 0 {
        return;
    }
    // Если VSync включён и целевой лимит выше частоты монитора —
    // пусть VSync сам решает. Sleep всё равно сработает, если
    // кадр получился быстрее целевого.
    let target_dt = 1.0 / limit as f32;
    let frame_dt = time.delta_secs();
    if frame_dt < target_dt {
        let sleep_for = target_dt - frame_dt;
        std::thread::sleep(std::time::Duration::from_secs_f32(sleep_for));
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