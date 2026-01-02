//! Spawning logic for spell slot visuals.
//!
//! This module provides unified spawn functions and visual constants for spell slots,
//! used in both the active spell bar and inventory bag.

// Re-export empty slot colors from the shared module
pub use crate::ui::components::empty_slot;

/// Standard size for spell icon slots in pixels.
/// Used consistently in the active spell bar and inventory bag.
pub const SLOT_SIZE: f32 = 50.0;

/// Alpha/opacity for spell element background tint.
/// Applied to the element color when rendering slot backgrounds.
pub const BACKGROUND_ALPHA: f32 = 0.4;

/// Vertical offset for level indicator from the top of the slot.
/// Negative value positions the level box above the slot edge.
pub const LEVEL_TOP_OFFSET: f32 = -6.0;

/// Font size for the level number text.
pub const LEVEL_FONT_SIZE: f32 = 9.0;

/// Horizontal padding for the level indicator box (left/right).
pub const LEVEL_PADDING_X: f32 = 3.0;

/// Vertical padding for the level indicator box (top/bottom).
pub const LEVEL_PADDING_Y: f32 = 1.0;

/// Border radius for slot containers.
pub const BORDER_RADIUS: f32 = 6.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_module_exists() {
        // Placeholder test to verify module structure
        assert!(true);
    }

    mod slot_constants_tests {
        use super::*;

        #[test]
        fn slot_size_is_50() {
            assert_eq!(SLOT_SIZE, 50.0);
        }

        #[test]
        fn background_alpha_is_reasonable() {
            assert!(BACKGROUND_ALPHA > 0.0);
            assert!(BACKGROUND_ALPHA <= 1.0);
            assert_eq!(BACKGROUND_ALPHA, 0.4);
        }

        #[test]
        fn level_top_offset_is_negative() {
            assert!(LEVEL_TOP_OFFSET < 0.0);
            assert_eq!(LEVEL_TOP_OFFSET, -6.0);
        }

        #[test]
        fn level_font_size_is_9() {
            assert_eq!(LEVEL_FONT_SIZE, 9.0);
        }

        #[test]
        fn level_padding_values() {
            assert_eq!(LEVEL_PADDING_X, 3.0);
            assert_eq!(LEVEL_PADDING_Y, 1.0);
        }

        #[test]
        fn border_radius_is_6() {
            assert_eq!(BORDER_RADIUS, 6.0);
        }
    }

    mod empty_slot_reexport_tests {
        use super::*;

        #[test]
        fn empty_slot_background_is_accessible() {
            let _color = empty_slot::SLOT_BACKGROUND;
        }

        #[test]
        fn empty_slot_border_is_accessible() {
            let _color = empty_slot::SLOT_BORDER;
        }

        #[test]
        fn empty_slot_hover_background_is_accessible() {
            let _color = empty_slot::SLOT_BACKGROUND_HOVER;
        }

        #[test]
        fn empty_slot_hover_border_is_accessible() {
            let _color = empty_slot::SLOT_BORDER_HOVER;
        }
    }
}
