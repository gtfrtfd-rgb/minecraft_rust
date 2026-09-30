use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::player::controller::Player;
use crate::GAME_VERSION;

#[derive(Component)]
struct CoordsText;

#[derive(Component)]
struct FpsText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
           .add_systems(Startup, setup_hud)
           .add_systems(Update, (update_coords, update_fps));
        info!("HudPlugin loaded.");
    }
}

fn setup_hud(mut commands: Commands) {
    // ---- Прицел ----
    commands.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        position_type: PositionType::Absolute,
        ..default()
    })
    .with_children(|parent| {
        // Вертикальная полоса креста
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                width: Val::Px(2.0),
                height: Val::Px(18.0),
                margin: UiRect::new(
                    Val::Px(-1.0),
                    Val::Auto,
                    Val::Px(-9.0),
                    Val::Auto,
                ),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        ));

        // Горизонтальная полоса креста
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                width: Val::Px(18.0),
                height: Val::Px(2.0),
                margin: UiRect::new(
                    Val::Px(-9.0),
                    Val::Auto,
                    Val::Px(-1.0),
                    Val::Auto,
                ),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        ));
    });

    // ---- Левый верхний угол: координаты + FPS + версия ----
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(12.0),
        top: Val::Px(10.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|parent| {
        // Координаты
        parent.spawn((
            Text::new("XYZ: 0.0 / 0.0 / 0.0"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            CoordsText,
        ));

        // FPS
        parent.spawn((
            Text::new("FPS: --"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.95, 0.65)),
            FpsText,
        ));

        // Версия (берётся из Cargo.toml автоматически)
        parent.spawn((
            Text::new(format!("Minecraft Rust v{}", GAME_VERSION)),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
        ));
    });

    // ---- Правый верхний угол: подсказки ----
    commands.spawn((
        Text::new(
            "WASD / Arrows - move\n\
             Space - jump   Ctrl - sprint   Shift - crouch\n\
             F - fly   LMB - break   RMB - place\n\
             1-9 / MouseWheel - slot   Esc - exit"
        ),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(10.0),
            ..default()
        },
        TextLayout::new_with_justify(JustifyText::Right),
    ));
}

fn update_coords(
    player_q: Query<&Transform, With<Player>>,
    mut text_q: Query<&mut Text, With<CoordsText>>,
) {
    let Ok(tf) = player_q.get_single() else { return; };
    let pos = tf.translation;

    for mut text in text_q.iter_mut() {
        text.0 = format!(
            "XYZ: {:.1} / {:.1} / {:.1}",
            pos.x, pos.y, pos.z
        );
    }
}

fn update_fps(
    diagnostics: Res<DiagnosticsStore>,
    mut text_q: Query<&mut Text, With<FpsText>>,
) {
    let Some(fps_diag) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) else {
        return;
    };
    let Some(value) = fps_diag.smoothed() else {
        return;
    };

    for mut text in text_q.iter_mut() {
        text.0 = format!("FPS: {:.0}", value);
    }
}