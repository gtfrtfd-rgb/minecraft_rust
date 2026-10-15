use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::state::{WorldInfo, worlds_dir};
use super::persistence::{SaveMeta, PlayerMeta};

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Сканирует папку `worlds/` и возвращает список миров,
/// отсортированный по времени последнего сохранения (свежие — сверху).
pub fn scan_worlds() -> Vec<WorldInfo> {
    let dir = worlds_dir();
    let Ok(entries) = fs::read_dir(&dir) else { return Vec::new(); };
    let mut out: Vec<WorldInfo> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
        let Ok(s) = fs::read_to_string(&path) else { continue; };
        let Ok(meta) = serde_json::from_str::<SaveMeta>(&s) else { continue; };
        let name = if meta.world_name.is_empty() {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("World")
                .to_string()
        } else {
            meta.world_name.clone()
        };
        out.push(WorldInfo {
            name,
            path: path.clone(),
            seed: meta.seed,
            saved_at: meta.saved_at,
            edits: meta.edits.len(),
        });
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.saved_at));
    out
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
        } else if c == ' ' {
            out.push('_');
        }
    }
    if out.is_empty() { out.push_str("world"); }
    out
}

pub fn create_world(name: &str) -> std::io::Result<PathBuf> {
    let dir = worlds_dir();
    fs::create_dir_all(&dir)?;

    let slug = slugify(name);
    let mut filename = format!("{}.json", slug);
    let mut path = dir.join(&filename);
    let mut n = 2;
    while path.exists() {
        filename = format!("{}_{}.json", slug, n);
        path = dir.join(&filename);
        n += 1;
    }

    let meta = SaveMeta {
        format_version: 2,
        game_version: env!("CARGO_PKG_VERSION").into(),
        world_name: name.to_string(),
        seed: rand::random(),
        saved_at: now_secs(),
        player: PlayerMeta {
            x: 0.5, y: 40.0, z: 0.5,
            yaw: 0.0, pitch: 0.0, fly: false,
        },
        edits: Vec::new(),
    };

    let json = serde_json::to_string_pretty(&meta)
        .map_err(std::io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

pub fn delete_world(path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)
}

/// Переименовывает мир: читает JSON, меняет поле `world_name`,
/// пишет обратно. Имя файла не меняется — оно не видно пользователю.
pub fn rename_world(path: &Path, new_name: &str) -> std::io::Result<()> {
    let s = fs::read_to_string(path)?;
    let mut meta: SaveMeta = serde_json::from_str(&s)
        .map_err(std::io::Error::other)?;
    meta.world_name = new_name.to_string();
    let json = serde_json::to_string_pretty(&meta)
        .map_err(std::io::Error::other)?;
    fs::write(path, json)?;
    Ok(())
}

/// Генерирует свободное имя вида "New World", "New World 2", ...
pub fn generate_world_name() -> String {
    let dir = worlds_dir();
    if !dir.exists() { return "New World".into(); }

    let existing: Vec<String> = fs::read_dir(&dir)
        .ok()
        .map(|it| it.flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) != Some("json") {
                    return None;
                }
                fs::read_to_string(&p).ok()
                    .and_then(|s| serde_json::from_str::<SaveMeta>(&s).ok())
                    .map(|m| m.world_name)
            })
            .collect())
        .unwrap_or_default();

    if !existing.iter().any(|n| n == "New World") {
        return "New World".into();
    }
    for i in 2..1000 {
        let candidate = format!("New World {}", i);
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    "New World".into()
}