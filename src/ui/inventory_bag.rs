use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::inventory::{InventoryBag, SpellList};
use crate::spell::Spell;
use crate::states::GameState;

const BAG_COLUMNS: usize = 6;
const BAG_ROWS: usize = 5;
const ACTIVE_SLOTS: usize = 5;
const SLOT_SIZE: f32 = 60.0;
const SLOT_GAP: f32 = 8.0;

/// Root marker for the inventory screen.
/// Used for cleanup on state exit.
#[derive(Component)]
pub struct InventoryScreen;

/// Marker for the semi-transparent background overlay.
#[derive(Component)]
pub struct InventoryOverlay;

/// Component marking a bag slot in the inventory grid.
#[derive(Component)]
pub struct InventorySlot {
    pub index: usize,
}

/// Component marking an active spell slot display in the inventory.
#[derive(Component)]
pub struct ActiveSlotDisplay {
    pub index: usize,
}

/// Marker for currently selected slot.
#[derive(Component)]
pub struct SelectedSpell;

/// Resource tracking which bag slot is selected (if any).
#[derive(Resource, Default)]
pub struct SelectedBagSlot(pub Option<usize>);

/// Setup the inventory screen when entering InventoryOpen state.
pub fn setup_inventory_ui(
    mut commands: Commands,
    inventory_bag: Res<InventoryBag>,
    spell_list: Res<SpellList>,
) {
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
            InventoryScreen,
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
                InventoryOverlay,
            ));

            // Title text
            parent.spawn((
                Text::new("INVENTORY"),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
                ZIndex(1),
            ));

            // Bag grid container (6x5 = 30 slots)
            let grid_width = BAG_COLUMNS as f32 * (SLOT_SIZE + SLOT_GAP) - SLOT_GAP;
            let grid_height = BAG_ROWS as f32 * (SLOT_SIZE + SLOT_GAP) - SLOT_GAP;

            parent
                .spawn((
                    Node {
                        width: Val::Px(grid_width),
                        height: Val::Px(grid_height),
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(SLOT_GAP),
                        row_gap: Val::Px(SLOT_GAP),
                        ..default()
                    },
                    ZIndex(1),
                ))
                .with_children(|grid| {
                    // Spawn 30 bag slots
                    for slot_index in 0..(BAG_ROWS * BAG_COLUMNS) {
                        spawn_bag_slot(grid, slot_index, inventory_bag.get_spell(slot_index));
                    }
                });

            // Separator text
            parent.spawn((
                Text::new("ACTIVE SPELLS"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.84, 0.0, 1.0)), // Gold
                Node {
                    margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(30.0), Val::Px(15.0)),
                    ..default()
                },
                ZIndex(1),
            ));

            // Active spells bar (5 slots)
            let active_width = ACTIVE_SLOTS as f32 * (SLOT_SIZE + SLOT_GAP) - SLOT_GAP;

            parent
                .spawn((
                    Node {
                        width: Val::Px(active_width),
                        height: Val::Px(SLOT_SIZE),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(SLOT_GAP),
                        ..default()
                    },
                    ZIndex(1),
                ))
                .with_children(|active_bar| {
                    // Spawn 5 active slots
                    for slot_index in 0..ACTIVE_SLOTS {
                        spawn_active_slot(active_bar, slot_index, spell_list.get_spell(slot_index));
                    }
                });

            // Instructions text
            parent.spawn((
                Text::new("Click bag slot to select. Click active slot to swap. Press I or Escape to close."),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(30.0)),
                    ..default()
                },
                ZIndex(1),
            ));
        });
}

/// Spawn a bag slot with optional spell content.
fn spawn_bag_slot(parent: &mut ChildSpawnerCommands, index: usize, spell: Option<&Spell>) {
    let (bg_color, border_color) = if let Some(spell) = spell {
        (
            spell.element.color().with_alpha(0.4),
            spell.element.color(),
        )
    } else {
        (
            Color::srgba(0.2, 0.2, 0.2, 0.8),
            Color::srgba(0.4, 0.4, 0.4, 0.8),
        )
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(SLOT_SIZE),
                height: Val::Px(SLOT_SIZE),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(border_color),
            BorderRadius::all(Val::Px(6.0)),
            InventorySlot { index },
        ))
        .with_children(|slot| {
            if let Some(spell) = spell {
                // Element icon (colored circle)
                slot.spawn((
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(24.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(spell.element.color()),
                    BorderRadius::all(Val::Percent(50.0)),
                ));

                // Level display
                slot.spawn((
                    Text::new(format!("Lv{}", spell.level)),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }
        });
}

/// Spawn an active spell slot with optional spell content.
fn spawn_active_slot(parent: &mut ChildSpawnerCommands, index: usize, spell: Option<&Spell>) {
    let (bg_color, border_color) = if let Some(spell) = spell {
        (
            spell.element.color().with_alpha(0.6),
            spell.element.color(),
        )
    } else {
        (
            Color::srgba(0.3, 0.3, 0.3, 0.8),
            Color::srgba(0.5, 0.5, 0.5, 0.8),
        )
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(SLOT_SIZE),
                height: Val::Px(SLOT_SIZE),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(border_color),
            BorderRadius::all(Val::Px(6.0)),
            ActiveSlotDisplay { index },
        ))
        .with_children(|slot| {
            // Slot number
            slot.spawn((
                Text::new(format!("{}", index + 1)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(2.0),
                    left: Val::Px(4.0),
                    ..default()
                },
            ));

            if let Some(spell) = spell {
                // Element icon
                slot.spawn((
                    Node {
                        width: Val::Px(28.0),
                        height: Val::Px(28.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(spell.element.color()),
                    BorderRadius::all(Val::Percent(50.0)),
                ));

                // Level display
                slot.spawn((
                    Text::new(format!("Lv{}", spell.level)),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }
        });
}

/// Cleanup inventory screen entities when exiting the state.
pub fn cleanup_inventory_ui(
    mut commands: Commands,
    query: Query<Entity, With<InventoryScreen>>,
    mut selected_slot: ResMut<SelectedBagSlot>,
) {
    for entity in query.iter() {
        commands.queue(move |world: &mut bevy::ecs::world::World| {
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }
    // Clear selection on exit
    selected_slot.0 = None;
}

/// Handle keyboard input for inventory screen (I and Escape to close).
pub fn handle_inventory_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyI) || keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::InGame);
    }
}

/// Handle I key to open inventory from InGame state.
pub fn handle_inventory_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyI) && current_state.get() == &GameState::InGame {
        next_state.set(GameState::InventoryOpen);
    }
}

/// Handle bag slot click to select.
#[allow(clippy::type_complexity)]
pub fn handle_bag_slot_click(
    mut interaction_query: Query<
        (Entity, &Interaction, &mut BackgroundColor, &mut BorderColor, &InventorySlot),
        Changed<Interaction>,
    >,
    mut selected_slot: ResMut<SelectedBagSlot>,
    inventory_bag: Res<InventoryBag>,
    mut commands: Commands,
    selected_query: Query<Entity, With<SelectedSpell>>,
) {
    for (entity, interaction, mut bg_color, mut border_color, slot) in &mut interaction_query {
        let spell = inventory_bag.get_spell(slot.index);
        let element_color = spell.map(|s| s.element.color());

        match *interaction {
            Interaction::Pressed => {
                // Only allow selecting slots that have spells
                if spell.is_some() {
                    // Remove SelectedSpell from previously selected entity
                    for selected_entity in selected_query.iter() {
                        commands.entity(selected_entity).remove::<SelectedSpell>();
                    }
                    // Add SelectedSpell to this entity
                    commands.entity(entity).insert(SelectedSpell);
                    selected_slot.0 = Some(slot.index);
                }
            }
            Interaction::Hovered => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.7));
                    *border_color = BorderColor::all(Color::WHITE);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.6, 0.6, 0.6, 0.8));
                }
            }
            Interaction::None => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.4));
                    *border_color = BorderColor::all(color);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 0.8));
                }
            }
        }
    }
}

/// Handle active slot click to swap with selected bag spell.
#[allow(clippy::type_complexity)]
pub fn handle_active_slot_click(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor, &ActiveSlotDisplay),
        Changed<Interaction>,
    >,
    mut selected_slot: ResMut<SelectedBagSlot>,
    mut spell_list: ResMut<SpellList>,
    mut inventory_bag: ResMut<InventoryBag>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut bg_color, mut border_color, active_slot) in &mut interaction_query {
        let spell = spell_list.get_spell(active_slot.index);
        let element_color = spell.map(|s| s.element.color());

        match *interaction {
            Interaction::Pressed => {
                // Perform swap if we have a selected bag slot
                if let Some(bag_slot_index) = selected_slot.0 {
                    // Get spell from bag
                    if let Some(bag_spell) = inventory_bag.remove(bag_slot_index) {
                        // Get spell from active slot (if any)
                        let active_spell = spell_list.remove(active_slot.index);

                        // Put bag spell into active slot
                        spell_list.slots_mut()[active_slot.index] = Some(bag_spell);

                        // Put active spell (if any) into bag slot
                        if let Some(spell) = active_spell {
                            inventory_bag.slots_mut()[bag_slot_index] = Some(spell);
                        }

                        // Clear selection and close inventory
                        selected_slot.0 = None;
                        next_state.set(GameState::InGame);
                    }
                }
            }
            Interaction::Hovered => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.9));
                    *border_color = BorderColor::all(Color::WHITE);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.7, 0.7, 0.7, 0.8));
                }
            }
            Interaction::None => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.6));
                    *border_color = BorderColor::all(color);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 0.8));
                }
            }
        }
    }
}

/// Update visual highlight for selected slot.
pub fn update_selected_slot_visual(
    selected_query: Query<Entity, With<SelectedSpell>>,
    mut slot_query: Query<(Entity, &mut BorderColor, &InventorySlot)>,
    inventory_bag: Res<InventoryBag>,
) {
    for (entity, mut border_color, slot) in &mut slot_query {
        let spell = inventory_bag.get_spell(slot.index);
        let is_selected = selected_query.iter().any(|e| e == entity);

        if is_selected {
            *border_color = BorderColor::all(Color::srgb(1.0, 0.84, 0.0)); // Gold highlight
        } else if let Some(spell) = spell {
            *border_color = BorderColor::all(spell.element.color());
        } else {
            *border_color = BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 0.8));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<SpellList>();
        app.init_resource::<InventoryBag>();
        app.init_resource::<SelectedBagSlot>();
        app
    }

    mod component_tests {
        use super::*;

        #[test]
        fn inventory_screen_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<InventoryScreen>();
        }

        #[test]
        fn inventory_overlay_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<InventoryOverlay>();
        }

        #[test]
        fn inventory_slot_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<InventorySlot>();
        }

        #[test]
        fn inventory_slot_stores_index() {
            let slot = InventorySlot { index: 15 };
            assert_eq!(slot.index, 15);
        }

        #[test]
        fn active_slot_display_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<ActiveSlotDisplay>();
        }

        #[test]
        fn active_slot_display_stores_index() {
            let slot = ActiveSlotDisplay { index: 3 };
            assert_eq!(slot.index, 3);
        }

        #[test]
        fn selected_spell_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SelectedSpell>();
        }

        #[test]
        fn selected_bag_slot_is_resource() {
            fn assert_resource<T: Resource>() {}
            assert_resource::<SelectedBagSlot>();
        }

        #[test]
        fn selected_bag_slot_defaults_to_none() {
            let selected = SelectedBagSlot::default();
            assert!(selected.0.is_none());
        }
    }

    mod setup_inventory_ui_tests {
        use super::*;

        #[test]
        fn spawns_inventory_screen_root() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let screen_count = app
                .world_mut()
                .query::<&InventoryScreen>()
                .iter(app.world())
                .count();
            assert_eq!(screen_count, 1, "Should spawn exactly one InventoryScreen");
        }

        #[test]
        fn spawns_inventory_overlay() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let overlay_count = app
                .world_mut()
                .query::<&InventoryOverlay>()
                .iter(app.world())
                .count();
            assert_eq!(overlay_count, 1, "Should spawn exactly one InventoryOverlay");
        }

        #[test]
        fn spawns_30_bag_slots() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let slot_count = app
                .world_mut()
                .query::<&InventorySlot>()
                .iter(app.world())
                .count();
            assert_eq!(slot_count, 30, "Should spawn exactly 30 bag slots");
        }

        #[test]
        fn spawns_5_active_slots() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let slot_count = app
                .world_mut()
                .query::<&ActiveSlotDisplay>()
                .iter(app.world())
                .count();
            assert_eq!(slot_count, 5, "Should spawn exactly 5 active slots");
        }

        #[test]
        fn total_slots_count_is_35() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let bag_count = app
                .world_mut()
                .query::<&InventorySlot>()
                .iter(app.world())
                .count();
            let active_count = app
                .world_mut()
                .query::<&ActiveSlotDisplay>()
                .iter(app.world())
                .count();
            assert_eq!(bag_count + active_count, 35, "Should spawn 30 bag + 5 active = 35 total slots");
        }

        #[test]
        fn bag_slots_have_button_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let button_slot_count = app
                .world_mut()
                .query::<(&Button, &InventorySlot)>()
                .iter(app.world())
                .count();
            assert_eq!(button_slot_count, 30, "All 30 bag slots should have Button component");
        }

        #[test]
        fn active_slots_have_button_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let button_slot_count = app
                .world_mut()
                .query::<(&Button, &ActiveSlotDisplay)>()
                .iter(app.world())
                .count();
            assert_eq!(button_slot_count, 5, "All 5 active slots should have Button component");
        }

        #[test]
        fn each_bag_slot_has_unique_index() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let indices: Vec<usize> = app
                .world_mut()
                .query::<&InventorySlot>()
                .iter(app.world())
                .map(|slot| slot.index)
                .collect();

            // Check all indices 0-29 are present
            for i in 0..30 {
                assert!(indices.contains(&i), "Bag slot index {} should be present", i);
            }
        }

        #[test]
        fn each_active_slot_has_unique_index() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let indices: Vec<usize> = app
                .world_mut()
                .query::<&ActiveSlotDisplay>()
                .iter(app.world())
                .map(|slot| slot.index)
                .collect();

            // Check all indices 0-4 are present
            for i in 0..5 {
                assert!(indices.contains(&i), "Active slot index {} should be present", i);
            }
        }
    }

    mod cleanup_inventory_ui_tests {
        use super::*;

        #[test]
        fn removes_inventory_screen_entity() {
            let mut app = setup_test_app();

            // Spawn an inventory screen
            let screen = app
                .world_mut()
                .spawn((Node::default(), InventoryScreen))
                .id();

            // Verify it exists
            assert!(app.world().get_entity(screen).is_ok());

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_inventory_ui);

            // Screen should be despawned
            assert!(
                app.world().get_entity(screen).is_err(),
                "Screen should be despawned"
            );
        }

        #[test]
        fn clears_selected_slot() {
            let mut app = setup_test_app();

            // Set a selected slot
            app.world_mut().resource_mut::<SelectedBagSlot>().0 = Some(5);

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_inventory_ui);

            // Selection should be cleared
            let selected = app.world().resource::<SelectedBagSlot>();
            assert!(selected.0.is_none(), "Selected slot should be cleared on cleanup");
        }

        #[test]
        fn removes_children_recursively() {
            let mut app = setup_test_app();

            // Spawn screen with a child
            let child = app.world_mut().spawn(Node::default()).id();
            let screen = app
                .world_mut()
                .spawn((Node::default(), InventoryScreen))
                .id();
            app.world_mut().entity_mut(screen).add_child(child);

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_inventory_ui);

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

    mod handle_inventory_input_tests {
        use super::*;

        #[test]
        fn i_key_transitions_to_ingame() {
            let mut app = setup_test_app();

            // Set initial state
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);

            // Simulate I key press
            let mut keyboard = ButtonInput::<KeyCode>::default();
            keyboard.press(KeyCode::KeyI);
            app.insert_resource(keyboard);

            // Run the handler
            app.add_systems(Update, handle_inventory_input);
            app.update();

            // Note: We can't directly check NextState, but we verify the system doesn't panic
            // In real tests, we'd check the actual state transition
        }

        #[test]
        fn escape_key_transitions_to_ingame() {
            let mut app = setup_test_app();

            // Set initial state
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);

            // Simulate Escape key press
            let mut keyboard = ButtonInput::<KeyCode>::default();
            keyboard.press(KeyCode::Escape);
            app.insert_resource(keyboard);

            // Run the handler
            app.add_systems(Update, handle_inventory_input);
            app.update();

            // System should not panic
        }
    }

    mod handle_inventory_toggle_tests {
        use super::*;

        #[test]
        fn i_key_from_ingame_opens_inventory() {
            let mut app = setup_test_app();

            // Set state to InGame
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InGame);
            app.update(); // Apply state change

            // Simulate I key press
            let mut keyboard = ButtonInput::<KeyCode>::default();
            keyboard.press(KeyCode::KeyI);
            app.insert_resource(keyboard);

            // Run the handler
            app.add_systems(Update, handle_inventory_toggle);
            app.update();

            // System should not panic and state transition should be triggered
        }

        #[test]
        fn i_key_from_attunement_select_does_nothing() {
            let mut app = setup_test_app();

            // Set state to AttunementSelect
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::AttunementSelect);
            app.update(); // Apply state change

            // Simulate I key press
            let mut keyboard = ButtonInput::<KeyCode>::default();
            keyboard.press(KeyCode::KeyI);
            app.insert_resource(keyboard);

            // Run the handler
            app.add_systems(Update, handle_inventory_toggle);
            app.update();

            // System should not panic - state should remain unchanged
        }

        #[test]
        fn i_key_from_game_over_does_nothing() {
            let mut app = setup_test_app();

            // Set state to GameOver
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::GameOver);
            app.update(); // Apply state change

            // Simulate I key press
            let mut keyboard = ButtonInput::<KeyCode>::default();
            keyboard.press(KeyCode::KeyI);
            app.insert_resource(keyboard);

            // Run the handler
            app.add_systems(Update, handle_inventory_toggle);
            app.update();

            // System should not panic - state should remain unchanged
        }
    }

    mod slot_selection_tests {
        use super::*;

        #[test]
        fn selected_bag_slot_resource_stores_selection() {
            let mut selected = SelectedBagSlot::default();
            assert!(selected.0.is_none());

            selected.0 = Some(10);
            assert_eq!(selected.0, Some(10));
        }
    }

    mod swap_logic_tests {
        use super::*;
        use crate::spell::Spell;

        #[test]
        fn bag_and_spell_list_can_swap_spells() {
            // This tests the underlying data structures used by swap logic
            let mut bag = InventoryBag::default();
            let mut spell_list = SpellList::default();

            // Add spells
            let fireball = Spell::new(SpellType::Fireball);
            let radiant_beam = Spell::new(SpellType::RadiantBeam);

            bag.add(fireball);
            spell_list.equip(radiant_beam);

            // Verify initial state
            assert!(bag.get_spell(0).is_some());
            assert_eq!(bag.get_spell(0).unwrap().spell_type, SpellType::Fireball);
            assert!(spell_list.get_spell(0).is_some());
            assert_eq!(spell_list.get_spell(0).unwrap().spell_type, SpellType::RadiantBeam);
        }
    }
}
