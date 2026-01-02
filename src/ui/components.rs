use bevy::prelude::*;

#[derive(Component)]
pub struct MenuButton;

#[derive(Component)]
pub struct StartGameButton;

#[derive(Component)]
pub struct ExitGameButton;

#[derive(Component)]
pub struct HealthDisplay;

#[derive(Component)]
pub struct HealthBar;

#[derive(Component)]
pub struct ScreenTint;

/// Marker component for a spell slot in the spell bar UI.
#[derive(Component)]
pub struct SpellSlot {
    pub slot_index: usize,
}

/// Marker component for a spell icon in the spell bar.
/// The slot_index links this to the corresponding spell in SpellList.
#[derive(Component)]
pub struct SpellIcon {
    pub slot_index: usize,
}

/// Marker component for the cooldown timer overlay.
#[derive(Component)]
pub struct SpellCooldownTimer {
    pub slot_index: usize,
}

/// Marker component for the cooldown timer fill (the growing circle).
#[derive(Component)]
pub struct SpellCooldownTimerFill {
    pub slot_index: usize,
}

/// Marker component for spell level display text.
#[derive(Component)]
pub struct SpellLevelDisplay {
    pub slot_index: usize,
}

/// Marker component for the spell bar container
#[derive(Component)]
pub struct SpellBar;

// Debug HUD components
#[derive(Component)]
pub struct DebugHud;

#[derive(Component)]
pub struct DebugPlayerPosition;

#[derive(Component)]
pub struct DebugCameraPosition;

#[derive(Component)]
pub struct DebugEnemyCount;

#[derive(Component)]
pub struct DebugFpsDisplay;

#[derive(Component)]
pub struct GameLevelDisplay;

#[derive(Component)]
pub struct KillProgressDisplay;

#[derive(Component)]
pub struct XpProgressBar;

#[derive(Component)]
pub struct XpProgressBarFill;

/// Root marker for level complete UI - all level complete screen elements are children of this
#[derive(Component)]
pub struct LevelCompleteScreen;

/// The black overlay that animates opacity
#[derive(Component)]
pub struct LevelCompleteOverlay {
    pub target_opacity: f32,
    pub current_opacity: f32,
    pub animation_speed: f32,
}

impl Default for LevelCompleteOverlay {
    fn default() -> Self {
        Self {
            target_opacity: 0.85,
            current_opacity: 0.0,
            animation_speed: 2.0, // Fully opaque in ~0.5 seconds
        }
    }
}

/// Marker for the continue button
#[derive(Component)]
pub struct ContinueButton;

/// Floating damage number that animates upward and fades out.
/// Spawned when enemies take damage, colored by element type.
/// Uses world position tracking for screen-space rendering.
#[derive(Component)]
pub struct FloatingDamageNumber {
    /// World position to track (moves upward over time)
    pub world_position: Vec3,
    /// Upward movement velocity in world units
    pub velocity: f32,
    /// Total animation duration
    pub lifetime: Timer,
    /// When to start fading (0.0-1.0 of lifetime progress)
    pub fade_start: f32,
}

impl FloatingDamageNumber {
    /// Creates a new floating damage number at the given world position
    pub fn new(world_position: Vec3) -> Self {
        Self {
            world_position,
            velocity: 2.0,
            lifetime: Timer::from_seconds(0.8, TimerMode::Once),
            fade_start: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_level_display_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<GameLevelDisplay>();
    }

    #[test]
    fn kill_progress_display_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<KillProgressDisplay>();
    }

    #[test]
    fn xp_progress_bar_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<XpProgressBar>();
    }

    #[test]
    fn xp_progress_bar_fill_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<XpProgressBarFill>();
    }

    #[test]
    fn spell_slot_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellSlot>();
    }

    #[test]
    fn spell_slot_stores_index() {
        let slot = SpellSlot { slot_index: 2 };
        assert_eq!(slot.slot_index, 2);
    }

    #[test]
    fn spell_icon_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellIcon>();
    }

    #[test]
    fn spell_icon_stores_slot_index() {
        let icon = SpellIcon { slot_index: 3 };
        assert_eq!(icon.slot_index, 3);
    }

    #[test]
    fn spell_cooldown_timer_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellCooldownTimer>();
    }

    #[test]
    fn spell_cooldown_timer_fill_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellCooldownTimerFill>();
    }

    #[test]
    fn spell_level_display_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellLevelDisplay>();
    }

    #[test]
    fn spell_bar_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<SpellBar>();
    }

    #[test]
    fn debug_fps_display_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<DebugFpsDisplay>();
    }

    #[test]
    fn level_complete_screen_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<LevelCompleteScreen>();
    }

    #[test]
    fn level_complete_overlay_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<LevelCompleteOverlay>();
    }

    #[test]
    fn continue_button_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<ContinueButton>();
    }

    #[test]
    fn level_complete_overlay_default_values() {
        let overlay = LevelCompleteOverlay::default();
        assert_eq!(overlay.target_opacity, 0.85);
        assert_eq!(overlay.current_opacity, 0.0);
        assert_eq!(overlay.animation_speed, 2.0);
    }

    #[test]
    fn level_complete_overlay_animates_correctly() {
        let mut overlay = LevelCompleteOverlay::default();
        assert_eq!(overlay.current_opacity, 0.0);

        // Simulate animation
        let delta = 0.25; // quarter second
        overlay.current_opacity += delta * overlay.animation_speed;
        overlay.current_opacity = overlay.current_opacity.min(overlay.target_opacity);

        assert!(overlay.current_opacity > 0.0);
        assert!(overlay.current_opacity <= overlay.target_opacity);
    }

    #[test]
    fn level_complete_overlay_caps_at_target() {
        let mut overlay = LevelCompleteOverlay::default();
        overlay.current_opacity = 0.8;

        // Large delta should cap at target
        let delta = 1.0;
        overlay.current_opacity += delta * overlay.animation_speed;
        overlay.current_opacity = overlay.current_opacity.min(overlay.target_opacity);

        assert_eq!(overlay.current_opacity, overlay.target_opacity);
    }

    #[test]
    fn floating_damage_number_is_a_component() {
        fn assert_component<T: Component>() {}
        assert_component::<FloatingDamageNumber>();
    }

    #[test]
    fn floating_damage_number_new_sets_position() {
        let pos = Vec3::new(10.0, 5.0, 3.0);
        let damage_num = FloatingDamageNumber::new(pos);
        assert_eq!(damage_num.world_position, pos);
        assert_eq!(damage_num.velocity, 2.0);
        assert_eq!(damage_num.fade_start, 0.5);
        assert!(!damage_num.lifetime.is_finished());
    }
}