use bevy::prelude::*;
use crate::spell::{Spell, SpellType};

const BAG_SIZE: usize = 30;

/// Player's inventory bag for storing spells not currently equipped.
/// Contains up to 30 spell slots for spell storage.
#[derive(Resource)]
pub struct InventoryBag {
    slots: [Option<Spell>; BAG_SIZE],
}

impl Default for InventoryBag {
    fn default() -> Self {
        Self {
            slots: [const { None }; BAG_SIZE],
        }
    }
}

impl InventoryBag {
    /// Add spell to first empty slot, returns slot index or None if full.
    pub fn add(&mut self, spell: Spell) -> Option<usize> {
        if let Some(slot_index) = self.find_empty_slot() {
            self.slots[slot_index] = Some(spell);
            Some(slot_index)
        } else {
            None
        }
    }

    /// Remove spell from specific slot, returns removed spell.
    pub fn remove(&mut self, slot: usize) -> Option<Spell> {
        if slot < BAG_SIZE {
            self.slots[slot].take()
        } else {
            None
        }
    }

    /// Find slot containing specific spell type.
    pub fn find_spell(&self, spell_type: &SpellType) -> Option<usize> {
        self.slots.iter().position(|s| {
            s.as_ref()
                .is_some_and(|spell| &spell.spell_type == spell_type)
        })
    }

    /// Get spell at slot (for viewing).
    pub fn get_spell(&self, slot: usize) -> Option<&Spell> {
        self.slots.get(slot)?.as_ref()
    }

    /// Get mutable spell at slot (for leveling up).
    pub fn get_spell_mut(&mut self, slot: usize) -> Option<&mut Spell> {
        self.slots.get_mut(slot)?.as_mut()
    }

    /// Check if bag is full (no empty slots).
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    /// Count of spells currently in bag.
    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Find first empty slot.
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// Iterate over all spells with their slot indices.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &Spell)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|spell| (i, spell)))
    }

    /// Get mutable access to slots array for direct manipulation.
    pub fn slots_mut(&mut self) -> &mut [Option<Spell>; BAG_SIZE] {
        &mut self.slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_fireball_spell() -> Spell {
        Spell::new(SpellType::Fireball)
    }

    fn create_radiant_beam_spell() -> Spell {
        Spell::new(SpellType::RadiantBeam)
    }

    fn create_thunder_strike_spell() -> Spell {
        Spell::new(SpellType::ThunderStrike)
    }

    fn fill_bag_with_n_spells(bag: &mut InventoryBag, n: usize) {
        for _ in 0..n {
            bag.add(create_fireball_spell());
        }
    }

    mod inventory_bag_add_tests {
        use super::*;

        #[test]
        fn add_to_empty_bag_returns_slot_0() {
            let mut bag = InventoryBag::default();
            let result = bag.add(create_fireball_spell());
            assert_eq!(result, Some(0));
        }

        #[test]
        fn add_to_empty_bag_stores_spell() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            assert!(bag.get_spell(0).is_some());
        }

        #[test]
        fn add_multiple_spells_fills_consecutive_slots() {
            let mut bag = InventoryBag::default();
            assert_eq!(bag.add(create_fireball_spell()), Some(0));
            assert_eq!(bag.add(create_radiant_beam_spell()), Some(1));
            assert_eq!(bag.add(create_thunder_strike_spell()), Some(2));
        }

        #[test]
        fn add_returns_none_when_full() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 30);
            let result = bag.add(create_fireball_spell());
            assert_eq!(result, None);
        }
    }

    mod inventory_bag_remove_tests {
        use super::*;

        #[test]
        fn remove_from_valid_slot_returns_spell() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            let removed = bag.remove(0);
            assert!(removed.is_some());
        }

        #[test]
        fn remove_clears_slot() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            bag.remove(0);
            assert!(bag.get_spell(0).is_none());
        }

        #[test]
        fn remove_from_empty_slot_returns_none() {
            let mut bag = InventoryBag::default();
            let removed = bag.remove(0);
            assert!(removed.is_none());
        }

        #[test]
        fn remove_from_out_of_bounds_returns_none() {
            let mut bag = InventoryBag::default();
            let removed = bag.remove(30);
            assert!(removed.is_none());
            let removed_large = bag.remove(100);
            assert!(removed_large.is_none());
        }
    }

    mod inventory_bag_find_spell_tests {
        use super::*;

        #[test]
        fn find_spell_returns_correct_index() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            bag.add(create_radiant_beam_spell());
            let slot = bag.find_spell(&SpellType::RadiantBeam);
            assert_eq!(slot, Some(1));
        }

        #[test]
        fn find_spell_returns_none_for_missing() {
            let bag = InventoryBag::default();
            let slot = bag.find_spell(&SpellType::RadiantBeam);
            assert_eq!(slot, None);
        }

        #[test]
        fn find_spell_with_filled_bag_finds_spell() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 15);
            bag.add(create_radiant_beam_spell());
            let slot = bag.find_spell(&SpellType::RadiantBeam);
            assert_eq!(slot, Some(15));
        }
    }

    mod inventory_bag_get_spell_tests {
        use super::*;

        #[test]
        fn get_spell_returns_some_for_valid_slot() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            assert!(bag.get_spell(0).is_some());
        }

        #[test]
        fn get_spell_returns_none_for_empty_slot() {
            let bag = InventoryBag::default();
            assert!(bag.get_spell(0).is_none());
        }

        #[test]
        fn get_spell_returns_none_for_out_of_bounds() {
            let bag = InventoryBag::default();
            assert!(bag.get_spell(30).is_none());
            assert!(bag.get_spell(100).is_none());
        }

        #[test]
        fn get_spell_returns_correct_spell() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            bag.add(create_radiant_beam_spell());
            let spell = bag.get_spell(1).unwrap();
            assert_eq!(spell.spell_type, SpellType::RadiantBeam);
        }
    }

    mod inventory_bag_get_spell_mut_tests {
        use super::*;

        #[test]
        fn get_spell_mut_allows_level_up() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            if let Some(spell) = bag.get_spell_mut(0) {
                let old_level = spell.level;
                spell.level_up();
                assert_eq!(spell.level, old_level + 1);
            } else {
                panic!("Expected spell at slot 0");
            }
        }

        #[test]
        fn get_spell_mut_returns_none_for_empty_slot() {
            let mut bag = InventoryBag::default();
            assert!(bag.get_spell_mut(0).is_none());
        }

        #[test]
        fn get_spell_mut_returns_none_for_out_of_bounds() {
            let mut bag = InventoryBag::default();
            assert!(bag.get_spell_mut(30).is_none());
        }
    }

    mod inventory_bag_is_full_tests {
        use super::*;

        #[test]
        fn is_full_returns_false_when_empty() {
            let bag = InventoryBag::default();
            assert!(!bag.is_full());
        }

        #[test]
        fn is_full_returns_false_with_29_spells() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 29);
            assert!(!bag.is_full());
        }

        #[test]
        fn is_full_returns_true_with_30_spells() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 30);
            assert!(bag.is_full());
        }
    }

    mod inventory_bag_count_tests {
        use super::*;

        #[test]
        fn count_returns_0_when_empty() {
            let bag = InventoryBag::default();
            assert_eq!(bag.count(), 0);
        }

        #[test]
        fn count_returns_correct_number() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 15);
            assert_eq!(bag.count(), 15);
        }

        #[test]
        fn count_updates_after_remove() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 10);
            bag.remove(5);
            assert_eq!(bag.count(), 9);
        }
    }

    mod inventory_bag_iter_tests {
        use super::*;

        #[test]
        fn iter_returns_empty_for_empty_bag() {
            let bag = InventoryBag::default();
            let spells: Vec<_> = bag.iter().collect();
            assert!(spells.is_empty());
        }

        #[test]
        fn iter_returns_all_spells() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            bag.add(create_radiant_beam_spell());
            bag.add(create_thunder_strike_spell());
            let spells: Vec<_> = bag.iter().collect();
            assert_eq!(spells.len(), 3);
        }

        #[test]
        fn iter_includes_slot_indices() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            bag.add(create_radiant_beam_spell());
            let spells: Vec<_> = bag.iter().collect();
            assert_eq!(spells[0].0, 0);
            assert_eq!(spells[1].0, 1);
        }

        #[test]
        fn iter_skips_empty_slots() {
            let mut bag = InventoryBag::default();
            bag.add(create_fireball_spell());
            bag.add(create_radiant_beam_spell());
            bag.add(create_thunder_strike_spell());
            bag.remove(1); // Remove middle spell
            let spells: Vec<_> = bag.iter().collect();
            assert_eq!(spells.len(), 2);
            assert_eq!(spells[0].0, 0);
            assert_eq!(spells[1].0, 2);
        }
    }

    mod inventory_bag_find_empty_slot_tests {
        use super::*;

        #[test]
        fn find_empty_slot_returns_0_when_empty() {
            let bag = InventoryBag::default();
            assert_eq!(bag.find_empty_slot(), Some(0));
        }

        #[test]
        fn find_empty_slot_returns_next_available() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 10);
            assert_eq!(bag.find_empty_slot(), Some(10));
        }

        #[test]
        fn find_empty_slot_returns_none_when_full() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 30);
            assert_eq!(bag.find_empty_slot(), None);
        }

        #[test]
        fn find_empty_slot_finds_gap_after_removal() {
            let mut bag = InventoryBag::default();
            fill_bag_with_n_spells(&mut bag, 10);
            bag.remove(5);
            assert_eq!(bag.find_empty_slot(), Some(5));
        }
    }
}
