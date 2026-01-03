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

    /// Returns the path to the default spell texture for this element.
    ///
    /// Used when a spell doesn't have a custom texture.
    pub fn default_texture_path(&self) -> &'static str {
        match self {
            Element::Fire => "textures/spell-fire-default.png",
            Element::Frost => "textures/spell-frost-default.png",
            Element::Poison => "textures/spell-poison-default.png",
            Element::Lightning => "textures/spell-lightning-default.png",
            Element::Light => "textures/spell-light-default.png",
            Element::Dark => "textures/spell-dark-default.png",
            Element::Chaos => "textures/spell-chaos-default.png",
            Element::Psychic => "textures/spell-psychic-default.png",
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

    mod element_default_texture_path_tests {
        use super::*;

        #[test]
        fn fire_returns_fire_default_texture() {
            assert_eq!(Element::Fire.default_texture_path(), "textures/spell-fire-default.png");
        }

        #[test]
        fn frost_returns_frost_default_texture() {
            assert_eq!(Element::Frost.default_texture_path(), "textures/spell-frost-default.png");
        }

        #[test]
        fn poison_returns_poison_default_texture() {
            assert_eq!(Element::Poison.default_texture_path(), "textures/spell-poison-default.png");
        }

        #[test]
        fn lightning_returns_lightning_default_texture() {
            assert_eq!(Element::Lightning.default_texture_path(), "textures/spell-lightning-default.png");
        }

        #[test]
        fn light_returns_light_default_texture() {
            assert_eq!(Element::Light.default_texture_path(), "textures/spell-light-default.png");
        }

        #[test]
        fn dark_returns_dark_default_texture() {
            assert_eq!(Element::Dark.default_texture_path(), "textures/spell-dark-default.png");
        }

        #[test]
        fn chaos_returns_chaos_default_texture() {
            assert_eq!(Element::Chaos.default_texture_path(), "textures/spell-chaos-default.png");
        }

        #[test]
        fn psychic_returns_psychic_default_texture() {
            assert_eq!(Element::Psychic.default_texture_path(), "textures/spell-psychic-default.png");
        }

        #[test]
        fn all_elements_have_default_texture_paths() {
            for element in Element::all() {
                let path = element.default_texture_path();
                assert!(!path.is_empty(), "{:?} should have a default texture path", element);
                assert!(path.ends_with(".png"), "{:?} path should end with .png", element);
                assert!(path.starts_with("textures/"), "{:?} path should start with textures/", element);
            }
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
