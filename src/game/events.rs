use bevy::prelude::*;

/// Message fired when an enemy dies
#[derive(Message)]
pub struct EnemyDeathEvent {
    pub enemy_entity: Entity,
    pub position: Vec3,
    /// Level of the enemy that died (1-5)
    pub enemy_level: u8,
}

/// Message fired when loot should be dropped (typically when an enemy dies)
#[derive(Message)]
pub struct LootDropEvent {
    pub position: Vec3,
    /// Level of the enemy that died (1-5), determines XP orb quality
    pub enemy_level: u8,
}

/// Event fired when player collides with an enemy
#[derive(Message)]
pub struct PlayerEnemyCollisionEvent {
    pub player_entity: Entity,
    pub enemy_entity: Entity,
}

/// Event fired when a bullet collides with an enemy
#[derive(Message)]
pub struct BulletEnemyCollisionEvent {
    pub bullet_entity: Entity,
    pub enemy_entity: Entity,
}

/// Event fired when the game ends (player death)
#[derive(Message)]
pub struct GameOverEvent {
    pub final_score: u32,
    pub survival_time: f32,
}

/// Fired when the game advances to a new level
#[derive(Message, Debug)]
pub struct GameLevelUpEvent {
    pub new_level: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;

    #[test]
    fn test_game_over_event_creation() {
        let event = GameOverEvent {
            final_score: 1500,
            survival_time: 120.5,
        };
        assert_eq!(event.final_score, 1500);
        assert_eq!(event.survival_time, 120.5);
    }

    #[test]
    fn test_game_over_event_can_be_registered() {
        let mut app = App::new();
        app.add_message::<GameOverEvent>();
        // Should not panic
        app.update();
    }

    #[test]
    fn test_game_level_up_event_creation() {
        let event = GameLevelUpEvent { new_level: 5 };
        assert_eq!(event.new_level, 5);
    }

    #[test]
    fn test_game_level_up_event_can_be_registered() {
        let mut app = App::new();
        app.add_message::<GameLevelUpEvent>();
        // Should not panic
        app.update();
    }
}