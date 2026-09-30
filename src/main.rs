mod core;
mod world;
mod player;
mod interaction;
mod mobs;
mod ui;
mod save;

use bevy::prelude::*;
use core::state::GameStatePlugin;
use world::WorldPlugin;
use player::PlayerPlugin;
use interaction::InteractionPlugin;
use mobs::MobPlugin;
use ui::UiPlugin;
use save::SavePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minecraft Rust — v0.1".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            GameStatePlugin,
            WorldPlugin,
            PlayerPlugin,
            InteractionPlugin,
            MobPlugin,
            UiPlugin,
            SavePlugin,
        ))
        .run();
}