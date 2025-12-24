use bevy::prelude::*;
use crate::states::*;
use crate::loot::systems::*;

pub fn plugin(app: &mut App) {
    app
        .add_systems(Update, (
            loot_spawning_system,
            player_loot_collision_system,
        ).run_if(in_state(GameState::InGame)));
}