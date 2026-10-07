use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use crate::core::state::AppState;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component, Clone, Copy)]
enum MenuAction {
    Play,
    Resume,
    ExitToMenu,
    ExitGame,
}

const BTN_NORMAL: Color   = Color::srgb(0.18, 0.26, 0.38);
const BTN_HOVER:  Color   = Color::srgb(0.30, 0.50, 0.72);
const BORDER_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
const BORDER_HOVER:  Color = Color::WHITE;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_menus)
            .add_systems(Update, toggle_menu_visibility)
            .add_systems(Update, (
                handle_menu_buttons,
                handle_escape_key,
                update_cursor_for_state,
            ));
        info!("MenuPlugin loaded.");
    }
}

macro_rules! menu_button {
    ($parent:expr, $action:expr, $label:expr) => {
        $parent
            .spawn((
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
                $action,
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new($label),
                    TextFont {
                        font_size: FontSize::Px(30.0), // Изменено
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
    };
}

fn spawn_menus(mut commands: Commands) {
    info!("Spawning menus");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(10.0),
                display: Display::Flex,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.10, 0.16, 0.98)),
            MainMenuRoot,
            Name::new("MainMenu"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("MINECRAFT RUST"),
                TextFont {
                    font_size: FontSize::Px(82.0), // Изменено
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.85, 0.55)),
            ));

            root.spawn((
                TextFont {
                    font_size: FontSize::Px(16.0), // Изменено
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
            ));

            root.spawn(Node {
                height: Val::Px(50.0),
                ..default()
            });

            menu_button!(root, MenuAction::Play, "PLAY");
            menu_button!(root, MenuAction::ExitGame, "EXIT");
        });

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(10.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            PauseMenuRoot,
            Name::new("PauseMenu"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: FontSize::Px(72.0), // Изменено
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            root.spawn(Node {
                height: Val::Px(40.0),
                ..default()
            });

            menu_button!(root, MenuAction::Resume, "RESUME");
            menu_button!(root, MenuAction::ExitToMenu, "EXIT TO MENU");
            menu_button!(root, MenuAction::ExitGame, "QUIT GAME");

            root.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });

            root.spawn((
                Text::new("Press ESC to resume"),
                TextFont {
                    font_size: FontSize::Px(14.0), // Изменено
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
            ));
        });
}

fn toggle_menu_visibility(
    state: Res<State<AppState>>,
    mut main_q:  Query<&mut Node, (With<MainMenuRoot>,  Without<PauseMenuRoot>)>,
    mut pause_q: Query<&mut Node, (With<PauseMenuRoot>, Without<MainMenuRoot>)>,
) {
    if !state.is_changed() {
        return;
    }

    let show_main  = *state.get() == AppState::Menu;
    let show_pause = *state.get() == AppState::Paused;

    for mut n in main_q.iter_mut() {
        n.display = if show_main { Display::Flex } else { Display::None };
    }
    for mut n in pause_q.iter_mut() {
        n.display = if show_pause { Display::Flex } else { Display::None };
    }
}

fn handle_menu_buttons(
    mut q: Query<
        (&Interaction, &MenuAction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action, mut bg, mut border) in q.iter_mut() {
        match *interaction {
            Interaction::Pressed => match action {
                MenuAction::Play | MenuAction::Resume => {
                    info!("Menu action: Play / Resume");
                    next_state.set(AppState::InGame);
                }
                MenuAction::ExitToMenu => {
                    info!("Menu action: Exit to menu");
                    next_state.set(AppState::Menu);
                }
                MenuAction::ExitGame => {
                    info!("Menu action: Quit game");
                    exit.write(AppExit::Success);
                }
            },
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

fn handle_escape_key(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    match state.get() {
        AppState::InGame => {
            info!("Escape -> Paused");
            next_state.set(AppState::Paused);
        }
        AppState::Paused => {
            info!("Escape -> InGame");
            next_state.set(AppState::InGame);
        }
        AppState::Menu => {
        }
    }
}

fn update_cursor_for_state(
    state: Res<State<AppState>>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !state.is_changed() {
        return;
    }

    let (visible, mode) = match state.get() {
        AppState::InGame => (false, CursorGrabMode::Confined),
        AppState::Paused => (true,  CursorGrabMode::None),
        AppState::Menu   => (true,  CursorGrabMode::None),
    };

    for mut cursor in cursor_q.iter_mut() {
        cursor.visible = visible;
        cursor.grab_mode = mode;
    }

    info!("Cursor updated: visible={}, mode={:?}", visible, mode);
}