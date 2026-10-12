use bevy::prelude::*;
use bevy::app::AppExit;
use std::fs;
use std::path::PathBuf;

use crate::core::state::{AppState, Settings};

const BTN_NORMAL: Color   = Color::srgb(0.18, 0.26, 0.38);
const BTN_HOVER:  Color   = Color::srgb(0.30, 0.50, 0.72);
const BORDER_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
const BORDER_HOVER:  Color = Color::WHITE;

#[derive(Component)]
struct SettingsRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum SettingsField {
    MasterVolume,
    MouseSensitivity,
    RenderDistance,
}

#[derive(Component, Clone, Copy)]
struct SettingValueText(SettingsField);

#[derive(Component, Clone, Copy)]
struct SettingsButton {
    field: SettingsField,
    delta: f32,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_settings_from_disk)
            .add_systems(Startup, spawn_settings_menu)
            .add_systems(
                Update,
                (
                    toggle_settings_visibility,
                    handle_settings_buttons,
                    update_setting_texts,
                )
                    .chain(),
            )
            .add_systems(Last, save_settings_on_exit);
        info!("SettingsPlugin loaded.");
    }
}

fn settings_path() -> PathBuf {
    let mut p = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    p.push("saves");
    p.push("settings.json");
    p
}

fn load_settings_from_disk(mut settings: ResMut<Settings>) {
    let path = settings_path();
    if !path.exists() {
        return;
    }
    let Ok(s) = fs::read_to_string(&path) else { return; };
    match serde_json::from_str::<Settings>(&s) {
        Ok(loaded) => {
            *settings = loaded;
            info!("Settings loaded from {:?}", path);
        }
        Err(e) => warn!("Failed to parse settings.json: {}", e),
    }
}

fn save_settings_on_exit(
    mut exit: MessageReader<AppExit>,
    settings: Res<Settings>,
) {
    let mut any = false;
    for _ in exit.read() { any = true; }
    if !any { return; }

    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&*settings) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                warn!("Cannot write settings.json: {}", e);
            } else {
                info!("Settings saved to {:?}", path);
            }
        }
        Err(e) => warn!("Cannot serialize settings: {}", e),
    }
}

macro_rules! settings_row {
    ($parent:expr, $field:expr, $label:expr) => {
        $parent
            .spawn(Node {
                width: Val::Px(560.0),
                height: Val::Px(52.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Text::new($label),
                    TextFont {
                        font_size: FontSize::Px(22.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                ));

                // Минус
                row.spawn((
                    Button,
                    Interaction::default(),
                    Node {
                        width: Val::Px(42.0),
                        height: Val::Px(42.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BTN_NORMAL),
                    BorderColor::all(BORDER_NORMAL),
                    SettingsButton { field: $field, delta: -1.0 },
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("-"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

                row.spawn((
                    Text::new("--"),
                    TextFont {
                        font_size: FontSize::Px(22.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.95, 0.65)),
                    Node {
                        width: Val::Px(140.0),
                        ..default()
                    },
                    SettingValueText($field),
                ));

                row.spawn((
                    Button,
                    Interaction::default(),
                    Node {
                        width: Val::Px(42.0),
                        height: Val::Px(42.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BTN_NORMAL),
                    BorderColor::all(BORDER_NORMAL),
                    SettingsButton { field: $field, delta: 1.0 },
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("+"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            });
    };
}

fn spawn_settings_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.10, 0.16, 0.98)),
            SettingsRoot,
            Name::new("SettingsMenu"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("SETTINGS"),
                TextFont {
                    font_size: FontSize::Px(72.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.85, 0.55)),
            ));

            root.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });

            settings_row!(root, SettingsField::MasterVolume,     "Master Volume");
            settings_row!(root, SettingsField::MouseSensitivity, "Mouse Sensitivity");
            settings_row!(root, SettingsField::RenderDistance,   "Render Distance");

            root.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });

            root.spawn((
                Button,
                Interaction::default(),
                Node {
                    width: Val::Px(340.0),
                    height: Val::Px(64.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                BorderColor::all(BORDER_NORMAL),
                crate::ui::menu::MenuAction::ExitToMenu,
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new("BACK"),
                    TextFont {
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn toggle_settings_visibility(
    state: Res<State<AppState>>,
    mut q: Query<&mut Node, With<SettingsRoot>>,
) {
    if !state.is_changed() {
        return;
    }
    let visible = *state.get() == AppState::Settings;
    for mut n in q.iter_mut() {
        n.display = if visible { Display::Flex } else { Display::None };
    }
}

fn handle_settings_buttons(
    state: Res<State<AppState>>,
    mut q: Query<
        (&Interaction, &SettingsButton, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<Settings>,
) {
    if *state.get() != AppState::Settings {
        return;
    }

    for (interaction, button, mut bg, mut border) in q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                apply_delta(&mut settings, button.field, button.delta);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(BTN_HOVER);
                *border = BorderColor::all(BORDER_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(BTN_NORMAL);
                *border = BorderColor::all(BORDER_NORMAL);
            }
        }
    }
}

fn apply_delta(settings: &mut Settings, field: SettingsField, dir: f32) {
    match field {
        SettingsField::MasterVolume => {
            let step = 0.05 * dir;
            settings.master_volume =
                (settings.master_volume + step).clamp(0.0, 1.0);
        }
        SettingsField::MouseSensitivity => {
            let step = 0.0002 * dir;
            settings.mouse_sensitivity =
                (settings.mouse_sensitivity + step).clamp(0.0005, 0.0060);
        }
        SettingsField::RenderDistance => {
            let step = 16.0 * dir;
            settings.render_distance =
                (settings.render_distance + step).clamp(32.0, 256.0);
        }
    }
}

fn update_setting_texts(
    settings: Res<Settings>,
    mut q: Query<(&SettingValueText, &mut Text)>,
) {
    if !settings.is_changed() {
        return;
    }
    for (field, mut text) in q.iter_mut() {
        text.0 = match field.0 {
            SettingsField::MasterVolume => {
                format!("{:.0}%", settings.master_volume * 100.0)
            }
            SettingsField::MouseSensitivity => {
                format!("{:.4}", settings.mouse_sensitivity)
            }
            SettingsField::RenderDistance => {
                format!("{:.0}", settings.render_distance)
            }
        };
    }
}