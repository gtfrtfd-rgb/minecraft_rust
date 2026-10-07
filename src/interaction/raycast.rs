use bevy::prelude::*;
use crate::core::state::AppState;
use crate::world::chunk::WorldData;
use crate::player::camera::PlayerCamera;
use crate::mobs::ai::Mob;

pub const REACH_DISTANCE: f32 = 5.0;
const HIGHLIGHT_MIN_DIST: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
pub struct TargetHit {
    pub block: IVec3,
    pub normal: IVec3,
    pub prev: IVec3,
}

#[derive(Resource, Default)]
pub struct TargetBlock {
    pub hit: Option<TargetHit>,
}

#[derive(Resource, Default)]
pub struct TargetMob {
    pub entity: Option<Entity>,
}

pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetBlock>()
            .init_resource::<TargetMob>()
            .add_systems(
                Update,
                (update_target, update_target_mob, draw_target_highlight)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn update_target(
    camera_q: Query<&Transform, With<PlayerCamera>>,
    world: Res<WorldData>,
    mut target: ResMut<TargetBlock>,
) {
    let Ok(cam_tf) = camera_q.single() else {
        target.hit = None;
        return;
    };
    let origin = cam_tf.translation;
    let dir = cam_tf.forward().as_vec3();
    target.hit = raycast(&world, origin, dir, REACH_DISTANCE);
}

fn update_target_mob(
    camera_q: Query<&Transform, With<PlayerCamera>>,
    mob_q: Query<(Entity, &Transform, &Mob)>,
    block_target: Res<TargetBlock>,
    mut target_mob: ResMut<TargetMob>,
) {
    let Ok(cam_tf) = camera_q.single() else {
        target_mob.entity = None;
        return;
    };

    let origin = cam_tf.translation;
    let dir = cam_tf.forward().as_vec3();

    let block_dist = block_target
        .hit
        .map(|h| {
            let center = Vec3::new(
                h.block.x as f32 + 0.5,
                h.block.y as f32 + 0.5,
                h.block.z as f32 + 0.5,
            );
            (center - origin).length()
        })
        .unwrap_or(REACH_DISTANCE);

    let mut best: Option<(Entity, f32)> = None;
    let mut best_t = block_dist.min(REACH_DISTANCE);

    for (entity, mob_tf, mob) in mob_q.iter() {
        let center = mob_tf.translation
            + Vec3::new(0.0, mob.mob_type.height() * 0.5, 0.0);

        let to_mob = center - origin;
        let along = to_mob.dot(dir);
        if along < 0.0 || along > best_t {
            continue;
        }

        let perp_sq = to_mob.length_squared() - along * along;
        let r = mob.mob_type.radius().max(0.4);
        if perp_sq > r * r {
            continue;
        }

        best_t = along;
        best = Some((entity, along));
    }

    target_mob.entity = best.map(|(e, _)| e);
}

fn draw_target_highlight(
    mut gizmos: Gizmos,
    target: Res<TargetBlock>,
    cam_q: Query<&Transform, With<PlayerCamera>>,
) {
    let Some(hit) = target.hit else { return; };

    if !WorldData::in_bounds(hit.block.x, hit.block.y, hit.block.z) {
        return;
    }
    if hit.block.y <= 0 {
        return;
    }

    let Ok(cam_tf) = cam_q.single() else { return; };

    let bmin = hit.block.as_vec3();
    let bmax = bmin + Vec3::ONE;
    let p = cam_tf.translation;
    let nearest = p.clamp(bmin, bmax);
    if (nearest - p).length() < HIGHLIGHT_MIN_DIST {
        return;
    }

    let center = bmin + Vec3::splat(0.5);
    gizmos.cube(
        Transform::from_translation(center).with_scale(Vec3::splat(1.002)),
        Color::BLACK,
    );
}

pub fn raycast(world: &WorldData, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<TargetHit> {
    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;

    let step_x: i32 = if dir.x > 0.0 { 1 } else if dir.x < 0.0 { -1 } else { 0 };
    let step_y: i32 = if dir.y > 0.0 { 1 } else if dir.y < 0.0 { -1 } else { 0 };
    let step_z: i32 = if dir.z > 0.0 { 1 } else if dir.z < 0.0 { -1 } else { 0 };

    let t_delta_x = if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::INFINITY };
    let t_delta_y = if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::INFINITY };
    let t_delta_z = if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::INFINITY };

    let mut t_max_x = if step_x != 0 {
        let next = if step_x > 0 { x as f32 + 1.0 } else { x as f32 };
        ((next - origin.x) / dir.x).abs()
    } else { f32::INFINITY };
    let mut t_max_y = if step_y != 0 {
        let next = if step_y > 0 { y as f32 + 1.0 } else { y as f32 };
        ((next - origin.y) / dir.y).abs()
    } else { f32::INFINITY };
    let mut t_max_z = if step_z != 0 {
        let next = if step_z > 0 { z as f32 + 1.0 } else { z as f32 };
        ((next - origin.z) / dir.z).abs()
    } else { f32::INFINITY };

    let mut normal = IVec3::ZERO;

    for _ in 0..256 {
        if WorldData::in_bounds(x, y, z) && world.get(x, y, z).is_solid() {
            return Some(TargetHit {
                block: IVec3::new(x, y, z),
                normal,
                prev: IVec3::new(x, y, z) + normal,
            });
        }
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                x += step_x;
                if t_max_x > max_dist { return None; }
                t_max_x += t_delta_x;
                normal = IVec3::new(-step_x, 0, 0);
            } else {
                z += step_z;
                if t_max_z > max_dist { return None; }
                t_max_z += t_delta_z;
                normal = IVec3::new(0, 0, -step_z);
            }
        } else if t_max_y < t_max_z {
            y += step_y;
            if t_max_y > max_dist { return None; }
            t_max_y += t_delta_y;
            normal = IVec3::new(0, -step_y, 0);
        } else {
            z += step_z;
            if t_max_z > max_dist { return None; }
            t_max_z += t_delta_z;
            normal = IVec3::new(0, 0, -step_z);
        }
    }
    None
}