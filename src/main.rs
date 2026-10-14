#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::manual_range_contains,
)]

mod core;
mod world;
mod player;
mod interaction;
mod mobs;
mod ui;
mod save;
mod audio;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, Window, WindowPlugin};
use core::state::GameStatePlugin;
use world::WorldPlugin;
use player::PlayerPlugin;
use interaction::InteractionPlugin;
use mobs::MobPlugin;
use ui::UiPlugin;
use save::SavePlugin;
use audio::GameAudioPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minecraft Rust".to_string(),
                resolution: (1280_u32, 720_u32).into(),
                ..default()
            }),
            primary_cursor_options: Some(CursorOptions {
                visible: true,
                grab_mode: CursorGrabMode::None,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            GameStatePlugin,
            GameAudioPlugin,
            SavePlugin,
            WorldPlugin,
            PlayerPlugin,
            InteractionPlugin,
            MobPlugin,
            UiPlugin,
        ))
        .run();
}