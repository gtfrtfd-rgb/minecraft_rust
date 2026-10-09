use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy::app::TerminalCtrlCHandlerPlugin;
use std::sync::atomic::{AtomicBool, Ordering};

mod core;
mod world;
mod player;
mod interaction;
mod mobs;
mod ui;
mod save;

use core::state::GameStatePlugin;
use world::WorldPlugin;
use player::PlayerPlugin;
use interaction::InteractionPlugin;
use mobs::MobPlugin;
use ui::UiPlugin;
use save::SavePlugin;

/// Флаг, чтобы Bevy инициализировался только один раз за время жизни процесса.
/// На Android `android_main` может вызываться несколько раз (например, при
/// повторном открытии приложения из недавних), и повторный `App::run()`
/// приводит к панике `RecreationAttempt`.
static GAME_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[bevy_main]
pub fn main() {
    run_game();
}

pub fn run_game() {
    if GAME_INITIALIZED.swap(true, Ordering::SeqCst) {
        // Приложение уже было запущено ранее в этом процессе.
        // Просто выходим, чтобы не пытаться создать второй event loop.
        info!("Game already initialized, skipping re-initialization.");
        return;
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Minecraft Rust".to_string(),
                        resolution: (1280_u32, 720_u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "assets".to_string(),
                    ..default()
                })
                .disable::<TerminalCtrlCHandlerPlugin>(),
        )
        .add_plugins(virtual_joystick::VirtualJoystickPlugin::<()>::default())
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