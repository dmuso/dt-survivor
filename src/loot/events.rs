use bevy::prelude::*;

/// Event fired when a pickup item enters the player's pickup radius
#[derive(Message)]
pub struct PickupEvent {
    pub item_entity: Entity,
    pub player_entity: Entity,
}

/// Event fired when a pickup item collides with the player and effects should be applied
#[derive(Message)]
pub struct ItemEffectEvent {
    pub item_entity: Entity,
    pub item_data: crate::loot::components::ItemData,
    pub player_entity: Entity,
}

