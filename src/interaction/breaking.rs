use bevy::prelude::*;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
use crate::core::state::{AppState, HOTBAR, SelectedSlot};
use crate::world::chunk::{WorldData, DirtyChunks, BlockType};
use crate::player::controller::Player;
use crate::mobs::ai::{Mob, MobState};
use crate::audio::SoundHandles;
use super::raycast::{TargetBlock, TargetMob};

/// Сколько секунд нужно удерживать ЛКМ, чтобы сломать блок.
const BREAK_TIME: f32 = 0.35;

/// Прогресс ломания текущего блока.
#[derive(Resource, Default)]
pub struct BreakProgress {
    pub target: Option<IVec3>,
    pub progress: f32,
}

pub struct BreakingPlugin;

impl Plugin for BreakingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BreakProgress>()
            .add_systems(
                Update,
                (
                    handle_attack_mob,
                    handle_break_block,
                    handle_place,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
        info!("BreakingPlugin loaded.");
    }
}

/// Игрок ударил моба (мгновенно на just_pressed ЛКМ).
fn handle_attack_mob(
    mouse: Res<ButtonInput<MouseButton>>,
    target_mob: Res<TargetMob>,
    mut mob_q: Query<&mut Mob>,
    sounds: Option<Res<SoundHandles>>,
    mut commands: Commands,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(mob_entity) = target_mob.entity else { return; };
    let Ok(mut mob) = mob_q.get_mut(mob_entity) else { return; };

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

    if let Some(s) = sounds.as_ref() {
        commands.spawn((
            AudioPlayer(s.hit_mob.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.6)),
        ));
    }
}

/// Ломание блока — удержанием ЛКМ, с прогрессом и звуком.
fn handle_break_block(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    target: Res<TargetBlock>,
    target_mob: Res<TargetMob>,
    mut progress: ResMut<BreakProgress>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    sounds: Option<Res<SoundHandles>>,
    mut commands: Commands,
) {
    let held = mouse.pressed(MouseButton::Left);
    // Если в прицеле моб — блоки не ломаем, приоритет у атаки.
    let mob_targeted = target_mob.entity.is_some();

    let new_target = if held && !mob_targeted {
        target.hit.map(|h| h.block)
    } else {
        None
    };

    // Сменили блок или отпустили ЛКМ — сбрасываем прогресс.
    if progress.target != new_target {
        progress.target = new_target;
        progress.progress = 0.0;
    }

    let Some(pos) = progress.target else { return; };

    // Бедрок (нижний слой) не ломается.
    if pos.y <= 0 {
        return;
    }

    progress.progress += time.delta_secs() / BREAK_TIME;

    if progress.progress < 1.0 {
        return;
    }

    // Блок сломан — обнуляем состояние и удаляем его из мира.
    progress.progress = 0.0;
    progress.target = None;

    let block = world.get(pos.x, pos.y, pos.z);
    if !block.is_solid() {
        return;
    }

    world.set(pos.x, pos.y, pos.z, BlockType::Air);
    dirty.mark(pos.x, pos.z);

    // Звук — по материалу сломанного блока.
    if let Some(s) = sounds.as_ref() {
        commands.spawn((
            AudioPlayer(s.break_sound_for(block)),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.55)),
        ));
    }
}

/// Установка блока ПКМ — мгновенно, со звуком.
fn handle_place(
    mouse: Res<ButtonInput<MouseButton>>,
    target: Res<TargetBlock>,
    selected: Res<SelectedSlot>,
    mut world: ResMut<WorldData>,
    mut dirty: ResMut<DirtyChunks>,
    player_q: Query<&Transform, With<Player>>,
    sounds: Option<Res<SoundHandles>>,
    mut commands: Commands,
) {
    if !mouse.just_pressed(MouseButton::Right) { return; }
    let Some(hit) = target.hit else { return; };

    if hit.normal == IVec3::ZERO { return; }

    let pos = hit.prev;
    if !WorldData::in_bounds(pos.x, pos.y, pos.z) { return; }
    if world.get(pos.x, pos.y, pos.z).is_solid() { return; }

    if let Ok(player_tf) = player_q.single() {
        if block_intersects_player(pos, player_tf.translation) { return; }
    }

    let block_to_place = HOTBAR[selected.0];
    world.set(pos.x, pos.y, pos.z, block_to_place);
    dirty.mark(pos.x, pos.z);

    if let Some(s) = sounds.as_ref() {
        commands.spawn((
            AudioPlayer(s.place_block.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.55)),
        ));
    }
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