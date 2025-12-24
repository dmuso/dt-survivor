use bevy::prelude::*;
use crate::weapon::components::*;

#[derive(Resource)]
pub struct Inventory {
    pub slots: Vec<Option<Weapon>>, // Vec with 5 slots
}

impl Default for Inventory {
    fn default() -> Self {
        let mut slots = vec![None; 5];
        // Slot 0: Default pistol weapon
        slots[0] = Some(Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0
            },
            fire_rate: 2.0,
            damage: 1.0,
            last_fired: -2.0, // Prevent immediate firing at startup
        });
        Self { slots }
    }
}