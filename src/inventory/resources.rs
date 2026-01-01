use bevy::prelude::*;
use crate::weapon::components::*;
use crate::spell::{Spell, SpellType};

/// Player's weapon inventory. Starts empty until Whisper is collected.
#[derive(Resource, Default)]
pub struct Inventory {
    pub weapons: std::collections::HashMap<String, Weapon>, // weapon_id -> Weapon
}

impl Inventory {
    pub fn get_weapon(&self, weapon_type: &WeaponType) -> Option<&Weapon> {
        self.weapons.get(weapon_type.id())
    }

    pub fn get_weapon_mut(&mut self, weapon_type: &WeaponType) -> Option<&mut Weapon> {
        self.weapons.get_mut(weapon_type.id())
    }

    pub fn add_or_level_weapon(&mut self, mut weapon: Weapon) -> bool {
        let id = weapon.weapon_type.id().to_string();
        if let Some(existing_weapon) = self.weapons.get_mut(&id) {
            if existing_weapon.can_level_up() {
                existing_weapon.level_up();
                true // Successfully leveled up
            } else {
                false // Already at max level
            }
        } else {
            // New weapon, add at level 1
            weapon.level = 1;
            self.weapons.insert(id, weapon);
            true
        }
    }

    pub fn iter_weapons(&self) -> impl Iterator<Item = (&String, &Weapon)> {
        self.weapons.iter()
    }
}

/// Player's active spell slots. Contains up to 5 equipped spells for combat.
#[derive(Resource, Default)]
pub struct SpellList {
    slots: [Option<Spell>; 5],
}

impl SpellList {
    /// Equip spell to first empty slot, returns slot index or None if full.
    pub fn equip(&mut self, spell: Spell) -> Option<usize> {
        if let Some(slot_index) = self.find_empty_slot() {
            self.slots[slot_index] = Some(spell);
            Some(slot_index)
        } else {
            None
        }
    }

    /// Find first empty slot index.
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// Check if specific spell type is already equipped.
    pub fn has_spell(&self, spell_type: &SpellType) -> bool {
        self.slots.iter().any(|s| {
            s.as_ref()
                .is_some_and(|spell| &spell.spell_type == spell_type)
        })
    }

    /// Get spell at specific slot (0-4).
    pub fn get_spell(&self, slot: usize) -> Option<&Spell> {
        self.slots.get(slot)?.as_ref()
    }

    /// Get mutable spell at specific slot for leveling up.
    pub fn get_spell_mut(&mut self, slot: usize) -> Option<&mut Spell> {
        self.slots.get_mut(slot)?.as_mut()
    }

    /// Iterate over all equipped spells with their slot indices.
    pub fn iter_spells(&self) -> impl Iterator<Item = (usize, &Spell)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|spell| (i, spell)))
    }

    /// Find slot containing specific spell type.
    pub fn find_spell_slot(&self, spell_type: &SpellType) -> Option<usize> {
        self.slots.iter().position(|s| {
            s.as_ref()
                .is_some_and(|spell| &spell.spell_type == spell_type)
        })
    }

    /// Remove spell from slot, returns removed spell.
    pub fn remove(&mut self, slot: usize) -> Option<Spell> {
        if slot < 5 {
            self.slots[slot].take()
        } else {
            None
        }
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

    fn create_frost_nova_spell() -> Spell {
        Spell::new(SpellType::FrostNova)
    }

    fn create_void_rift_spell() -> Spell {
        Spell::new(SpellType::VoidRift)
    }

    mod spell_list_equip_tests {
        use super::*;

        #[test]
        fn equip_to_empty_spell_list_returns_slot_0() {
            let mut spell_list = SpellList::default();
            let spell = create_fireball_spell();
            let result = spell_list.equip(spell);
            assert_eq!(result, Some(0));
        }

        #[test]
        fn equip_to_empty_spell_list_stores_spell() {
            let mut spell_list = SpellList::default();
            let spell = create_fireball_spell();
            spell_list.equip(spell);
            assert!(spell_list.get_spell(0).is_some());
        }

        #[test]
        fn equip_multiple_spells_fills_consecutive_slots() {
            let mut spell_list = SpellList::default();
            assert_eq!(spell_list.equip(create_fireball_spell()), Some(0));
            assert_eq!(spell_list.equip(create_radiant_beam_spell()), Some(1));
            assert_eq!(spell_list.equip(create_thunder_strike_spell()), Some(2));
        }

        #[test]
        fn equip_returns_none_when_full() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            spell_list.equip(create_thunder_strike_spell());
            spell_list.equip(create_frost_nova_spell());
            spell_list.equip(create_void_rift_spell());
            // SpellList is now full (5 spells)
            let result = spell_list.equip(create_fireball_spell());
            assert_eq!(result, None);
        }
    }

    mod spell_list_find_empty_slot_tests {
        use super::*;

        #[test]
        fn find_empty_slot_returns_0_when_empty() {
            let spell_list = SpellList::default();
            assert_eq!(spell_list.find_empty_slot(), Some(0));
        }

        #[test]
        fn find_empty_slot_returns_next_available() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            assert_eq!(spell_list.find_empty_slot(), Some(2));
        }

        #[test]
        fn find_empty_slot_returns_none_when_full() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            spell_list.equip(create_thunder_strike_spell());
            spell_list.equip(create_frost_nova_spell());
            spell_list.equip(create_void_rift_spell());
            assert_eq!(spell_list.find_empty_slot(), None);
        }
    }

    mod spell_list_has_spell_tests {
        use super::*;

        #[test]
        fn has_spell_returns_true_for_equipped_spell() {
            let mut spell_list = SpellList::default();
            let spell_type = SpellType::Fireball;
            spell_list.equip(create_fireball_spell());
            assert!(spell_list.has_spell(&spell_type));
        }

        #[test]
        fn has_spell_returns_false_for_missing_spell() {
            let spell_list = SpellList::default();
            let spell_type = SpellType::Fireball;
            assert!(!spell_list.has_spell(&spell_type));
        }

        #[test]
        fn has_spell_compares_by_spell_type() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            // Same spell type matches
            assert!(spell_list.has_spell(&SpellType::Fireball));
            // Different spell type does not match
            assert!(!spell_list.has_spell(&SpellType::IceShard));
        }
    }

    mod spell_list_get_spell_tests {
        use super::*;

        #[test]
        fn get_spell_returns_some_for_valid_slot() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            assert!(spell_list.get_spell(0).is_some());
        }

        #[test]
        fn get_spell_returns_none_for_empty_slot() {
            let spell_list = SpellList::default();
            assert!(spell_list.get_spell(0).is_none());
        }

        #[test]
        fn get_spell_returns_none_for_out_of_bounds() {
            let spell_list = SpellList::default();
            assert!(spell_list.get_spell(5).is_none());
            assert!(spell_list.get_spell(100).is_none());
        }

        #[test]
        fn get_spell_returns_correct_spell() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            let spell = spell_list.get_spell(1).unwrap();
            assert_eq!(spell.spell_type, SpellType::RadiantBeam);
        }
    }

    mod spell_list_get_spell_mut_tests {
        use super::*;

        #[test]
        fn get_spell_mut_allows_level_up() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            if let Some(spell) = spell_list.get_spell_mut(0) {
                let old_level = spell.level;
                spell.level_up();
                assert_eq!(spell.level, old_level + 1);
            } else {
                panic!("Expected spell at slot 0");
            }
        }

        #[test]
        fn get_spell_mut_returns_none_for_empty_slot() {
            let mut spell_list = SpellList::default();
            assert!(spell_list.get_spell_mut(0).is_none());
        }

        #[test]
        fn get_spell_mut_returns_none_for_out_of_bounds() {
            let mut spell_list = SpellList::default();
            assert!(spell_list.get_spell_mut(5).is_none());
        }
    }

    mod spell_list_iter_spells_tests {
        use super::*;

        #[test]
        fn iter_spells_returns_empty_for_empty_list() {
            let spell_list = SpellList::default();
            let spells: Vec<_> = spell_list.iter_spells().collect();
            assert!(spells.is_empty());
        }

        #[test]
        fn iter_spells_returns_all_equipped_spells() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            let spells: Vec<_> = spell_list.iter_spells().collect();
            assert_eq!(spells.len(), 2);
        }

        #[test]
        fn iter_spells_includes_slot_indices() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            let spells: Vec<_> = spell_list.iter_spells().collect();
            assert_eq!(spells[0].0, 0);
            assert_eq!(spells[1].0, 1);
        }

        #[test]
        fn iter_spells_skips_empty_slots_after_removal() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            spell_list.equip(create_thunder_strike_spell());
            spell_list.remove(1); // Remove middle spell
            let spells: Vec<_> = spell_list.iter_spells().collect();
            assert_eq!(spells.len(), 2);
            assert_eq!(spells[0].0, 0); // Fireball at slot 0
            assert_eq!(spells[1].0, 2); // Thunder Strike at slot 2
        }
    }

    mod spell_list_find_spell_slot_tests {
        use super::*;

        #[test]
        fn find_spell_slot_returns_correct_index() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.equip(create_radiant_beam_spell());
            let slot = spell_list.find_spell_slot(&SpellType::RadiantBeam);
            assert_eq!(slot, Some(1));
        }

        #[test]
        fn find_spell_slot_returns_none_for_missing() {
            let spell_list = SpellList::default();
            let slot = spell_list.find_spell_slot(&SpellType::RadiantBeam);
            assert_eq!(slot, None);
        }
    }

    mod spell_list_remove_tests {
        use super::*;

        #[test]
        fn remove_returns_spell_from_valid_slot() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            let removed = spell_list.remove(0);
            assert!(removed.is_some());
            assert_eq!(removed.unwrap().spell_type, SpellType::Fireball);
        }

        #[test]
        fn remove_clears_slot() {
            let mut spell_list = SpellList::default();
            spell_list.equip(create_fireball_spell());
            spell_list.remove(0);
            assert!(spell_list.get_spell(0).is_none());
        }

        #[test]
        fn remove_returns_none_for_empty_slot() {
            let mut spell_list = SpellList::default();
            let removed = spell_list.remove(0);
            assert!(removed.is_none());
        }

        #[test]
        fn remove_returns_none_for_out_of_bounds() {
            let mut spell_list = SpellList::default();
            let removed = spell_list.remove(5);
            assert!(removed.is_none());
        }
    }
}