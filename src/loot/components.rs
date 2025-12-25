use bevy::prelude::*;
use crate::weapon::components::Weapon;

#[derive(Component)]
pub struct DroppedItem {
    pub pickup_state: PickupState,
    pub item_data: ItemData,
    pub velocity: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PickupState {
    Idle,           // Waiting to be picked up
    BeingAttracted, // Moving toward player
    PickedUp,       // Just picked up, effects being applied
    Consumed,       // Effects applied, ready for cleanup
}

#[derive(Clone, Debug)]
pub enum ItemData {
    Weapon(Weapon),
    HealthPack { heal_amount: f32 },
    Experience { amount: u32 },
    Powerup(crate::powerup::components::PowerupType),
}

// Legacy components for backwards compatibility during migration
#[derive(Component)]
pub struct LootItem {
    pub loot_type: LootType,
    pub velocity: Vec2,
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