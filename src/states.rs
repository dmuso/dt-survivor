use bevy::prelude::*;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
    Intro,
    InGame,
    LevelComplete,
    GameOver,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_default_is_intro() {
        assert_eq!(GameState::default(), GameState::Intro);
    }

    #[test]
    fn game_state_has_level_complete() {
        let state = GameState::LevelComplete;
        assert_ne!(state, GameState::InGame);
        assert_ne!(state, GameState::GameOver);
    }
}