use bevy::prelude::*;
use bevy::ecs::world::World;
use crate::combat::components::Health;
use crate::enemies::components::Enemy;
use crate::states::*;
use crate::ui::components::*;
use crate::player::components::*;
use crate::weapon::components::*;
use crate::inventory::{Inventory, EquippedWeapon};

/// Resource to track debug HUD visibility
#[derive(Resource, Default)]
pub struct DebugHudVisible(pub bool);

pub fn setup_intro(
    mut commands: Commands,
    camera_query: Query<Entity, With<Camera>>,
) {
    // Reuse existing camera if available, otherwise spawn new one
    if camera_query.is_empty() {
        commands.spawn(Camera2d);
    }
    // Create UI elements (camera is reused if it exists)

    // Spawn basic UI root
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    ))
    .with_children(|parent| {
        // Title
        parent.spawn((
            Text::new("Donny Tango: Survivor"),
            TextFont {
                font_size: 60.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));

        // Menu container
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(50.0)),
                ..default()
            },
        ))
        .with_children(|menu| {
            // Start Game button
            menu.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                MenuButton,
                StartGameButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Start Game"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            // Exit Game button
            menu.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.6, 0.2, 0.2)),
                MenuButton,
                ExitGameButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Exit Game"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

#[allow(clippy::type_complexity)]
pub fn button_interactions(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&StartGameButton>,
            Option<&ExitGameButton>,
        ),
        (Changed<Interaction>, With<MenuButton>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, mut background_color, start_button, exit_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if start_button.is_some() {
                    next_state.set(GameState::InGame);
                } else if exit_button.is_some() {
                    app_exit.write(AppExit::Success);
                }
            }
            Interaction::Hovered => {
                *background_color = BackgroundColor(Color::srgb(0.4, 0.4, 0.4));
            }
            Interaction::None => {
                if start_button.is_some() {
                    *background_color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                } else if exit_button.is_some() {
                    *background_color = BackgroundColor(Color::srgb(0.6, 0.2, 0.2));
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn cleanup_intro(
    mut commands: Commands,
    ui_query: Query<Entity, Or<(With<MenuButton>, With<Node>, With<Text>)>>,
    camera_query: Query<Entity, With<Camera2d>>,
) {
    // Clean up UI elements
    let entities: Vec<Entity> = ui_query.iter().collect();
    for entity in entities {
        commands.queue(move |world: &mut World| {
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }

    // Despawn Camera2d so that setup_game can spawn Camera3d with proper lighting
    for entity in camera_query.iter() {
        commands.queue(move |world: &mut World| {
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }
}

pub fn setup_game_ui(
    mut commands: Commands,
) {
    // Create UI root for game HUD
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ))
    .with_children(|parent| {
        // Screen tint overlay (initially transparent)
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ScreenTint,
        ));
        // Health display container (top left)
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|health_container| {
            // Health text
            health_container.spawn((
                Text::new("Health: 100"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                HealthDisplay,
            ));

            // Health bar background
            health_container.spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(20.0),
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)), // Dark gray background
            ))
            .with_children(|bar_container| {
                // Health bar fill
                bar_container.spawn((
                    Node {
                        width: Val::Percent(100.0), // Will be updated by system
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.0, 1.0, 0.0)), // Green fill
                    HealthBar,
                ));
            });

            // Level display
            health_container.spawn((
                Text::new("Lv. 1"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)), // Yellow for level
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
                crate::experience::components::PlayerLevelDisplay,
            ));

            // XP Progress Bar Container
            health_container.spawn((
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(8.0),
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                XpProgressBar,
            ))
            .with_children(|bar_container| {
                // XP Progress Bar Fill
                bar_container.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.8, 1.0)), // Light blue for XP
                    XpProgressBarFill,
                ));
            });
        });

        // Game Level display (top center)
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Percent(50.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|game_level_container| {
            // Game level text
            game_level_container.spawn((
                Text::new("Game Level: 1"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                GameLevelDisplay,
            ));

            // Kill progress text
            game_level_container.spawn((
                Text::new("Kills: 0/10"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
                KillProgressDisplay,
            ));
        });
    });
}

pub fn update_screen_tint(
    screen_tint_effect: Res<crate::game::resources::ScreenTintEffect>,
    mut tint_query: Query<&mut BackgroundColor, With<ScreenTint>>,
) {
    for mut background_color in &mut tint_query {
        *background_color = BackgroundColor(screen_tint_effect.color);
    }
}

pub fn update_health_display(
    player_query: Query<&Health, With<Player>>,
    mut health_text_query: Query<&mut Text, (With<HealthDisplay>, Without<HealthBar>)>,
    mut health_bar_query: Query<(&mut Node, &mut BackgroundColor), With<HealthBar>>,
) {
    if let Ok(health) = player_query.single() {
        // Update health text
        for mut text in &mut health_text_query {
            *text = Text::new(format!("Health: {:.0}", health.current));
        }

        // Update health bar width and color
        let health_percentage = health.percentage();
        let bar_color = if health.current > 60.0 {
            Color::srgb(0.0, 1.0, 0.0) // Green
        } else if health.current > 30.0 {
            Color::srgb(1.0, 1.0, 0.0) // Yellow
        } else {
            Color::srgb(1.0, 0.0, 0.0) // Red
        };

        for (mut node, mut background_color) in &mut health_bar_query {
            node.width = Val::Percent(health_percentage * 100.0);
            *background_color = BackgroundColor(bar_color);
        }
    }
}

pub fn update_game_level_display(
    game_level: Res<crate::game::resources::GameLevel>,
    mut query: Query<&mut Text, With<GameLevelDisplay>>,
) {
    if game_level.is_changed() {
        for mut text in query.iter_mut() {
            **text = format!("Game Level: {}", game_level.level);
        }
    }
}

pub fn update_kill_progress_display(
    game_level: Res<crate::game::resources::GameLevel>,
    mut query: Query<&mut Text, With<KillProgressDisplay>>,
) {
    if game_level.is_changed() {
        for mut text in query.iter_mut() {
            **text = format!(
                "Kills: {}/{}",
                game_level.kills_this_level,
                game_level.kills_to_advance()
            );
        }
    }
}

pub fn update_xp_progress_bar(
    player_query: Query<&crate::experience::components::PlayerExperience, With<Player>>,
    mut bar_query: Query<&mut Node, With<XpProgressBarFill>>,
) {
    if let Ok(exp) = player_query.single() {
        for mut node in bar_query.iter_mut() {
            node.width = Val::Percent(exp.progress() * 100.0);
        }
    }
}

pub fn setup_game_over_ui(
    mut commands: Commands,
    score: Res<crate::score::Score>,
) {
    // Create game over UI
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)), // Semi-transparent black overlay
    ))
    .with_children(|parent| {
        // Game Over title
        parent.spawn((
            Text::new("Game Over"),
            TextFont {
                font_size: 60.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(30.0)),
                ..default()
            },
        ));

        // Final score
        parent.spawn((
            Text::new(format!("Final Score: {}", score.0)),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(50.0)),
                ..default()
            },
        ));

        // Restart instruction
        parent.spawn((
            Text::new("Press R to restart or ESC for menu"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ));
    });
}

pub fn setup_weapon_slots(
    mut commands: Commands,
) {
    // Create weapon slots container at bottom center
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Percent(50.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.0),
            ..default()
        },
    ))
    .with_children(|container| {
        // Create slots for each weapon type (pistol, laser, rocket_launcher)
        let weapon_types = [
            WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
            WeaponType::Laser,
            WeaponType::RocketLauncher,
        ];

        for (i, weapon_type) in weapon_types.iter().enumerate() {
            container.spawn((
                Node {
                    width: Val::Px(50.0),  // Larger to accommodate level text
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                WeaponSlot { slot_index: i },
            ))
            .with_children(|slot| {
                // Weapon icon
                slot.spawn((
                    Node {
                        width: Val::Px(30.0),
                        height: Val::Px(30.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.5)), // Initially gray/transparent
                    WeaponIcon { weapon_type: weapon_type.clone() },
                ));

                // Level text
                slot.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    WeaponLevelDisplay { weapon_type: weapon_type.clone() },
                ));

                if i == 0 {
                    // Circular timer overlay only for the first weapon (pistol)
                    slot.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(35.0),
                            height: Val::Px(35.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)), // Dark background ring
                        WeaponTimer,
                    ))
                    .with_children(|timer_container| {
                        // Inner fill circle that grows with progress
                        timer_container.spawn((
                            Node {
                                width: Val::Px(25.0), // Will be scaled
                                height: Val::Px(25.0), // Will be scaled
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.5, 0.7, 1.0, 0.0)), // Initially transparent
                            WeaponTimerFill,
                        ));
                    });
                }
            });
        }
    });
}

pub fn update_weapon_slots(
    time: Res<Time>,
    weapon_query: Query<(&Weapon, &EquippedWeapon)>,
    mut fill_query: Query<&mut BackgroundColor, With<WeaponTimerFill>>,
    mut size_query: Query<&mut Node, With<WeaponTimerFill>>,
) {
    // Find the pistol weapon for the timer display
    for (weapon, equipped) in weapon_query.iter() {
        if matches!(equipped.weapon_type, WeaponType::Pistol { .. }) {
            let time_since_fired = time.elapsed_secs() - weapon.last_fired;
            let progress = (time_since_fired / weapon.fire_rate).clamp(0.0, 1.0);

            // Circular progress timer: scale the inner circle and change color based on progress
            let min_size = 0.0;
            let max_size = 25.0;
            let current_size = min_size + (max_size - min_size) * progress;

            // Update size
            for mut node in size_query.iter_mut() {
                node.width = Val::Px(current_size);
                node.height = Val::Px(current_size);
            }

            // Update color alpha based on progress
            let alpha = progress * 0.8; // Max 80% opacity
            for mut bg_color in fill_query.iter_mut() {
                *bg_color = BackgroundColor(Color::srgba(0.5, 0.7, 1.0, alpha));
            }
            break; // Only update for the first weapon found in slot 0
        }
    }
}

pub fn update_weapon_icons(
    inventory: Res<Inventory>,
    mut icon_query: Query<(&mut BackgroundColor, &WeaponIcon)>,
) {
    for (mut bg_color, icon) in icon_query.iter_mut() {
        if let Some(weapon) = inventory.get_weapon(&icon.weapon_type) {
            // Set color based on weapon type
            *bg_color = BackgroundColor(match weapon.weapon_type {
                WeaponType::Pistol { .. } => Color::srgb(1.0, 1.0, 0.0), // Yellow for pistol
                WeaponType::Laser => Color::srgb(0.0, 0.0, 1.0), // Blue for laser
                WeaponType::RocketLauncher => Color::srgb(1.0, 0.5, 0.0), // Orange for rocket launcher
                _ => Color::srgb(0.5, 0.5, 0.5), // Gray for other weapons
            });
        } else {
            // No weapon of this type - make it transparent
            *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.3));
        }
    }
}

pub fn update_weapon_level_displays(
    inventory: Res<Inventory>,
    mut text_query: Query<(&mut Text, &WeaponLevelDisplay)>,
) {
    for (mut text, display) in text_query.iter_mut() {
        if let Some(weapon) = inventory.get_weapon(&display.weapon_type) {
            **text = format!("Lv.{}", weapon.level);
        } else {
            **text = "".to_string();
        }
    }
}

pub fn game_over_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        // Restart game - go back to InGame state
        next_state.set(GameState::InGame);
    } else if keyboard_input.just_pressed(KeyCode::Escape) {
        // Go back to intro menu
        next_state.set(GameState::Intro);
    }
}

#[allow(clippy::type_complexity)]
pub fn cleanup_game_over(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Node>, With<Text>)>>,
) {
    // Clean up game over UI elements and any remaining InGame UI
    let entities: Vec<Entity> = query.iter().collect();
    for entity in entities {
        commands.queue(move |world: &mut World| {
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }
}

/// Setup debug HUD (hidden by default, toggle with D key)
pub fn setup_debug_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            right: Val::Px(20.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        Visibility::Hidden, // Hidden by default
        DebugHud,
    ))
    .with_children(|parent| {
        // Title
        parent.spawn((
            Text::new("DEBUG"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 0.0)),
            Node {
                margin: UiRect::bottom(Val::Px(5.0)),
                ..default()
            },
        ));

        // Player position
        parent.spawn((
            Text::new("Player: (0.0, 0.0, 0.0)"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            DebugPlayerPosition,
        ));

        // Camera position
        parent.spawn((
            Text::new("Camera: (0.0, 0.0, 0.0)"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            DebugCameraPosition,
        ));

        // Enemy count
        parent.spawn((
            Text::new("Enemies: 0"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            DebugEnemyCount,
        ));

        // FPS display
        parent.spawn((
            Text::new("FPS: 0"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            DebugFpsDisplay,
        ));
    });
}

/// Toggle debug HUD visibility with D key
pub fn toggle_debug_hud(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut debug_visible: ResMut<DebugHudVisible>,
    mut hud_query: Query<&mut Visibility, With<DebugHud>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyD) {
        debug_visible.0 = !debug_visible.0;
        for mut visibility in hud_query.iter_mut() {
            *visibility = if debug_visible.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Update debug HUD with current values
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_debug_hud(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    camera_query: Query<&Transform, With<Camera>>,
    enemy_query: Query<&Enemy>,
    mut player_text: Query<&mut Text, (With<DebugPlayerPosition>, Without<DebugCameraPosition>, Without<DebugEnemyCount>, Without<DebugFpsDisplay>)>,
    mut camera_text: Query<&mut Text, (With<DebugCameraPosition>, Without<DebugPlayerPosition>, Without<DebugEnemyCount>, Without<DebugFpsDisplay>)>,
    mut enemy_text: Query<&mut Text, (With<DebugEnemyCount>, Without<DebugPlayerPosition>, Without<DebugCameraPosition>, Without<DebugFpsDisplay>)>,
    mut fps_text: Query<&mut Text, (With<DebugFpsDisplay>, Without<DebugPlayerPosition>, Without<DebugCameraPosition>, Without<DebugEnemyCount>)>,
) {
    // Update player position
    if let Ok(player_transform) = player_query.single() {
        let pos = player_transform.translation;
        for mut text in player_text.iter_mut() {
            **text = format!("Player: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
        }
    }

    // Update camera position
    if let Ok(camera_transform) = camera_query.single() {
        let pos = camera_transform.translation;
        for mut text in camera_text.iter_mut() {
            **text = format!("Camera: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
        }
    }

    // Update enemy count
    let enemy_count = enemy_query.iter().count();
    for mut text in enemy_text.iter_mut() {
        **text = format!("Enemies: {}", enemy_count);
    }

    // Update FPS
    let delta = time.delta_secs();
    let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };
    for mut text in fps_text.iter_mut() {
        **text = format!("FPS: {:.0}", fps);
    }
}

/// Check if debug HUD is visible (for run conditions)
pub fn debug_hud_enabled(debug_visible: Res<DebugHudVisible>) -> bool {
    debug_visible.0
}

/// Draw XYZ axis gizmos on all game entities when debug mode is on
pub fn draw_debug_axis_gizmos(
    mut gizmos: Gizmos,
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<&Transform, With<Enemy>>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    let axis_length = 1.0; // Double the previous length for visibility

    // Draw axes for player (slightly longer)
    for transform in player_query.iter() {
        draw_axes(&mut gizmos, transform.translation, axis_length * 1.5);
    }

    // Draw axes for enemies
    for transform in enemy_query.iter() {
        draw_axes(&mut gizmos, transform.translation, axis_length);
    }

    // Draw axes for camera
    for transform in camera_query.iter() {
        draw_axes(&mut gizmos, transform.translation, axis_length * 2.0);
    }
}

/// Configure gizmo line width on startup
pub fn configure_gizmos(mut config_store: ResMut<bevy::gizmos::config::GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<bevy::gizmos::config::DefaultGizmoConfigGroup>();
    config.line.width = 5.0; // 5 pixels wide
}

/// Helper to draw RGB XYZ axes at a position
fn draw_axes(gizmos: &mut Gizmos, position: Vec3, length: f32) {
    // X axis - Red
    gizmos.line(
        position,
        position + Vec3::X * length,
        Color::srgb(1.0, 0.0, 0.0),
    );
    // Y axis - Green
    gizmos.line(
        position,
        position + Vec3::Y * length,
        Color::srgb(0.0, 1.0, 0.0),
    );
    // Z axis - Blue
    gizmos.line(
        position,
        position + Vec3::Z * length,
        Color::srgb(0.0, 0.0, 1.0),
    );
}

/// Setup the level complete screen UI
pub fn setup_level_complete_screen(
    mut commands: Commands,
    game_level: Res<crate::game::resources::GameLevel>,
    level_stats: Res<crate::game::resources::LevelStats>,
) {
    // Root container for entire level complete UI
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        LevelCompleteScreen,
    )).with_children(|parent| {
        // Black overlay background (animates opacity)
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            LevelCompleteOverlay::default(),
        ));

        // Content container (centered)
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(40.0)),
                ..default()
            },
            ZIndex(1), // Above overlay
        )).with_children(|content| {
            // "Level X Complete" title - show level that was just completed (current - 1)
            let completed_level = if game_level.level > 1 { game_level.level - 1 } else { 1 };
            content.spawn((
                Text::new(format!("Level {} Complete!", completed_level)),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.0)), // Gold
            ));

            // Stats container
            content.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Start,
                    row_gap: Val::Px(10.0),
                    margin: UiRect::vertical(Val::Px(20.0)),
                    ..default()
                },
            )).with_children(|stats| {
                // Time taken
                stats.spawn((
                    Text::new(format!("Time: {}", level_stats.formatted_time())),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                // Enemies killed
                stats.spawn((
                    Text::new(format!("Enemies Killed: {}", level_stats.enemies_killed)),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                // XP gained
                stats.spawn((
                    Text::new(format!("XP Gained: {}", level_stats.xp_gained)),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            // Continue button
            content.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(40.0), Val::Px(15.0)),
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                Button,
                ContinueButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("Continue"),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

/// Animate the level complete overlay (fade in)
pub fn animate_level_complete_overlay(
    time: Res<Time>,
    mut query: Query<(&mut LevelCompleteOverlay, &mut BackgroundColor)>,
) {
    for (mut overlay, mut bg_color) in query.iter_mut() {
        if overlay.current_opacity < overlay.target_opacity {
            overlay.current_opacity += time.delta_secs() * overlay.animation_speed;
            overlay.current_opacity = overlay.current_opacity.min(overlay.target_opacity);
            bg_color.0 = Color::srgba(0.0, 0.0, 0.0, overlay.current_opacity);
        }
    }
}

/// Handle continue button interaction
#[allow(clippy::type_complexity)]
pub fn handle_continue_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ContinueButton>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    enemies_query: Query<Entity, With<Enemy>>,
    loot_query: Query<Entity, With<crate::loot::components::DroppedItem>>,
    mut level_stats: ResMut<crate::game::resources::LevelStats>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    for (interaction, mut bg_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                // Despawn all enemies
                let enemies: Vec<Entity> = enemies_query.iter().collect();
                for entity in enemies {
                    commands.queue(move |world: &mut World| {
                        if world.get_entity(entity).is_ok() {
                            let _ = world.despawn(entity);
                        }
                    });
                }

                // Despawn all loot
                let loot: Vec<Entity> = loot_query.iter().collect();
                for entity in loot {
                    commands.queue(move |world: &mut World| {
                        if world.get_entity(entity).is_ok() {
                            let _ = world.despawn(entity);
                        }
                    });
                }

                // Reset player position to center
                for mut transform in player_query.iter_mut() {
                    transform.translation = Vec3::ZERO;
                }

                // Reset level stats for new level
                level_stats.reset();

                // Return to game
                next_state.set(GameState::InGame);
            }
            Interaction::Hovered => {
                bg_color.0 = Color::srgb(0.3, 0.7, 0.3);
            }
            Interaction::None => {
                bg_color.0 = Color::srgb(0.2, 0.6, 0.2);
            }
        }
    }
}

/// Play level complete sound when entering LevelComplete state
pub fn play_level_complete_sound(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/790472__organizedlaziness__level-completed.wav")),
        PlaybackSettings::DESPAWN,
    ));
}

/// Cleanup the level complete screen when exiting the state
pub fn cleanup_level_complete_screen(
    mut commands: Commands,
    query: Query<Entity, With<LevelCompleteScreen>>,
) {
    let entities: Vec<Entity> = query.iter().collect();
    for entity in entities {
        commands.queue(move |world: &mut World| {
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::resources::GameLevel;
    use crate::experience::components::PlayerExperience;

    mod update_game_level_display_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.insert_resource(GameLevel::new());
            app
        }

        #[test]
        fn updates_text_when_game_level_changes() {
            let mut app = setup_test_app();

            // Spawn a text entity with GameLevelDisplay marker
            app.world_mut().spawn((
                Text::new("Game Level: 1"),
                GameLevelDisplay,
            ));

            // Change game level
            app.world_mut().resource_mut::<GameLevel>().level = 5;

            // Run the system
            app.add_systems(Update, update_game_level_display);
            app.update();

            // Verify text updated
            let text = app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .next()
                .unwrap();
            assert!(text.0.contains("5"), "Text should contain level 5, got: {}", text.0);
        }
    }

    mod update_kill_progress_display_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.insert_resource(GameLevel::new());
            app
        }

        #[test]
        fn updates_text_with_kill_count() {
            let mut app = setup_test_app();

            // Spawn a text entity with KillProgressDisplay marker
            app.world_mut().spawn((
                Text::new("Kills: 0/10"),
                KillProgressDisplay,
            ));

            // Add some kills
            {
                let mut game_level = app.world_mut().resource_mut::<GameLevel>();
                game_level.kills_this_level = 5;
            }

            // Run the system
            app.add_systems(Update, update_kill_progress_display);
            app.update();

            // Verify text updated
            let text = app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .next()
                .unwrap();
            assert!(text.0.contains("5/10"), "Text should contain 5/10, got: {}", text.0);
        }
    }

    mod update_xp_progress_bar_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app
        }

        #[test]
        fn updates_bar_width_based_on_progress() {
            let mut app = setup_test_app();

            // Spawn a player with experience at 50% progress
            let mut exp = PlayerExperience::new();
            exp.current = 50; // 50 out of 100 for level 1
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 0.0,
                    pickup_radius: 0.0,
                    last_movement_direction: Vec3::ZERO,
                },
                exp,
            ));

            // Spawn XP progress bar fill
            app.world_mut().spawn((
                Node {
                    width: Val::Percent(0.0),
                    ..default()
                },
                XpProgressBarFill,
            ));

            // Run the system
            app.add_systems(Update, update_xp_progress_bar);
            app.update();

            // Verify bar width updated to 50%
            let node = app.world_mut()
                .query::<&Node>()
                .iter(app.world())
                .next()
                .unwrap();
            if let Val::Percent(width) = node.width {
                assert!((width - 50.0).abs() < 0.1, "Width should be ~50%, got: {}", width);
            } else {
                panic!("Width should be a percentage");
            }
        }
    }

    mod progression_ui_components_tests {
        use super::*;

        #[test]
        fn game_level_display_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<GameLevelDisplay>();
        }

        #[test]
        fn kill_progress_display_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<KillProgressDisplay>();
        }

        #[test]
        fn xp_progress_bar_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<XpProgressBar>();
        }

        #[test]
        fn xp_progress_bar_fill_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<XpProgressBarFill>();
        }
    }

    mod level_complete_tests {
        use super::*;
        use crate::game::resources::LevelStats;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.init_state::<GameState>();
            app.insert_resource(GameLevel::new());
            app.insert_resource(LevelStats::new());
            app
        }

        #[test]
        fn animate_overlay_increases_opacity() {
            use std::time::Duration;

            let mut app = App::new();
            app.init_resource::<Time>();

            // Spawn overlay with default values
            app.world_mut().spawn((
                LevelCompleteOverlay::default(),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.25));
            }

            // Run the animation system
            let _ = app.world_mut().run_system_once(animate_level_complete_overlay);

            // Check opacity increased
            let (overlay, bg) = app.world_mut()
                .query::<(&LevelCompleteOverlay, &BackgroundColor)>()
                .iter(app.world())
                .next()
                .unwrap();

            assert!(overlay.current_opacity > 0.0, "Opacity should have increased");
            // BackgroundColor alpha should match
            if let Color::Srgba(color) = bg.0 {
                assert!(color.alpha > 0.0, "Background alpha should have increased");
            }
        }

        #[test]
        fn animate_overlay_caps_at_target() {
            use std::time::Duration;

            let mut app = App::new();
            app.init_resource::<Time>();

            // Spawn overlay already at 80% opacity
            app.world_mut().spawn((
                LevelCompleteOverlay {
                    target_opacity: 0.85,
                    current_opacity: 0.8,
                    animation_speed: 2.0,
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ));

            // Advance time by 1 second (way more than needed)
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            // Run the animation system
            let _ = app.world_mut().run_system_once(animate_level_complete_overlay);

            // Check opacity capped at target
            let overlay = app.world_mut()
                .query::<&LevelCompleteOverlay>()
                .iter(app.world())
                .next()
                .unwrap();

            assert_eq!(overlay.current_opacity, 0.85, "Opacity should cap at target");
        }

        #[test]
        fn cleanup_level_complete_screen_removes_screen_entity() {
            let mut app = App::new();

            // Spawn level complete screen
            let screen = app.world_mut().spawn((
                Node::default(),
                LevelCompleteScreen,
            )).id();

            // Verify it exists
            assert!(app.world().get_entity(screen).is_ok());

            // Run cleanup system
            app.add_systems(Update, cleanup_level_complete_screen);
            app.update();

            // Screen should be despawned
            assert!(app.world().get_entity(screen).is_err(), "Screen should be despawned");
        }

        #[test]
        fn setup_level_complete_screen_spawns_ui() {
            let mut app = setup_test_app();

            // Set some stats
            {
                let mut stats = app.world_mut().resource_mut::<LevelStats>();
                stats.time_elapsed = 65.0; // 1:05
                stats.enemies_killed = 25;
                stats.xp_gained = 500;
            }
            {
                let mut level = app.world_mut().resource_mut::<GameLevel>();
                level.level = 2; // Just advanced to level 2
            }

            // Run setup system
            app.add_systems(Update, setup_level_complete_screen);
            app.update();

            // Check that LevelCompleteScreen was spawned
            let screen_count = app.world_mut()
                .query::<&LevelCompleteScreen>()
                .iter(app.world())
                .count();
            assert_eq!(screen_count, 1, "Should spawn exactly one LevelCompleteScreen");

            // Check that overlay was spawned
            let overlay_count = app.world_mut()
                .query::<&LevelCompleteOverlay>()
                .iter(app.world())
                .count();
            assert_eq!(overlay_count, 1, "Should spawn exactly one LevelCompleteOverlay");

            // Check that continue button was spawned
            let button_count = app.world_mut()
                .query::<&ContinueButton>()
                .iter(app.world())
                .count();
            assert_eq!(button_count, 1, "Should spawn exactly one ContinueButton");
        }

        #[test]
        fn level_complete_shows_correct_level() {
            let mut app = setup_test_app();

            // Set level to 3 (just advanced from 2)
            {
                let mut level = app.world_mut().resource_mut::<GameLevel>();
                level.level = 3;
            }

            // Run setup
            app.add_systems(Update, setup_level_complete_screen);
            app.update();

            // Find text that contains "Level 2 Complete"
            let texts: Vec<_> = app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .collect();

            let has_correct_text = texts.iter().any(|t| t.0.contains("Level 2 Complete"));
            assert!(has_correct_text, "Should show 'Level 2 Complete!' for completing level 2");
        }
    }
}