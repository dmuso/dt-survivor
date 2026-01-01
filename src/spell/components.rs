use bevy::prelude::*;
use crate::element::Element;
use crate::spell::spell_type::SpellType;

#[derive(Component, Clone, Debug)]
pub struct Spell {
    pub spell_type: SpellType,
    pub element: Element,
    pub name: String,
    pub description: String,
    pub level: u32,       // 1-10
    pub fire_rate: f32,   // seconds between casts (1/shots_per_second)
    pub base_damage: f32, // base damage at level 1
    pub last_fired: f32,  // timestamp
}

impl Spell {
    /// Create a new spell with the given type and default values from SpellType.
    pub fn new(spell_type: SpellType) -> Self {
        let element = spell_type.element();
        let name = spell_type.name().to_string();
        let description = spell_type.description().to_string();
        let base_damage = spell_type.base_damage();
        // Convert fire_rate (shots/sec) to seconds between casts
        let fire_rate = 1.0 / spell_type.fire_rate();
        Self {
            spell_type,
            element,
            name,
            description,
            level: 1,
            fire_rate,
            base_damage,
            last_fired: 0.0,
        }
    }

    /// Calculate actual damage based on spell level.
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

    /// Calculate effective cast rate based on spell level.
    ///
    /// # Formula
    /// `effective_rate = fire_rate * (1.0 - (level - 1) * 0.5 / 9.0)`
    ///
    /// This creates linear scaling from 100% at level 1 to 50% at level 10:
    /// - Level 1: 100% of base fire_rate
    /// - Level 5: ~77.8% of base fire_rate
    /// - Level 10: 50% of base fire_rate (twice as fast)
    ///
    /// # Examples
    /// With fire_rate = 2.0:
    /// - Level 1: 2.0s between casts
    /// - Level 5: 1.56s between casts
    /// - Level 10: 1.0s between casts
    pub fn effective_fire_rate(&self) -> f32 {
        // Scale from 1.0 at level 1 to 0.5 at level 10
        let scale = 1.0 - (self.level - 1) as f32 * 0.5 / 9.0;
        self.fire_rate * scale
    }

    /// Calculate projectile count for projectile-based spells based on level.
    ///
    /// Projectile spells start with 1 projectile and gain +1 every 5 levels:
    /// - Levels 1-4: 1 projectile
    /// - Levels 5-9: 2 projectiles
    /// - Level 10: 3 projectiles
    ///
    /// Only applicable to projectile-type spells like Fireball, IceShard, etc.
    pub fn projectile_count(&self) -> usize {
        match self.spell_type {
            SpellType::Fireball
            | SpellType::IceShard
            | SpellType::VenomBolt
            | SpellType::ShadowBolt
            | SpellType::ChaosBolt => {
                // 1 projectile at levels 1-4, 2 at 5-9, 3 at 10
                1 + (self.level as usize / 5)
            }
            _ => 0,
        }
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

    fn create_spell(spell_type: SpellType, level: u32) -> Spell {
        let mut spell = Spell::new(spell_type);
        spell.level = level;
        spell
    }

    mod spell_creation_tests {
        use super::*;

        #[test]
        fn spell_new_creates_with_correct_element() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(spell.element, Element::Fire);
        }

        #[test]
        fn spell_new_creates_with_correct_name() {
            let spell = Spell::new(SpellType::RadiantBeam);
            assert_eq!(spell.name, "Radiant Beam");
        }

        #[test]
        fn spell_new_creates_with_correct_description() {
            let spell = Spell::new(SpellType::ThunderStrike);
            assert_eq!(
                spell.description,
                "Lightning strikes from above, dealing area damage."
            );
        }

        #[test]
        fn spell_new_sets_base_damage_from_spell_type() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(spell.base_damage, SpellType::Fireball.base_damage());
        }

        #[test]
        fn spell_new_sets_fire_rate_from_spell_type() {
            let spell = Spell::new(SpellType::Fireball);
            // fire_rate in Spell is seconds between casts (inverse of shots/sec)
            let expected = 1.0 / SpellType::Fireball.fire_rate();
            assert!((spell.fire_rate - expected).abs() < 0.001);
        }

        #[test]
        fn spell_new_starts_at_level_1() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(spell.level, 1);
        }

        #[test]
        fn spell_new_starts_with_zero_last_fired() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(spell.last_fired, 0.0);
        }

        #[test]
        fn spell_element_field_accessible() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(spell.element, Element::Fire);
        }

        #[test]
        fn spell_name_field_accessible() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(spell.name, "Fireball");
        }

        #[test]
        fn each_element_has_spell_with_matching_element() {
            for element in Element::all() {
                let spells = SpellType::by_element(*element);
                for spell_type in spells {
                    let spell = Spell::new(*spell_type);
                    assert_eq!(
                        spell.element, *element,
                        "Spell {:?} should have element {:?}",
                        spell_type, element
                    );
                }
            }
        }
    }

    mod spell_damage_tests {
        use super::*;

        #[test]
        fn spell_damage_scales_with_level() {
            let mut spell = Spell::new(SpellType::Fireball);
            spell.base_damage = 10.0; // Override for predictable testing

            spell.level = 1;
            assert_eq!(spell.damage(), 12.5);

            spell.level = 5;
            assert_eq!(spell.damage(), 62.5);

            spell.level = 10;
            assert_eq!(spell.damage(), 125.0);
        }

        #[test]
        fn spell_damage_increases_linearly_with_level() {
            let mut spell = Spell::new(SpellType::RadiantBeam);
            spell.base_damage = 20.0;

            spell.level = 1;
            let damage_1 = spell.damage();

            spell.level = 2;
            let damage_2 = spell.damage();

            // Damage should double when level doubles (linear scaling)
            assert!((damage_2 / damage_1 - 2.0).abs() < 0.01);
        }

        #[test]
        fn all_spell_types_scale_damage_correctly() {
            let spell_types = [
                SpellType::Fireball,
                SpellType::RadiantBeam,
                SpellType::ThunderStrike,
            ];

            for spell_type in spell_types {
                let mut spell = Spell::new(spell_type);
                spell.base_damage = 10.0;
                spell.level = 5;
                // 10 * 5 * 1.25 = 62.5
                assert_eq!(spell.damage(), 62.5);
            }
        }

        #[test]
        fn spell_damage_at_all_levels() {
            let mut spell = Spell::new(SpellType::Fireball);
            spell.base_damage = 10.0;

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
                spell.level = level;
                assert_eq!(
                    spell.damage(),
                    expected_damage,
                    "Level {} should have damage {}",
                    level,
                    expected_damage
                );
            }
        }

        #[test]
        fn spell_damage_with_different_base_damages() {
            let mut spell = Spell::new(SpellType::Fireball);
            spell.level = 5;

            let test_cases = [
                (5.0, 31.25),   // 5 * 5 * 1.25
                (10.0, 62.5),   // 10 * 5 * 1.25
                (20.0, 125.0),  // 20 * 5 * 1.25
                (100.0, 625.0), // 100 * 5 * 1.25
            ];

            for (base_damage, expected_damage) in test_cases {
                spell.base_damage = base_damage;
                assert_eq!(
                    spell.damage(),
                    expected_damage,
                    "Base damage {} at level 5 should have damage {}",
                    base_damage,
                    expected_damage
                );
            }
        }
    }

    mod spell_level_tests {
        use super::*;

        #[test]
        fn spell_can_level_up_below_max() {
            let spell = create_spell(SpellType::Fireball, 5);
            assert!(spell.can_level_up());
        }

        #[test]
        fn spell_cannot_level_up_at_max() {
            let spell = create_spell(SpellType::Fireball, 10);
            assert!(!spell.can_level_up());
        }

        #[test]
        fn spell_level_up_increases_level() {
            let mut spell = create_spell(SpellType::Fireball, 5);
            let old_level = spell.level;
            spell.level_up();
            assert_eq!(spell.level, old_level + 1);
        }

        #[test]
        fn spell_level_up_does_not_exceed_max() {
            let mut spell = create_spell(SpellType::Fireball, 10);
            spell.level_up();
            assert_eq!(spell.level, 10);
        }

        #[test]
        fn spell_level_up_increases_damage() {
            let mut spell = Spell::new(SpellType::Fireball);
            spell.base_damage = 10.0;
            spell.level = 1;

            let damage_before = spell.damage();
            spell.level_up();
            let damage_after = spell.damage();

            assert!(
                damage_after > damage_before,
                "Damage should increase after level up"
            );
            // Level 1->2: (10*1*1.25)=12.5 -> (10*2*1.25)=25.0
            assert_eq!(damage_before, 12.5);
            assert_eq!(damage_after, 25.0);
        }
    }

    mod spell_type_tests {
        use super::*;

        #[test]
        fn spell_type_ids_are_distinct() {
            assert_ne!(SpellType::Fireball.id(), SpellType::RadiantBeam.id());
            assert_ne!(SpellType::Fireball.id(), SpellType::ThunderStrike.id());
            assert_ne!(SpellType::FrostNova.id(), SpellType::VoidRift.id());
        }

        #[test]
        fn spell_type_element_returns_correct_element() {
            assert_eq!(SpellType::Fireball.element(), Element::Fire);
            assert_eq!(SpellType::RadiantBeam.element(), Element::Light);
            assert_eq!(SpellType::ThunderStrike.element(), Element::Lightning);
            assert_eq!(SpellType::FrostNova.element(), Element::Frost);
            assert_eq!(SpellType::VoidRift.element(), Element::Dark);
        }

        #[test]
        fn spell_type_name_returns_display_name() {
            assert_eq!(SpellType::Fireball.name(), "Fireball");
            assert_eq!(SpellType::RadiantBeam.name(), "Radiant Beam");
            assert_eq!(SpellType::ThunderStrike.name(), "Thunder Strike");
            assert_eq!(SpellType::FrostNova.name(), "Frost Nova");
        }

        #[test]
        fn spell_type_description_returns_text() {
            assert!(!SpellType::Fireball.description().is_empty());
            assert!(!SpellType::RadiantBeam.description().is_empty());
            assert!(!SpellType::ThunderStrike.description().is_empty());
            assert!(!SpellType::FrostNova.description().is_empty());
        }
    }

    mod fire_rate_tests {
        use super::*;

        #[test]
        fn effective_fire_rate_decreases_with_level() {
            let mut spell = Spell::new(SpellType::Fireball);

            spell.level = 1;
            let rate_1 = spell.effective_fire_rate();

            spell.level = 5;
            let rate_5 = spell.effective_fire_rate();

            spell.level = 10;
            let rate_10 = spell.effective_fire_rate();

            assert!(
                rate_5 < rate_1,
                "Level 5 should cast faster than level 1"
            );
            assert!(
                rate_10 < rate_5,
                "Level 10 should cast faster than level 5"
            );
        }

        #[test]
        fn effective_fire_rate_at_level_1_equals_base() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(
                spell.effective_fire_rate(),
                spell.fire_rate,
                "Level 1 should use base fire rate"
            );
        }

        #[test]
        fn effective_fire_rate_at_level_10_is_half_base() {
            let mut spell = Spell::new(SpellType::Fireball);
            spell.level = 10;
            // At level 10, fire rate should be 50% of base (twice as fast)
            assert!(
                (spell.effective_fire_rate() - spell.fire_rate * 0.5).abs() < 0.001,
                "Level 10 should have 50% of base fire rate"
            );
        }

        #[test]
        fn effective_fire_rate_scales_linearly() {
            let mut spell = Spell::new(SpellType::RadiantBeam);
            spell.fire_rate = 3.0;
            spell.level = 5;
            // Level 5: 1.0 - (5-1) * (0.5/9) = 1.0 - 4 * 0.0556 = 0.778
            let expected = 3.0 * (1.0 - (4.0 * 0.5 / 9.0));
            assert!((spell.effective_fire_rate() - expected).abs() < 0.01);
        }
    }

    mod projectile_count_tests {
        use super::*;

        #[test]
        fn fireball_projectile_count_starts_at_1() {
            let spell = Spell::new(SpellType::Fireball);
            assert_eq!(
                spell.projectile_count(),
                1,
                "Level 1 fireball should have 1 projectile"
            );
        }

        #[test]
        fn fireball_projectile_count_increases_every_5_levels() {
            let mut spell = Spell::new(SpellType::Fireball);

            spell.level = 1;
            assert_eq!(
                spell.projectile_count(),
                1,
                "Levels 1-4 should have 1 projectile"
            );

            spell.level = 4;
            assert_eq!(
                spell.projectile_count(),
                1,
                "Levels 1-4 should have 1 projectile"
            );

            spell.level = 5;
            assert_eq!(
                spell.projectile_count(),
                2,
                "Levels 5-9 should have 2 projectiles"
            );

            spell.level = 9;
            assert_eq!(
                spell.projectile_count(),
                2,
                "Levels 5-9 should have 2 projectiles"
            );

            spell.level = 10;
            assert_eq!(
                spell.projectile_count(),
                3,
                "Level 10 should have 3 projectiles"
            );
        }

        #[test]
        fn ice_shard_is_projectile_spell() {
            let spell = Spell::new(SpellType::IceShard);
            assert_eq!(
                spell.projectile_count(),
                1,
                "Ice Shard should be a projectile spell"
            );
        }

        #[test]
        fn venom_bolt_is_projectile_spell() {
            let spell = Spell::new(SpellType::VenomBolt);
            assert_eq!(
                spell.projectile_count(),
                1,
                "Venom Bolt should be a projectile spell"
            );
        }

        #[test]
        fn non_projectile_spells_return_zero() {
            let radiant_beam = Spell::new(SpellType::RadiantBeam);
            let thunder_strike = Spell::new(SpellType::ThunderStrike);
            let frost_nova = Spell::new(SpellType::FrostNova);

            assert_eq!(
                radiant_beam.projectile_count(),
                0,
                "Radiant Beam should return 0 projectile count"
            );
            assert_eq!(
                thunder_strike.projectile_count(),
                0,
                "Thunder Strike should return 0 projectile count"
            );
            assert_eq!(
                frost_nova.projectile_count(),
                0,
                "Frost Nova should return 0 projectile count"
            );
        }
    }
}
