use bevy::prelude::*;
use bevy_hanabi::prelude::*;

/// Resource holding the Whisper spark particle effect asset
#[derive(Resource)]
pub struct WhisperSparkEffect(pub Handle<EffectAsset>);

/// Resource tracking the weapon origin position.
/// When Whisper is not collected, weapons are disabled.
/// When Whisper is collected, this contains Whisper's position.
#[derive(Resource, Default)]
pub struct WeaponOrigin {
    /// None = weapons disabled, Some(pos) = fire from this position
    pub position: Option<Vec2>,
}

impl WeaponOrigin {
    pub fn is_active(&self) -> bool {
        self.position.is_some()
    }
}

/// Resource tracking whether Whisper has been collected this game
#[derive(Resource, Default)]
pub struct WhisperState {
    pub collected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weapon_origin_default() {
        let origin = WeaponOrigin::default();
        assert!(origin.position.is_none());
        assert!(!origin.is_active());
    }

    #[test]
    fn test_weapon_origin_active() {
        let origin = WeaponOrigin {
            position: Some(Vec2::new(10.0, 20.0)),
        };
        assert!(origin.is_active());
        assert_eq!(origin.position.unwrap(), Vec2::new(10.0, 20.0));
    }

    #[test]
    fn test_whisper_state_default() {
        let state = WhisperState::default();
        assert!(!state.collected);
    }

    #[test]
    fn test_whisper_spark_effect_holds_handle() {
        // WhisperSparkEffect should be a newtype wrapper around Handle<EffectAsset>
        // This is a simple test to verify the struct exists and compiles
        // Full integration testing requires the HanabiPlugin which has many dependencies
        use bevy::asset::Handle;

        // Verify the type is correctly defined - it wraps Handle<EffectAsset>
        // We can't easily create a valid handle in tests without full plugin setup,
        // but we can verify the struct exists and the type signature is correct
        fn _type_check(handle: Handle<EffectAsset>) -> WhisperSparkEffect {
            WhisperSparkEffect(handle)
        }

        // Just verify the type compiles - actual handle creation is tested in integration
    }
}
