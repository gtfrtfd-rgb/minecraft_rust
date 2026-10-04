use bevy::prelude::*;
use crate::core::state::{AppState, HOTBAR, SelectedSlot};
use crate::world::chunk::{WorldData, DirtyChunks, BlockType};
use crate::player::controller::Player;
use crate::mobs::ai::{Mob, MobState};
use super::raycast::{TargetBlock, TargetMob};

pub struct BreakingPlugin;

impl Plugin for BreakingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_attack_or_break, handle_place).run_if(in_state(AppState::InGame)),
        );
        info!("BreakingPlugin loaded.");
    }
}

fn handle_attack_or_break(
    mouse: Res<ButtonInput<MouseButton>>,
    target_block: Res<TargetBlock>,
    target_mob: Res<TargetMob>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    mut mob_q: Query<&mut Mob>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    if let Some(mob_entity) = target_mob.entity {
        if let Ok(mut mob) = mob_q.get_mut(mob_entity) {
            const DAMAGE: i32 = 3;
            mob.hp -= DAMAGE;
            mob.hurt_timer = 0.3;
            mob.panic_timer = 5.0;
            mob.state = MobState::Flee;

            if mob.hp <= 0 {
                info!("Mob killed");
            } else {
                info!("Mob hit! HP: {}/{} (fleeing)", mob.hp, mob.max_hp);
            }
            return;
        }
    }

    let Some(hit) = target_block.hit else { return; };
    if hit.block.y <= 0 { return; }

    let block = world.get(hit.block.x, hit.block.y, hit.block.z);
    if !block.is_solid() { return; }

    world.set(hit.block.x, hit.block.y, hit.block.z, BlockType::Air);
    dirty.mark(hit.block.x, hit.block.z);
}

fn handle_place(
    mouse: Res<ButtonInput<MouseButton>>,
    target: Res<TargetBlock>,
    selected: Res<SelectedSlot>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    player_q: Query<&Transform, With<Player>>,
) {
    if !mouse.just_pressed(MouseButton::Right) { return; }
    let Some(hit) = target.hit else { return; };

    if hit.normal == IVec3::ZERO { return; }

    let pos = hit.prev;
    if !WorldData::in_bounds(pos.x, pos.y, pos.z) { return; }
    if world.get(pos.x, pos.y, pos.z).is_solid() { return; }

    if let Ok(player_tf) = player_q.get_single() {
        if block_intersects_player(pos, player_tf.translation) { return; }
    }

    let block_to_place = HOTBAR[selected.0];
    world.set(pos.x, pos.y, pos.z, block_to_place);
    dirty.mark(pos.x, pos.z);
}

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