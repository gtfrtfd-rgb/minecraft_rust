use bevy::prelude::*;

pub struct HotbarPlugin;

impl Plugin for HotbarPlugin {
    fn build(&self, _app: &mut App) {
        info!("HotbarPlugin loaded (stub).");
    }
}