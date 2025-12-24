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
            spawn_rate_per_second: 5.0, // Start with 5 enemies per second
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