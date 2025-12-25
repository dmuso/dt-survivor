use bevy::prelude::*;

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
}
