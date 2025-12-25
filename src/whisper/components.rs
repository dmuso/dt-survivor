use bevy::prelude::*;

/// Marker component for Whisper when it's a dropped collectible (before pickup)
#[derive(Component)]
pub struct WhisperDrop {
    pub pickup_radius: f32,
}

impl Default for WhisperDrop {
    fn default() -> Self {
        Self { pickup_radius: 25.0 }
    }
}

/// Marker component for Whisper when it's the active companion (after pickup)
#[derive(Component)]
pub struct WhisperCompanion {
    /// Offset above player where Whisper floats
    pub follow_offset: Vec3,
    /// Bobbing animation phase
    pub bob_phase: f32,
    /// Bobbing amplitude in pixels
    pub bob_amplitude: f32,
}

impl Default for WhisperCompanion {
    fn default() -> Self {
        Self {
            follow_offset: Vec3::new(0.0, 30.0, 0.5),
            bob_phase: 0.0,
            bob_amplitude: 5.0,
        }
    }
}

/// Timer for spawning lightning arc bursts
#[derive(Component)]
pub struct ArcBurstTimer(pub Timer);

impl Default for ArcBurstTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.12, TimerMode::Repeating))
    }
}

/// Component for individual lightning arc sprites
#[derive(Component)]
pub struct WhisperArc {
    pub lifetime: Timer,
}

impl WhisperArc {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            lifetime: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }
}

/// Marker for the core glow sprite (inner bright part)
#[derive(Component)]
pub struct WhisperCoreGlow;

/// Marker for the outer glow sprite (larger, more transparent)
#[derive(Component)]
pub struct WhisperOuterGlow;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_drop_default() {
        let drop = WhisperDrop::default();
        assert_eq!(drop.pickup_radius, 25.0);
    }

    #[test]
    fn test_whisper_companion_default() {
        let companion = WhisperCompanion::default();
        assert_eq!(companion.follow_offset, Vec3::new(0.0, 30.0, 0.5));
        assert_eq!(companion.bob_phase, 0.0);
        assert_eq!(companion.bob_amplitude, 5.0);
    }

    #[test]
    fn test_arc_burst_timer_default() {
        let timer = ArcBurstTimer::default();
        assert!(!timer.0.is_finished());
        assert_eq!(timer.0.duration().as_secs_f32(), 0.12);
    }

    #[test]
    fn test_whisper_arc_creation() {
        let arc = WhisperArc::new(0.06);
        assert!(!arc.lifetime.is_finished());
        assert_eq!(arc.lifetime.duration().as_secs_f32(), 0.06);
    }
}
