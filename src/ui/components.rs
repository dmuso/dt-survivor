use bevy::prelude::*;
use crate::weapon::components::WeaponType;

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

#[derive(Component)]
pub struct WeaponSlot {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct WeaponIcon {
    pub weapon_type: WeaponType,
}

#[derive(Component)]
pub struct WeaponTimer;

#[derive(Component)]
pub struct WeaponTimerFill;

#[derive(Component)]
pub struct WeaponTimerType {
    pub weapon_type: WeaponType,
}

#[derive(Component)]
pub struct WeaponLevelDisplay {
    pub weapon_type: WeaponType,
}

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
    fn weapon_icon_uses_weapon_type_enum() {
        let icon = WeaponIcon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
        };
        assert_eq!(icon.weapon_type.id(), "pistol");
    }

    #[test]
    fn weapon_level_display_uses_weapon_type_enum() {
        let display = WeaponLevelDisplay {
            weapon_type: WeaponType::Laser,
        };
        assert_eq!(display.weapon_type.id(), "laser");
    }

    #[test]
    fn weapon_icon_compares_by_id_not_variant_data() {
        let icon1 = WeaponIcon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
        };
        let icon2 = WeaponIcon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 10, // Different values
                spread_angle: 30.0,
            },
        };
        // WeaponType equality is based on id(), so these should be equal
        assert_eq!(icon1.weapon_type, icon2.weapon_type);
    }

    #[test]
    fn debug_fps_display_is_a_component() {
        // Verify DebugFpsDisplay can be used as a component marker
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
}