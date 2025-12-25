use bevy::prelude::*;
use crate::states::*;
use crate::loot::systems::*;
use crate::loot::events::*;
use crate::game::events::LootDropEvent;

pub fn plugin(app: &mut App) {
    app
        .add_message::<LootDropEvent>()
        .add_message::<PickupEvent>()
        .add_message::<ItemEffectEvent>()
        .add_systems(Update, (
            // Loot drop system spawns DroppedItem entities from enemy deaths
            loot_drop_system.run_if(in_state(GameState::InGame)),

            // ECS-based pickup systems
            detect_pickup_collisions.run_if(in_state(GameState::InGame)),
            update_item_attraction.run_if(in_state(GameState::InGame)),
            update_item_movement.run_if(in_state(GameState::InGame)),
            process_pickup_events.run_if(in_state(GameState::InGame)),
            apply_item_effects.run_if(in_state(GameState::InGame)),
            cleanup_consumed_items.run_if(in_state(GameState::InGame)),
        ));
}