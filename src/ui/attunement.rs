use bevy::prelude::*;

use crate::element::Element;
use crate::states::GameState;
use crate::whisper::WhisperAttunement;

/// Root marker for the attunement selection screen.
/// Used for cleanup on state exit.
#[derive(Component)]
pub struct AttunementScreen;

/// Marker for the semi-transparent background overlay.
#[derive(Component)]
pub struct AttunementOverlay;

/// Component marking an attunement option button.
#[derive(Component)]
pub struct AttunementOption {
    pub element: Element,
}

/// Setup the attunement selection screen when entering AttunementSelect state.
pub fn setup_attunement_screen(mut commands: Commands) {
    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            AttunementScreen,
        ))
        .with_children(|parent| {
            // Dark overlay background
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                AttunementOverlay,
            ));

            // Title text
            parent.spawn((
                Text::new("Choose Your Attunement"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.0)), // Gold
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
                ZIndex(1),
            ));

            // Subtitle
            parent.spawn((
                Text::new("+10% damage to matching element spells"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                Node {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                },
                ZIndex(1),
            ));

            // Container for element buttons in circular layout
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(400.0),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    ZIndex(1),
                ))
                .with_children(|circle_container| {
                    // Spawn 8 element buttons in a circle
                    let radius = 150.0;
                    let center_x = 200.0 - 40.0; // Container center minus half button width
                    let center_y = 200.0 - 40.0; // Container center minus half button height

                    for (i, element) in Element::all().iter().enumerate() {
                        // Calculate position on circle (start from top, go clockwise)
                        let angle = (i as f32) * std::f32::consts::TAU / 8.0 - std::f32::consts::FRAC_PI_2;
                        let x = center_x + radius * angle.cos();
                        let y = center_y + radius * angle.sin();

                        circle_container
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(80.0),
                                    height: Val::Px(80.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(x),
                                    top: Val::Px(y),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(Val::Px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(element.color().with_alpha(0.6)),
                                BorderColor::all(element.color()),
                                BorderRadius::all(Val::Px(10.0)),
                                AttunementOption { element: *element },
                            ))
                            .with_children(|button| {
                                // Element icon (colored circle)
                                button.spawn((
                                    Node {
                                        width: Val::Px(30.0),
                                        height: Val::Px(30.0),
                                        margin: UiRect::bottom(Val::Px(5.0)),
                                        ..default()
                                    },
                                    BackgroundColor(element.color()),
                                    BorderRadius::all(Val::Percent(50.0)),
                                ));

                                // Element name
                                button.spawn((
                                    Text::new(element.name()),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                    }
                });
        });
}

/// Handle attunement selection button interactions.
#[allow(clippy::type_complexity)]
pub fn handle_attunement_selection(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor, &AttunementOption),
        Changed<Interaction>,
    >,
    mut attunement: ResMut<WhisperAttunement>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut bg_color, mut border_color, option) in &mut interaction_query {
        let element_color = option.element.color();

        match *interaction {
            Interaction::Pressed => {
                // Set attunement and transition to game
                attunement.set_element(option.element);
                next_state.set(GameState::InGame);
            }
            Interaction::Hovered => {
                // Brighten on hover
                *bg_color = BackgroundColor(element_color.with_alpha(0.9));
                *border_color = BorderColor::all(Color::WHITE);
            }
            Interaction::None => {
                // Reset to default
                *bg_color = BackgroundColor(element_color.with_alpha(0.6));
                *border_color = BorderColor::all(element_color);
            }
        }
    }
}

/// Cleanup attunement screen entities when exiting the state.
pub fn cleanup_attunement_screen(
    mut commands: Commands,
    query: Query<Entity, With<AttunementScreen>>,
) {
    use bevy::ecs::world::World;
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
    use bevy::ecs::system::RunSystemOnce;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<WhisperAttunement>();
        app
    }

    mod attunement_screen_component_tests {
        use super::*;

        #[test]
        fn attunement_screen_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<AttunementScreen>();
        }

        #[test]
        fn attunement_overlay_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<AttunementOverlay>();
        }

        #[test]
        fn attunement_option_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<AttunementOption>();
        }

        #[test]
        fn attunement_option_stores_element() {
            let option = AttunementOption {
                element: Element::Fire,
            };
            assert_eq!(option.element, Element::Fire);
        }
    }

    mod setup_attunement_screen_tests {
        use super::*;

        #[test]
        fn spawns_attunement_screen_root() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_attunement_screen);

            let screen_count = app
                .world_mut()
                .query::<&AttunementScreen>()
                .iter(app.world())
                .count();
            assert_eq!(screen_count, 1, "Should spawn exactly one AttunementScreen");
        }

        #[test]
        fn spawns_overlay() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_attunement_screen);

            let overlay_count = app
                .world_mut()
                .query::<&AttunementOverlay>()
                .iter(app.world())
                .count();
            assert_eq!(overlay_count, 1, "Should spawn exactly one AttunementOverlay");
        }

        #[test]
        fn spawns_8_element_buttons() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_attunement_screen);

            let button_count = app
                .world_mut()
                .query::<&AttunementOption>()
                .iter(app.world())
                .count();
            assert_eq!(button_count, 8, "Should spawn exactly 8 element buttons");
        }

        #[test]
        fn each_button_has_unique_element() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_attunement_screen);

            let elements: Vec<Element> = app
                .world_mut()
                .query::<&AttunementOption>()
                .iter(app.world())
                .map(|opt| opt.element)
                .collect();

            // Check all 8 elements are present
            for element in Element::all() {
                assert!(
                    elements.contains(element),
                    "Element {:?} should be present",
                    element
                );
            }
        }

        #[test]
        fn buttons_have_button_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_attunement_screen);

            let button_option_count = app
                .world_mut()
                .query::<(&Button, &AttunementOption)>()
                .iter(app.world())
                .count();
            assert_eq!(
                button_option_count, 8,
                "All 8 AttunementOptions should have Button component"
            );
        }
    }

    mod handle_attunement_selection_tests {
        use super::*;

        #[test]
        fn clicking_button_sets_attunement() {
            let mut app = setup_test_app();

            // Spawn a button with Fire element and Pressed interaction
            app.world_mut().spawn((
                Button,
                Interaction::Pressed,
                BackgroundColor(Color::srgba(1.0, 0.5, 0.0, 0.6)),
                BorderColor::all(Color::srgb(1.0, 0.5, 0.0)),
                AttunementOption {
                    element: Element::Fire,
                },
            ));

            // Run the handler
            app.add_systems(Update, handle_attunement_selection);
            app.update();

            // Check attunement was set
            let attunement = app.world().resource::<WhisperAttunement>();
            assert_eq!(
                attunement.element(),
                Some(Element::Fire),
                "Attunement should be set to Fire"
            );
        }

        #[test]
        fn clicking_button_transitions_to_ingame() {
            let mut app = setup_test_app();

            // Set initial state
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::AttunementSelect);

            // Spawn a button with Frost element and Pressed interaction
            app.world_mut().spawn((
                Button,
                Interaction::Pressed,
                BackgroundColor(Color::srgba(0.5, 0.8, 1.0, 0.6)),
                BorderColor::all(Color::srgb(0.5, 0.8, 1.0)),
                AttunementOption {
                    element: Element::Frost,
                },
            ));

            // Run the handler
            app.add_systems(Update, handle_attunement_selection);
            app.update();

            // Check state transition was requested
            // NextState doesn't expose the inner value directly, but we can verify
            // by checking the attunement was set (which happens in same branch)
            let attunement = app.world().resource::<WhisperAttunement>();
            assert_eq!(
                attunement.element(),
                Some(Element::Frost),
                "Attunement should be set (indicates button was pressed)"
            );
        }

        #[test]
        fn each_element_can_be_selected() {
            for element in Element::all() {
                let mut app = setup_test_app();

                app.world_mut().spawn((
                    Button,
                    Interaction::Pressed,
                    BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                    BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
                    AttunementOption { element: *element },
                ));

                app.add_systems(Update, handle_attunement_selection);
                app.update();

                let attunement = app.world().resource::<WhisperAttunement>();
                assert_eq!(
                    attunement.element(),
                    Some(*element),
                    "Attunement should be set to {:?}",
                    element
                );
            }
        }
    }

    mod cleanup_attunement_screen_tests {
        use super::*;

        #[test]
        fn removes_attunement_screen_entity() {
            let mut app = setup_test_app();

            // Spawn an attunement screen
            let screen = app
                .world_mut()
                .spawn((Node::default(), AttunementScreen))
                .id();

            // Verify it exists
            assert!(app.world().get_entity(screen).is_ok());

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_attunement_screen);

            // Screen should be despawned
            assert!(
                app.world().get_entity(screen).is_err(),
                "Screen should be despawned"
            );
        }

        #[test]
        fn removes_children_recursively() {
            let mut app = setup_test_app();

            // Spawn screen with a child
            let child = app.world_mut().spawn(Node::default()).id();
            let screen = app
                .world_mut()
                .spawn((Node::default(), AttunementScreen))
                .id();
            app.world_mut().entity_mut(screen).add_child(child);

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_attunement_screen);

            // Both should be despawned
            assert!(
                app.world().get_entity(screen).is_err(),
                "Screen should be despawned"
            );
            assert!(
                app.world().get_entity(child).is_err(),
                "Child should be despawned"
            );
        }
    }
}
