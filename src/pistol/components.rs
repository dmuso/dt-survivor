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
            bullet_speed: 10.0, // 3D world units/sec (was 200 pixels/sec in 2D)
            bullet_lifetime: 5.0, // Reduced for smaller world
            bullet_color: Color::srgb(1.0, 1.0, 0.0), // Yellow
            bullet_size: Vec2::new(0.3, 0.3), // 3D world units (was 8 pixels in 2D)
        }
    }
}