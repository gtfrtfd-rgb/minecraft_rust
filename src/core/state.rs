use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::window::{
    PrimaryWindow, Window, WindowResolution, WindowMode, WindowPosition,
    PresentMode, Monitor, MonitorSelection,
};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use crate::world::chunk::BlockType;

pub const SY: i32 = 48;
pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_AREA: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * SY as usize;

pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut p = PathBuf::from(appdata);
            p.push("minecraft_rust");
            return p;
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            let mut p = PathBuf::from(home);
            p.push("Library");
            p.push("Application Support");
            p.push("minecraft_rust");
            return p;
        }
    } else {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                let mut p = PathBuf::from(xdg);
                p.push("minecraft_rust");
                return p;
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let mut p = PathBuf::from(home);
            p.push(".local");
            p.push("share");
            p.push("minecraft_rust");
            return p;
        }
    }

    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn worlds_dir() -> PathBuf {
    let mut p = data_dir();
    p.push("worlds");
    p
}

// ============================================================
// СОСТОЯНИЯ ИГРЫ
// ============================================================
#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Menu,
    Worlds,
    Settings,
    Keybinds,
    InGame,
    Paused,
}

pub const FPS_OPTIONS: [u32; 8] = [30, 60, 90, 120, 144, 165, 180, 240];

pub const RESOLUTION_OPTIONS: [(u32, u32); 5] = [
    (1280, 720),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];

#[derive(Resource)]
pub struct MaxResolutionIndex(pub usize);

impl Default for MaxResolutionIndex {
    fn default() -> Self {
        Self(RESOLUTION_OPTIONS.len() - 1)
    }
}

// ============================================================
// ДЕЙСТВИЯ ДЛЯ ПЕРЕНАЗНАЧЕНИЯ
// ============================================================
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BindAction {
    Forward,
    Backward,
    Left,
    Right,
    Jump,
    Sprint,
    Sneak,
    Fly,
}

impl BindAction {
    pub const ALL: [BindAction; 8] = [
        BindAction::Forward,
        BindAction::Backward,
        BindAction::Left,
        BindAction::Right,
        BindAction::Jump,
        BindAction::Sprint,
        BindAction::Sneak,
        BindAction::Fly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BindAction::Forward  => "Move Forward",
            BindAction::Backward => "Move Backward",
            BindAction::Left     => "Move Left",
            BindAction::Right    => "Move Right",
            BindAction::Jump     => "Jump",
            BindAction::Sprint   => "Sprint",
            BindAction::Sneak    => "Sneak",
            BindAction::Fly      => "Toggle Fly",
        }
    }
}

pub fn parse_key(s: &str) -> KeyCode {
    use KeyCode::*;
    match s {
        "KeyW" => KeyW, "KeyA" => KeyA, "KeyS" => KeyS, "KeyD" => KeyD,
        "KeyQ" => KeyQ, "KeyE" => KeyE, "KeyR" => KeyR, "KeyF" => KeyF,
        "KeyZ" => KeyZ, "KeyX" => KeyX, "KeyC" => KeyC, "KeyV" => KeyV,
        "KeyB" => KeyB, "KeyG" => KeyG, "KeyH" => KeyH, "KeyJ" => KeyJ,
        "KeyK" => KeyK, "KeyL" => KeyL, "KeyM" => KeyM, "KeyN" => KeyN,
        "KeyO" => KeyO, "KeyP" => KeyP, "KeyT" => KeyT, "KeyU" => KeyU,
        "KeyI" => KeyI, "KeyY" => KeyY,
        "Space" => Space,
        "Tab" => Tab,
        "Enter" => Enter,
        "Escape" => Escape,
        "ShiftLeft" => ShiftLeft, "ShiftRight" => ShiftRight,
        "ControlLeft" => ControlLeft, "ControlRight" => ControlRight,
        "AltLeft" => AltLeft, "AltRight" => AltRight,
        "SuperLeft" => SuperLeft, "SuperRight" => SuperRight,
        "ArrowUp" => ArrowUp, "ArrowDown" => ArrowDown,
        "ArrowLeft" => ArrowLeft, "ArrowRight" => ArrowRight,
        "Digit0" => Digit0, "Digit1" => Digit1, "Digit2" => Digit2,
        "Digit3" => Digit3, "Digit4" => Digit4, "Digit5" => Digit5,
        "Digit6" => Digit6, "Digit7" => Digit7, "Digit8" => Digit8,
        "Digit9" => Digit9,
        _ => KeyW,
    }
}

pub fn key_to_string(k: KeyCode) -> String {
    format!("{:?}", k)
}

// ============================================================
// ИНФОРМАЦИЯ О МИРЕ
// ============================================================
#[derive(Clone)]
pub struct WorldInfo {
    pub name: String,
    pub path: PathBuf,
    pub seed: u32,
    pub saved_at: u64,
    pub edits: usize,
}

#[derive(Resource, Default)]
pub struct WorldsList(pub Vec<WorldInfo>);

#[derive(Resource, Default)]
pub struct SelectedWorldIdx(pub usize);

#[derive(Resource, Default)]
pub struct PendingLoadWorld(pub Option<PathBuf>);

#[derive(Resource, Default)]
pub struct ActiveWorld(pub Option<PathBuf>);

/// Активное переименование мира. Если `Some`, в меню миров
/// открыт инлайн-редактор имени, и символы с клавиатуры идут в черновик.
#[derive(Resource, Default)]
pub struct RenamingWorld(pub Option<RenameState>);

#[derive(Clone)]
pub struct RenameState {
    /// Путь к файлу мира, который переименовываем.
    pub path: PathBuf,
    /// Текущий черновик имени.
    pub draft: String,
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

    pub key_forward: String,
    pub key_backward: String,
    pub key_left: String,
    pub key_right: String,
    pub key_jump: String,
    pub key_sprint: String,
    pub key_sneak: String,
    pub key_fly: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            mouse_sensitivity: 0.0022,
            render_distance_chunks: 12,
            fps_limit_idx: 1,
            resolution_idx: 0,
            vsync: true,
            key_forward:  "KeyW".into(),
            key_backward: "KeyS".into(),
            key_left:     "KeyA".into(),
            key_right:    "KeyD".into(),
            key_jump:     "Space".into(),
            key_sprint:   "ControlLeft".into(),
            key_sneak:    "ShiftLeft".into(),
            key_fly:      "KeyF".into(),
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
    pub fn key_for(&self, action: BindAction) -> KeyCode {
        parse_key(self.key_string_for(action))
    }
    pub fn key_string_for(&self, action: BindAction) -> &str {
        match action {
            BindAction::Forward  => &self.key_forward,
            BindAction::Backward => &self.key_backward,
            BindAction::Left     => &self.key_left,
            BindAction::Right    => &self.key_right,
            BindAction::Jump     => &self.key_jump,
            BindAction::Sprint   => &self.key_sprint,
            BindAction::Sneak    => &self.key_sneak,
            BindAction::Fly      => &self.key_fly,
        }
    }
    pub fn set_key_string(&mut self, action: BindAction, s: String) {
        match action {
            BindAction::Forward  => self.key_forward = s,
            BindAction::Backward => self.key_backward = s,
            BindAction::Left     => self.key_left = s,
            BindAction::Right    => self.key_right = s,
            BindAction::Jump     => self.key_jump = s,
            BindAction::Sprint   => self.key_sprint = s,
            BindAction::Sneak    => self.key_sneak = s,
            BindAction::Fly      => self.key_fly = s,
        }
    }
}

#[derive(Resource)]
pub struct KeyBinds {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub sprint: KeyCode,
    pub sneak: KeyCode,
    pub fly: KeyCode,
}

impl KeyBinds {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            forward:  s.key_for(BindAction::Forward),
            backward: s.key_for(BindAction::Backward),
            left:     s.key_for(BindAction::Left),
            right:    s.key_for(BindAction::Right),
            jump:     s.key_for(BindAction::Jump),
            sprint:   s.key_for(BindAction::Sprint),
            sneak:    s.key_for(BindAction::Sneak),
            fly:      s.key_for(BindAction::Fly),
        }
    }
}

#[derive(Resource, Default)]
pub struct RebindingState {
    pub action: Option<BindAction>,
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
            .init_resource::<RebindingState>()
            .init_resource::<WorldsList>()
            .init_resource::<SelectedWorldIdx>()
            .init_resource::<PendingLoadWorld>()
            .init_resource::<ActiveWorld>()
            .init_resource::<RenamingWorld>()
            .insert_resource(WorldSeed(rand::random()))
            .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
            .add_systems(Startup, init_keybinds)
            .add_systems(Update, handle_slot_input.run_if(in_state(AppState::InGame)))
            .add_systems(Update, compute_max_resolution)
            .add_systems(Update, update_keybinds)
            .add_systems(Last, (
                apply_window,
                apply_vsync,
                apply_fps_limit,
            ));
        info!("Core GameStatePlugin loaded.");
    }
}

fn init_keybinds(
    mut commands: Commands,
    settings: Res<Settings>,
) {
    let binds = KeyBinds::from_settings(&settings);
    commands.insert_resource(binds);
}

fn update_keybinds(
    settings: Res<Settings>,
    mut binds: ResMut<KeyBinds>,
) {
    if !settings.is_changed() { return; }
    *binds = KeyBinds::from_settings(&settings);
}

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

    let mut max_idx = 0;
    for (i, &(w, h)) in RESOLUTION_OPTIONS.iter().enumerate() {
        if w <= max_w && h <= max_h {
            max_idx = i;
        }
    }

    if settings.resolution_idx > max_idx {
        settings.resolution_idx = max_idx;
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

fn apply_vsync(
    settings: Res<Settings>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() { return; }
    let desired = if settings.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };

    for mut win in window_q.iter_mut() {
        if win.present_mode == desired { continue; }
        win.present_mode = desired;
        info!("VSync: {}", settings.vsync);
    }
}

fn apply_fps_limit(
    settings: Res<Settings>,
    time: Res<Time>,
) {
    if settings.vsync {
        return;
    }

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