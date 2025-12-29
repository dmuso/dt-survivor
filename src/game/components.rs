use bevy::prelude::*;

#[derive(Component)]
pub struct Rock;

/// Marker component for the ground plane in 3D space
#[derive(Component)]
pub struct GroundPlane;

/// Rarity tiers for game entities, matching the standard RPG color scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Rarity {
    #[default]
    Common,    // Level 1 - Grey
    Uncommon,  // Level 2 - Green
    Rare,      // Level 3 - Blue
    Epic,      // Level 4 - Purple
    Legendary, // Level 5 - Gold
}

impl Rarity {
    pub fn from_level(level: u8) -> Self {
        match level {
            1 => Rarity::Common,
            2 => Rarity::Uncommon,
            3 => Rarity::Rare,
            4 => Rarity::Epic,
            5.. => Rarity::Legendary,
            _ => Rarity::Common,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Rarity::Common => Color::srgb(0.6, 0.6, 0.6),      // Grey
            Rarity::Uncommon => Color::srgb(0.0, 0.8, 0.2),   // Green
            Rarity::Rare => Color::srgb(0.2, 0.4, 1.0),       // Blue
            Rarity::Epic => Color::srgb(0.6, 0.2, 0.8),       // Purple
            Rarity::Legendary => Color::srgb(1.0, 0.84, 0.0), // Gold
        }
    }
}

/// Level component for entities that have progression levels (1-5)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Level(pub u8);

impl Level {
    /// Creates a new Level, clamping the value between 1 and 5
    pub fn new(level: u8) -> Self {
        Self(level.clamp(1, 5))
    }

    /// Returns the level value
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Returns the rarity tier for this level
    pub fn rarity(&self) -> Rarity {
        Rarity::from_level(self.0)
    }

    /// Returns the color associated with this level's rarity
    pub fn color(&self) -> Color {
        self.rarity().color()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rock_component_can_be_created() {
        let _rock = Rock;
    }

    #[test]
    fn test_ground_plane_component_can_be_created() {
        let _ground = GroundPlane;
    }

    #[test]
    fn level_clamps_to_valid_range() {
        assert_eq!(Level::new(0).value(), 1);
        assert_eq!(Level::new(1).value(), 1);
        assert_eq!(Level::new(3).value(), 3);
        assert_eq!(Level::new(5).value(), 5);
        assert_eq!(Level::new(10).value(), 5);
        assert_eq!(Level::new(255).value(), 5);
    }

    #[test]
    fn rarity_from_level_maps_correctly() {
        assert_eq!(Rarity::from_level(1), Rarity::Common);
        assert_eq!(Rarity::from_level(2), Rarity::Uncommon);
        assert_eq!(Rarity::from_level(3), Rarity::Rare);
        assert_eq!(Rarity::from_level(4), Rarity::Epic);
        assert_eq!(Rarity::from_level(5), Rarity::Legendary);
        assert_eq!(Rarity::from_level(6), Rarity::Legendary);
        assert_eq!(Rarity::from_level(100), Rarity::Legendary);
    }

    #[test]
    fn rarity_from_level_zero_returns_common() {
        assert_eq!(Rarity::from_level(0), Rarity::Common);
    }

    #[test]
    fn level_rarity_returns_correct_rarity() {
        assert_eq!(Level::new(1).rarity(), Rarity::Common);
        assert_eq!(Level::new(2).rarity(), Rarity::Uncommon);
        assert_eq!(Level::new(3).rarity(), Rarity::Rare);
        assert_eq!(Level::new(4).rarity(), Rarity::Epic);
        assert_eq!(Level::new(5).rarity(), Rarity::Legendary);
    }

    #[test]
    fn rarity_color_returns_correct_rgb_values() {
        // Common - Grey
        let common_color = Rarity::Common.color();
        assert_eq!(common_color, Color::srgb(0.6, 0.6, 0.6));

        // Uncommon - Green
        let uncommon_color = Rarity::Uncommon.color();
        assert_eq!(uncommon_color, Color::srgb(0.0, 0.8, 0.2));

        // Rare - Blue
        let rare_color = Rarity::Rare.color();
        assert_eq!(rare_color, Color::srgb(0.2, 0.4, 1.0));

        // Epic - Purple
        let epic_color = Rarity::Epic.color();
        assert_eq!(epic_color, Color::srgb(0.6, 0.2, 0.8));

        // Legendary - Gold
        let legendary_color = Rarity::Legendary.color();
        assert_eq!(legendary_color, Color::srgb(1.0, 0.84, 0.0));
    }

    #[test]
    fn level_color_returns_correct_color_for_level() {
        assert_eq!(Level::new(1).color(), Color::srgb(0.6, 0.6, 0.6));
        assert_eq!(Level::new(2).color(), Color::srgb(0.0, 0.8, 0.2));
        assert_eq!(Level::new(3).color(), Color::srgb(0.2, 0.4, 1.0));
        assert_eq!(Level::new(4).color(), Color::srgb(0.6, 0.2, 0.8));
        assert_eq!(Level::new(5).color(), Color::srgb(1.0, 0.84, 0.0));
    }

    #[test]
    fn rarity_default_is_common() {
        let rarity: Rarity = Default::default();
        assert_eq!(rarity, Rarity::Common);
    }

    #[test]
    fn level_default_is_zero() {
        let level: Level = Default::default();
        assert_eq!(level.0, 0);
    }
}