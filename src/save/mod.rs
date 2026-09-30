pub mod persistence;

use bevy::prelude::*;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(persistence::PersistencePlugin);
        info!("SavePlugin loaded.");
    }
}