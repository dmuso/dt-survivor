use bevy::prelude::*;
use crate::states::*;
use crate::ui::systems::*;
use crate::score::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Intro), setup_intro)
        .add_systems(Update, button_interactions.run_if(in_state(GameState::Intro)))
        .add_systems(OnExit(GameState::Intro), cleanup_intro)
        .add_systems(OnEnter(GameState::InGame), (setup_score_display, setup_game_ui))
        .add_systems(Update, update_score_display.run_if(in_state(GameState::InGame)))
        .add_systems(OnEnter(GameState::GameOver), setup_game_over_ui)
        .add_systems(Update, game_over_input.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), cleanup_game_over);
}