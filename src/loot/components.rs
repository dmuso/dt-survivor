use bevy::prelude::*;
use crate::weapon::components::Weapon;

#[derive(Component)]
pub struct LootItem {
    pub loot_type: LootType,
}

#[derive(Clone, Debug)]
pub enum LootType {
    Weapon(Weapon),
    HealthPack { heal_amount: f32 },
}

/// Marker component for loot pickup sound effects
#[derive(Component)]
pub struct LootPickupSound;

/// Timer component for loot pickup audio cleanup
#[derive(Component)]
pub struct LootAudioCleanupTimer(pub Timer);