use bevy::prelude::*;
use bevy::app::AppExit;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::world::chunk::{
    WorldData, BlockType, ChunkMesh, DirtyChunks,
    PendingChunks, PendingMeshes,
};
use crate::player::controller::Player;
use crate::core::state::{
    WorldSeed, PlayerLook, SY, CHUNK_SIZE, AppState,
    data_dir, worlds_dir, PendingLoadWorld, ActiveWorld,
    
};
use crate::mobs::ai::Mob;

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

/// Мигрирует старый одиночный сейв в worlds/Default.json.
fn migrate_legacy_save_if_needed() {
    let worlds = worlds_dir();

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

/// Загружает мир при входе в InGame.
///
/// КРИТИЧНО: файл читается и парсится ПЕРВЫМ. Если он битый или
/// отсутствует — мы ничего не стираем и не трогаем `active.0`,
/// а возвращаемся в меню. Иначе бы стёрли текущий мир и оставили
/// `active.0` указывающим на ПРОШЛЫЙ мир — автосейв перезаписал
/// бы чужой файл пустотой.
fn on_enter_ingame(
    mut pending_load: ResMut<PendingLoadWorld>,
    mut loaded: ResMut<LoadedSave>,
    mut active: ResMut<ActiveWorld>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    mut pending_chunks: ResMut<PendingChunks>,
    mut pending_meshes: ResMut<PendingMeshes>,
    mut seed: ResMut<WorldSeed>,
    mut look: ResMut<PlayerLook>,
    mut player_q: Query<(&mut Transform, &mut Player)>,
    chunk_q: Query<Entity, With<ChunkMesh>>,
    mob_q: Query<Entity, With<Mob>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(path) = pending_load.0.take() else { return; };

    // ── Шаг 1. Читаем файл. Ничего не трогаем, пока не убедимся,
    //           что данные валидны. ──
    let meta = match fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str::<SaveMeta>(&s) {
            Ok(m) => m,
            Err(e) => {
                error!(
                    "Failed to parse world file {:?}: {}. \
                     Returning to menu, previous world untouched.",
                    path, e
                );
                next_state.set(AppState::Menu);
                return;
            }
        },
        Err(e) => {
            error!(
                "Failed to read world file {:?}: {}. \
                 Returning to menu, previous world untouched.",
                path, e
            );
            next_state.set(AppState::Menu);
            return;
        }
    };

    // ── Шаг 2. Файл валиден. Стираем старый мир и всех мобов. ──
    for entity in chunk_q.iter() {
        commands.entity(entity).despawn();
    }
    for entity in mob_q.iter() {
        commands.entity(entity).despawn();
    }
    world.chunks.clear();
    world.edits.clear();
    dirty.0.clear();
    pending_chunks.tasks.clear();
    pending_meshes.tasks.clear();

    // ── Шаг 3. Применяем данные из meta. ──
    seed.0 = meta.seed;

    for edit in &meta.edits {
        if edit.y < 0 || edit.y >= SY { continue; }
        let cx = edit.x.div_euclid(CHUNK_SIZE);
        let cz = edit.z.div_euclid(CHUNK_SIZE);
        let key = IVec2::new(cx, cz);
        world.edits.entry(key).or_default()
            .insert(IVec3::new(edit.x, edit.y, edit.z), BlockType::from_u8(edit.block));
    }

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

    // ── Шаг 4. Только ТЕПЕРЬ помечаем мир активным. ──
    active.0 = Some(path);
    loaded.meta = Some(meta);
}

/// Сохраняет мир при выходе из игры.
fn on_exit_ingame(
    active: Res<ActiveWorld>,
    world: Res<WorldData>,
    seed: Res<WorldSeed>,
    look: Res<PlayerLook>,
    player_q: Query<(&Transform, &Player)>,
    mut pending_chunks: ResMut<PendingChunks>,
    mut pending_meshes: ResMut<PendingMeshes>,
) {
    let Some(path) = active.0.clone() else { return; };
    match save_game_to(path, &world, &seed, &look, &player_q) {
        Ok(()) => {
            pending_chunks.tasks.clear();
            pending_meshes.tasks.clear();
        }
        Err(e) => {
            warn!(
                "Save on exit failed: {}. \
                 Original file on disk is intact (atomic write).",
                e
            );
            // Активные pending-задачи не трогаем — если игрок опять
            // зайдёт в мир, будет повторная попытка сохранить.
        }
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
        warn!("Autosave failed: {}. Disk intact.", e);
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
        warn!("Save on app exit failed: {}. Disk intact.", e);
    }
}

/// Атомарное сохранение мира.
///
/// Записываем в `.json.tmp`, потом rename поверх основного файла.
/// Rename атомарен на Linux/macOS (POSIX) и Windows
/// (`MoveFileExW` с `MOVEFILE_REPLACE_EXISTING`). Если запись в
/// tmp упадёт — оригинал не тронут. Плюс сохраняем `.json.bak`
/// предыдущей версии — на случай, если пользователь захочет
/// вручную восстановить.
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

    // Имя мира — не перезаписываем, читаем старое (если файл ещё цел).
    let world_name = fs::read_to_string(&path).ok()
        .and_then(|s| serde_json::from_str::<SaveMeta>(&s).ok())
        .map(|m| m.world_name)
        .unwrap_or_else(|| "Unnamed".into());

    let mut edits: Vec<BlockEdit> = Vec::new();
    for chunk_edits in world.edits.values() {
        for (&pos, &block) in chunk_edits {
            edits.push(BlockEdit {
                x: pos.x, y: pos.y, z: pos.z, block: block.to_u8(),
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
        fs::create_dir_all(parent)
            .map_err(|e| format!("create dir: {}", e))?;
    }

    let json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("serialize: {}", e))?;

    // 1) Пишем в tmp-файл. Если упадёт — оригинал цел.
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json)
        .map_err(|e| format!("write tmp: {}", e))?;

    // 2) Делаем .bak (best-effort) — на случай ручного восстановления.
    if path.exists() {
        let bak_path = path.with_extension("json.bak");
        let _ = fs::copy(&path, &bak_path);
    }

    // 3) Атомарный rename. Если упадёт — чистим tmp.
    if let Err(e) = fs::rename(&tmp_path, &path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("rename: {}", e));
    }

    info!("Saved world: {} edits → {:?}", meta.edits.len(), path);
    Ok(())
}