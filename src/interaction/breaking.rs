use bevy::prelude::*;
use crate::world::chunk::{WorldData, DirtyChunks, BlockType};
use crate::player::controller::Player;
use super::raycast::TargetBlock;

/// Какой блок ставить по ПКМ (пока что фиксированный — хотбар будет позже)
const PLACE_BLOCK: BlockType = BlockType::Stone;

pub struct BreakingPlugin;

impl Plugin for BreakingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_break, handle_place));
        info!("BreakingPlugin loaded.");
    }
}

/// ЛКМ — сломать блок под прицелом
fn handle_break(
    mouse: Res<ButtonInput<MouseButton>>,
    target: Res<TargetBlock>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(hit) = target.hit else { return; };

    // Нельзя ломать самый низ мира (как bedrock)
    if hit.block.y <= 0 {
        return;
    }

    let block = world.get(hit.block.x, hit.block.y, hit.block.z);
    if !block.is_solid() {
        return;
    }

    world.set(hit.block.x, hit.block.y, hit.block.z, BlockType::Air);
    dirty.mark(hit.block.x, hit.block.z);
    info!("Broke block at {:?}", hit.block);
}

/// ПКМ — поставить блок
fn handle_place(
    mouse: Res<ButtonInput<MouseButton>>,
    target: Res<TargetBlock>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    player_q: Query<&Transform, With<Player>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(hit) = target.hit else { return; };

    // Если камера внутри блока — нормаль = 0, ставить некуда
    if hit.normal == IVec3::ZERO {
        return;
    }

    let pos = hit.prev;
    if !WorldData::in_bounds(pos.x, pos.y, pos.z) {
        return;
    }
    if world.get(pos.x, pos.y, pos.z).is_solid() {
        return;
    }

    // Нельзя ставить блок внутрь игрока
    if let Ok(player_tf) = player_q.get_single() {
        if block_intersects_player(pos, player_tf.translation) {
            return;
        }
    }

    world.set(pos.x, pos.y, pos.z, PLACE_BLOCK);
    dirty.mark(pos.x, pos.z);
    info!("Placed block at {:?}", pos);
}

/// Пересекается ли блок (x,y,z) с AABB игрока
fn block_intersects_player(block: IVec3, player_pos: Vec3) -> bool {
    const PR: f32 = 0.3;
    const PH: f32 = 1.8;

    let px_min = player_pos.x - PR;
    let px_max = player_pos.x + PR;
    let py_min = player_pos.y;
    let py_max = player_pos.y + PH;
    let pz_min = player_pos.z - PR;
    let pz_max = player_pos.z + PR;

    let bx_min = block.x as f32;
    let bx_max = bx_min + 1.0;
    let by_min = block.y as f32;
    let by_max = by_min + 1.0;
    let bz_min = block.z as f32;
    let bz_max = bz_min + 1.0;

    px_min < bx_max && px_max > bx_min
        && py_min < by_max && py_max > by_min
        && pz_min < bz_max && pz_max > bz_min
}