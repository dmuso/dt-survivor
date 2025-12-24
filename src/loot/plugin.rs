use bevy::prelude::*;
use crate::states::*;
use crate::loot::systems::*;
use crate::game::events::LootDropEvent;

pub fn plugin(app: &mut App) {
    app
        .add_message::<LootDropEvent>()
        .add_systems(Update, (
            loot_spawning_system,
            loot_attraction_system,
            loot_movement_system,
            loot_drop_system,
            player_loot_collision_system,
        ).run_if(in_state(GameState::InGame)));
}