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
    pub health_per_level: f32,
    pub base_damage: f32,
    pub damage_per_level: f32,
}

impl Default for EnemyScaling {
    fn default() -> Self {
        Self {
            base_health: 10.0,
            health_per_level: 15.0, // Level 5 = 10 + (4 * 15) = 70 HP
            base_damage: 10.0,
            damage_per_level: 5.0, // Level 5 = 10 + (4 * 5) = 30 damage
        }
    }
}

impl EnemyScaling {
    /// Calculate health for a given enemy level (1-5)
    pub fn health_for_level(&self, level: u8) -> f32 {
        self.base_health + (level.saturating_sub(1) as f32 * self.health_per_level)
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
        assert_eq!(scaling.base_health, 10.0);
        assert_eq!(scaling.health_per_level, 15.0);
        assert_eq!(scaling.base_damage, 10.0);
        assert_eq!(scaling.damage_per_level, 5.0);
    }

    #[test]
    fn enemy_scaling_calculates_correct_health() {
        let scaling = EnemyScaling::default();
        assert_eq!(scaling.health_for_level(1), 10.0);
        assert_eq!(scaling.health_for_level(2), 25.0);
        assert_eq!(scaling.health_for_level(3), 40.0);
        assert_eq!(scaling.health_for_level(4), 55.0);
        assert_eq!(scaling.health_for_level(5), 70.0);
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
        assert_eq!(scaling.health_for_level(0), 10.0);
        assert_eq!(scaling.damage_for_level(0), 10.0);
    }

    #[test]
    fn enemy_scaling_high_level_extrapolates() {
        let scaling = EnemyScaling::default();
        // Level 10 (if ever used): 10 + (9 * 15) = 145 HP
        assert_eq!(scaling.health_for_level(10), 145.0);
        // Level 10 damage: 10 + (9 * 5) = 55
        assert_eq!(scaling.damage_for_level(10), 55.0);
    }
}