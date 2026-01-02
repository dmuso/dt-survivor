use bevy::prelude::*;

#[derive(Component)]
pub struct ExperienceOrb {
    pub value: u32,
    pub velocity: Vec2,
}

/// Configuration for player XP requirements
#[derive(Debug, Clone)]
pub struct ExperienceConfig {
    /// Base XP needed for level 2
    pub base_xp: u32,
    /// Multiplier per level (xp_needed = base_xp * multiplier^(level-1))
    pub xp_multiplier: f32,
}

impl Default for ExperienceConfig {
    fn default() -> Self {
        Self {
            base_xp: 500, // 5x increase from original 100
            xp_multiplier: 1.5,
        }
    }
}

/// Tracks player experience and level
#[derive(Component, Debug)]
pub struct PlayerExperience {
    /// Current XP in this level
    pub current: u32,
    /// Current player level (starts at 1)
    pub level: u32,
    /// Total XP ever gained (for stats)
    pub total_xp: u32,
    /// Configuration
    pub config: ExperienceConfig,
}

impl Default for PlayerExperience {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerExperience {
    pub fn new() -> Self {
        Self {
            current: 0,
            level: 1,
            total_xp: 0,
            config: ExperienceConfig::default(),
        }
    }

    /// XP needed to advance from current level
    pub fn xp_to_next_level(&self) -> u32 {
        let multiplier = self.config.xp_multiplier.powi(self.level as i32 - 1);
        (self.config.base_xp as f32 * multiplier).ceil() as u32
    }

    /// Add XP and return number of levels gained
    pub fn add_xp(&mut self, amount: u32) -> u32 {
        self.current += amount;
        self.total_xp += amount;

        let mut levels_gained = 0;
        while self.current >= self.xp_to_next_level() {
            self.current -= self.xp_to_next_level();
            self.level += 1;
            levels_gained += 1;
        }
        levels_gained
    }

    /// Progress percentage toward next level (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        self.current as f32 / self.xp_to_next_level() as f32
    }
}

#[derive(Component)]
pub struct PlayerLevelDisplay;

/// Marker component for experience pickup sound effects
#[derive(Component)]
pub struct ExperiencePickupSound;

/// Fired when player gains a level
#[derive(Message, Debug)]
pub struct PlayerLevelUpEvent {
    pub new_level: u32,
    pub levels_gained: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_experience_starts_at_level_one() {
        let exp = PlayerExperience::new();
        assert_eq!(exp.level, 1);
        assert_eq!(exp.current, 0);
        assert_eq!(exp.total_xp, 0);
    }

    #[test]
    fn player_experience_default_matches_new() {
        let exp_new = PlayerExperience::new();
        let exp_default = PlayerExperience::default();
        assert_eq!(exp_new.level, exp_default.level);
        assert_eq!(exp_new.current, exp_default.current);
        assert_eq!(exp_new.total_xp, exp_default.total_xp);
    }

    #[test]
    fn xp_to_next_level_at_level_one() {
        let exp = PlayerExperience::new();
        // base_xp=500, multiplier=1.5, level=1
        // 500 * 1.5^0 = 500
        assert_eq!(exp.xp_to_next_level(), 500);
    }

    #[test]
    fn xp_to_next_level_increases_each_level() {
        let mut exp = PlayerExperience::new();
        let xp_1 = exp.xp_to_next_level();
        exp.level = 2;
        let xp_2 = exp.xp_to_next_level();
        exp.level = 3;
        let xp_3 = exp.xp_to_next_level();
        assert!(xp_2 > xp_1, "Level 2 should need more XP than level 1");
        assert!(xp_3 > xp_2, "Level 3 should need more XP than level 2");
    }

    #[test]
    fn xp_to_next_level_follows_multiplier_formula() {
        let mut exp = PlayerExperience::new();
        // Level 1: 500 * 1.5^0 = 500
        assert_eq!(exp.xp_to_next_level(), 500);

        exp.level = 2;
        // Level 2: 500 * 1.5^1 = 750
        assert_eq!(exp.xp_to_next_level(), 750);

        exp.level = 3;
        // Level 3: 500 * 1.5^2 = 1125
        assert_eq!(exp.xp_to_next_level(), 1125);

        exp.level = 4;
        // Level 4: 500 * 1.5^3 = 1687.5 -> ceil = 1688
        assert_eq!(exp.xp_to_next_level(), 1688);
    }

    #[test]
    fn add_xp_accumulates_without_level_up() {
        let mut exp = PlayerExperience::new();
        let levels = exp.add_xp(250);
        assert_eq!(levels, 0);
        assert_eq!(exp.current, 250);
        assert_eq!(exp.total_xp, 250);
        assert_eq!(exp.level, 1);
    }

    #[test]
    fn add_xp_levels_up_and_keeps_remainder() {
        let mut exp = PlayerExperience::new();
        let threshold = exp.xp_to_next_level(); // 500
        let levels = exp.add_xp(threshold + 50);
        assert_eq!(levels, 1);
        assert_eq!(exp.level, 2);
        assert_eq!(exp.current, 50);
        assert_eq!(exp.total_xp, 550);
    }

    #[test]
    fn add_xp_handles_exact_threshold() {
        let mut exp = PlayerExperience::new();
        let threshold = exp.xp_to_next_level(); // 500
        let levels = exp.add_xp(threshold);
        assert_eq!(levels, 1);
        assert_eq!(exp.level, 2);
        assert_eq!(exp.current, 0);
        assert_eq!(exp.total_xp, 500);
    }

    #[test]
    fn add_xp_handles_multiple_level_ups() {
        let mut exp = PlayerExperience::new();
        // Level 1 -> 2: 500 XP
        // Level 2 -> 3: 750 XP
        // Total for 2 levels: 1250 XP
        let levels = exp.add_xp(1300);
        assert_eq!(levels, 2);
        assert_eq!(exp.level, 3);
        assert_eq!(exp.current, 50);
        assert_eq!(exp.total_xp, 1300);
    }

    #[test]
    fn add_xp_handles_many_level_ups() {
        let mut exp = PlayerExperience::new();
        // Level 1->2: 500, Level 2->3: 750, Level 3->4: 1125
        // Total for 3 levels: 2375 XP
        let levels = exp.add_xp(5000);
        assert!(levels > 2, "5000 XP should grant more than 2 levels");
        assert!(exp.level > 3, "Should be above level 3");
        assert_eq!(exp.total_xp, 5000);
    }

    #[test]
    fn progress_returns_correct_percentage() {
        let mut exp = PlayerExperience::new();
        exp.current = 250;
        // With xp_to_next_level=500, progress should be 0.5
        assert!((exp.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn progress_at_zero_is_zero() {
        let exp = PlayerExperience::new();
        assert!((exp.progress() - 0.0).abs() < 0.01);
    }

    #[test]
    fn progress_near_level_up_is_near_one() {
        let mut exp = PlayerExperience::new();
        exp.current = 495;
        // With xp_to_next_level=500, progress should be 0.99
        assert!((exp.progress() - 0.99).abs() < 0.01);
    }

    #[test]
    fn experience_config_can_be_customized() {
        let config = ExperienceConfig {
            base_xp: 50,
            xp_multiplier: 2.0,
        };
        let mut exp = PlayerExperience {
            current: 0,
            level: 1,
            total_xp: 0,
            config,
        };
        // Level 1: 50 * 2.0^0 = 50
        assert_eq!(exp.xp_to_next_level(), 50);

        exp.level = 2;
        // Level 2: 50 * 2.0^1 = 100
        assert_eq!(exp.xp_to_next_level(), 100);
    }

    #[test]
    fn player_level_up_event_stores_correct_values() {
        let event = PlayerLevelUpEvent {
            new_level: 5,
            levels_gained: 2,
        };
        assert_eq!(event.new_level, 5);
        assert_eq!(event.levels_gained, 2);
    }
}