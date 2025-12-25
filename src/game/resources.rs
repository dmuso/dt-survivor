use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct PlayerPosition(pub Vec2);

#[derive(Resource)]
pub struct EnemySpawnState {
    pub time_since_last_spawn: f32,
    pub spawn_rate_per_second: f32,
    pub time_since_last_rate_increase: f32,
    pub rate_level: u32,
}

impl Default for EnemySpawnState {
    fn default() -> Self {
        Self {
            time_since_last_spawn: 0.0,
            spawn_rate_per_second: 1.25, // Start with 1.25 enemies per second
            time_since_last_rate_increase: 0.0,
            rate_level: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct PlayerDamageTimer {
    pub time_since_last_damage: f32,
    pub has_taken_damage: bool,
}

#[derive(Resource, Default)]
pub struct ScreenTintEffect {
    pub remaining_duration: f32,
    pub color: Color,
}

/// Tracks how long the player has survived in the current game session
#[derive(Resource, Default)]
pub struct SurvivalTime(pub f32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_survival_time_default() {
        let time = SurvivalTime::default();
        assert_eq!(time.0, 0.0);
    }

    #[test]
    fn test_survival_time_increment() {
        let mut time = SurvivalTime::default();
        time.0 += 1.5;
        assert_eq!(time.0, 1.5);
    }
}