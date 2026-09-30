mod core;
mod world;
mod player;
mod interaction;
mod mobs;
mod ui;
mod save;

use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::window::{CursorGrabMode, CursorOptions};
use core::state::GameStatePlugin;
use world::WorldPlugin;
use player::PlayerPlugin;
use interaction::InteractionPlugin;
use mobs::MobPlugin;
use ui::UiPlugin;
use save::SavePlugin;

/// Версия берётся из Cargo.toml (version = "0.2.0").
/// Меняешь только там — обновится везде.
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Minecraft Rust — v{}", GAME_VERSION),
                resolution: (1280.0_f32, 720.0_f32).into(),
                cursor_options: CursorOptions {
                    visible: false,
                    grab_mode: CursorGrabMode::Confined,
                    ..default()
                },
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
        // ВРЕМЕННО: выход по Escape
        .add_systems(Update, exit_on_escape)
        .run();
}

/// ВРЕМЕННО: выход из игры по Escape.
/// Убрать после того, как появится нормальное меню паузы.
fn exit_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        info!("Escape pressed — exiting...");
        exit.send(AppExit::Success);
    }
}