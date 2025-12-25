use bevy::prelude::*;
use crate::weapon::components::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct Inventory {
    pub weapons: std::collections::HashMap<String, Weapon>, // weapon_id -> Weapon
}

impl Default for Inventory {
    fn default() -> Self {
        let mut weapons = HashMap::new();
        // Default pistol weapon
        weapons.insert("pistol".to_string(), Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0
            },
            level: 1,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: -2.0, // Prevent immediate firing at startup
        });
        Self { weapons }
    }
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