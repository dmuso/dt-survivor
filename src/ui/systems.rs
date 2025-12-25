use bevy::prelude::*;
use bevy::ecs::world::World;
use crate::combat::components::Health;
use crate::states::*;
use crate::ui::components::*;
use crate::player::components::*;
use crate::weapon::components::*;
use crate::inventory::{Inventory, EquippedWeapon};

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
    query: Query<Entity, Or<(With<MenuButton>, With<Node>, With<Text>)>>,
) {
    // Clean up UI elements but preserve the camera
    let entities: Vec<Entity> = query.iter().collect();
    for entity in entities {
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