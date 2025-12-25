use bevy::prelude::*;
use crate::weapon::components::*;

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