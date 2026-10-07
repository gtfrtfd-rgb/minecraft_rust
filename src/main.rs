mod core;
mod world;
mod player;
mod interaction;
mod mobs;
mod ui;
mod save;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, Window, WindowPlugin};
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
                title: "Minecraft Rust".to_string(),
                resolution: (1280.0_f32, 720.0_f32).into(),
                cursor_options: CursorOptions {
                    visible: true,
                    grab_mode: CursorGrabMode::None,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            GameStatePlugin,
            SavePlugin,
            WorldPlugin,
            PlayerPlugin,
            InteractionPlugin,
            MobPlugin,
            UiPlugin,
        ))
        .run();
}