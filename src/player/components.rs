use bevy::animation::graph::AnimationNodeIndex;
use bevy::prelude::*;

/// Marker component for the spotlight that follows the player
#[derive(Component)]
pub struct PlayerSpotlight;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub regen_rate: f32, // health per second
    pub pickup_radius: f32, // Radius within which loot is attracted to player
    /// Last non-zero movement direction (normalized) for pickup rotation effect
    pub last_movement_direction: Vec3,
}

#[derive(Component)]
pub struct SlowModifier {
    pub remaining_duration: f32,
    pub speed_multiplier: f32, // 0.6 for 40% reduction
}

/// Marker component for the player's 3D model entity (child of the Player entity)
#[derive(Component)]
pub struct PlayerModel;

/// Tracks the current animation state for the player
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerAnimationState {
    #[default]
    Idle,
    Run,
}

/// Resource holding handles to the player's animation clips
#[derive(Resource)]
pub struct PlayerAnimations {
    /// Handle to the player GLTF scene
    pub scene: Handle<Scene>,
    /// Animation graph for the player
    pub graph: Handle<AnimationGraph>,
    /// Node index for idle animation
    pub idle_node: AnimationNodeIndex,
    /// Node index for run animation
    pub run_node: AnimationNodeIndex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_component_creation() {
        // Test that player can be created with speed and regen properties
        let player = Player {
            speed: 150.0,
            regen_rate: 1.0,
            pickup_radius: 50.0,
            last_movement_direction: Vec3::ZERO,
        };
        assert_eq!(player.speed, 150.0);
        assert_eq!(player.regen_rate, 1.0);
        assert_eq!(player.pickup_radius, 50.0);

        // Test default player speed
        let default_player = Player {
            speed: 200.0,
            regen_rate: 1.0,
            pickup_radius: 50.0,
            last_movement_direction: Vec3::ZERO,
        };
        assert_eq!(default_player.speed, 200.0);
        assert_eq!(default_player.regen_rate, 1.0);
        assert_eq!(default_player.pickup_radius, 50.0);
    }

    #[test]
    fn test_player_component_different_speeds() {
        let slow_player = Player {
            speed: 100.0,
            regen_rate: 1.0,
            pickup_radius: 50.0,
            last_movement_direction: Vec3::ZERO,
        };
        let fast_player = Player {
            speed: 500.0,
            regen_rate: 1.0,
            pickup_radius: 50.0,
            last_movement_direction: Vec3::ZERO,
        };

        assert_eq!(slow_player.speed, 100.0);
        assert_eq!(fast_player.speed, 500.0);
        assert!(fast_player.speed > slow_player.speed);
    }

    #[test]
    fn test_player_animation_state_default_is_idle() {
        let state = PlayerAnimationState::default();
        assert_eq!(state, PlayerAnimationState::Idle);
    }

    #[test]
    fn test_player_animation_state_equality() {
        assert_eq!(PlayerAnimationState::Idle, PlayerAnimationState::Idle);
        assert_eq!(PlayerAnimationState::Run, PlayerAnimationState::Run);
        assert_ne!(PlayerAnimationState::Idle, PlayerAnimationState::Run);
    }

    #[test]
    fn test_player_model_marker_component() {
        // PlayerModel is a unit struct marker component
        let _marker = PlayerModel;
        // Just verify it can be instantiated
    }
}