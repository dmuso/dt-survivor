use bevy::prelude::*;

#[derive(Component)]
pub struct LaserBeam {
    pub start_pos: Vec2,
    pub end_pos: Vec2,
    pub direction: Vec2,
    pub lifetime: Timer,
    pub max_lifetime: f32,
    pub damage: f32,
    /// Y height in 3D world (fires from Whisper's height)
    pub y_height: f32,
}

impl LaserBeam {
    /// Creates a new LaserBeam at the default height (0.5).
    /// Prefer `with_height` for proper 3D positioning from Whisper.
    pub fn new(start_pos: Vec2, direction: Vec2, damage: f32) -> Self {
        Self::with_height(start_pos, direction, damage, 0.5)
    }

    /// Creates a new LaserBeam at the specified Y height.
    pub fn with_height(start_pos: Vec2, direction: Vec2, damage: f32, y_height: f32) -> Self {
        let end_pos = start_pos + direction * 800.0;
        Self {
            start_pos,
            end_pos,
            direction,
            lifetime: Timer::from_seconds(0.5, TimerMode::Once), // 0.5 second duration
            max_lifetime: 0.5,
            damage,
            y_height,
        }
    }

    pub fn get_thickness(&self) -> f32 {
        let progress = self.lifetime.elapsed_secs() / self.max_lifetime;
        if progress < 0.3 {
            // First 30%: thin to medium (2 to 8 pixels)
            2.0 + (progress / 0.3) * 6.0
        } else if progress < 0.7 {
            // Next 40%: medium to thick (8 to 15 pixels)
            8.0 + ((progress - 0.3) / 0.4) * 7.0
        } else {
            // Last 30%: dissipate (15 to 0 pixels)
            let dissipate_progress = (progress - 0.7) / 0.3;
            15.0 * (1.0 - dissipate_progress)
        }
    }

    pub fn is_active(&self) -> bool {
        self.lifetime.elapsed_secs() < self.max_lifetime
    }
}