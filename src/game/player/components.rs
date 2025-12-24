use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_component_creation() {
        // Test that player can be created with speed
        let player = Player { speed: 150.0 };
        assert_eq!(player.speed, 150.0);

        // Test default player speed
        let default_player = Player { speed: 200.0 };
        assert_eq!(default_player.speed, 200.0);
    }

    #[test]
    fn test_player_component_different_speeds() {
        let slow_player = Player { speed: 100.0 };
        let fast_player = Player { speed: 500.0 };

        assert_eq!(slow_player.speed, 100.0);
        assert_eq!(fast_player.speed, 500.0);
        assert!(fast_player.speed > slow_player.speed);
    }
}