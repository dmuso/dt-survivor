use bevy::prelude::*;
use crate::states::*;
use crate::game::systems::*;
use crate::game::player::systems::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(Update, (
            game_input,
            player_movement,
            camera_follow_player,
        ).run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game);
}