use bevy::prelude::*;
use crate::states::*;
use crate::audio::systems::*;

pub fn plugin(app: &mut App) {
    app
        .add_systems(OnEnter(GameState::Intro), setup_background_music)
        .add_systems(OnEnter(GameState::InGame), setup_background_music)
        .add_systems(Update, (
            maintain_background_music,
            cleanup_weapon_sounds,
            cleanup_enemy_death_sounds,
            cleanup_loot_pickup_sounds,
        ).run_if(in_state(GameState::Intro).or(in_state(GameState::InGame))));
}