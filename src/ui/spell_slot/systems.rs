//! Systems for spell slot visual updates.
//!
//! This module contains the refresh and update systems for spell slots,
//! enabling unified visual updates for both active spell bar and inventory bag.

use bevy::prelude::*;

use crate::inventory::{InventoryBag, SpellList};
use crate::spell::Spell;
use crate::ui::spell_slot::components::{
    LevelIndicatorContainer, SlotSource, SpellAbbreviation, SpellIconImage, SpellLevelIndicator,
    SpellSlotVisual,
};
use crate::ui::spell_slot::spawn::spell_slot_background;

/// Refreshes all spell slot visuals based on current SpellList and InventoryBag state.
///
/// This system handles all three cases for each slot:
/// 1. Spell with texture: load ImageNode, hide abbreviation, use element color background
/// 2. Spell without texture: clear ImageNode, show abbreviation, use element color background
/// 3. Empty slot: clear ImageNode, hide abbreviation, use empty slot colors
///
/// The system queries slots by their SlotSource to determine which resource to read from.
pub fn refresh_spell_slot_visuals(
    spell_list: Res<SpellList>,
    inventory_bag: Res<InventoryBag>,
    asset_server: Res<AssetServer>,
    mut slots: Query<(&SpellSlotVisual, &mut BackgroundColor)>,
    mut icons: Query<(&SpellIconImage, &mut ImageNode, &mut Visibility, &ChildOf)>,
    mut levels: Query<(&SpellLevelIndicator, &mut Text, &ChildOf)>,
    mut level_containers: Query<
        (&LevelIndicatorContainer, &mut Visibility),
        (Without<SpellLevelIndicator>, Without<SpellIconImage>),
    >,
    mut abbrevs: Query<
        (&SpellAbbreviation, &mut Text, &mut Visibility, &ChildOf),
        (Without<SpellLevelIndicator>, Without<LevelIndicatorContainer>, Without<SpellIconImage>),
    >,
) {
    // Update slot container background colors
    for (slot_visual, mut bg_color) in &mut slots {
        let spell = get_spell_for_slot(slot_visual, &spell_list, &inventory_bag);
        *bg_color = spell_slot_background(spell);
    }

    // Update all icon images
    for (_icon, mut image_node, mut visibility, child_of) in &mut icons {
        // Find the parent slot to get the spell
        if let Ok((slot_visual, _)) = slots.get(child_of.parent()) {
            let spell = get_spell_for_slot(slot_visual, &spell_list, &inventory_bag);

            // Update image and visibility based on spell
            if let Some(spell) = spell {
                *visibility = Visibility::Visible;
                if let Some(path) = spell.spell_type.icon_path() {
                    image_node.image = asset_server.load(path);
                } else {
                    // Clear image for spells without textures
                    image_node.image = Handle::default();
                }
            } else {
                // Hide and clear image for empty slots
                *visibility = Visibility::Hidden;
                image_node.image = Handle::default();
            }
        }
    }

    // Update level indicator container visibility (fixes corner border artifacts)
    for (container, mut visibility) in &mut level_containers {
        let spell = find_spell_for_source(container.source, container.index, &spell_list, &inventory_bag);
        *visibility = if spell.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update all level indicator text
    for (indicator, mut text, _child_of) in &mut levels {
        // Use source-aware lookup (fixes ghost level indicators from index collision)
        let spell = find_spell_for_source(indicator.source, indicator.index, &spell_list, &inventory_bag);

        if let Some(spell) = spell {
            **text = format!("{}", spell.level);
        } else {
            **text = String::new();
        }
    }

    // Update all abbreviations
    for (abbrev, mut text, mut visibility, _child_of) in &mut abbrevs {
        // Use source-aware lookup
        let spell = find_spell_for_source(abbrev.source, abbrev.index, &spell_list, &inventory_bag);

        match spell {
            Some(spell) if spell.spell_type.icon_path().is_none() => {
                // Spell without texture - show abbreviation
                **text = spell.spell_type.abbreviation().to_string();
                *visibility = Visibility::Visible;
            }
            _ => {
                // Spell with texture or empty slot - hide abbreviation
                **text = String::new();
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// Gets the spell for a given slot visual based on its source and index.
fn get_spell_for_slot<'a>(
    slot_visual: &SpellSlotVisual,
    spell_list: &'a SpellList,
    inventory_bag: &'a InventoryBag,
) -> Option<&'a Spell> {
    match slot_visual.source {
        SlotSource::Active => spell_list.get_spell(slot_visual.index),
        SlotSource::Bag => inventory_bag.get_spell(slot_visual.index),
    }
}

/// Finds a spell by source and index directly from the appropriate resource.
///
/// This is source-aware, fixing the index collision bug where bag slot 0 and
/// active slot 0 would return the wrong spell.
fn find_spell_for_source<'a>(
    source: SlotSource,
    index: usize,
    spell_list: &'a SpellList,
    inventory_bag: &'a InventoryBag,
) -> Option<&'a Spell> {
    match source {
        SlotSource::Active => spell_list.get_spell(index),
        SlotSource::Bag => inventory_bag.get_spell(index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::SpellType;
    use crate::ui::components::empty_slot;
    use crate::ui::spell_slot::spawn::{spawn_spell_slot, BACKGROUND_ALPHA};
    use bevy::ecs::system::RunSystemOnce;

    /// Test marker to find our spawned parent
    #[derive(Component)]
    struct TestParent;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::prelude::TaskPoolPlugin::default());
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(bevy::prelude::ImagePlugin::default());
        app.init_resource::<SpellList>();
        app.init_resource::<InventoryBag>();
        app
    }

    mod refresh_spell_slot_visuals_tests {
        use super::*;

        #[test]
        fn is_a_system() {
            fn assert_system<T: bevy::ecs::system::IntoSystem<(), (), M>, M>(_: T) {}
            assert_system(refresh_spell_slot_visuals);
        }

        #[test]
        fn empty_slot_has_empty_slot_background() {
            let mut app = setup_test_app();

            // Spawn an empty slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check background color
            let (bg, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(bg.0, empty_slot::SLOT_BACKGROUND);
        }

        #[test]
        fn slot_with_spell_has_element_background() {
            let mut app = setup_test_app();

            // Add a spell to the spell list
            let fireball = Spell::new(SpellType::Fireball);
            let expected_bg = fireball.element.color().with_alpha(BACKGROUND_ALPHA);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Spawn a slot (starts empty)
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system - should update background based on spell list
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check background color
            let (bg, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(bg.0, expected_bg);
        }

        #[test]
        fn level_indicator_shows_spell_level() {
            let mut app = setup_test_app();

            // Add a spell at level 5
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.level = 5;
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Spawn a slot with a spell initially so level indicator is created
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>, spell_list: Res<SpellList>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, spell_list.get_spell(0), &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check level text
            let (indicator, text) = app
                .world_mut()
                .query::<(&SpellLevelIndicator, &Text)>()
                .iter(app.world())
                .next()
                .expect("Level indicator should exist");

            assert_eq!(indicator.index, 0);
            assert_eq!(text.0, "5");
        }

        #[test]
        fn spell_without_texture_shows_abbreviation() {
            let mut app = setup_test_app();

            // Add a spell without a texture (IceShard has no icon)
            let ice_shard = Spell::new(SpellType::IceShard);
            app.world_mut().resource_mut::<SpellList>().equip(ice_shard);

            // Spawn a slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>, spell_list: Res<SpellList>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, spell_list.get_spell(0), &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check abbreviation is visible with correct text
            let (_, text, visibility) = app
                .world_mut()
                .query::<(&SpellAbbreviation, &Text, &Visibility)>()
                .iter(app.world())
                .next()
                .expect("Abbreviation should exist");

            assert_eq!(*visibility, Visibility::Visible);
            assert_eq!(text.0, "IS"); // IceShard abbreviation
        }

        #[test]
        fn spell_with_texture_hides_abbreviation() {
            let mut app = setup_test_app();

            // Add a spell with a texture (Fireball has an icon)
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Spawn a slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>, spell_list: Res<SpellList>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, spell_list.get_spell(0), &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check abbreviation is hidden
            let (_, _, visibility) = app
                .world_mut()
                .query::<(&SpellAbbreviation, &Text, &Visibility)>()
                .iter(app.world())
                .next()
                .expect("Abbreviation should exist");

            assert_eq!(*visibility, Visibility::Hidden);
        }

        #[test]
        fn bag_slot_reads_from_inventory_bag() {
            let mut app = setup_test_app();

            // Add a spell to the inventory bag
            let frost_nova = Spell::new(SpellType::FrostNova);
            let expected_bg = frost_nova.element.color().with_alpha(BACKGROUND_ALPHA);
            app.world_mut().resource_mut::<InventoryBag>().add(frost_nova);

            // Spawn a bag slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Bag, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check background comes from bag spell
            let (bg, slot) = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(slot.source, SlotSource::Bag);
            assert_eq!(bg.0, expected_bg);
        }

        #[test]
        fn updates_multiple_slots_correctly() {
            let mut app = setup_test_app();

            // Add spells to both sources
            let fireball = Spell::new(SpellType::Fireball);
            let ice_shard = Spell::new(SpellType::IceShard);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);
            app.world_mut().resource_mut::<InventoryBag>().add(ice_shard);

            // Spawn an active slot and a bag slot
            let spawn_slots = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                    spawn_spell_slot(parent, SlotSource::Bag, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slots);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check both slots have different element colors
            let slots: Vec<_> = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .collect();

            assert_eq!(slots.len(), 2);

            // Find the active and bag slots
            let active_slot = slots.iter().find(|(_, s)| s.source == SlotSource::Active).unwrap();
            let bag_slot = slots.iter().find(|(_, s)| s.source == SlotSource::Bag).unwrap();

            // They should have different background colors (Fire vs Frost)
            assert_ne!(active_slot.0 .0, bag_slot.0 .0);
        }

        #[test]
        fn empty_slot_icon_is_hidden() {
            let mut app = setup_test_app();

            // Spawn an empty slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check icon visibility
            let (visibility, _) = app
                .world_mut()
                .query::<(&Visibility, &SpellIconImage)>()
                .iter(app.world())
                .next()
                .expect("Icon should exist");

            assert_eq!(*visibility, Visibility::Hidden);
        }

        #[test]
        fn slot_with_spell_icon_is_visible() {
            let mut app = setup_test_app();

            // Add a spell to the spell list
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Spawn a slot
            let spawn_slot = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slot);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Check icon visibility
            let (visibility, _) = app
                .world_mut()
                .query::<(&Visibility, &SpellIconImage)>()
                .iter(app.world())
                .next()
                .expect("Icon should exist");

            assert_eq!(*visibility, Visibility::Visible);
        }

        #[test]
        fn spell_swap_from_bag_to_active_updates_both_slots() {
            let mut app = setup_test_app();

            // Initially, spell is in bag, active slot is empty
            let fireball = Spell::new(SpellType::Fireball);
            let bag_spell_bg = fireball.element.color().with_alpha(BACKGROUND_ALPHA);
            app.world_mut().resource_mut::<InventoryBag>().add(fireball);

            // Spawn both slots (empty initially - refresh system will pick up the state)
            let spawn_slots = |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, SlotSource::Active, 0, None, &asset_server);
                    spawn_spell_slot(parent, SlotSource::Bag, 0, None, &asset_server);
                });
            };
            let _ = app.world_mut().run_system_once(spawn_slots);

            // First refresh - active is empty, bag has spell
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Verify initial state
            let slots: Vec<_> = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .collect();
            let active_slot = slots.iter().find(|(_, s)| s.source == SlotSource::Active).unwrap();
            let bag_slot = slots.iter().find(|(_, s)| s.source == SlotSource::Bag).unwrap();

            assert_eq!(active_slot.0 .0, empty_slot::SLOT_BACKGROUND, "Active slot should be empty initially");
            assert_eq!(bag_slot.0 .0, bag_spell_bg, "Bag slot should have spell color");

            // Simulate swap: move spell from bag to active
            let spell = app.world_mut().resource_mut::<InventoryBag>().remove(0).unwrap();
            app.world_mut().resource_mut::<SpellList>().equip(spell);

            // Second refresh - active has spell, bag is empty
            let _ = app.world_mut().run_system_once(refresh_spell_slot_visuals);

            // Verify swapped state
            let slots: Vec<_> = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .collect();
            let active_slot = slots.iter().find(|(_, s)| s.source == SlotSource::Active).unwrap();
            let bag_slot = slots.iter().find(|(_, s)| s.source == SlotSource::Bag).unwrap();

            assert_eq!(active_slot.0 .0, bag_spell_bg, "Active slot should have spell color after swap");
            assert_eq!(bag_slot.0 .0, empty_slot::SLOT_BACKGROUND, "Bag slot should be empty after swap");
        }
    }

    mod get_spell_for_slot_tests {
        use super::*;

        #[test]
        fn returns_spell_from_spell_list_for_active_source() {
            let mut spell_list = SpellList::default();
            let inventory_bag = InventoryBag::default();

            let fireball = Spell::new(SpellType::Fireball);
            spell_list.equip(fireball);

            let slot_visual = SpellSlotVisual {
                source: SlotSource::Active,
                index: 0,
            };

            let spell = get_spell_for_slot(&slot_visual, &spell_list, &inventory_bag);
            assert!(spell.is_some());
            assert_eq!(spell.unwrap().spell_type, SpellType::Fireball);
        }

        #[test]
        fn returns_spell_from_inventory_bag_for_bag_source() {
            let spell_list = SpellList::default();
            let mut inventory_bag = InventoryBag::default();

            let ice_shard = Spell::new(SpellType::IceShard);
            inventory_bag.add(ice_shard);

            let slot_visual = SpellSlotVisual {
                source: SlotSource::Bag,
                index: 0,
            };

            let spell = get_spell_for_slot(&slot_visual, &spell_list, &inventory_bag);
            assert!(spell.is_some());
            assert_eq!(spell.unwrap().spell_type, SpellType::IceShard);
        }

        #[test]
        fn returns_none_for_empty_slot() {
            let spell_list = SpellList::default();
            let inventory_bag = InventoryBag::default();

            let slot_visual = SpellSlotVisual {
                source: SlotSource::Active,
                index: 0,
            };

            let spell = get_spell_for_slot(&slot_visual, &spell_list, &inventory_bag);
            assert!(spell.is_none());
        }

        #[test]
        fn returns_correct_spell_for_specific_index() {
            let mut spell_list = SpellList::default();
            let inventory_bag = InventoryBag::default();

            spell_list.equip(Spell::new(SpellType::Fireball));
            spell_list.equip(Spell::new(SpellType::IceShard));
            spell_list.equip(Spell::new(SpellType::VenomBolt));

            let slot_visual = SpellSlotVisual {
                source: SlotSource::Active,
                index: 2,
            };

            let spell = get_spell_for_slot(&slot_visual, &spell_list, &inventory_bag);
            assert!(spell.is_some());
            assert_eq!(spell.unwrap().spell_type, SpellType::VenomBolt);
        }
    }

    mod find_spell_for_source_tests {
        use super::*;

        #[test]
        fn active_source_returns_from_spell_list() {
            let mut spell_list = SpellList::default();
            let inventory_bag = InventoryBag::default();

            spell_list.equip(Spell::new(SpellType::Fireball));

            let spell = find_spell_for_source(SlotSource::Active, 0, &spell_list, &inventory_bag);
            assert!(spell.is_some());
            assert_eq!(spell.unwrap().spell_type, SpellType::Fireball);
        }

        #[test]
        fn bag_source_returns_from_inventory_bag() {
            let spell_list = SpellList::default();
            let mut inventory_bag = InventoryBag::default();

            inventory_bag.add(Spell::new(SpellType::IceShard));

            let spell = find_spell_for_source(SlotSource::Bag, 0, &spell_list, &inventory_bag);
            assert!(spell.is_some());
            assert_eq!(spell.unwrap().spell_type, SpellType::IceShard);
        }

        #[test]
        fn same_index_different_sources_return_different_spells() {
            // This is the key test for the index collision bug fix
            let mut spell_list = SpellList::default();
            let mut inventory_bag = InventoryBag::default();

            // Both index 0, but different sources have different spells
            spell_list.equip(Spell::new(SpellType::Fireball));
            inventory_bag.add(Spell::new(SpellType::IceShard));

            let active_spell = find_spell_for_source(SlotSource::Active, 0, &spell_list, &inventory_bag);
            let bag_spell = find_spell_for_source(SlotSource::Bag, 0, &spell_list, &inventory_bag);

            assert!(active_spell.is_some());
            assert!(bag_spell.is_some());
            assert_eq!(active_spell.unwrap().spell_type, SpellType::Fireball);
            assert_eq!(bag_spell.unwrap().spell_type, SpellType::IceShard);
        }

        #[test]
        fn returns_none_for_empty_slot_in_active() {
            let spell_list = SpellList::default();
            let inventory_bag = InventoryBag::default();

            let spell = find_spell_for_source(SlotSource::Active, 0, &spell_list, &inventory_bag);
            assert!(spell.is_none());
        }

        #[test]
        fn returns_none_for_empty_slot_in_bag() {
            let spell_list = SpellList::default();
            let inventory_bag = InventoryBag::default();

            let spell = find_spell_for_source(SlotSource::Bag, 0, &spell_list, &inventory_bag);
            assert!(spell.is_none());
        }
    }
}
