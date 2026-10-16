use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::core::state::{
    AppState, Settings, RebindingState, BindAction, key_to_string,
};

const BTN_NORMAL: Color   = Color::srgb(0.18, 0.26, 0.38);
const BTN_HOVER:  Color   = Color::srgb(0.30, 0.50, 0.72);
const BTN_ACTIVE: Color   = Color::srgb(0.85, 0.45, 0.10);
const BORDER_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
const BORDER_HOVER:  Color = Color::WHITE;
const BORDER_ACTIVE: Color = Color::srgb(1.0, 0.85, 0.30);
const WARNING_COLOR: Color = Color::srgb(1.0, 0.35, 0.35);

#[derive(Component)]
struct KeybindsRoot;

#[derive(Component, Clone, Copy)]
struct KeybindButton(BindAction);

#[derive(Component, Clone, Copy)]
struct KeybindValueText(BindAction);

#[derive(Component)]
struct RebindWarningText;

/// Предупреждение о конфликте клавиш.
/// Показывается, если игрок пытается назначить уже занятую клавишу.
#[derive(Resource, Default)]
pub struct RebindWarning {
    pub message: Option<String>,
    pub timer: f32,
}

impl RebindWarning {
    fn show(&mut self, msg: String) {
        self.message = Some(msg);
        self.timer = 3.0;
    }
    fn clear(&mut self) {
        self.message = None;
        self.timer = 0.0;
    }
}

pub struct KeybindsPlugin;

impl Plugin for KeybindsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RebindWarning>()
            .add_systems(Startup, spawn_keybinds_menu)
            .add_systems(
                Update,
                (
                    toggle_keybinds_visibility,
                    reset_stale_button_colors,
                    handle_keybind_buttons,
                    catch_rebind_input,
                    tick_rebind_warning,
                    handle_escape_in_keybinds,
                    update_keybind_texts,
                    update_rebind_warning_text,
                ).chain(),
            );
        info!("KeybindsPlugin loaded.");
    }
}

fn spawn_keybinds_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(4.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.10, 0.16, 0.98)),
            KeybindsRoot,
            Name::new("KeybindsMenu"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("CONTROLS"),
                TextFont { font_size: FontSize::Px(48.0), ..default() },
                TextColor(Color::srgb(0.55, 0.85, 0.55)),
            ));

            root.spawn(Node { height: Val::Px(14.0), ..default() });

            for action in BindAction::ALL {
                spawn_keybind_row(root, action);
            }

            root.spawn(Node { height: Val::Px(16.0), ..default() });

            // Строка предупреждения о конфликте. По умолчанию пустая,
            // заполняется при попытке назначить занятую клавишу.
            root.spawn((
                Text::new(""),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(WARNING_COLOR),
                Node {
                    height: Val::Px(24.0),
                    ..default()
                },
                RebindWarningText,
            ));

            root.spawn((
                Button,
                Interaction::default(),
                Node {
                    width: Val::Px(340.0),
                    height: Val::Px(48.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                BorderColor::all(BORDER_NORMAL),
                crate::ui::menu::MenuAction::ExitToSettings,
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            root.spawn(Node { height: Val::Px(8.0), ..default() });

            root.spawn((
                Text::new("Click a key to rebind, then press a new key. ESC to cancel."),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
            ));
        });
}

fn spawn_keybind_row(parent: &mut ChildSpawnerCommands, action: BindAction) {
    parent
        .spawn(Node {
            width: Val::Px(620.0),
            height: Val::Px(42.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(action.label()),
                TextFont { font_size: FontSize::Px(20.0), ..default() },
                TextColor(Color::WHITE),
                Node { flex_grow: 1.0, ..default() },
            ));

            row.spawn((
                Button,
                Interaction::default(),
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                BorderColor::all(BORDER_NORMAL),
                KeybindButton(action),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new("--"),
                    TextFont { font_size: FontSize::Px(20.0), ..default() },
                    TextColor(Color::srgb(0.85, 0.95, 0.65)),
                    KeybindValueText(action),
                ));
            });
        });
}

fn toggle_keybinds_visibility(
    state: Res<State<AppState>>,
    mut q: Query<&mut Node, With<KeybindsRoot>>,
) {
    if !state.is_changed() { return; }
    let visible = *state.get() == AppState::Keybinds;
    for mut n in q.iter_mut() {
        n.display = if visible { Display::Flex } else { Display::None };
    }
}

fn reset_stale_button_colors(
    rebinding: Res<RebindingState>,
    mut q: Query<
        (&KeybindButton, &mut BackgroundColor, &mut BorderColor),
        With<Button>,
    >,
) {
    if !rebinding.is_changed() { return; }
    if rebinding.action.is_some() { return; }

    for (_, mut bg, mut border) in q.iter_mut() {
        *bg = BackgroundColor(BTN_NORMAL);
        *border = BorderColor::all(BORDER_NORMAL);
    }
}

fn handle_keybind_buttons(
    state: Res<State<AppState>>,
    mut rebinding: ResMut<RebindingState>,
    mut warning: ResMut<RebindWarning>,
    mut q: Query<
        (&Interaction, &KeybindButton, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    if *state.get() != AppState::Keybinds { return; }

    for (interaction, button, mut bg, mut border) in q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                rebinding.action = Some(button.0);
                warning.clear();
                *bg = BackgroundColor(BTN_ACTIVE);
                *border = BorderColor::all(BORDER_ACTIVE);
            }
            Interaction::Hovered => {
                if rebinding.action != Some(button.0) {
                    *bg = BackgroundColor(BTN_HOVER);
                    *border = BorderColor::all(BORDER_HOVER);
                }
            }
            Interaction::None => {
                if rebinding.action != Some(button.0) {
                    *bg = BackgroundColor(BTN_NORMAL);
                    *border = BorderColor::all(BORDER_NORMAL);
                }
            }
        }
    }
}

/// Ищет действие, у которого уже назначена эта клавиша.
/// Возвращает `Some(action)` при конфликте, `None` если клавиша свободна.
fn find_conflict(
    settings: &Settings,
    action: BindAction,
    key_str: &str,
) -> Option<BindAction> {
    for other in BindAction::ALL {
        if other == action { continue; }
        if settings.key_string_for(other) == key_str {
            return Some(other);
        }
    }
    None
}

fn catch_rebind_input(
    state: Res<State<AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut rebinding: ResMut<RebindingState>,
    mut settings: ResMut<Settings>,
    mut warning: ResMut<RebindWarning>,
) {
    if *state.get() != AppState::Keybinds { return; }
    let Some(action) = rebinding.action else { return; };

    for key in keys.get_just_pressed() {
        if *key == KeyCode::Escape { continue; }

        let key_str = key_to_string(*key);

        // Проверка конфликта: если эта клавиша уже занята другим
        // действием — предупреждение, назначение отменяется.
        // Режим переназначения остаётся активным, чтобы можно было
        // сразу нажать другую клавишу.
        if let Some(conflict) = find_conflict(&settings, action, &key_str) {
            warning.show(format!(
                "Key '{}' is already bound to '{}'. Press a different key.",
                key_str, conflict.label()
            ));
            info!(
                "Rebind conflict: {} already used by {}",
                key_str, conflict.label()
            );
            return;
        }

        settings.set_key_string(action, key_str.clone());
        rebinding.action = None;
        warning.clear();
        info!("Rebound {:?} -> {}", action, key_str);
        return;
    }
}

/// Уменьшает таймер предупреждения и сбрасывает его по истечении.
fn tick_rebind_warning(
    time: Res<Time>,
    mut warning: ResMut<RebindWarning>,
) {
    if warning.message.is_none() { return; }
    warning.timer -= time.delta_secs();
    if warning.timer <= 0.0 {
        warning.clear();
    }
}

fn handle_escape_in_keybinds(
    state: Res<State<AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut rebinding: ResMut<RebindingState>,
    mut warning: ResMut<RebindWarning>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::Keybinds { return; }
    if !keys.just_pressed(KeyCode::Escape) { return; }

    if rebinding.action.is_some() {
        rebinding.action = None;
        warning.clear();
        info!("Rebind cancelled");
    } else {
        next_state.set(AppState::Settings);
        info!("Keybinds -> Settings");
    }
}

fn update_keybind_texts(
    state: Res<State<AppState>>,
    settings: Res<Settings>,
    rebinding: Res<RebindingState>,
    mut q: Query<(&KeybindValueText, &mut Text)>,
) {
    if !state.is_changed() && !settings.is_changed() && !rebinding.is_changed() {
        return;
    }
    if *state.get() != AppState::Keybinds { return; }

    for (val, mut text) in q.iter_mut() {
        if rebinding.action == Some(val.0) {
            text.0 = "...".into();
        } else {
            text.0 = settings.key_string_for(val.0).to_string();
        }
    }
}

fn update_rebind_warning_text(
    warning: Res<RebindWarning>,
    mut q: Query<&mut Text, With<RebindWarningText>>,
) {
    if !warning.is_changed() { return; }
    for mut text in q.iter_mut() {
        text.0 = warning.message.clone().unwrap_or_default();
    }
}