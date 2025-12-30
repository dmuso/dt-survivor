use bevy::prelude::*;

#[derive(Component, Clone, Debug)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub level: u32, // 1-10
    pub fire_rate: f32, // seconds between shots
    pub base_damage: f32, // base damage at level 1
    pub last_fired: f32, // timestamp
}

#[derive(Clone, Debug)]
pub enum WeaponType {
    Pistol {
        bullet_count: usize,
        spread_angle: f32,
    },
    Laser,
    RocketLauncher, // Future: homing projectiles
    Bomb,           // Future: area damage
    BouncingLaser,  // Future: chain damage
}

impl PartialEq for WeaponType {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for WeaponType {}

impl std::hash::Hash for WeaponType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl WeaponType {
    pub fn id(&self) -> &'static str {
        match self {
            WeaponType::Pistol { .. } => "pistol",
            WeaponType::Laser => "laser",
            WeaponType::RocketLauncher => "rocket_launcher",
            WeaponType::Bomb => "bomb",
            WeaponType::BouncingLaser => "bouncing_laser",
        }
    }
}

impl Weapon {
    /// Calculate actual damage based on weapon level.
    ///
    /// # Formula
    /// `damage = base_damage * level * 1.25`
    ///
    /// This creates linear scaling where each level adds 125% of base damage:
    /// - Level 1: base_damage * 1.25
    /// - Level 2: base_damage * 2.5
    /// - Level 5: base_damage * 6.25
    /// - Level 10: base_damage * 12.5
    ///
    /// # Examples
    /// With base_damage = 10.0:
    /// - Level 1: 12.5 damage
    /// - Level 2: 25.0 damage
    /// - Level 5: 62.5 damage
    /// - Level 10: 125.0 damage
    pub fn damage(&self) -> f32 {
        self.base_damage * self.level as f32 * 1.25
    }

    pub fn can_level_up(&self) -> bool {
        self.level < 10
    }

    pub fn level_up(&mut self) {
        if self.can_level_up() {
            self.level += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pistol_weapon(level: u32, base_damage: f32) -> Weapon {
        Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 1,
                spread_angle: 0.0,
            },
            level,
            fire_rate: 1.0,
            base_damage,
            last_fired: 0.0,
        }
    }

    fn laser_weapon(level: u32, base_damage: f32) -> Weapon {
        Weapon {
            weapon_type: WeaponType::Laser,
            level,
            fire_rate: 0.5,
            base_damage,
            last_fired: 0.0,
        }
    }

    fn rocket_weapon(level: u32, base_damage: f32) -> Weapon {
        Weapon {
            weapon_type: WeaponType::RocketLauncher,
            level,
            fire_rate: 2.0,
            base_damage,
            last_fired: 0.0,
        }
    }

    #[test]
    fn weapon_damage_scales_with_level() {
        let weapon = pistol_weapon(1, 10.0);
        assert_eq!(weapon.damage(), 12.5);

        let weapon_5 = pistol_weapon(5, 10.0);
        assert_eq!(weapon_5.damage(), 62.5);

        let weapon_10 = pistol_weapon(10, 10.0);
        assert_eq!(weapon_10.damage(), 125.0);
    }

    #[test]
    fn weapon_damage_increases_linearly_with_level() {
        let base_weapon = laser_weapon(1, 20.0);
        let damage_1 = base_weapon.damage();

        let weapon_2 = laser_weapon(2, 20.0);
        let damage_2 = weapon_2.damage();

        // Damage should double when level doubles (linear scaling)
        assert!((damage_2 / damage_1 - 2.0).abs() < 0.01);
    }

    #[test]
    fn all_weapon_types_scale_damage_correctly() {
        let weapons = vec![
            pistol_weapon(5, 10.0),
            laser_weapon(5, 10.0),
            rocket_weapon(5, 10.0),
        ];

        for weapon in weapons {
            // 10 * 5 * 1.25 = 62.5
            assert_eq!(weapon.damage(), 62.5);
        }
    }

    #[test]
    fn weapon_damage_at_all_levels() {
        let base_damage = 10.0;
        let expected_damages = [
            (1, 12.5),
            (2, 25.0),
            (3, 37.5),
            (4, 50.0),
            (5, 62.5),
            (6, 75.0),
            (7, 87.5),
            (8, 100.0),
            (9, 112.5),
            (10, 125.0),
        ];

        for (level, expected_damage) in expected_damages {
            let weapon = pistol_weapon(level, base_damage);
            assert_eq!(
                weapon.damage(),
                expected_damage,
                "Level {} should have damage {}",
                level,
                expected_damage
            );
        }
    }

    #[test]
    fn weapon_damage_with_different_base_damages() {
        let level = 5;
        let test_cases = [
            (5.0, 31.25),   // 5 * 5 * 1.25
            (10.0, 62.5),   // 10 * 5 * 1.25
            (20.0, 125.0),  // 20 * 5 * 1.25
            (100.0, 625.0), // 100 * 5 * 1.25
        ];

        for (base_damage, expected_damage) in test_cases {
            let weapon = pistol_weapon(level, base_damage);
            assert_eq!(
                weapon.damage(),
                expected_damage,
                "Base damage {} at level {} should have damage {}",
                base_damage,
                level,
                expected_damage
            );
        }
    }

    #[test]
    fn weapon_can_level_up_below_max() {
        let weapon = pistol_weapon(5, 10.0);
        assert!(weapon.can_level_up());
    }

    #[test]
    fn weapon_cannot_level_up_at_max() {
        let weapon = pistol_weapon(10, 10.0);
        assert!(!weapon.can_level_up());
    }

    #[test]
    fn weapon_level_up_increases_level() {
        let mut weapon = pistol_weapon(5, 10.0);
        let old_level = weapon.level;
        weapon.level_up();
        assert_eq!(weapon.level, old_level + 1);
    }

    #[test]
    fn weapon_level_up_does_not_exceed_max() {
        let mut weapon = pistol_weapon(10, 10.0);
        weapon.level_up();
        assert_eq!(weapon.level, 10);
    }

    #[test]
    fn weapon_level_up_increases_damage() {
        let mut weapon = pistol_weapon(1, 10.0);
        let damage_before = weapon.damage();
        weapon.level_up();
        let damage_after = weapon.damage();

        assert!(
            damage_after > damage_before,
            "Damage should increase after level up"
        );
        // Level 1->2: (10*1*1.25)=12.5 -> (10*2*1.25)=25.0
        assert_eq!(damage_before, 12.5);
        assert_eq!(damage_after, 25.0);
    }

    #[test]
    fn weapon_type_ids_are_distinct() {
        let pistol = WeaponType::Pistol {
            bullet_count: 1,
            spread_angle: 0.0,
        };
        let laser = WeaponType::Laser;
        let rocket = WeaponType::RocketLauncher;
        let bomb = WeaponType::Bomb;
        let bouncing = WeaponType::BouncingLaser;

        assert_eq!(pistol.id(), "pistol");
        assert_eq!(laser.id(), "laser");
        assert_eq!(rocket.id(), "rocket_launcher");
        assert_eq!(bomb.id(), "bomb");
        assert_eq!(bouncing.id(), "bouncing_laser");
    }

    #[test]
    fn weapon_type_equality_ignores_params() {
        let pistol1 = WeaponType::Pistol {
            bullet_count: 1,
            spread_angle: 0.0,
        };
        let pistol2 = WeaponType::Pistol {
            bullet_count: 5,
            spread_angle: 15.0,
        };
        assert_eq!(pistol1, pistol2, "Pistol variants should be equal regardless of params");
    }
}
