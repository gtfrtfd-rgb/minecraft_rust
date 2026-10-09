use bevy::prelude::*;
use bevy::app::AppExit;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::world::chunk::{WorldData, BlockType};
use crate::player::controller::Player;
use crate::core::state::{WorldSeed, PlayerLook};

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
pub struct SaveMeta {
    pub format_version: u32,
    pub game_version: String,
    pub seed: u32,
    pub saved_at: u64,
    pub player: PlayerMeta,
}

#[derive(Resource, Default)]
pub struct LoadedSave {
    pub meta: Option<SaveMeta>,
    pub rle_bytes: Option<Vec<u8>>,
}

/// Возвращает путь к папке сохранений.
///
/// На Android `current_exe()` возвращает `/system/bin/...`, куда писать нельзя,
/// поэтому используем приватную директорию приложения (обычно
/// `/data/user/0/<package>/files`), путь к которой передаётся через `HOME`.
fn saves_dir() -> PathBuf {
    if cfg!(target_os = "android") {
        if let Ok(home) = std::env::var("HOME") {
            let mut p = PathBuf::from(home);
            p.push("saves");
            return p;
        }
        // Запасной вариант, если HOME почему-то не установлена.
        PathBuf::from("/data/data/com.example.minecraft_rust/files/saves")
    } else {
        let mut p = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        p.push("saves");
        p
    }
}

fn meta_path() -> PathBuf { saves_dir().join("meta.json") }
fn world_path() -> PathBuf { saves_dir().join("world.bin") }

fn rle_encode(blocks: &[BlockType]) -> Vec<u8> {
    let mut out = Vec::with_capacity(blocks.len() / 8);
    let mut i = 0;
    let n = blocks.len();
    while i < n {
        let v = blocks[i].to_u8();
        let mut run = 1usize;
        while i + run < n
            && blocks[i + run].to_u8() == v
            && run < 65535
        {
            run += 1;
        }
        out.push(v);
        out.push((run & 0xFF) as u8);
        out.push(((run >> 8) & 0xFF) as u8);
        i += run;
    }
    out
}

pub fn rle_decode(data: &[u8], total_len: usize) -> Vec<BlockType> {
    let mut out = Vec::with_capacity(total_len);
    let mut i = 0;
    while i + 2 < data.len() && out.len() < total_len {
        let v = data[i];
        let run = (data[i + 1] as usize) | ((data[i + 2] as usize) << 8);
        let b = BlockType::from_u8(v);
        for _ in 0..run {
            if out.len() >= total_len { break; }
            out.push(b);
        }
        i += 3;
    }
    while out.len() < total_len {
        out.push(BlockType::Air);
    }
    out
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedSave>()
           .add_systems(PreStartup, load_save_data)
           .add_systems(Update, auto_save)
           .add_systems(Last, save_on_exit);
        info!("PersistencePlugin loaded.");
    }
}

fn load_save_data(mut loaded: ResMut<LoadedSave>) {
    let mpath = meta_path();
    let wpath = world_path();

    if !mpath.exists() || !wpath.exists() {
        info!("No save files found — new world will be generated");
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
    let bytes = match fs::read(&wpath) {
        Ok(b) => b,
        Err(e) => { warn!("read world.bin: {}", e); return; }
    };

    info!(
        "Loaded save: seed={}, {} RLE bytes, game_version={}, saved_at={}",
        meta.seed, bytes.len(), meta.game_version, meta.saved_at
    );

    loaded.meta = Some(meta);
    loaded.rle_bytes = Some(bytes);
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

    let t0 = std::time::Instant::now();
    let rle = rle_encode(&world.blocks);
    let rle_time = t0.elapsed();

    if let Err(e) = fs::write(world_path(), &rle) {
        error!("Cannot write world.bin: {}", e);
        return;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let meta = SaveMeta {
        format_version: 1,
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
    };

    match serde_json::to_string_pretty(&meta) {
        Ok(s) => {
            if let Err(e) = fs::write(meta_path(), s) {
                error!("Cannot write meta.json: {}", e);
            } else {
                info!(
                    "Saved: {} blocks → {} RLE bytes in {:?}",
                    world.blocks.len(),
                    rle.len(),
                    rle_time
                );
            }
        }
        Err(e) => error!("Cannot serialize meta: {}", e),
    }
}