use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;

use crate::core::state::{
    AppState, WorldsList, SelectedWorldIdx, PendingLoadWorld, WorldInfo,
    RenamingWorld, RenameState,
};
use crate::save::worlds::{
    scan_worlds, create_world, delete_world, rename_world, generate_world_name,
};

const BG_COLOR: Color     = Color::srgba(0.06, 0.10, 0.16, 0.98);
const LIST_BG: Color      = Color::srgba(0.10, 0.14, 0.20, 0.9);
const BTN_NORMAL: Color   = Color::srgb(0.18, 0.26, 0.38);
const BTN_HOVER:  Color   = Color::srgb(0.30, 0.50, 0.72);
const BTN_SELECTED: Color = Color::srgb(0.28, 0.52, 0.32);
const BTN_DANGER: Color   = Color::srgb(0.55, 0.20, 0.20);
const BTN_DANGER_HOVER: Color = Color::srgb(0.75, 0.30, 0.30);
const BTN_ACTIVE: Color   = Color::srgb(0.55, 0.40, 0.10);
const BORDER_NORMAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
const BORDER_HOVER:  Color = Color::WHITE;
const BORDER_SELECTED: Color = Color::srgb(0.85, 0.95, 0.65);
const BORDER_ACTIVE: Color = Color::srgb(1.0, 0.85, 0.30);

const MAX_NAME_LEN: usize = 32;

#[derive(Component)]
struct WorldsRoot;

#[derive(Component)]
struct WorldsListRoot;

#[derive(Component, Clone, Copy)]
enum WorldAction {
    Select(usize),
    Play,
    Create,
    Rename,
    Delete,
    Back,
}

#[derive(Component, Clone, Copy)]
struct WorldRow(usize);

#[derive(Component)]
struct WorldInfoText;

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_worlds_menu)
            .add_systems(OnEnter(AppState::Worlds), on_enter_worlds)
            .add_systems(OnExit(AppState::Worlds), cancel_rename_on_exit)
            .add_systems(Update, (
                toggle_worlds_visibility,
                rebuild_world_list,
                capture_rename_input,
                commit_or_cancel_rename,
                handle_world_actions,
                update_world_highlights,
                update_info_text,
                handle_escape_in_worlds,
            ).chain());
        info!("WorldsPlugin loaded.");
    }
}

fn spawn_worlds_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG_COLOR),
            WorldsRoot,
            Name::new("WorldsMenu"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("SELECT WORLD"),
                TextFont { font_size: FontSize::Px(56.0), ..default() },
                TextColor(Color::srgb(0.55, 0.85, 0.55)),
            ));

            // Список миров.
            root.spawn((
                Node {
                    width: Val::Px(900.0),
                    height: Val::Px(380.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(LIST_BG),
                BorderColor::all(BORDER_NORMAL),
                WorldsListRoot,
            ));

            // Информация о выбранном мире (или draft при переименовании).
            root.spawn((
                Text::new("No world selected"),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::srgb(0.85, 0.95, 0.65)),
                WorldInfoText,
            ));

            // Нижняя панель кнопок.
            root.spawn(Node {
                width: Val::Px(900.0),
                height: Val::Px(60.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::top(Val::Px(14.0)),
                ..default()
            })
            .with_children(|row| {
                action_button(row, WorldAction::Play,   "PLAY",   BTN_NORMAL, 160.0);
                action_button(row, WorldAction::Create, "CREATE", BTN_NORMAL, 160.0);
                action_button(row, WorldAction::Rename, "RENAME", BTN_NORMAL, 160.0);
                action_button(row, WorldAction::Delete, "DELETE", BTN_DANGER, 160.0);
                action_button(row, WorldAction::Back,   "BACK",   BTN_NORMAL, 160.0);
            });
        });
}

fn action_button(
    parent: &mut ChildSpawnerCommands,
    action: WorldAction,
    label: &str,
    color: Color,
    width: f32,
) {
    parent.spawn((
        Button,
        Interaction::default(),
        Node {
            width: Val::Px(width),
            height: Val::Px(54.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(color),
        BorderColor::all(BORDER_NORMAL),
        action,
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(label),
            TextFont { font_size: FontSize::Px(22.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn on_enter_worlds(
    mut worlds: ResMut<WorldsList>,
    mut selected: ResMut<SelectedWorldIdx>,
    mut renaming: ResMut<RenamingWorld>,
) {
    worlds.0 = scan_worlds();
    if selected.0 >= worlds.0.len() {
        selected.0 = 0;
    }
    renaming.0 = None;
    info!("Worlds menu: found {} worlds", worlds.0.len());
}

fn cancel_rename_on_exit(mut renaming: ResMut<RenamingWorld>) {
    renaming.0 = None;
}

fn toggle_worlds_visibility(
    state: Res<State<AppState>>,
    mut q: Query<&mut Node, With<WorldsRoot>>,
) {
    if !state.is_changed() { return; }
    let visible = *state.get() == AppState::Worlds;
    for mut n in q.iter_mut() {
        n.display = if visible { Display::Flex } else { Display::None };
    }
}

fn rebuild_world_list(
    mut commands: Commands,
    worlds: Res<WorldsList>,
    list_root_q: Query<Entity, With<WorldsListRoot>>,
    children_q: Query<&Children>,
) {
    if !worlds.is_changed() { return; }
    let Ok(root) = list_root_q.single() else { return; };

    if let Ok(children) = children_q.get(root) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let worlds_snapshot: Vec<(usize, WorldInfo)> = worlds.0.iter().cloned().enumerate().collect();

    commands.entity(root).with_children(|list| {
        if worlds_snapshot.is_empty() {
            list.spawn((
                Text::new("No worlds yet. Click CREATE to make one."),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
                Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
            ));
            return;
        }

        for (i, world) in worlds_snapshot {
            list.spawn((
                Button,
                Interaction::default(),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(58.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::new(
                        Val::Px(14.0), Val::Px(14.0),
                        Val::Px(4.0), Val::Px(4.0),
                    ),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                BorderColor::all(BORDER_NORMAL),
                WorldAction::Select(i),
                WorldRow(i),
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(world.name.clone()),
                    TextFont { font_size: FontSize::Px(20.0), ..default() },
                    TextColor(Color::WHITE),
                ));
                row.spawn((
                    Text::new(format!(
                        "Seed: {}   Edits: {}   {}",
                        world.seed,
                        world.edits,
                        time_ago(world.saved_at),
                    )),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                ));
            });
        }
    });
}

fn time_ago(saved_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if saved_at == 0 || saved_at > now { return "just now".into(); }
    let secs = now - saved_at;
    if secs < 60 { return format!("{}s ago", secs); }
    if secs < 3600 { return format!("{}m ago", secs / 60); }
    if secs < 86400 { return format!("{}h ago", secs / 3600); }
    format!("{}d ago", secs / 86400)
}

/// Принимает символы с клавиатуры в черновик переименования.
/// Backspace — удаляет последний символ.
fn capture_rename_input(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut renaming: ResMut<RenamingWorld>,
) {
    let Some(state) = renaming.0.as_mut() else { return; };

    // Собираем символы из событий клавиатуры.
    for ev in events.read() {
        if ev.state != ButtonState::Pressed { continue; }
        if let Some(text) = ev.text.as_ref() {
            for c in text.chars() {
                // Разрешаем только буквы, цифры, пробел, '_', '-'.
                                if (c.is_alphanumeric() || c == ' ' || c == '_' || c == '-')
                    && state.draft.chars().count() < MAX_NAME_LEN
                {
                    state.draft.push(c);
                }
            }
        }
    }

    if keys.just_pressed(KeyCode::Backspace) {
        state.draft.pop();
    }
}

/// Enter — сохранить новое имя и обновить список.
/// Escape — отменить без сохранения.
fn commit_or_cancel_rename(
    keys: Res<ButtonInput<KeyCode>>,
    mut renaming: ResMut<RenamingWorld>,
    mut worlds: ResMut<WorldsList>,
    mut selected: ResMut<SelectedWorldIdx>,
) {
    let Some(state) = renaming.0.as_ref() else { return; };

    if keys.just_pressed(KeyCode::Escape) {
        info!("Rename cancelled");
        renaming.0 = None;
        return;
    }

    if !keys.just_pressed(KeyCode::Enter) { return; }

    let trimmed = state.draft.trim();
    if trimmed.is_empty() {
        warn!("Rename: empty name, keeping old");
        renaming.0 = None;
        return;
    }
    if trimmed == state.draft && trimmed.chars().count() > MAX_NAME_LEN {
        // не должно случиться, но на всякий
        renaming.0 = None;
        return;
    }

    let path = state.path.clone();
    let new_name = trimmed.to_string();

    match rename_world(&path, &new_name) {
        Ok(()) => {
            info!("Renamed world to '{}'", new_name);
            worlds.0 = scan_worlds();
            if let Some(i) = worlds.0.iter().position(|w| w.path == path) {
                selected.0 = i;
            }
        }
        Err(e) => warn!("Rename failed: {}", e),
    }

    renaming.0 = None;
}

fn handle_world_actions(
    state: Res<State<AppState>>,
    mut worlds: ResMut<WorldsList>,
    mut selected: ResMut<SelectedWorldIdx>,
    mut pending: ResMut<PendingLoadWorld>,
    mut renaming: ResMut<RenamingWorld>,
    mut next_state: ResMut<NextState<AppState>>,
    mut q: Query<
        (&Interaction, &WorldAction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    if *state.get() != AppState::Worlds { return; }

    // Если идёт переименование — кнопки не реагируют.
    let busy = renaming.0.is_some();

    for (interaction, action, mut bg, mut border) in q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if busy { continue; }
                match action {
                    WorldAction::Select(i) => {
                        selected.0 = *i;
                    }
                    WorldAction::Play => {
                        if worlds.0.is_empty() { continue; }
                        let idx = selected.0.min(worlds.0.len() - 1);
                        let path = worlds.0[idx].path.clone();
                        pending.0 = Some(path);
                        next_state.set(AppState::InGame);
                    }
                    WorldAction::Create => {
                        let name = generate_world_name();
                        match create_world(&name) {
                            Ok(path) => {
                                info!("Created world '{}' at {:?}", name, path);
                                worlds.0 = scan_worlds();
                                if let Some(i) = worlds.0.iter().position(|w| w.path == path) {
                                    selected.0 = i;
                                }
                            }
                            Err(e) => warn!("Create world failed: {}", e),
                        }
                    }
                    WorldAction::Rename => {
                        if worlds.0.is_empty() { continue; }
                        let idx = selected.0.min(worlds.0.len() - 1);
                        let w = &worlds.0[idx];
                        renaming.0 = Some(RenameState {
                            path: w.path.clone(),
                            draft: w.name.clone(),
                        });
                        info!("Rename started for '{}'", w.name);
                    }
                    WorldAction::Delete => {
                        if worlds.0.is_empty() { continue; }
                        let idx = selected.0.min(worlds.0.len() - 1);
                        let path = worlds.0[idx].path.clone();
                        match delete_world(&path) {
                            Ok(()) => {
                                info!("Deleted world {:?}", path);
                                worlds.0 = scan_worlds();
                                if worlds.0.is_empty() {
                                    selected.0 = 0;
                                } else if selected.0 >= worlds.0.len() {
                                    selected.0 = worlds.0.len() - 1;
                                }
                            }
                            Err(e) => warn!("Delete world failed: {}", e),
                        }
                    }
                    WorldAction::Back => {
                        next_state.set(AppState::Menu);
                    }
                }
            }
            Interaction::Hovered => {
                if busy { continue; }
                if matches!(action, WorldAction::Select(_)) { continue; }
                if matches!(action, WorldAction::Delete) {
                    *bg = BackgroundColor(BTN_DANGER_HOVER);
                } else {
                    *bg = BackgroundColor(BTN_HOVER);
                }
                *border = BorderColor::all(BORDER_HOVER);
            }
            Interaction::None => {
                if matches!(action, WorldAction::Select(_)) { continue; }
                if matches!(action, WorldAction::Delete) {
                    *bg = BackgroundColor(BTN_DANGER);
                } else {
                    *bg = BackgroundColor(BTN_NORMAL);
                }
                *border = BorderColor::all(BORDER_NORMAL);
            }
        }
    }

    // Подсветка кнопки RENAME, пока идёт переименование.
    // (отдельный проход, чтобы не мешать основному)
    let _ = (busy, &mut renaming);
}

fn update_world_highlights(
    renaming: Res<RenamingWorld>,
    selected: Res<SelectedWorldIdx>,
    mut q: Query<(&WorldRow, &Interaction, &mut BackgroundColor, &mut BorderColor)>,
) {
    let busy = renaming.0.is_some();
    for (row, inter, mut bg, mut border) in q.iter_mut() {
        let is_selected = row.0 == selected.0;

        // Во время переименования подсвечиваем только выбранную строку
        // ярко-оранжевым, а остальные — приглушённо.
        let (color, bcolor) = if busy && is_selected {
            (BTN_ACTIVE, BORDER_ACTIVE)
        } else if is_selected {
            (BTN_SELECTED, BORDER_SELECTED)
        } else {
            match inter {
                Interaction::Hovered => (BTN_HOVER, BORDER_HOVER),
                _ => (BTN_NORMAL, BORDER_NORMAL),
            }
        };
        *bg = BackgroundColor(color);
        *border = BorderColor::all(bcolor);
    }
}

fn update_info_text(
    worlds: Res<WorldsList>,
    selected: Res<SelectedWorldIdx>,
    renaming: Res<RenamingWorld>,
    mut q: Query<&mut Text, With<WorldInfoText>>,
) {
    if !worlds.is_changed() && !selected.is_changed() && !renaming.is_changed() {
        return;
    }
    let Ok(mut text) = q.single_mut() else { return; };

    // Режим переименования — показываем черновик.
    if let Some(state) = renaming.0.as_ref() {
        let chars = state.draft.chars().count();
        text.0 = format!(
            "RENAMING: {}_   ({}/{} chars)   [Enter = confirm, Esc = cancel, Backspace = delete]",
            state.draft, chars, MAX_NAME_LEN,
        );
        return;
    }

    if worlds.0.is_empty() {
        text.0 = "No world selected".into();
        return;
    }
    let idx = selected.0.min(worlds.0.len() - 1);
    let w = &worlds.0[idx];
    text.0 = format!(
        "Name: {}   |   Seed: {}   |   Edits: {}   |   Last played: {}",
        w.name, w.seed, w.edits, time_ago(w.saved_at),
    );
}

fn handle_escape_in_worlds(
    state: Res<State<AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    renaming: Res<RenamingWorld>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::Worlds { return; }
    if !keys.just_pressed(KeyCode::Escape) { return; }

    // Если идёт переименование — Escape уже обработан в
    // commit_or_cancel_rename, не мешаем.
    if renaming.0.is_some() { return; }

    next_state.set(AppState::Menu);
}