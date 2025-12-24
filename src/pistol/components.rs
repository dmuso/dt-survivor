use bevy::prelude::*;

/// Pistol-specific weapon configuration
#[derive(Clone, Debug, PartialEq)]
pub struct PistolConfig {
    pub bullet_count: usize,
    pub spread_angle: f32,
    pub bullet_speed: f32,
    pub bullet_lifetime: f32,
    pub bullet_color: Color,
    pub bullet_size: Vec2,
}

impl Default for PistolConfig {
    fn default() -> Self {
        Self {
            bullet_count: 5,
            spread_angle: 15.0,
            bullet_speed: 200.0,
            bullet_lifetime: 15.0,
            bullet_color: Color::srgb(1.0, 1.0, 0.0), // Yellow
            bullet_size: Vec2::new(8.0, 8.0),
        }
    }
}