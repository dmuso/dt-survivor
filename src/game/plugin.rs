use bevy::prelude::*;
use crate::states::*;
use crate::enemies::systems::*;
use crate::game::systems::*;
use crate::player::systems::*;
use crate::game::resources::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<PlayerPosition>()
        .add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(Update, (
            game_input,
            player_movement,
            camera_follow_player,
            enemy_spawning_system,
            enemy_movement_system,
        ).chain().run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game);
}