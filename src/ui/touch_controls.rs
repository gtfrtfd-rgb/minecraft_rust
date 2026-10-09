use bevy::prelude::*;
use virtual_joystick::VirtualJoystickBundle;
use crate::core::state::{is_mobile, AppState};
use crate::player::controller::{FlyButton, JumpButton, MoveJoystick};

pub struct TouchControlsPlugin;

impl Plugin for TouchControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_touch_controls)
            .add_systems(Update, update_touch_visibility);
        info!("TouchControlsPlugin loaded.");
    }
}

fn setup_touch_controls(mut commands: Commands) {
    // Показываем только на мобильных платформах
    let display = if is_mobile() { Display::Flex } else { Display::None };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                display,
                ..default()
            },
            Name::new("TouchControlsRoot"),
        ))
        .with_children(|parent| {
            // === Виртуальный джойстик (слева внизу) ===
            // Внешняя обёртка задаёт позицию, внутри — сам VirtualJoystickBundle,
            // который уже содержит свой Node (поэтому здесь его не дублируем).
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(40.0),
                        bottom: Val::Px(40.0),
                        width: Val::Px(180.0),
                        height: Val::Px(180.0),
                        ..default()
                    },
                    MoveJoystick,
                    Name::new("MoveJoystickWrapper"),
                ))
                .with_children(|wrapper| {
                    wrapper.spawn(VirtualJoystickBundle::<()>::default());
                });

            // === Кнопка прыжка (справа, выше кнопки полёта) ===
            parent
                .spawn((
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(40.0),
                        bottom: Val::Px(160.0),
                        width: Val::Px(80.0),
                        height: Val::Px(80.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                    BorderColor::all(Color::WHITE),
                    JumpButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("JUMP"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // === Кнопка полёта (справа, ниже кнопки прыжка) ===
            parent
                .spawn((
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(40.0),
                        bottom: Val::Px(40.0),
                        width: Val::Px(80.0),
                        height: Val::Px(80.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                    BorderColor::all(Color::WHITE),
                    FlyButton,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("FLY"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

/// Скрывает/показывает элементы тач-управления в зависимости от состояния игры.
/// Активны только в `InGame` и только на мобильной платформе.
fn update_touch_visibility(
    state: Res<State<AppState>>,
    mut joystick_q: Query<&mut Node, (With<MoveJoystick>, Without<JumpButton>, Without<FlyButton>)>,
    mut jump_q: Query<&mut Node, (With<JumpButton>, Without<MoveJoystick>, Without<FlyButton>)>,
    mut fly_q: Query<&mut Node, (With<FlyButton>, Without<MoveJoystick>, Without<JumpButton>)>,
) {
    if !state.is_changed() {
        return;
    }

    let display = if *state.get() == AppState::InGame && is_mobile() {
        Display::Flex
    } else {
        Display::None
    };

    for mut n in joystick_q.iter_mut() {
        n.display = display;
    }
    for mut n in jump_q.iter_mut() {
        n.display = display;
    }
    for mut n in fly_q.iter_mut() {
        n.display = display;
    }
}