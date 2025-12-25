use bevy::prelude::*;

/// Timer resource to throttle enemy death sounds
/// Prevents sounds from playing too frequently
#[derive(Resource, Default)]
pub struct EnemyDeathSoundTimer {
    pub time_since_last_sound: f32,
}