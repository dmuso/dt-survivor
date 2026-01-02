use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::arena::components::TorchLight;
use crate::enemies::components::Enemy;
use crate::loot::components::DroppedItem;
use crate::pause::components::*;
use crate::states::GameState;
use crate::ui::systems::DebugHudVisible;

/// Button colors
const BUTTON_GREEN: Color = Color::srgb(0.2, 0.6, 0.2);
const BUTTON_BLUE: Color = Color::srgb(0.2, 0.4, 0.7);
const BUTTON_RED: Color = Color::srgb(0.6, 0.2, 0.2);
const BUTTON_ORANGE: Color = Color::srgb(0.7, 0.4, 0.1);
const BUTTON_HOVER: Color = Color::srgb(0.4, 0.4, 0.4);

/// Sets up the pause menu UI
pub fn setup_pause_menu(
    mut commands: Commands,
    debug_visible: Res<DebugHudVisible>,
    wall_lights_enabled: Res<WallLightsEnabled>,
) {
    // Create pause menu UI root with semi-transparent overlay
    commands
        .spawn((
            PauseMenu,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        ))
        .with_children(|parent| {
            // PAUSED title
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Menu buttons container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(15.0),
                    ..default()
                })
                .with_children(|menu| {
                    // Continue button
                    spawn_menu_button(menu, "Continue", BUTTON_GREEN, ContinueButton);

                    // New Game button
                    spawn_menu_button(menu, "New Game", BUTTON_BLUE, NewGameButton);

                    // Exit Game button
                    spawn_menu_button(menu, "Exit Game", BUTTON_RED, ExitGameButton);
                });

            // Debug section (only visible if debug mode is on)
            if debug_visible.0 {
                parent
                    .spawn((
                        DebugSection,
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(40.0)),
                            row_gap: Val::Px(10.0),
                            ..default()
                        },
                    ))
                    .with_children(|debug| {
                        // Debug section title
                        debug.spawn((
                            Text::new("Debug Actions"),
                            TextFont {
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.3)),
                            Node {
                                margin: UiRect::bottom(Val::Px(10.0)),
                                ..default()
                            },
                        ));

                        // Debug buttons in a row
                        debug
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(10.0),
                                ..default()
                            })
                            .with_children(|row| {
                                spawn_debug_button(row, "Despawn Enemies", DespawnEnemiesButton);
                                spawn_debug_button(row, "Despawn Loot", DespawnLootButton);
                            });

                        debug
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(10.0),
                                ..default()
                            })
                            .with_children(|row| {
                                let lights_label = if wall_lights_enabled.0 { "Lights: ON" } else { "Lights: OFF" };
                                spawn_debug_button(row, lights_label, ToggleWallLightsButton);
                            });
                    });
            }
        });
}

/// Helper to spawn a main menu button
fn spawn_menu_button<T: Component>(parent: &mut ChildSpawnerCommands, label: &str, color: Color, marker: T) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(color),
            marker,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Helper to spawn a debug button (smaller, orange)
fn spawn_debug_button<T: Component>(parent: &mut ChildSpawnerCommands, label: &str, marker: T) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(160.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_ORANGE),
            marker,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Handles pause menu button interactions
#[allow(clippy::type_complexity)]
pub fn pause_menu_interactions(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&ContinueButton>,
            Option<&NewGameButton>,
            Option<&ExitGameButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, mut background_color, continue_btn, new_game_btn, exit_btn) in
        &mut interaction_query
    {
        match *interaction {
            Interaction::Pressed => {
                if continue_btn.is_some() {
                    next_state.set(GameState::InGame);
                } else if new_game_btn.is_some() {
                    // Go to Intro which will trigger cleanup, then immediately to InGame
                    next_state.set(GameState::Intro);
                } else if exit_btn.is_some() {
                    app_exit.write(AppExit::Success);
                }
            }
            Interaction::Hovered => {
                *background_color = BackgroundColor(BUTTON_HOVER);
            }
            Interaction::None => {
                if continue_btn.is_some() {
                    *background_color = BackgroundColor(BUTTON_GREEN);
                } else if new_game_btn.is_some() {
                    *background_color = BackgroundColor(BUTTON_BLUE);
                } else if exit_btn.is_some() {
                    *background_color = BackgroundColor(BUTTON_RED);
                }
            }
        }
    }
}

/// Handles debug button interactions
#[allow(clippy::type_complexity)]
pub fn debug_button_interactions(
    mut commands: Commands,
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&DespawnEnemiesButton>,
            Option<&DespawnLootButton>,
            Option<&ToggleWallLightsButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    enemies: Query<Entity, With<Enemy>>,
    loot: Query<Entity, With<DroppedItem>>,
    mut lights: Query<&mut Visibility, With<TorchLight>>,
    mut wall_lights_enabled: ResMut<WallLightsEnabled>,
) {
    for (interaction, mut background_color, despawn_enemies, despawn_loot, toggle_lights) in
        &mut interaction_query
    {
        match *interaction {
            Interaction::Pressed => {
                if despawn_enemies.is_some() {
                    for entity in enemies.iter() {
                        commands.entity(entity).despawn();
                    }
                } else if despawn_loot.is_some() {
                    for entity in loot.iter() {
                        commands.entity(entity).despawn();
                    }
                } else if toggle_lights.is_some() {
                    // Toggle wall lights visibility
                    wall_lights_enabled.0 = !wall_lights_enabled.0;
                    let new_visibility = if wall_lights_enabled.0 {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                    for mut visibility in lights.iter_mut() {
                        *visibility = new_visibility;
                    }
                }
            }
            Interaction::Hovered => {
                *background_color = BackgroundColor(BUTTON_HOVER);
            }
            Interaction::None => {
                if despawn_enemies.is_some()
                    || despawn_loot.is_some()
                    || toggle_lights.is_some()
                {
                    *background_color = BackgroundColor(BUTTON_ORANGE);
                }
            }
        }
    }
}

/// Updates toggle button text to reflect current state
pub fn update_toggle_button_text(
    wall_lights_enabled: Res<WallLightsEnabled>,
    lights_btn_query: Query<&Children, With<ToggleWallLightsButton>>,
    mut text_query: Query<&mut Text>,
) {
    // Update wall lights button text
    for children in lights_btn_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                let label = if wall_lights_enabled.0 { "Lights: ON" } else { "Lights: OFF" };
                if text.0 != label {
                    text.0 = label.to_string();
                }
            }
        }
    }
}

/// Handles ESC key to resume game from pause
pub fn pause_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::InGame);
    }
}

/// Handles ESC key to pause the game from InGame state
pub fn enter_pause_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Paused);
    }
}

/// Cleans up pause menu UI
pub fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));
        app.init_state::<GameState>();
        app.init_resource::<DebugHudVisible>();
        app.init_resource::<WallLightsEnabled>();
        app
    }

    #[test]
    fn setup_pause_menu_creates_pause_menu_entity() {
        let mut app = setup_test_app();

        let _ = app.world_mut().run_system_once(setup_pause_menu);

        let pause_menu_count = app
            .world_mut()
            .query::<&PauseMenu>()
            .iter(app.world())
            .count();
        assert_eq!(pause_menu_count, 1, "Should create one PauseMenu entity");
    }

    #[test]
    fn setup_pause_menu_creates_main_buttons() {
        let mut app = setup_test_app();

        let _ = app.world_mut().run_system_once(setup_pause_menu);

        let continue_count = app
            .world_mut()
            .query::<&ContinueButton>()
            .iter(app.world())
            .count();
        let new_game_count = app
            .world_mut()
            .query::<&NewGameButton>()
            .iter(app.world())
            .count();
        let exit_count = app
            .world_mut()
            .query::<&ExitGameButton>()
            .iter(app.world())
            .count();

        assert_eq!(continue_count, 1, "Should create ContinueButton");
        assert_eq!(new_game_count, 1, "Should create NewGameButton");
        assert_eq!(exit_count, 1, "Should create ExitGameButton");
    }

    #[test]
    fn setup_pause_menu_hides_debug_when_disabled() {
        let mut app = setup_test_app();

        let _ = app.world_mut().run_system_once(setup_pause_menu);

        let debug_section_count = app
            .world_mut()
            .query::<&DebugSection>()
            .iter(app.world())
            .count();
        assert_eq!(
            debug_section_count, 0,
            "Debug section should not be created when debug is disabled"
        );
    }

    #[test]
    fn setup_pause_menu_shows_debug_when_enabled() {
        let mut app = setup_test_app();
        app.world_mut().resource_mut::<DebugHudVisible>().0 = true;

        let _ = app.world_mut().run_system_once(setup_pause_menu);

        let debug_section_count = app
            .world_mut()
            .query::<&DebugSection>()
            .iter(app.world())
            .count();
        assert_eq!(
            debug_section_count, 1,
            "Debug section should be created when debug is enabled"
        );

        let despawn_enemies_count = app
            .world_mut()
            .query::<&DespawnEnemiesButton>()
            .iter(app.world())
            .count();
        let despawn_loot_count = app
            .world_mut()
            .query::<&DespawnLootButton>()
            .iter(app.world())
            .count();
        let toggle_lights_count = app
            .world_mut()
            .query::<&ToggleWallLightsButton>()
            .iter(app.world())
            .count();

        assert_eq!(despawn_enemies_count, 1, "Should create DespawnEnemiesButton");
        assert_eq!(despawn_loot_count, 1, "Should create DespawnLootButton");
        assert_eq!(toggle_lights_count, 1, "Should create ToggleWallLightsButton");
    }

    #[test]
    fn cleanup_pause_menu_removes_pause_menu() {
        let mut app = setup_test_app();

        // Create pause menu
        let _ = app.world_mut().run_system_once(setup_pause_menu);

        let before_count = app
            .world_mut()
            .query::<&PauseMenu>()
            .iter(app.world())
            .count();
        assert_eq!(before_count, 1, "Should have PauseMenu before cleanup");

        // Cleanup
        let _ = app.world_mut().run_system_once(cleanup_pause_menu);

        let after_count = app
            .world_mut()
            .query::<&PauseMenu>()
            .iter(app.world())
            .count();
        assert_eq!(after_count, 0, "PauseMenu should be removed after cleanup");
    }

}
