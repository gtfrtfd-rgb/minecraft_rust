use bevy::prelude::*;

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, auto_save_system);
    }
}

fn auto_save_system(_time: Res<Time>) {
    // Периодическое автосохранение мира (RLE + JSON)
}