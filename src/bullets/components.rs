use bevy::prelude::*;

#[derive(Component)]
pub struct Bullet {
    pub direction: Vec2,
    pub speed: f32,
    pub lifetime: Timer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bullet_component_creation() {
        let bullet = Bullet {
            direction: Vec2::new(1.0, 0.0),
            speed: 300.0,
            lifetime: Timer::from_seconds(15.0, TimerMode::Once),
        };
        assert_eq!(bullet.direction, Vec2::new(1.0, 0.0));
        assert_eq!(bullet.speed, 300.0);
        assert_eq!(bullet.lifetime.duration(), std::time::Duration::from_secs_f32(15.0));
    }

    #[test]
    fn test_bullet_component_different_directions() {
        let bullet_right = Bullet {
            direction: Vec2::new(1.0, 0.0),
            speed: 250.0,
            lifetime: Timer::from_seconds(15.0, TimerMode::Once),
        };
        let bullet_up = Bullet {
            direction: Vec2::new(0.0, 1.0),
            speed: 250.0,
            lifetime: Timer::from_seconds(15.0, TimerMode::Once),
        };

        assert_eq!(bullet_right.direction, Vec2::new(1.0, 0.0));
        assert_eq!(bullet_up.direction, Vec2::new(0.0, 1.0));
        assert_eq!(bullet_right.speed, bullet_up.speed);
        assert_eq!(bullet_right.lifetime.duration(), bullet_up.lifetime.duration());
    }
}