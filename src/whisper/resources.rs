use bevy::animation::graph::AnimationNodeIndex;
use bevy::prelude::*;

/// Resource holding handles to the whisper's animation clips and scene
#[derive(Resource)]
pub struct WhisperAnimations {
    /// Handle to the whisper GLTF scene
    pub scene: Handle<Scene>,
    /// Animation graph for the whisper
    pub graph: Handle<AnimationGraph>,
    /// Node indices for all animations (one per mesh)
    pub animation_nodes: Vec<AnimationNodeIndex>,
}

/// Resource tracking the spell origin position.
/// When Whisper is not collected, spells are disabled.
/// When Whisper is collected, this contains Whisper's 3D position.
#[derive(Resource, Default)]
pub struct SpellOrigin {
    /// None = spells disabled, Some(pos) = cast from this 3D position
    pub position: Option<Vec3>,
}

impl SpellOrigin {
    pub fn is_active(&self) -> bool {
        self.position.is_some()
    }
}

/// Type alias for backward compatibility during migration
pub type WeaponOrigin = SpellOrigin;

/// Resource tracking whether Whisper has been collected this game
#[derive(Resource, Default)]
pub struct WhisperState {
    pub collected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_origin_default() {
        let origin = SpellOrigin::default();
        assert!(origin.position.is_none());
        assert!(!origin.is_active());
    }

    #[test]
    fn test_spell_origin_active() {
        let origin = SpellOrigin {
            position: Some(Vec3::new(10.0, 3.0, 20.0)),
        };
        assert!(origin.is_active());
        assert_eq!(origin.position.unwrap(), Vec3::new(10.0, 3.0, 20.0));
    }

    #[test]
    fn test_weapon_origin_alias_works() {
        // WeaponOrigin is now an alias for SpellOrigin
        let origin: WeaponOrigin = SpellOrigin::default();
        assert!(origin.position.is_none());
    }

    #[test]
    fn test_whisper_state_default() {
        let state = WhisperState::default();
        assert!(!state.collected);
    }

    #[test]
    fn test_whisper_animations_type_check() {
        // WhisperAnimations holds scene, graph, and animation node handles
        // This test verifies the struct exists and type signatures are correct
        // Full integration testing requires asset loading plugins
        use bevy::animation::graph::AnimationNodeIndex;
        use bevy::asset::Handle;

        // Verify the type is correctly defined
        fn _type_check(
            scene: Handle<Scene>,
            graph: Handle<AnimationGraph>,
            animation_nodes: Vec<AnimationNodeIndex>,
        ) -> WhisperAnimations {
            WhisperAnimations {
                scene,
                graph,
                animation_nodes,
            }
        }

        // Just verify the type compiles - actual handle creation is tested in integration
    }
}
