mod core;
mod world;
mod player;
mod interaction;
mod mobs;
mod ui;
mod save;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // --- Ядро ---
        .add_plugins(core::state::GameStatePlugin)
        // --- Мир и чанки ---
        .add_plugins(world::WorldPlugin)
        // --- Игрок ---
        .add_plugins(player::PlayerPlugin)
        // --- Взаимодействие (рейкаст, ломание/установка) ---
        .add_plugins(interaction::InteractionPlugin)
        // --- Мобы ---
        .add_plugins(mobs::MobPlugin)
        // --- Интерфейс ---
        .add_plugins(ui::UiPlugin)
        // --- Сохранения ---
        .add_plugins(save::SavePlugin)
        .run();
}