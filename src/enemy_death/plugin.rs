use bevy::prelude::*;
use crate::states::*;
use crate::enemy_death::systems::*;
use crate::enemy_death::components::*;
use crate::game::events::{EnemyDeathEvent, LootDropEvent};

pub fn plugin(app: &mut App) {
    app
        .add_message::<EnemyDeathEvent>()
        .add_message::<LootDropEvent>()
        .init_resource::<EnemyDeathSoundTimer>()
        .add_systems(Update, enemy_death_system.run_if(in_state(GameState::InGame)));
}