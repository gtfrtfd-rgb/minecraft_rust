use bevy::prelude::*;

pub struct BreakingPlugin;

impl Plugin for BreakingPlugin {
    fn build(&self, _app: &mut App) {
        info!("BreakingPlugin loaded (stub).");
    }
}