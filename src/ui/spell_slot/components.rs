//! Components for spell slot visuals.
//!
//! These components allow the refresh system to query and update slot visuals
//! in both the active spell bar and inventory bag.

use bevy::prelude::*;

/// Identifies the source of spell data for a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotSource {
    /// Slot reads from the active spell bar (SpellList resource)
    Active,
    /// Slot reads from the inventory bag
    Bag,
}

/// Marker component for spell slot visual containers.
/// Used to identify and update slot visuals when spells change.
#[derive(Component)]
pub struct SpellSlotVisual {
    /// Which resource this slot reads spell data from
    pub source: SlotSource,
    /// Index into the source's spell list
    pub index: usize,
}

/// Marker component for spell level indicator text.
#[derive(Component)]
pub struct SpellLevelIndicator {
    /// Index into the spell list
    pub index: usize,
}

/// Marker component for spell abbreviation text.
#[derive(Component)]
pub struct SpellAbbreviation {
    /// Index into the spell list
    pub index: usize,
}

/// Marker component for spell icon image nodes.
#[derive(Component)]
pub struct SpellIconImage {
    /// Index into the spell list
    pub index: usize,
}

/// Marker component for standalone spell icon visuals.
/// Used for drag visuals and other non-slot spell icons that don't need refresh system.
#[derive(Component)]
pub struct SpellIconVisual;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_source_active_variant_exists() {
        let source = SlotSource::Active;
        assert_eq!(source, SlotSource::Active);
    }

    #[test]
    fn slot_source_bag_variant_exists() {
        let source = SlotSource::Bag;
        assert_eq!(source, SlotSource::Bag);
    }

    #[test]
    fn slot_source_is_copy() {
        let source = SlotSource::Active;
        let copy = source;
        assert_eq!(source, copy);
    }

    #[test]
    fn spell_slot_visual_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellSlotVisual>();
    }

    #[test]
    fn spell_slot_visual_stores_source_and_index() {
        let slot = SpellSlotVisual {
            source: SlotSource::Active,
            index: 2,
        };
        assert_eq!(slot.source, SlotSource::Active);
        assert_eq!(slot.index, 2);
    }

    #[test]
    fn spell_slot_visual_bag_source() {
        let slot = SpellSlotVisual {
            source: SlotSource::Bag,
            index: 5,
        };
        assert_eq!(slot.source, SlotSource::Bag);
        assert_eq!(slot.index, 5);
    }

    #[test]
    fn spell_level_indicator_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellLevelIndicator>();
    }

    #[test]
    fn spell_level_indicator_stores_index() {
        let indicator = SpellLevelIndicator { index: 3 };
        assert_eq!(indicator.index, 3);
    }

    #[test]
    fn spell_abbreviation_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellAbbreviation>();
    }

    #[test]
    fn spell_abbreviation_stores_index() {
        let abbrev = SpellAbbreviation { index: 4 };
        assert_eq!(abbrev.index, 4);
    }

    #[test]
    fn spell_icon_image_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellIconImage>();
    }

    #[test]
    fn spell_icon_image_stores_index() {
        let icon = SpellIconImage { index: 1 };
        assert_eq!(icon.index, 1);
    }

    #[test]
    fn spell_icon_visual_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellIconVisual>();
    }
}
