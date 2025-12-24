use bevy::prelude::*;

#[derive(Component)]
pub struct RocketProjectile {
    pub velocity: Vec2,
    pub speed: f32,
    pub damage: f32,
    pub target_position: Option<Vec2>,
    pub homing_strength: f32,
    pub state: RocketState,
    pub pause_timer: Timer,
}

#[derive(Clone, Debug)]
pub enum RocketState {
    Pausing,      // Initial 0.5s pause
    Targeting,    // Acquiring target
    Homing,       // Tracking target
    Exploding,    // Hit something, creating explosion
}

impl RocketProjectile {
    pub fn new(position: Vec2, initial_direction: Vec2, damage: f32) -> (Self, Transform) {
        (
            Self {
                velocity: initial_direction.normalize() * 100.0, // Initial speed
                speed: 150.0, // Homing speed
                damage,
                target_position: None,
                homing_strength: 2.0, // How quickly it turns toward target
                state: RocketState::Pausing,
                pause_timer: Timer::from_seconds(0.5, TimerMode::Once),
            },
            Transform::from_translation(position.extend(0.3)), // Above other projectiles
        )
    }
}

#[derive(Component)]
pub struct TargetMarker {
    pub target_entity: Entity,
    pub lifetime: Timer,
}

impl TargetMarker {
    pub fn new(target_entity: Entity) -> Self {
        Self {
            target_entity,
            lifetime: Timer::from_seconds(1.0, TimerMode::Once), // Marker lasts 1 second
        }
    }
}

#[derive(Component)]
pub struct Explosion {
    pub center: Vec2,
    pub current_radius: f32,
    pub max_radius: f32,
    pub damage: f32,
    pub expansion_rate: f32,
    pub lifetime: Timer,
    pub max_lifetime: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rocket_projectile_creation() {
        let (rocket, transform) = RocketProjectile::new(Vec2::new(100.0, 50.0), Vec2::new(1.0, 0.0), 30.0);

        assert_eq!(rocket.damage, 30.0);
        assert_eq!(rocket.speed, 150.0);
        assert_eq!(rocket.homing_strength, 2.0);
        assert!(matches!(rocket.state, RocketState::Pausing));
        assert_eq!(rocket.pause_timer.duration(), std::time::Duration::from_secs_f32(0.5));

        assert_eq!(transform.translation, Vec3::new(100.0, 50.0, 0.3));
    }

    #[test]
    fn test_rocket_projectile_state_transitions() {
        let mut rocket = RocketProjectile {
            velocity: Vec2::new(100.0, 0.0),
            speed: 150.0,
            damage: 30.0,
            target_position: None,
            homing_strength: 2.0,
            state: RocketState::Pausing,
            pause_timer: Timer::from_seconds(0.5, TimerMode::Once),
        };

        // Initially pausing
        assert!(matches!(rocket.state, RocketState::Pausing));

        // After pause timer finishes, should transition to targeting
        rocket.pause_timer.tick(std::time::Duration::from_secs_f32(0.6));
        assert!(rocket.pause_timer.is_finished());
        // Note: State transition happens in the system, not automatically in the component
    }



    #[test]
    fn test_explosion_creation_and_expansion() {
        let center = Vec2::new(50.0, 75.0);
        let mut explosion = Explosion::new(center, 30.0);

        assert_eq!(explosion.center, center);
        assert_eq!(explosion.current_radius, 0.0);
        assert_eq!(explosion.max_radius, 150.0);
        assert_eq!(explosion.damage, 30.0);
        assert_eq!(explosion.expansion_rate, 500.0);
        assert_eq!(explosion.max_lifetime, 0.3);

        // Test expansion
        explosion.current_radius += explosion.expansion_rate * 0.1; // 0.1 seconds
        assert_eq!(explosion.current_radius, 50.0);

        // Test opacity fade
        let initial_opacity = explosion.get_opacity();
        explosion.lifetime.tick(std::time::Duration::from_secs_f32(0.2));
        let later_opacity = explosion.get_opacity();
        assert!(later_opacity < initial_opacity); // Should be fading
    }

    #[test]
    fn test_explosion_is_expanding() {
        let mut explosion = Explosion::new(Vec2::ZERO, 30.0);

        assert!(explosion.is_expanding()); // Starts at 0, max is 150

        explosion.current_radius = 150.0;
        assert!(!explosion.is_expanding()); // At max radius

        explosion.current_radius = 200.0;
        assert!(!explosion.is_expanding()); // Beyond max
    }
}

impl Explosion {
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: 150.0,
            damage,
            expansion_rate: 500.0, // pixels per second
            lifetime: Timer::from_seconds(0.3, TimerMode::Once), // 0.3 second expansion
            max_lifetime: 0.3,
        }
    }

    pub fn get_opacity(&self) -> f32 {
        let progress = self.lifetime.elapsed_secs() / self.max_lifetime;
        1.0 - progress // Fade from 1.0 to 0.0
    }

    pub fn is_expanding(&self) -> bool {
        self.current_radius < self.max_radius
    }
}