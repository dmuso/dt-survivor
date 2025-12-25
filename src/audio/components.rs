use bevy::prelude::*;

/// Marker component for background music entity
#[derive(Component)]
pub struct BackgroundMusic;

/// Marker component for weapon sound effects
#[derive(Component)]
pub struct WeaponSound;

/// Marker component for enemy death sound effects
#[derive(Component)]
pub struct EnemyDeathSound;

/// Marker component for loot pickup sound effects
#[derive(Component)]
pub struct LootPickupSound;

/// Types of cleanup that can be performed by the CleanupTimer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupType {
    /// Audio entity cleanup (general audio sounds)
    Audio,
    /// Experience pickup sound cleanup
    Experience,
    /// Loot pickup sound cleanup
    Loot,
}

/// Generic timer component for entity cleanup, replacing separate audio cleanup timers
#[derive(Component)]
pub struct CleanupTimer {
    pub timer: Timer,
    pub cleanup_type: CleanupType,
}

impl CleanupTimer {
    pub fn new(duration: std::time::Duration, cleanup_type: CleanupType) -> Self {
        Self {
            timer: Timer::new(duration, bevy::time::TimerMode::Once),
            cleanup_type,
        }
    }

    pub fn from_secs(secs: f32, cleanup_type: CleanupType) -> Self {
        Self::new(std::time::Duration::from_secs_f32(secs), cleanup_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cleanup_type_equality() {
        assert_eq!(CleanupType::Audio, CleanupType::Audio);
        assert_ne!(CleanupType::Audio, CleanupType::Experience);
        assert_ne!(CleanupType::Experience, CleanupType::Loot);
    }

    #[test]
    fn test_cleanup_timer_new() {
        let timer = CleanupTimer::new(Duration::from_secs(2), CleanupType::Audio);
        assert_eq!(timer.cleanup_type, CleanupType::Audio);
        assert!(!timer.timer.is_finished());
    }

    #[test]
    fn test_cleanup_timer_from_secs() {
        let timer = CleanupTimer::from_secs(1.5, CleanupType::Loot);
        assert_eq!(timer.cleanup_type, CleanupType::Loot);
        assert!(!timer.timer.is_finished());
    }
}