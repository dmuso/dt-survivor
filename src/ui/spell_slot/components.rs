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

/// Visual state for spell slots - determines styling priority.
/// The refresh system uses this to compute the appropriate colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlotVisualState {
    /// Normal state - base colors from spell element
    #[default]
    Normal,
    /// Hovered state - brighter colors with white border
    Hovered,
    /// Selected state - gold border highlight
    Selected,
    /// Dragging state - dimmed appearance
    Dragging,
}

/// Component tracking the current visual state of a spell slot.
/// Interaction systems update this; the refresh system reads it to apply colors.
#[derive(Component, Default)]
pub struct SpellSlotState {
    /// Current visual state determining styling
    pub visual_state: SlotVisualState,
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
    /// Which resource this indicator reads spell data from
    pub source: SlotSource,
    /// Index into the spell list
    pub index: usize,
}

/// Marker component for spell abbreviation text.
#[derive(Component)]
pub struct SpellAbbreviation {
    /// Which resource this abbreviation reads spell data from
    pub source: SlotSource,
    /// Index into the spell list
    pub index: usize,
}

/// Marker component for the level indicator container box.
/// Used to control visibility - hidden when slot is empty.
#[derive(Component)]
pub struct LevelIndicatorContainer {
    /// Which resource this indicator reads spell data from
    pub source: SlotSource,
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

    mod slot_source_tests {
        use super::*;

        #[test]
        fn active_variant_exists() {
            let source = SlotSource::Active;
            assert_eq!(source, SlotSource::Active);
        }

        #[test]
        fn bag_variant_exists() {
            let source = SlotSource::Bag;
            assert_eq!(source, SlotSource::Bag);
        }

        #[test]
        fn is_copy() {
            let source = SlotSource::Active;
            let copy = source;
            assert_eq!(source, copy);
        }
    }

    mod slot_visual_state_tests {
        use super::*;

        #[test]
        fn default_is_normal() {
            let state = SlotVisualState::default();
            assert_eq!(state, SlotVisualState::Normal);
        }

        #[test]
        fn has_normal_variant() {
            assert_eq!(SlotVisualState::Normal, SlotVisualState::Normal);
        }

        #[test]
        fn has_hovered_variant() {
            assert_eq!(SlotVisualState::Hovered, SlotVisualState::Hovered);
        }

        #[test]
        fn has_selected_variant() {
            assert_eq!(SlotVisualState::Selected, SlotVisualState::Selected);
        }

        #[test]
        fn has_dragging_variant() {
            assert_eq!(SlotVisualState::Dragging, SlotVisualState::Dragging);
        }

        #[test]
        fn is_copy() {
            let state = SlotVisualState::Hovered;
            let copy = state;
            assert_eq!(state, copy);
        }
    }

    mod spell_slot_state_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SpellSlotState>();
        }

        #[test]
        fn default_visual_state_is_normal() {
            let state = SpellSlotState::default();
            assert_eq!(state.visual_state, SlotVisualState::Normal);
        }

        #[test]
        fn stores_visual_state() {
            let state = SpellSlotState {
                visual_state: SlotVisualState::Selected,
            };
            assert_eq!(state.visual_state, SlotVisualState::Selected);
        }
    }

    mod spell_slot_visual_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SpellSlotVisual>();
        }

        #[test]
        fn stores_source_and_index() {
            let slot = SpellSlotVisual {
                source: SlotSource::Active,
                index: 2,
            };
            assert_eq!(slot.source, SlotSource::Active);
            assert_eq!(slot.index, 2);
        }

        #[test]
        fn bag_source() {
            let slot = SpellSlotVisual {
                source: SlotSource::Bag,
                index: 5,
            };
            assert_eq!(slot.source, SlotSource::Bag);
            assert_eq!(slot.index, 5);
        }
    }

    mod spell_level_indicator_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SpellLevelIndicator>();
        }

        #[test]
        fn stores_source_and_index() {
            let indicator = SpellLevelIndicator {
                source: SlotSource::Active,
                index: 3,
            };
            assert_eq!(indicator.source, SlotSource::Active);
            assert_eq!(indicator.index, 3);
        }

        #[test]
        fn bag_source() {
            let indicator = SpellLevelIndicator {
                source: SlotSource::Bag,
                index: 10,
            };
            assert_eq!(indicator.source, SlotSource::Bag);
            assert_eq!(indicator.index, 10);
        }
    }

    mod spell_abbreviation_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SpellAbbreviation>();
        }

        #[test]
        fn stores_source_and_index() {
            let abbrev = SpellAbbreviation {
                source: SlotSource::Active,
                index: 4,
            };
            assert_eq!(abbrev.source, SlotSource::Active);
            assert_eq!(abbrev.index, 4);
        }

        #[test]
        fn bag_source() {
            let abbrev = SpellAbbreviation {
                source: SlotSource::Bag,
                index: 15,
            };
            assert_eq!(abbrev.source, SlotSource::Bag);
            assert_eq!(abbrev.index, 15);
        }
    }

    mod level_indicator_container_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<LevelIndicatorContainer>();
        }

        #[test]
        fn stores_source_and_index() {
            let container = LevelIndicatorContainer {
                source: SlotSource::Active,
                index: 2,
            };
            assert_eq!(container.source, SlotSource::Active);
            assert_eq!(container.index, 2);
        }

        #[test]
        fn bag_source() {
            let container = LevelIndicatorContainer {
                source: SlotSource::Bag,
                index: 20,
            };
            assert_eq!(container.source, SlotSource::Bag);
            assert_eq!(container.index, 20);
        }
    }

    mod spell_icon_image_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SpellIconImage>();
        }

        #[test]
        fn stores_index() {
            let icon = SpellIconImage { index: 1 };
            assert_eq!(icon.index, 1);
        }
    }

    mod spell_icon_visual_tests {
        use super::*;

        #[test]
        fn is_a_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SpellIconVisual>();
        }
    }
}
