use bevy::prelude::*;
use crate::core::state::{AppState, HOTBAR, SelectedSlot};

#[derive(Component)]
struct HotbarSlot {
    index: usize,
}

#[derive(Component)]
struct HotbarRoot;

pub struct HotbarPlugin;

impl Plugin for HotbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hotbar)
            .add_systems(Update, update_hotbar_highlight)
            .add_systems(Update, update_visibility);
        info!("HotbarPlugin loaded.");
    }
}

fn setup_hotbar(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
            HotbarRoot,
            Name::new("HotbarRoot"),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
                ))
                .with_children(|row| {
                    for (i, &block) in HOTBAR.iter().enumerate() {
                        row.spawn((
                            Node {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                border: UiRect::all(Val::Px(2.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::FlexStart,
                                ..default()
                            },
                            BackgroundColor(block.icon_color()),
                            BorderColor(Color::srgb(0.35, 0.35, 0.35)),
                            HotbarSlot { index: i },
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                Text::new(format!("{}", i + 1)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(3.0),
                                    top: Val::Px(1.0),
                                    ..default()
                                },
                            ));
                        });
                    }
                });
        });
}

fn update_hotbar_highlight(
    selected: Res<SelectedSlot>,
    mut q: Query<(&HotbarSlot, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !selected.is_changed() {
        return;
    }

    for (slot, mut bg, mut border) in q.iter_mut() {
        if slot.index == selected.0 {
            let base = HOTBAR[slot.index].icon_color();
            bg.0 = lighten(base, 0.3);
            *border = BorderColor(Color::WHITE);
        } else {
            bg.0 = HOTBAR[slot.index].icon_color();
            *border = BorderColor(Color::srgb(0.35, 0.35, 0.35));
        }
    }
}

fn update_visibility(
    state: Res<State<AppState>>,
    mut q: Query<&mut Node, With<HotbarRoot>>,
) {
    if !state.is_changed() {
        return;
    }

    let display = if *state.get() == AppState::InGame {
        Display::Flex
    } else {
        Display::None
    };

    for mut n in q.iter_mut() {
        n.display = display;
    }
}

fn lighten(c: Color, amt: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(
        (s.red + amt).min(1.0),
        (s.green + amt).min(1.0),
        (s.blue + amt).min(1.0),
        s.alpha,
    )
}