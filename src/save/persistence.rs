use bevy::prelude::*;
use bevy::app::AppExit;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::world::chunk::{WorldData, BlockType, ChunkMesh, DirtyChunks};
use crate::player::controller::Player;
use crate::core::state::{
    WorldSeed, PlayerLook, SY, CHUNK_SIZE, AppState,
    data_dir, worlds_dir, PendingLoadWorld, ActiveWorld,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerMeta {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fly: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BlockEdit {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveMeta {
    pub format_version: u32,
    pub game_version: String,
    /// Имя мира. Опционально — старые сейвы могут его не иметь.
    #[serde(default)]
    pub world_name: String,
    pub seed: u32,
    pub saved_at: u64,
    pub player: PlayerMeta,
    pub edits: Vec<BlockEdit>,
}

#[derive(Resource, Default)]
pub struct LoadedSave {
    pub meta: Option<SaveMeta>,
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedSave>()
            .add_systems(PreStartup, migrate_legacy_save_if_needed)
            .add_systems(OnEnter(AppState::InGame), on_enter_ingame)
            .add_systems(OnExit(AppState::InGame), on_exit_ingame)
            .add_systems(
                Update,
                auto_save.run_if(in_state(AppState::InGame)),
            )
            .add_systems(Last, save_on_app_exit);
        info!("PersistencePlugin loaded.");
    }
}

/// Мигрирует старый одиночный сейв (рядом с exe или в data_dir/saves)
/// в worlds/Default.json.
fn migrate_legacy_save_if_needed() {
    let worlds = worlds_dir();

    // Если worlds/ уже содержит хоть один файл — миграция не нужна.
    if worlds.exists() {
        if let Ok(mut it) = fs::read_dir(&worlds) {
            if it.next().is_some() { return; }
        }
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let candidates = [
        data_dir().join("saves").join("meta.json"),
        exe_dir.join("saves").join("meta.json"),
    ];

    for legacy in &candidates {
        if !legacy.exists() { continue; }
        let Ok(s) = fs::read_to_string(legacy) else { continue; };
        let Ok(mut meta) = serde_json::from_str::<SaveMeta>(&s) else { continue; };

        if meta.world_name.is_empty() {
            meta.world_name = "Default".into();
        }

        if fs::create_dir_all(&worlds).is_err() { return; }
        let target = worlds.join("Default.json");
        let Ok(out) = serde_json::to_string_pretty(&meta) else { return; };

        if fs::write(&target, out).is_ok() {
            info!("Migrated legacy save {:?} -> {:?}", legacy, target);
        }
        return;
    }
}

/// Загружает мир, когда игрок заходит в InGame.
fn on_enter_ingame(
    mut pending: ResMut<PendingLoadWorld>,
    mut loaded: ResMut<LoadedSave>,
    mut active: ResMut<ActiveWorld>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    mut seed: ResMut<WorldSeed>,
    mut look: ResMut<PlayerLook>,
    mut player_q: Query<(&mut Transform, &mut Player)>,
    chunk_q: Query<Entity, With<ChunkMesh>>,
    mut commands: Commands,
) {
    let Some(path) = pending.0.take() else { return; };

    // Чистим старый мир.
    for entity in chunk_q.iter() {
        commands.entity(entity).despawn();
    }
    world.chunks.clear();
    world.edits.clear();
    dirty.0.clear();

    // Читаем meta.
    let Ok(s) = fs::read_to_string(&path) else {
        warn!("Cannot read world file {:?}", path);
        return;
    };
    let meta = match serde_json::from_str::<SaveMeta>(&s) {
        Ok(m) => m,
        Err(e) => { warn!("Parse world file error: {}", e); return; }
    };

    // Сид.
    seed.0 = meta.seed;

    // Правки.
    for edit in &meta.edits {
        if edit.y < 0 || edit.y >= SY { continue; }
        let cx = edit.x.div_euclid(CHUNK_SIZE);
        let cz = edit.z.div_euclid(CHUNK_SIZE);
        let key = IVec2::new(cx, cz);
        world.edits.entry(key).or_default()
            .insert(IVec3::new(edit.x, edit.y, edit.z), BlockType::from_u8(edit.block));
    }

    // Игрок.
    if let Ok((mut tf, mut player)) = player_q.single_mut() {
        tf.translation = Vec3::new(meta.player.x, meta.player.y, meta.player.z);
        player.fly = meta.player.fly;
        player.velocity = Vec3::ZERO;
    }
    look.yaw = meta.player.yaw;
    look.pitch = meta.player.pitch;

    info!(
        "Loaded world '{}': seed={}, edits={}",
        if meta.world_name.is_empty() { "<unnamed>" } else { &meta.world_name },
        meta.seed,
        meta.edits.len()
    );

    active.0 = Some(path);
    loaded.meta = Some(meta);
}

/// Сохраняет мир при выходе из игры (пауза, меню, app exit).
fn on_exit_ingame(
    active: Res<ActiveWorld>,
    world: Res<WorldData>,
    seed: Res<WorldSeed>,
    look: Res<PlayerLook>,
    player_q: Query<(&Transform, &Player)>,
) {
    let Some(path) = active.0.clone() else { return; };
    if let Err(e) = save_game_to(path, &world, &seed, &look, &player_q) {
        warn!("Save on exit failed: {}", e);
    }
}

fn auto_save(
    time: Res<Time>,
    mut timer: Local<f32>,
    active: Res<ActiveWorld>,
    world: Res<WorldData>,
    seed: Res<WorldSeed>,
    look: Res<PlayerLook>,
    player_q: Query<(&Transform, &Player)>,
) {
    let Some(path) = active.0.clone() else { return; };
    *timer += time.delta_secs();
    if *timer < 30.0 { return; }
    *timer = 0.0;

    if let Err(e) = save_game_to(path, &world, &seed, &look, &player_q) {
        warn!("Autosave failed: {}", e);
    }
}

fn save_on_app_exit(
    mut exit: MessageReader<AppExit>,
    active: Res<ActiveWorld>,
    world: Res<WorldData>,
    seed: Res<WorldSeed>,
    look: Res<PlayerLook>,
    player_q: Query<(&Transform, &Player)>,
) {
    let mut any = false;
    for _ in exit.read() { any = true; }
    if !any { return; }

    let Some(path) = active.0.clone() else { return; };
    info!("Saving world before exit...");
    if let Err(e) = save_game_to(path, &world, &seed, &look, &player_q) {
        warn!("Save on exit failed: {}", e);
    }
}

fn save_game_to(
    path: PathBuf,
    world: &WorldData,
    seed: &WorldSeed,
    look: &PlayerLook,
    player_q: &Query<(&Transform, &Player)>,
) -> Result<(), String> {
    let Ok((tf, player)) = player_q.single() else {
        return Err("player not found".into());
    };

    // Имя мира — не перезаписываем, читаем старое.
    let world_name = fs::read_to_string(&path).ok()
        .and_then(|s| serde_json::from_str::<SaveMeta>(&s).ok())
        .map(|m| m.world_name)
        .unwrap_or_else(|| "Unnamed".into());

    let mut edits: Vec<BlockEdit> = Vec::new();
    for chunk_edits in world.edits.values() {
        for (&pos, &block) in chunk_edits {
            edits.push(BlockEdit {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                block: block.to_u8(),
            });
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let meta = SaveMeta {
        format_version: 2,
        game_version: env!("CARGO_PKG_VERSION").into(),
        world_name,
        seed: seed.0,
        saved_at: now,
        player: PlayerMeta {
            x: tf.translation.x,
            y: tf.translation.y,
            z: tf.translation.z,
            yaw: look.yaw,
            pitch: look.pitch,
            fly: player.fly,
        },
        edits,
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    info!("Saved {} block edits", meta.edits.len());
    Ok(())
}