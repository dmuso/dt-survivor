use bevy::prelude::*;

/// Message fired when an enemy dies
#[derive(Message)]
pub struct EnemyDeathEvent {
    pub enemy_entity: Entity,
    pub position: Vec2,
}

/// Message fired when loot should be dropped (typically when an enemy dies)
#[derive(Message)]
pub struct LootDropEvent {
    pub position: Vec2,
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
}