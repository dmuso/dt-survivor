use bevy::prelude::*;
use crate::states::*;
use crate::loot::systems::*;
use crate::game::events::EnemyDeathEvent;

pub fn plugin(app: &mut App) {
    app
        .add_message::<EnemyDeathEvent>()
        .add_systems(Update, (
            loot_spawning_system,
            loot_attraction_system,
            loot_movement_system,
            enemy_death_system,
            player_loot_collision_system,
        ).run_if(in_state(GameState::InGame)));
}