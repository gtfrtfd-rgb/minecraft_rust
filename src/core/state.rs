use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::window::{
    PrimaryWindow, Window, WindowResolution, WindowMode, WindowPosition,
    PresentMode, Monitor, MonitorSelection,
};
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
// СПИСКИ ВАРИАНТОВ
// ============================================================

/// Варианты ограничения FPS.
pub const FPS_OPTIONS: [u32; 8] = [30, 60, 90, 120, 144, 165, 180, 240];

/// Варианты разрешения окна (все 16:9).
///
/// Если пользователь выбирает вариант, совпадающий с физическим
/// разрешением монитора, окно автоматически разворачивается
/// в borderless fullscreen. Более низкие варианты — обычное окно.
pub const RESOLUTION_OPTIONS: [(u32, u32); 5] = [
    (1280, 720),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];

/// Максимальный индекс в `RESOLUTION_OPTIONS`, который не превышает
/// физическое разрешение самого большого монитора. Вычисляется
/// один раз на старте.
#[derive(Resource)]
pub struct MaxResolutionIndex(pub usize);

impl Default for MaxResolutionIndex {
    fn default() -> Self {
        // Пока монитор неизвестен — разрешаем всё.
        Self(RESOLUTION_OPTIONS.len() - 1)
    }
}

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
    pub vsync: bool,
    /// Оставлено для обратной совместимости со старыми сейвами —
    /// больше не используется в UI, режим окна определяется
    /// автоматически по выбранному разрешению.
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            mouse_sensitivity: 0.0022,
            render_distance_chunks: 12,
            fps_limit_idx: 1,       // 60
            // По умолчанию — 720p в окне. Если у монитора физическое
            // разрешение тоже 720p — окно развернётся на весь экран.
            resolution_idx: 0,
            vsync: true,
            fullscreen: false,
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
            .init_resource::<MaxResolutionIndex>()
            .insert_resource(WorldSeed(rand::random()))
            .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
            .add_systems(Update, handle_slot_input.run_if(in_state(AppState::InGame)))
            .add_systems(Update, compute_max_resolution)
            .add_systems(Last, (
                apply_window,
                apply_vsync,
                apply_fps_limit,
            ));
        info!("Core GameStatePlugin loaded.");
    }
}

/// Один раз вычисляет максимальный индекс в `RESOLUTION_OPTIONS`,
/// который помещается на самый большой монитор. Использует
/// **физическое** разрешение без вычетов — специально, чтобы
/// пользователь мог выбрать нативное разрешение и получить
/// fullscreen.
fn compute_max_resolution(
    mut commands: Commands,
    mut done: Local<bool>,
    monitor_q: Query<&Monitor>,
    mut settings: ResMut<Settings>,
) {
    if *done { return; }

    let monitors: Vec<&Monitor> = monitor_q.iter().collect();
    if monitors.is_empty() { return; }

    let mut max_w: u32 = 0;
    let mut max_h: u32 = 0;
    for m in &monitors {
        let size = m.physical_size();
        if (size.x as u64) * (size.y as u64) > (max_w as u64) * (max_h as u64) {
            max_w = size.x;
            max_h = size.y;
        }
    }

    // Ищем самый большой вариант, который не превышает физическое.
    let mut max_idx = 0;
    for (i, &(w, h)) in RESOLUTION_OPTIONS.iter().enumerate() {
        if w <= max_w && h <= max_h {
            max_idx = i;
        }
    }

    // Если сохранённое разрешение больше физического — откатываем
    // до максимально доступного.
    if settings.resolution_idx > max_idx {
        settings.resolution_idx = max_idx;
        info!(
            "Resolution clamped to {}x{} (screen is {}x{})",
            RESOLUTION_OPTIONS[max_idx].0, RESOLUTION_OPTIONS[max_idx].1,
            max_w, max_h
        );
    }

    commands.insert_resource(MaxResolutionIndex(max_idx));
    *done = true;
    info!(
        "Max resolution index: {} ({}x{}), screen {}x{}",
        max_idx,
        RESOLUTION_OPTIONS[max_idx].0, RESOLUTION_OPTIONS[max_idx].1,
        max_w, max_h
    );
}

/// Применяет размер окна и режим. Одна система вместо двух.
///
/// Логика:
///   • Если выбранное разрешение == физическое (индекс = максимуму) —
///     окно разворачивается в borderless fullscreen. Разрешение
///     игнорируется, размер окна = размер монитора, ничего не
///     вылезает за границы.
///   • Иначе — обычное окно с указанным разрешением, центрировано.
fn apply_window(
    settings: Res<Settings>,
    max_res: Res<MaxResolutionIndex>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() && !max_res.is_changed() {
        return;
    }

    let want_fullscreen = settings.resolution_idx >= max_res.0;

    let desired_mode = if want_fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
    } else {
        WindowMode::Windowed
    };

    let (w, h) = settings.resolution();

    for mut win in window_q.iter_mut() {
        let mode_changed = win.mode != desired_mode;
        if mode_changed {
            win.mode = desired_mode;
        }

        // Разрешение применяем только в оконном режиме — иначе
        // оконный менеджер сам задаёт размер.
        if !want_fullscreen {
            let current_w = win.resolution.physical_width();
            let current_h = win.resolution.physical_height();
            if current_w != w || current_h != h {
                win.resolution = WindowResolution::new(w, h);
                info!("Window resolution set to {}x{}", w, h);
            }
            win.position = WindowPosition::Centered(MonitorSelection::Primary);
        }

        if mode_changed {
            if want_fullscreen {
                info!("Window mode: borderless fullscreen");
            } else {
                info!("Window mode: windowed");
            }
        }
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
fn apply_fps_limit(
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let limit = settings.fps_limit();
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