use bevy::prelude::*;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
    Intro,
    AttunementSelect,
    InGame,
    InventoryOpen,
    LevelComplete,
    GameOver,
    Paused,
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

    #[test]
    fn game_state_has_attunement_select() {
        let state = GameState::AttunementSelect;
        assert_ne!(state, GameState::InGame);
        assert_ne!(state, GameState::Intro);
    }

    #[test]
    fn game_state_has_inventory_open() {
        let state = GameState::InventoryOpen;
        assert_ne!(state, GameState::InGame);
        assert_ne!(state, GameState::AttunementSelect);
    }

    #[test]
    fn game_state_derives_clone() {
        let state = GameState::AttunementSelect;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn game_state_all_states_are_distinct() {
        let states = [
            GameState::Intro,
            GameState::AttunementSelect,
            GameState::InGame,
            GameState::InventoryOpen,
            GameState::LevelComplete,
            GameState::GameOver,
            GameState::Paused,
        ];
        // Check all pairs are distinct
        for (i, s1) in states.iter().enumerate() {
            for (j, s2) in states.iter().enumerate() {
                if i != j {
                    assert_ne!(s1, s2, "States at indices {} and {} should be distinct", i, j);
                }
            }
        }
    }

    #[test]
    fn game_state_has_paused() {
        let state = GameState::Paused;
        assert_ne!(state, GameState::InGame);
        assert_ne!(state, GameState::Intro);
    }
}