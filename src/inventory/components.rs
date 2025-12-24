use bevy::prelude::*;

#[derive(Component)]
pub struct EquippedWeapon {
    pub slot_index: usize,
}