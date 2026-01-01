use bevy::prelude::*;

/// Element types for the spell system.
/// Each element has a unique color for visual effects and a display name.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Element {
    #[default]
    Fire,
    Frost,
    Poison,
    Lightning,
    Light,
    Dark,
    Chaos,
    Psychic,
}

impl Element {
    /// Returns the visual color associated with this element.
    pub fn color(&self) -> Color {
        match self {
            Element::Fire => Color::srgb_u8(255, 128, 0),      // Orange
            Element::Frost => Color::srgb_u8(135, 206, 235),   // Ice Blue
            Element::Poison => Color::srgb_u8(0, 255, 0),      // Green
            Element::Lightning => Color::srgb_u8(255, 255, 0), // Yellow
            Element::Light => Color::srgb_u8(255, 255, 255),   // White
            Element::Dark => Color::srgb_u8(128, 0, 128),      // Purple
            Element::Chaos => Color::srgb_u8(255, 0, 255),     // Magenta
            Element::Psychic => Color::srgb_u8(255, 182, 193), // Pink
        }
    }

    /// Returns the display name for this element.
    pub fn name(&self) -> &'static str {
        match self {
            Element::Fire => "Fire",
            Element::Frost => "Frost",
            Element::Poison => "Poison",
            Element::Lightning => "Lightning",
            Element::Light => "Light",
            Element::Dark => "Dark",
            Element::Chaos => "Chaos",
            Element::Psychic => "Psychic",
        }
    }

    /// Returns all element variants for iteration.
    pub fn all() -> &'static [Element] {
        &[
            Element::Fire,
            Element::Frost,
            Element::Poison,
            Element::Lightning,
            Element::Light,
            Element::Dark,
            Element::Chaos,
            Element::Psychic,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod element_color_tests {
        use super::*;

        #[test]
        fn test_fire_returns_orange() {
            let color = Element::Fire.color();
            assert_eq!(color, Color::srgb_u8(255, 128, 0));
        }

        #[test]
        fn test_frost_returns_ice_blue() {
            let color = Element::Frost.color();
            assert_eq!(color, Color::srgb_u8(135, 206, 235));
        }

        #[test]
        fn test_poison_returns_green() {
            let color = Element::Poison.color();
            assert_eq!(color, Color::srgb_u8(0, 255, 0));
        }

        #[test]
        fn test_lightning_returns_yellow() {
            let color = Element::Lightning.color();
            assert_eq!(color, Color::srgb_u8(255, 255, 0));
        }

        #[test]
        fn test_light_returns_white() {
            let color = Element::Light.color();
            assert_eq!(color, Color::srgb_u8(255, 255, 255));
        }

        #[test]
        fn test_dark_returns_purple() {
            let color = Element::Dark.color();
            assert_eq!(color, Color::srgb_u8(128, 0, 128));
        }

        #[test]
        fn test_chaos_returns_magenta() {
            let color = Element::Chaos.color();
            assert_eq!(color, Color::srgb_u8(255, 0, 255));
        }

        #[test]
        fn test_psychic_returns_pink() {
            let color = Element::Psychic.color();
            assert_eq!(color, Color::srgb_u8(255, 182, 193));
        }
    }

    mod element_name_tests {
        use super::*;

        #[test]
        fn test_fire_name() {
            assert_eq!(Element::Fire.name(), "Fire");
        }

        #[test]
        fn test_frost_name() {
            assert_eq!(Element::Frost.name(), "Frost");
        }

        #[test]
        fn test_poison_name() {
            assert_eq!(Element::Poison.name(), "Poison");
        }

        #[test]
        fn test_lightning_name() {
            assert_eq!(Element::Lightning.name(), "Lightning");
        }

        #[test]
        fn test_light_name() {
            assert_eq!(Element::Light.name(), "Light");
        }

        #[test]
        fn test_dark_name() {
            assert_eq!(Element::Dark.name(), "Dark");
        }

        #[test]
        fn test_chaos_name() {
            assert_eq!(Element::Chaos.name(), "Chaos");
        }

        #[test]
        fn test_psychic_name() {
            assert_eq!(Element::Psychic.name(), "Psychic");
        }
    }

    mod element_trait_tests {
        use super::*;

        #[test]
        fn test_element_all_returns_8_variants() {
            assert_eq!(Element::all().len(), 8);
        }

        #[test]
        fn test_element_is_clone() {
            let fire = Element::Fire;
            let fire_clone = fire;
            assert_eq!(fire, fire_clone);
        }

        #[test]
        fn test_element_is_copy() {
            let lightning = Element::Lightning;
            let lightning_copy = lightning;
            assert_eq!(lightning, lightning_copy);
        }

        #[test]
        fn test_element_is_partial_eq() {
            assert_eq!(Element::Fire, Element::Fire);
            assert_ne!(Element::Fire, Element::Frost);
        }

        #[test]
        fn test_element_is_debug() {
            let debug_str = format!("{:?}", Element::Fire);
            assert_eq!(debug_str, "Fire");
        }

        #[test]
        fn test_element_default_is_fire() {
            assert_eq!(Element::default(), Element::Fire);
        }

        #[test]
        fn test_all_variants_are_distinct() {
            let all = Element::all();
            for (i, elem1) in all.iter().enumerate() {
                for (j, elem2) in all.iter().enumerate() {
                    if i != j {
                        assert_ne!(elem1, elem2, "Elements at {} and {} should be distinct", i, j);
                    }
                }
            }
        }

        #[test]
        fn test_element_is_hashable() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(Element::Fire);
            set.insert(Element::Frost);
            assert!(set.contains(&Element::Fire));
            assert!(set.contains(&Element::Frost));
            assert!(!set.contains(&Element::Poison));
        }
    }
}
