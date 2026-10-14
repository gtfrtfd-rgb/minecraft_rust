use bevy::prelude::*;

use crate::core::state::{AppState, SY, CHUNK_SIZE};
use crate::player::controller::Player;
use super::chunk::{WorldData, DirtyChunks, BlockType, ChunkMesh};

pub struct GrassSpreadPlugin;

impl Plugin for GrassSpreadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            grass_spread_system.run_if(in_state(AppState::InGame)),
        );
        info!("GrassSpreadPlugin loaded.");
    }
}

/// Как часто пытаемся распространить траву.
const SPREAD_INTERVAL: f32 = 0.4;

/// Сколько попыток распространения за один тик.
const ATTEMPTS_PER_TICK: usize = 12;

/// Радиус чанков вокруг игрока, в которых идёт рост.
const SPREAD_CHUNK_RADIUS: i32 = 3;

/// Проверка: с какого расстояния от игрока трава ещё растёт.
const PLAYER_ACTIVE_RADIUS: f32 = 96.0;

fn grass_spread_system(
    time: Res<Time>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    player_q: Query<&Transform, With<Player>>,
    chunk_q: Query<&ChunkMesh>,
    mut timer: Local<f32>,
    mut rng_state: Local<u32>,
) {
    let Ok(pt) = player_q.single() else { return; };
    let player_pos = pt.translation;

    *timer += time.delta_secs();
    if *timer < SPREAD_INTERVAL { return; }
    *timer = 0.0;

    let pcx = (player_pos.x.floor() as i32).div_euclid(CHUNK_SIZE);
    let pcz = (player_pos.z.floor() as i32).div_euclid(CHUNK_SIZE);

    // Список загруженных чанков рядом с игроком.
    let mut nearby_chunks: Vec<IVec2> = Vec::with_capacity(49);
    for chunk in chunk_q.iter() {
        let dx = (chunk.coord.x - pcx).abs();
        let dz = (chunk.coord.y - pcz).abs();
        if dx <= SPREAD_CHUNK_RADIUS && dz <= SPREAD_CHUNK_RADIUS {
            nearby_chunks.push(chunk.coord);
        }
    }
    if nearby_chunks.is_empty() { return; }

    for _ in 0..ATTEMPTS_PER_TICK {
        let idx = (lcg_rand(&mut rng_state) as usize) % nearby_chunks.len();
        let chunk = nearby_chunks[idx];

        let lx = (lcg_rand(&mut rng_state) as i32) % CHUNK_SIZE;
        let lz = (lcg_rand(&mut rng_state) as i32) % CHUNK_SIZE;
        let wx = chunk.x * CHUNK_SIZE + lx;
        let wz = chunk.y * CHUNK_SIZE + lz;

        let dx = wx as f32 - player_pos.x;
        let dz = wz as f32 - player_pos.z;
        if dx * dx + dz * dz > PLAYER_ACTIVE_RADIUS * PLAYER_ACTIVE_RADIUS {
            continue;
        }

        // Пробегаем по высоте сверху вниз.
        for y in (1..SY - 1).rev() {
            let here = world.get(wx, y, wz);
            let above = world.get(wx, y + 1, wz);

            if here == BlockType::Dirt && !above.is_solid() {
                if has_grass_neighbor(&world, wx, y, wz) {
                    world.set(wx, y, wz, BlockType::Grass);
                    dirty.mark(wx, wz);
                }
                break;
            }
        }
    }
}

/// Возвращает true, если в 6-соседних блоках есть grass.
fn has_grass_neighbor(world: &WorldData, x: i32, y: i32, z: i32) -> bool {
    const OFFSETS: [(i32, i32, i32); 6] = [
        ( 1, 0, 0),
        (-1, 0, 0),
        ( 0, 0, 1),
        ( 0, 0,-1),
        ( 0, 1, 0),
        ( 0,-1, 0),
    ];
    for (dx, dy, dz) in OFFSETS {
        if world.get(x + dx, y + dy, z + dz) == BlockType::Grass {
            return true;
        }
    }
    false
}

#[inline]
fn lcg_rand(state: &mut u32) -> u32 {
    if *state == 0 {
        *state = 0x9E37_79B9;
    }
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 16) & 0x7FFF
}