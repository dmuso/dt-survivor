//! Plugin for spell slot visual management.
//!
//! This plugin registers the refresh system that updates spell slot visuals
//! whenever the SpellList or InventoryBag resources change.

use bevy::prelude::*;

use crate::inventory::{InventoryBag, SpellList};
use crate::states::GameState;
use crate::ui::spell_slot::systems::refresh_spell_slot_visuals;

/// Plugin that manages spell slot visual updates.
///
/// Registers the `refresh_spell_slot_visuals` system to run in the Update schedule
/// when in InGame or InventoryOpen states, and only when SpellList or InventoryBag
/// has changed.
pub struct SpellSlotPlugin;

impl Plugin for SpellSlotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            refresh_spell_slot_visuals.run_if(
                (in_state(GameState::InGame).or(in_state(GameState::InventoryOpen)))
                    .and(resource_changed::<SpellList>.or(resource_changed::<InventoryBag>)),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::{Spell, SpellType};
    use crate::ui::spell_slot::components::{SlotSource, SpellSlotVisual};
    use crate::ui::spell_slot::spawn::spawn_spell_slot;
    use bevy::ecs::system::RunSystemOnce;

    /// Test marker to find spawned parent entities.
    #[derive(Component)]
    struct TestParent;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::prelude::TaskPoolPlugin::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(bevy::prelude::ImagePlugin::default());
        app.init_state::<GameState>();
        app.init_resource::<SpellList>();
        app.init_resource::<InventoryBag>();
        app
    }

    mod spell_slot_plugin_tests {
        use super::*;

        #[test]
        fn plugin_can_be_added_to_app() {
            let mut app = setup_test_app();
            app.add_plugins(SpellSlotPlugin);
            app.update();
        }

        #[test]
        fn plugin_is_a_plugin() {
            fn assert_plugin<T: Plugin>() {}
            assert_plugin::<SpellSlotPlugin>();
        }

        #[test]
        fn system_runs_in_ingame_state_when_spell_list_changes() {
            let mut app = setup_test_app();
            app.add_plugins(SpellSlotPlugin);

            // Transition to InGame state
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InGame);
            app.update();

            // Spawn a slot to be updated
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands
                    .spawn((Node::default(), TestParent))
                    .with_children(|parent| {
                        spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                    });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Add a spell to trigger change detection
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Run update - system should execute
            app.update();

            // Verify the slot was updated with spell colors
            let (bg, _, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &BorderColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            // Background should not be the empty slot color (gray)
            let empty_bg = crate::ui::components::empty_slot::SLOT_BACKGROUND;
            assert_ne!(bg.0, empty_bg, "Slot should have spell color, not empty color");
        }

        #[test]
        fn system_runs_in_inventory_open_state_when_inventory_bag_changes() {
            let mut app = setup_test_app();
            app.add_plugins(SpellSlotPlugin);

            // Transition to InventoryOpen state
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);
            app.update();

            // Spawn a bag slot to be updated
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands
                    .spawn((Node::default(), TestParent))
                    .with_children(|parent| {
                        spawn_spell_slot(parent, SlotSource::Bag, 0, None, &asset_server);
                    });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Add a spell to inventory bag to trigger change detection
            let frost_nova = Spell::new(SpellType::FrostNova);
            app.world_mut().resource_mut::<InventoryBag>().add(frost_nova);

            // Run update - system should execute
            app.update();

            // Verify the slot was updated with spell colors
            let (bg, _, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &BorderColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            // Background should not be the empty slot color (gray)
            let empty_bg = crate::ui::components::empty_slot::SLOT_BACKGROUND;
            assert_ne!(bg.0, empty_bg, "Slot should have spell color, not empty color");
        }

        #[test]
        fn system_does_not_run_in_intro_state() {
            let mut app = setup_test_app();
            app.add_plugins(SpellSlotPlugin);

            // Stay in Intro state (default)
            app.update();

            // Spawn a slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands
                    .spawn((Node::default(), TestParent))
                    .with_children(|parent| {
                        spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                    });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Record initial slot colors
            let initial_bg = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .map(|(bg, _)| bg.0)
                .expect("Slot should exist");

            // Add a spell
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Run update - system should NOT execute (wrong state)
            app.update();

            // Verify the slot was NOT updated (still empty colors)
            let final_bg = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .map(|(bg, _)| bg.0)
                .expect("Slot should exist");

            assert_eq!(
                initial_bg, final_bg,
                "Slot should not change in Intro state"
            );
        }

        #[test]
        fn system_does_not_run_when_resources_unchanged() {
            let mut app = setup_test_app();
            app.add_plugins(SpellSlotPlugin);

            // Transition to InGame state and allow initial change detection to pass
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InGame);
            app.update();
            app.update(); // Additional update to clear initial change detection

            // Spawn a slot manually with non-standard colors
            let spawn_colored_slot = |mut commands: Commands| {
                commands.spawn((
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.0, 0.0)), // Red - unusual color
                    BorderColor::all(Color::srgb(0.0, 1.0, 0.0)), // Green - unusual color
                    SpellSlotVisual {
                        source: SlotSource::Active,
                        index: 0,
                    },
                ));
            };
            let _ = app.world_mut().run_system_once(spawn_colored_slot);

            // Run update without changing resources
            app.update();

            // Verify the slot colors were NOT changed by the system
            let (bg, border, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &BorderColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            // Colors should remain the unusual red/green we set
            assert_eq!(
                bg.0,
                Color::srgb(1.0, 0.0, 0.0),
                "Background should remain red when no resource changed"
            );
            assert_eq!(
                border.top,
                Color::srgb(0.0, 1.0, 0.0),
                "Border should remain green when no resource changed"
            );
        }
    }
}
