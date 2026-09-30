use bevy::prelude::*;
use crate::world::chunk::WorldData;
use crate::player::camera::PlayerCamera;

pub const REACH_DISTANCE: f32 = 5.0;

#[derive(Debug, Clone, Copy)]
pub struct TargetHit {
    /// Координаты блока, в который попал луч
    pub block: IVec3,
    /// Нормаль грани (сторона, в которую мы смотрим)
    pub normal: IVec3,
    /// Свободная клетка перед блоком — сюда ставится новый блок
    pub prev: IVec3,
}

#[derive(Resource, Default)]
pub struct TargetBlock {
    pub hit: Option<TargetHit>,
}

pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetBlock>()
           .add_systems(Update, (update_target, draw_target_highlight));
    }
}

fn update_target(
    camera_q: Query<&Transform, With<PlayerCamera>>,
    world: Res<WorldData>,
    mut target: ResMut<TargetBlock>,
) {
    let Ok(cam_tf) = camera_q.get_single() else {
        target.hit = None;
        return;
    };
    let origin = cam_tf.translation;
    let dir = cam_tf.forward().as_vec3();   // Dir3 → Vec3

    target.hit = raycast(&world, origin, dir, REACH_DISTANCE);
}

fn draw_target_highlight(
    mut gizmos: Gizmos,
    target: Res<TargetBlock>,
) {
    if let Some(hit) = target.hit {
        let center = hit.block.as_vec3() + Vec3::splat(0.5);
        gizmos.cuboid(
            Transform::from_translation(center).with_scale(Vec3::splat(1.005)),
            Color::BLACK,
        );
    }
}

/// DDA-рейкаст по вокселям
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
    } else {
        f32::INFINITY
    };
    let mut t_max_y = if step_y != 0 {
        let next = if step_y > 0 { y as f32 + 1.0 } else { y as f32 };
        ((next - origin.y) / dir.y).abs()
    } else {
        f32::INFINITY
    };
    let mut t_max_z = if step_z != 0 {
        let next = if step_z > 0 { z as f32 + 1.0 } else { z as f32 };
        ((next - origin.z) / dir.z).abs()
    } else {
        f32::INFINITY
    };

    let mut normal = IVec3::ZERO;

    for _ in 0..256 {
        if world.is_solid(x, y, z) {
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
        } else {
            if t_max_y < t_max_z {
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
    }

    None
}