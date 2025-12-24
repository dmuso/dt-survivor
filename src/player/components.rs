use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub health: f32,
    pub max_health: f32,
    pub regen_rate: f32, // health per second
}

#[derive(Component)]
pub struct SlowModifier {
    pub remaining_duration: f32,
    pub speed_multiplier: f32, // 0.6 for 40% reduction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_component_creation() {
        // Test that player can be created with speed and health
        let player = Player {
            speed: 150.0,
            health: 100.0,
            max_health: 100.0,
            regen_rate: 1.0,
        };
        assert_eq!(player.speed, 150.0);
        assert_eq!(player.health, 100.0);
        assert_eq!(player.max_health, 100.0);
        assert_eq!(player.regen_rate, 1.0);

        // Test default player speed and health
        let default_player = Player {
            speed: 200.0,
            health: 100.0,
            max_health: 100.0,
            regen_rate: 1.0,
        };
        assert_eq!(default_player.speed, 200.0);
        assert_eq!(default_player.health, 100.0);
        assert_eq!(default_player.max_health, 100.0);
        assert_eq!(default_player.regen_rate, 1.0);
    }

    #[test]
    fn test_player_component_different_speeds() {
        let slow_player = Player {
            speed: 100.0,
            health: 100.0,
            max_health: 100.0,
            regen_rate: 1.0,
        };
        let fast_player = Player {
            speed: 500.0,
            health: 100.0,
            max_health: 100.0,
            regen_rate: 1.0,
        };

        assert_eq!(slow_player.speed, 100.0);
        assert_eq!(fast_player.speed, 500.0);
        assert!(fast_player.speed > slow_player.speed);
    }
}