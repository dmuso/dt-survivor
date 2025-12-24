use bevy::prelude::*;
use crate::states::*;
use crate::ui::systems::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Intro), setup_intro)
        .add_systems(Update, button_interactions.run_if(in_state(GameState::Intro)))
        .add_systems(OnExit(GameState::Intro), cleanup_intro);
}