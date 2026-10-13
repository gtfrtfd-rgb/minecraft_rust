use bevy::prelude::*;
use bevy::app::AppExit;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::world::chunk::{WorldData, BlockType};
use crate::player::controller::Player;
use crate::core::state::{WorldSeed, PlayerLook, SY};

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
    pub seed: u32,
    pub saved_at: u64,
    pub player: PlayerMeta,
    /// Все правки игрока: сломанные и поставленные блоки.
    pub edits: Vec<BlockEdit>,
}

#[derive(Resource, Default)]
pub struct LoadedSave {
    pub meta: Option<SaveMeta>,
}

fn saves_dir() -> PathBuf {
    let mut p = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    p.push("saves");
    p
}

fn meta_path() -> PathBuf { saves_dir().join("meta.json") }

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedSave>()
           .add_systems(PreStartup, load_save_data)
           .add_systems(PostStartup, apply_loaded_edits)
           .add_systems(Update, auto_save)
           .add_systems(Last, save_on_exit);
        info!("PersistencePlugin loaded.");
    }
}

fn load_save_data(mut loaded: ResMut<LoadedSave>) {
    let mpath = meta_path();

    if !mpath.exists() {
        info!("No save file found — new world will be generated");
        return;
    }

    let meta_str = match fs::read_to_string(&mpath) {
        Ok(s) => s,
        Err(e) => { warn!("read meta.json: {}", e); return; }
    };
    let meta: SaveMeta = match serde_json::from_str(&meta_str) {
        Ok(m) => m,
        Err(e) => { warn!("parse meta.json: {}", e); return; }
    };

    info!(
        "Loaded save: seed={}, edits={}, game_version={}",
        meta.seed, meta.edits.len(), meta.game_version
    );

    loaded.meta = Some(meta);
}

/// Применяет загруженные правки в WorldData.edits.
/// Сами чанки будут сгенерированы лениво, и при генерации эти
/// правки наложатся автоматически (см. `WorldData::insert_chunk`).
fn apply_loaded_edits(
    loaded: Res<LoadedSave>,
    mut world: ResMut<WorldData>,
    mut seed: ResMut<WorldSeed>,
) {
    let Some(meta) = &loaded.meta else { return; };

    seed.0 = meta.seed;

    use crate::core::state::CHUNK_SIZE;

    for edit in &meta.edits {
        if edit.y < 0 || edit.y >= SY { continue; }
        let cx = edit.x.div_euclid(CHUNK_SIZE);
        let cz = edit.z.div_euclid(CHUNK_SIZE);
        let key = IVec2::new(cx, cz);
        world
            .edits
            .entry(key)
            .or_default()
            .insert(IVec3::new(edit.x, edit.y, edit.z), BlockType::from_u8(edit.block));
    }

    info!("Applied {} block edits from save", meta.edits.len());
}

fn auto_save(
    time: Res<Time>,
    mut timer: Local<f32>,
    world: Res<WorldData>,
    seed: Res<WorldSeed>,
    look: Res<PlayerLook>,
    player_q: Query<(&Transform, &Player)>,
) {
    *timer += time.delta_secs();
    if *timer < 30.0 { return; }
    *timer = 0.0;

    save_game(&world, &seed, &look, &player_q);
}

fn save_on_exit(
    mut exit: MessageReader<AppExit>,
    world: Res<WorldData>,
    seed: Res<WorldSeed>,
    look: Res<PlayerLook>,
    player_q: Query<(&Transform, &Player)>,
) {
    let mut any = false;
    for _ in exit.read() { any = true; }
    if !any { return; }

    info!("Saving world before exit...");
    save_game(&world, &seed, &look, &player_q);
}

fn save_game(
    world: &WorldData,
    seed: &WorldSeed,
    look: &PlayerLook,
    player_q: &Query<(&Transform, &Player)>,
) {
    let Ok((tf, player)) = player_q.single() else {
        warn!("save_game: player not found");
        return;
    };

    let dir = saves_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        error!("Cannot create saves dir {:?}: {}", dir, e);
        return;
    }

    // Собираем все правки из всех чанков в плоский список.
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
        format_version: 2, // поменяли формат — теперь только правки
        game_version: env!("CARGO_PKG_VERSION").into(),
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

    match serde_json::to_string_pretty(&meta) {
        Ok(s) => {
            if let Err(e) = fs::write(meta_path(), s) {
                error!("Cannot write meta.json: {}", e);
            } else {
                info!("Saved: {} block edits", meta.edits.len());
            }
        }
        Err(e) => error!("Cannot serialize meta: {}", e),
    }
}