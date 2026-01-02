use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
    pub strength: f32,
}

/// Configuration for enemy stat scaling based on level
#[derive(Debug, Clone)]
pub struct EnemyScaling {
    pub base_health: f32,
    pub health_per_enemy_level: f32,
    pub health_per_game_level_percent: f32,
    pub base_damage: f32,
    pub damage_per_level: f32,
}

impl Default for EnemyScaling {
    fn default() -> Self {
        Self {
            base_health: 25.0,
            health_per_enemy_level: 15.0, // Enemy level 5 base = 25 + (4 * 15) = 85 HP
            health_per_game_level_percent: 0.10, // +10% HP per game level
            base_damage: 10.0,
            damage_per_level: 5.0, // Level 5 = 10 + (4 * 5) = 30 damage
        }
    }
}

impl EnemyScaling {
    /// Calculate health for a given enemy level (1-5) and game level
    /// Formula: (base + enemy_level_bonus) * (1 + game_level_bonus)
    pub fn health_for_level(&self, enemy_level: u8, game_level: u32) -> f32 {
        let base_hp =
            self.base_health + (enemy_level.saturating_sub(1) as f32 * self.health_per_enemy_level);
        let game_level_multiplier =
            1.0 + (game_level.saturating_sub(1) as f32 * self.health_per_game_level_percent);
        base_hp * game_level_multiplier
    }

    /// Calculate damage for a given enemy level (1-5)
    pub fn damage_for_level(&self, level: u8) -> f32 {
        self.base_damage + (level.saturating_sub(1) as f32 * self.damage_per_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_scaling_default_values() {
        let scaling = EnemyScaling::default();
        assert_eq!(scaling.base_health, 25.0);
        assert_eq!(scaling.health_per_enemy_level, 15.0);
        assert_eq!(scaling.health_per_game_level_percent, 0.10);
        assert_eq!(scaling.base_damage, 10.0);
        assert_eq!(scaling.damage_per_level, 5.0);
    }

    #[test]
    fn enemy_scaling_calculates_correct_health_at_game_level_1() {
        let scaling = EnemyScaling::default();
        // At game level 1, no multiplier (1.0x)
        assert_eq!(scaling.health_for_level(1, 1), 25.0);
        assert_eq!(scaling.health_for_level(2, 1), 40.0);
        assert_eq!(scaling.health_for_level(3, 1), 55.0);
        assert_eq!(scaling.health_for_level(4, 1), 70.0);
        assert_eq!(scaling.health_for_level(5, 1), 85.0);
    }

    #[test]
    fn enemy_scaling_health_increases_with_game_level() {
        let scaling = EnemyScaling::default();
        // Game level 1: 25 HP (enemy level 1)
        // Game level 2: 25 * 1.1 = 27.5 HP
        // Game level 11: 25 * 2.0 = 50 HP
        assert_eq!(scaling.health_for_level(1, 1), 25.0);
        assert_eq!(scaling.health_for_level(1, 2), 27.5);
        assert_eq!(scaling.health_for_level(1, 11), 50.0);
    }

    #[test]
    fn enemy_scaling_health_combines_enemy_and_game_level() {
        let scaling = EnemyScaling::default();
        // Enemy level 5 at game level 1: 85 HP
        // Enemy level 5 at game level 11: 85 * 2.0 = 170 HP
        assert_eq!(scaling.health_for_level(5, 1), 85.0);
        assert_eq!(scaling.health_for_level(5, 11), 170.0);
    }

    #[test]
    fn enemy_scaling_calculates_correct_damage() {
        let scaling = EnemyScaling::default();
        assert_eq!(scaling.damage_for_level(1), 10.0);
        assert_eq!(scaling.damage_for_level(2), 15.0);
        assert_eq!(scaling.damage_for_level(3), 20.0);
        assert_eq!(scaling.damage_for_level(4), 25.0);
        assert_eq!(scaling.damage_for_level(5), 30.0);
    }

    #[test]
    fn enemy_scaling_level_zero_same_as_level_one() {
        let scaling = EnemyScaling::default();
        // Level 0 (invalid) should have same stats as level 1 due to saturating_sub
        assert_eq!(scaling.health_for_level(0, 1), 25.0);
        assert_eq!(scaling.damage_for_level(0), 10.0);
    }

    #[test]
    fn enemy_scaling_game_level_zero_same_as_level_one() {
        let scaling = EnemyScaling::default();
        // Game level 0 (invalid) should have same multiplier as level 1
        assert_eq!(scaling.health_for_level(1, 0), 25.0);
    }
}