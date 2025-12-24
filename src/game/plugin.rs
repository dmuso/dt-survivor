use bevy::prelude::*;
use crate::states::*;
use crate::game::systems::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(Update, game_input.run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game);
}