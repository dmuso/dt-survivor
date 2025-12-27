use bevy::prelude::*;
use std::time::Duration;

/// Convert a 2D vector to a 3D vector on the XZ plane (Y=0).
/// Used for movement on the ground plane in 3D space.
#[inline]
pub fn to_xz(v: Vec2) -> Vec3 {
    Vec3::new(v.x, 0.0, v.y)
}

/// Extract XZ coordinates from a 3D position as a Vec2.
/// Used to get ground-plane position for distance calculations.
#[inline]
pub fn from_xz(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

/// Component for entities that have a movement speed.
/// This is a reusable component that can be used by any entity that needs to move.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Speed(pub f32);

impl Speed {
    pub fn new(speed: f32) -> Self {
        Self(speed)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self(100.0)
    }
}

/// Component for entities that have a velocity (direction + magnitude).
/// Velocity represents the current movement vector of an entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Default)]
pub struct Velocity(pub Vec2);

impl Velocity {
    pub fn new(velocity: Vec2) -> Self {
        Self(velocity)
    }

    pub fn from_direction_and_speed(direction: Vec2, speed: f32) -> Self {
        Self(direction.normalize_or_zero() * speed)
    }

    pub fn value(&self) -> Vec2 {
        self.0
    }

    pub fn magnitude(&self) -> f32 {
        self.0.length()
    }

    pub fn direction(&self) -> Vec2 {
        self.0.normalize_or_zero()
    }
}

/// Component for applying temporary knockback force to an entity.
/// Knockback has a direction, force magnitude, and duration timer.
/// When the timer finishes, the knockback effect should be removed.
#[derive(Component, Clone, Debug)]
pub struct Knockback {
    direction: Vec2,
    force: f32,
    duration: Timer,
}

impl Knockback {
    /// Default knockback force when using `from_direction`.
    pub const DEFAULT_FORCE: f32 = 300.0;
    /// Default knockback duration in seconds.
    pub const DEFAULT_DURATION: f32 = 0.2;

    /// Create a new knockback with specific direction, force, and duration.
    pub fn new(direction: Vec2, force: f32, duration_secs: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            force,
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }

    /// Create a knockback with default force and duration from a direction.
    pub fn from_direction(direction: Vec2) -> Self {
        Self::new(direction, Self::DEFAULT_FORCE, Self::DEFAULT_DURATION)
    }

    /// Get the normalized knockback direction.
    pub fn direction(&self) -> Vec2 {
        self.direction
    }

    /// Get the knockback force magnitude.
    pub fn force(&self) -> f32 {
        self.force
    }

    /// Get the velocity vector (direction * force).
    pub fn velocity(&self) -> Vec2 {
        self.direction * self.force
    }

    /// Advance the knockback timer by the given duration.
    pub fn tick(&mut self, delta: Duration) {
        self.duration.tick(delta);
    }

    /// Check if the knockback duration has finished.
    pub fn is_finished(&self) -> bool {
        self.duration.is_finished()
    }

    /// Get the remaining duration as a fraction (0.0 to 1.0).
    pub fn remaining_fraction(&self) -> f32 {
        1.0 - self.duration.fraction()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_creation() {
        let speed = Speed::new(150.0);
        assert_eq!(speed.value(), 150.0);
        assert_eq!(speed.0, 150.0);
    }

    #[test]
    fn test_speed_default() {
        let speed = Speed::default();
        assert_eq!(speed.value(), 100.0);
    }

    #[test]
    fn test_speed_clone_and_copy() {
        let speed = Speed::new(200.0);
        let cloned = speed.clone();
        let copied = speed;

        assert_eq!(speed, cloned);
        assert_eq!(speed, copied);
    }

    #[test]
    fn test_speed_debug() {
        let speed = Speed::new(50.0);
        let debug_str = format!("{:?}", speed);
        assert!(debug_str.contains("50.0"));
    }

    #[test]
    fn test_velocity_creation() {
        let velocity = Velocity::new(Vec2::new(100.0, 50.0));
        assert_eq!(velocity.value(), Vec2::new(100.0, 50.0));
        assert_eq!(velocity.0, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn test_velocity_default() {
        let velocity = Velocity::default();
        assert_eq!(velocity.value(), Vec2::ZERO);
    }

    #[test]
    fn test_velocity_from_direction_and_speed() {
        let velocity = Velocity::from_direction_and_speed(Vec2::new(1.0, 0.0), 100.0);
        assert_eq!(velocity.value(), Vec2::new(100.0, 0.0));

        // Test with unnormalized direction
        let velocity2 = Velocity::from_direction_and_speed(Vec2::new(3.0, 4.0), 50.0);
        let expected_direction = Vec2::new(3.0, 4.0).normalize();
        let expected = expected_direction * 50.0;
        assert!((velocity2.value() - expected).length() < 0.001);
    }

    #[test]
    fn test_velocity_from_zero_direction() {
        let velocity = Velocity::from_direction_and_speed(Vec2::ZERO, 100.0);
        assert_eq!(velocity.value(), Vec2::ZERO);
    }

    #[test]
    fn test_velocity_magnitude() {
        let velocity = Velocity::new(Vec2::new(3.0, 4.0));
        assert_eq!(velocity.magnitude(), 5.0);
    }

    #[test]
    fn test_velocity_direction() {
        let velocity = Velocity::new(Vec2::new(100.0, 0.0));
        assert_eq!(velocity.direction(), Vec2::new(1.0, 0.0));

        let velocity2 = Velocity::new(Vec2::new(3.0, 4.0));
        let expected = Vec2::new(3.0, 4.0).normalize();
        assert!((velocity2.direction() - expected).length() < 0.001);
    }

    #[test]
    fn test_velocity_direction_zero() {
        let velocity = Velocity::new(Vec2::ZERO);
        assert_eq!(velocity.direction(), Vec2::ZERO);
    }

    #[test]
    fn test_velocity_clone_and_copy() {
        let velocity = Velocity::new(Vec2::new(10.0, 20.0));
        let cloned = velocity.clone();
        let copied = velocity;

        assert_eq!(velocity, cloned);
        assert_eq!(velocity, copied);
    }

    #[test]
    fn test_components_can_be_added_to_entity() {
        use bevy::app::App;

        let mut app = App::new();

        let entity = app
            .world_mut()
            .spawn((Speed::new(200.0), Velocity::new(Vec2::new(50.0, 50.0))))
            .id();

        let speed = app.world().get::<Speed>(entity).unwrap();
        let velocity = app.world().get::<Velocity>(entity).unwrap();

        assert_eq!(speed.value(), 200.0);
        assert_eq!(velocity.value(), Vec2::new(50.0, 50.0));
    }

    #[test]
    fn test_knockback_creation() {
        let knockback = Knockback::new(Vec2::new(1.0, 0.0), 500.0, 0.3);
        assert_eq!(knockback.direction(), Vec2::new(1.0, 0.0));
        assert_eq!(knockback.force(), 500.0);
        assert!(!knockback.is_finished());
    }

    #[test]
    fn test_knockback_from_direction() {
        let knockback = Knockback::from_direction(Vec2::new(3.0, 4.0));
        let expected_dir = Vec2::new(3.0, 4.0).normalize();
        assert!((knockback.direction() - expected_dir).length() < 0.001);
        assert_eq!(knockback.force(), Knockback::DEFAULT_FORCE);
    }

    #[test]
    fn test_knockback_from_zero_direction() {
        let knockback = Knockback::from_direction(Vec2::ZERO);
        assert_eq!(knockback.direction(), Vec2::ZERO);
    }

    #[test]
    fn test_knockback_velocity() {
        let knockback = Knockback::new(Vec2::new(1.0, 0.0), 200.0, 0.5);
        assert_eq!(knockback.velocity(), Vec2::new(200.0, 0.0));

        let knockback2 = Knockback::new(Vec2::new(0.0, 1.0), 150.0, 0.5);
        assert_eq!(knockback2.velocity(), Vec2::new(0.0, 150.0));
    }

    #[test]
    fn test_knockback_tick_decreases_duration() {
        let mut knockback = Knockback::new(Vec2::new(1.0, 0.0), 500.0, 0.5);
        assert!(!knockback.is_finished());

        knockback.tick(Duration::from_secs_f32(0.3));
        assert!(!knockback.is_finished());

        knockback.tick(Duration::from_secs_f32(0.3));
        assert!(knockback.is_finished());
    }

    #[test]
    fn test_knockback_can_be_added_to_entity() {
        use bevy::app::App;

        let mut app = App::new();

        let entity = app
            .world_mut()
            .spawn(Knockback::new(Vec2::new(1.0, 0.0), 300.0, 0.2))
            .id();

        let knockback = app.world().get::<Knockback>(entity).unwrap();
        assert_eq!(knockback.force(), 300.0);
        assert!(!knockback.is_finished());
    }

    // XZ coordinate helper tests
    #[test]
    fn test_to_xz_converts_vec2_to_vec3() {
        let v2 = Vec2::new(10.0, 20.0);
        let v3 = to_xz(v2);
        assert_eq!(v3.x, 10.0);
        assert_eq!(v3.y, 0.0);
        assert_eq!(v3.z, 20.0);
    }

    #[test]
    fn test_to_xz_with_zero_vector() {
        let v2 = Vec2::ZERO;
        let v3 = to_xz(v2);
        assert_eq!(v3, Vec3::ZERO);
    }

    #[test]
    fn test_to_xz_with_negative_values() {
        let v2 = Vec2::new(-5.0, -15.0);
        let v3 = to_xz(v2);
        assert_eq!(v3.x, -5.0);
        assert_eq!(v3.y, 0.0);
        assert_eq!(v3.z, -15.0);
    }

    #[test]
    fn test_from_xz_extracts_x_and_z() {
        let v3 = Vec3::new(10.0, 5.0, 20.0);
        let v2 = from_xz(v3);
        assert_eq!(v2.x, 10.0);
        assert_eq!(v2.y, 20.0);
    }

    #[test]
    fn test_from_xz_ignores_y_component() {
        let v3_with_y = Vec3::new(1.0, 100.0, 2.0);
        let v2 = from_xz(v3_with_y);
        assert_eq!(v2.x, 1.0);
        assert_eq!(v2.y, 2.0);
    }

    #[test]
    fn test_from_xz_with_zero_vector() {
        let v3 = Vec3::ZERO;
        let v2 = from_xz(v3);
        assert_eq!(v2, Vec2::ZERO);
    }

    #[test]
    fn test_to_xz_and_from_xz_roundtrip() {
        let original = Vec2::new(42.0, 99.0);
        let converted = to_xz(original);
        let back = from_xz(converted);
        assert_eq!(original, back);
    }
}
