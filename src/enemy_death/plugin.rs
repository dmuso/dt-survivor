use bevy::prelude::*;
use crate::states::*;
use crate::enemy_death::systems::*;
use crate::game::events::{EnemyDeathEvent, LootDropEvent};

pub fn plugin(app: &mut App) {
    app
        .add_message::<EnemyDeathEvent>()
        .add_message::<LootDropEvent>()
        .add_systems(Update, enemy_death_system.run_if(in_state(GameState::InGame)));
}