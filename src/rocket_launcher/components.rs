use bevy::prelude::*;
use bevy_hanabi::EffectAsset;
use bevy_kira_audio::AudioInstance;

/// Component storing the audio handle for the rocket's hiss sound.
/// Used to stop the sound when the rocket explodes.
#[derive(Component)]
pub struct RocketHissSound(pub Handle<AudioInstance>);

/// Resource storing the handle to the rocket exhaust particle effect.
#[derive(Resource)]
pub struct RocketExhaustEffect(pub Handle<EffectAsset>);

#[derive(Component)]
pub struct RocketProjectile {
    /// Velocity on the XZ ground plane (Vec2 where x=X, y=Z).
    pub velocity: Vec2,
    pub speed: f32,
    pub damage: f32,
    /// Target position on the XZ ground plane (Vec2 where x=X, y=Z).
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
    /// Create a new rocket projectile.
    /// `position` is the full 3D position (from Whisper).
    /// `initial_direction` is the direction on the XZ plane.
    pub fn new(position: Vec3, initial_direction: Vec2, damage: f32) -> (Self, Transform) {
        // Calculate rotation to face the initial direction
        let angle = initial_direction.y.atan2(initial_direction.x);
        let rotation = Quat::from_rotation_y(-angle + std::f32::consts::FRAC_PI_2);

        (
            Self {
                velocity: initial_direction.normalize() * 10.0, // Initial speed (3D units/sec)
                speed: 16.0, // Homing speed (3D units/sec)
                damage,
                target_position: None,
                homing_strength: 2.0, // How quickly it turns toward target
                state: RocketState::Pausing,
                pause_timer: Timer::from_seconds(0.5, TimerMode::Once),
            },
            Transform {
                translation: position, // Use Whisper's full 3D position
                rotation,
                ..default()
            },
        )
    }
}

#[derive(Component)]
pub struct TargetMarker {
    pub target_entity: Option<Entity>,
    pub lifetime: Timer,
}

impl TargetMarker {
    pub fn new(target_entity: Option<Entity>) -> Self {
        Self {
            target_entity,
            lifetime: Timer::from_seconds(1.0, TimerMode::Once), // Marker lasts 1 second
        }
    }

    /// Create a marker for a position without tracking a specific entity
    pub fn position_only() -> Self {
        Self {
            target_entity: None,
            lifetime: Timer::from_seconds(1.0, TimerMode::Once),
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
        // Full 3D position: x=100, y=3 (Whisper height), z=50
        let (rocket, transform) = RocketProjectile::new(Vec3::new(100.0, 3.0, 50.0), Vec2::new(1.0, 0.0), 30.0);

        assert_eq!(rocket.damage, 30.0);
        assert_eq!(rocket.speed, 16.0); // 3D world units/sec
        assert_eq!(rocket.homing_strength, 2.0);
        assert!(matches!(rocket.state, RocketState::Pausing));
        assert_eq!(rocket.pause_timer.duration(), std::time::Duration::from_secs_f32(0.5));

        // Transform should be at Whisper's full 3D position
        assert_eq!(transform.translation.x, 100.0);
        assert_eq!(transform.translation.y, 3.0); // Whisper height
        assert_eq!(transform.translation.z, 50.0);
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
        assert_eq!(explosion.max_radius, 3.0);  // 3 world units
        assert_eq!(explosion.damage, 30.0);
        assert_eq!(explosion.expansion_rate, 10.0);  // world units per second
        assert_eq!(explosion.max_lifetime, 0.5);

        // Test expansion
        explosion.current_radius += explosion.expansion_rate * 0.1; // 0.1 seconds
        assert_eq!(explosion.current_radius, 1.0);  // 10.0 * 0.1 = 1.0 world units

        // Test opacity fade
        let initial_opacity = explosion.get_opacity();
        explosion.lifetime.tick(std::time::Duration::from_secs_f32(0.2));
        let later_opacity = explosion.get_opacity();
        assert!(later_opacity < initial_opacity); // Should be fading
    }

    #[test]
    fn test_explosion_is_expanding() {
        let mut explosion = Explosion::new(Vec2::ZERO, 30.0);

        assert!(explosion.is_expanding()); // Starts at 0, max is 3.0

        explosion.current_radius = 3.0;
        assert!(!explosion.is_expanding()); // At max radius

        explosion.current_radius = 4.0;
        assert!(!explosion.is_expanding()); // Beyond max
    }

    #[test]
    fn test_target_marker_with_entity() {
        let entity = Entity::from_raw_u32(42).unwrap();
        let marker = TargetMarker::new(Some(entity));

        assert_eq!(marker.target_entity, Some(entity));
        assert_eq!(marker.lifetime.duration(), std::time::Duration::from_secs(1));
    }

    #[test]
    fn test_target_marker_position_only() {
        let marker = TargetMarker::position_only();

        assert!(marker.target_entity.is_none());
        assert_eq!(marker.lifetime.duration(), std::time::Duration::from_secs(1));
    }

    #[test]
    fn test_rocket_hiss_sound_component_exists() {
        // Verify the RocketHissSound component can be created with a default handle
        let handle: Handle<AudioInstance> = Handle::default();
        let hiss_sound = RocketHissSound(handle.clone());
        assert_eq!(hiss_sound.0.id(), handle.id());
    }
}

impl Explosion {
    /// Create a new explosion at the given XZ position (Vec2 where x=X, y=Z).
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: 3.0,       // 3 world units radius
            damage,
            expansion_rate: 10.0,  // world units per second
            lifetime: Timer::from_seconds(0.5, TimerMode::Once),
            max_lifetime: 0.5,
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