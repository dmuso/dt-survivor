use bevy::prelude::*;
use crate::element::Element;

#[derive(Component, Clone, Debug)]
pub struct Spell {
    pub spell_type: SpellType,
    pub element: Element,
    pub name: String,
    pub description: String,
    pub level: u32,      // 1-10
    pub fire_rate: f32,  // seconds between casts
    pub base_damage: f32, // base damage at level 1
    pub last_fired: f32, // timestamp
}

#[derive(Clone, Debug)]
pub enum SpellType {
    Fireball {
        bullet_count: usize,
        spread_angle: f32,
    },
    RadiantBeam,
    ThunderStrike,
    FrostNova,
    VoidOrb,
}

impl PartialEq for SpellType {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for SpellType {}

impl std::hash::Hash for SpellType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl SpellType {
    pub fn id(&self) -> &'static str {
        match self {
            SpellType::Fireball { .. } => "fireball",
            SpellType::RadiantBeam => "radiant_beam",
            SpellType::ThunderStrike => "thunder_strike",
            SpellType::FrostNova => "frost_nova",
            SpellType::VoidOrb => "void_orb",
        }
    }

    pub fn element(&self) -> Element {
        match self {
            SpellType::Fireball { .. } => Element::Fire,
            SpellType::RadiantBeam => Element::Light,
            SpellType::ThunderStrike => Element::Lightning,
            SpellType::FrostNova => Element::Frost,
            SpellType::VoidOrb => Element::Dark,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SpellType::Fireball { .. } => "Fireball",
            SpellType::RadiantBeam => "Radiant Beam",
            SpellType::ThunderStrike => "Thunder Strike",
            SpellType::FrostNova => "Frost Nova",
            SpellType::VoidOrb => "Void Orb",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SpellType::Fireball { .. } => "A blazing projectile that ignites enemies on impact.",
            SpellType::RadiantBeam => "A focused beam of pure light energy.",
            SpellType::ThunderStrike => "Lightning strikes from above, dealing area damage.",
            SpellType::FrostNova => "An icy explosion that freezes nearby enemies.",
            SpellType::VoidOrb => "A dark sphere that consumes all in its path.",
        }
    }
}

impl Spell {
    /// Create a new spell with the given type and default values.
    pub fn new(spell_type: SpellType) -> Self {
        let element = spell_type.element();
        let name = spell_type.name().to_string();
        let description = spell_type.description().to_string();
        Self {
            spell_type,
            element,
            name,
            description,
            level: 1,
            fire_rate: 1.0,
            base_damage: 10.0,
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

    /// Calculate projectile count for fireball spells based on level.
    ///
    /// Fireballs start with 1 projectile and gain +1 every 5 levels:
    /// - Levels 1-4: 1 projectile
    /// - Levels 5-9: 2 projectiles
    /// - Level 10: 3 projectiles
    ///
    /// Returns 0 for non-fireball spells.
    pub fn projectile_count(&self) -> usize {
        match &self.spell_type {
            SpellType::Fireball { .. } => {
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

    fn fireball_spell(level: u32, base_damage: f32) -> Spell {
        Spell {
            spell_type: SpellType::Fireball {
                bullet_count: 1,
                spread_angle: 0.0,
            },
            element: Element::Fire,
            name: "Fireball".to_string(),
            description: "A blazing projectile.".to_string(),
            level,
            fire_rate: 1.0,
            base_damage,
            last_fired: 0.0,
        }
    }

    fn radiant_beam_spell(level: u32, base_damage: f32) -> Spell {
        Spell {
            spell_type: SpellType::RadiantBeam,
            element: Element::Light,
            name: "Radiant Beam".to_string(),
            description: "A beam of light.".to_string(),
            level,
            fire_rate: 0.5,
            base_damage,
            last_fired: 0.0,
        }
    }

    fn thunder_strike_spell(level: u32, base_damage: f32) -> Spell {
        Spell {
            spell_type: SpellType::ThunderStrike,
            element: Element::Lightning,
            name: "Thunder Strike".to_string(),
            description: "Lightning from above.".to_string(),
            level,
            fire_rate: 2.0,
            base_damage,
            last_fired: 0.0,
        }
    }

    mod spell_creation_tests {
        use super::*;

        #[test]
        fn spell_new_creates_with_correct_element() {
            let spell = Spell::new(SpellType::Fireball {
                bullet_count: 1,
                spread_angle: 0.0,
            });
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
            assert_eq!(spell.description, "Lightning strikes from above, dealing area damage.");
        }

        #[test]
        fn spell_element_field_accessible() {
            let spell = fireball_spell(1, 10.0);
            assert_eq!(spell.element, Element::Fire);
        }

        #[test]
        fn spell_name_field_accessible() {
            let spell = fireball_spell(1, 10.0);
            assert_eq!(spell.name, "Fireball");
        }

        #[test]
        fn spell_description_field_accessible() {
            let spell = fireball_spell(1, 10.0);
            assert_eq!(spell.description, "A blazing projectile.");
        }
    }

    mod spell_damage_tests {
        use super::*;

        #[test]
        fn spell_damage_scales_with_level() {
            let spell = fireball_spell(1, 10.0);
            assert_eq!(spell.damage(), 12.5);

            let spell_5 = fireball_spell(5, 10.0);
            assert_eq!(spell_5.damage(), 62.5);

            let spell_10 = fireball_spell(10, 10.0);
            assert_eq!(spell_10.damage(), 125.0);
        }

        #[test]
        fn spell_damage_increases_linearly_with_level() {
            let base_spell = radiant_beam_spell(1, 20.0);
            let damage_1 = base_spell.damage();

            let spell_2 = radiant_beam_spell(2, 20.0);
            let damage_2 = spell_2.damage();

            // Damage should double when level doubles (linear scaling)
            assert!((damage_2 / damage_1 - 2.0).abs() < 0.01);
        }

        #[test]
        fn all_spell_types_scale_damage_correctly() {
            let spells = vec![
                fireball_spell(5, 10.0),
                radiant_beam_spell(5, 10.0),
                thunder_strike_spell(5, 10.0),
            ];

            for spell in spells {
                // 10 * 5 * 1.25 = 62.5
                assert_eq!(spell.damage(), 62.5);
            }
        }

        #[test]
        fn spell_damage_at_all_levels() {
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
                let spell = fireball_spell(level, base_damage);
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
            let level = 5;
            let test_cases = [
                (5.0, 31.25),   // 5 * 5 * 1.25
                (10.0, 62.5),   // 10 * 5 * 1.25
                (20.0, 125.0),  // 20 * 5 * 1.25
                (100.0, 625.0), // 100 * 5 * 1.25
            ];

            for (base_damage, expected_damage) in test_cases {
                let spell = fireball_spell(level, base_damage);
                assert_eq!(
                    spell.damage(),
                    expected_damage,
                    "Base damage {} at level {} should have damage {}",
                    base_damage,
                    level,
                    expected_damage
                );
            }
        }
    }

    mod spell_level_tests {
        use super::*;

        #[test]
        fn spell_can_level_up_below_max() {
            let spell = fireball_spell(5, 10.0);
            assert!(spell.can_level_up());
        }

        #[test]
        fn spell_cannot_level_up_at_max() {
            let spell = fireball_spell(10, 10.0);
            assert!(!spell.can_level_up());
        }

        #[test]
        fn spell_level_up_increases_level() {
            let mut spell = fireball_spell(5, 10.0);
            let old_level = spell.level;
            spell.level_up();
            assert_eq!(spell.level, old_level + 1);
        }

        #[test]
        fn spell_level_up_does_not_exceed_max() {
            let mut spell = fireball_spell(10, 10.0);
            spell.level_up();
            assert_eq!(spell.level, 10);
        }

        #[test]
        fn spell_level_up_increases_damage() {
            let mut spell = fireball_spell(1, 10.0);
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
            let fireball = SpellType::Fireball {
                bullet_count: 1,
                spread_angle: 0.0,
            };
            let radiant_beam = SpellType::RadiantBeam;
            let thunder_strike = SpellType::ThunderStrike;
            let frost_nova = SpellType::FrostNova;
            let void_orb = SpellType::VoidOrb;

            assert_eq!(fireball.id(), "fireball");
            assert_eq!(radiant_beam.id(), "radiant_beam");
            assert_eq!(thunder_strike.id(), "thunder_strike");
            assert_eq!(frost_nova.id(), "frost_nova");
            assert_eq!(void_orb.id(), "void_orb");
        }

        #[test]
        fn spell_type_equality_ignores_params() {
            let fireball1 = SpellType::Fireball {
                bullet_count: 1,
                spread_angle: 0.0,
            };
            let fireball2 = SpellType::Fireball {
                bullet_count: 5,
                spread_angle: 15.0,
            };
            assert_eq!(fireball1, fireball2, "Fireball variants should be equal regardless of params");
        }

        #[test]
        fn spell_type_element_returns_correct_element() {
            assert_eq!(SpellType::Fireball { bullet_count: 1, spread_angle: 0.0 }.element(), Element::Fire);
            assert_eq!(SpellType::RadiantBeam.element(), Element::Light);
            assert_eq!(SpellType::ThunderStrike.element(), Element::Lightning);
            assert_eq!(SpellType::FrostNova.element(), Element::Frost);
            assert_eq!(SpellType::VoidOrb.element(), Element::Dark);
        }

        #[test]
        fn spell_type_name_returns_display_name() {
            assert_eq!(SpellType::Fireball { bullet_count: 1, spread_angle: 0.0 }.name(), "Fireball");
            assert_eq!(SpellType::RadiantBeam.name(), "Radiant Beam");
            assert_eq!(SpellType::ThunderStrike.name(), "Thunder Strike");
            assert_eq!(SpellType::FrostNova.name(), "Frost Nova");
            assert_eq!(SpellType::VoidOrb.name(), "Void Orb");
        }

        #[test]
        fn spell_type_description_returns_text() {
            assert!(!SpellType::Fireball { bullet_count: 1, spread_angle: 0.0 }.description().is_empty());
            assert!(!SpellType::RadiantBeam.description().is_empty());
            assert!(!SpellType::ThunderStrike.description().is_empty());
            assert!(!SpellType::FrostNova.description().is_empty());
            assert!(!SpellType::VoidOrb.description().is_empty());
        }
    }

    mod fire_rate_tests {
        use super::*;

        #[test]
        fn effective_fire_rate_decreases_with_level() {
            let spell_1 = fireball_spell(1, 10.0);
            let spell_5 = fireball_spell(5, 10.0);
            let spell_10 = fireball_spell(10, 10.0);

            assert!(
                spell_5.effective_fire_rate() < spell_1.effective_fire_rate(),
                "Level 5 should cast faster than level 1"
            );
            assert!(
                spell_10.effective_fire_rate() < spell_5.effective_fire_rate(),
                "Level 10 should cast faster than level 5"
            );
        }

        #[test]
        fn effective_fire_rate_at_level_1_equals_base() {
            let spell = fireball_spell(1, 10.0);
            assert_eq!(
                spell.effective_fire_rate(),
                spell.fire_rate,
                "Level 1 should use base fire rate"
            );
        }

        #[test]
        fn effective_fire_rate_at_level_10_is_half_base() {
            let spell = fireball_spell(10, 10.0);
            // At level 10, fire rate should be 50% of base (twice as fast)
            assert_eq!(
                spell.effective_fire_rate(),
                spell.fire_rate * 0.5,
                "Level 10 should have 50% of base fire rate"
            );
        }

        #[test]
        fn effective_fire_rate_scales_linearly() {
            // Formula: fire_rate * (1.0 - (level - 1) * 0.055555...)
            // At level 1: 100%, level 10: 50%
            let spell = Spell {
                spell_type: SpellType::RadiantBeam,
                element: Element::Light,
                name: "Radiant Beam".to_string(),
                description: "A beam of light.".to_string(),
                level: 5,
                fire_rate: 3.0,
                base_damage: 15.0,
                last_fired: 0.0,
            };
            // Level 5: 1.0 - (5-1) * (0.5/9) = 1.0 - 4 * 0.0556 = 0.778
            let expected = 3.0 * (1.0 - (4.0 * 0.5 / 9.0));
            assert!((spell.effective_fire_rate() - expected).abs() < 0.01);
        }
    }

    mod projectile_count_tests {
        use super::*;

        #[test]
        fn fireball_projectile_count_starts_at_1() {
            let spell = fireball_spell(1, 10.0);
            assert_eq!(spell.projectile_count(), 1, "Level 1 fireball should have 1 projectile");
        }

        #[test]
        fn fireball_projectile_count_increases_every_5_levels() {
            let level_1 = fireball_spell(1, 10.0);
            let level_4 = fireball_spell(4, 10.0);
            let level_5 = fireball_spell(5, 10.0);
            let level_9 = fireball_spell(9, 10.0);
            let level_10 = fireball_spell(10, 10.0);

            assert_eq!(level_1.projectile_count(), 1, "Levels 1-4 should have 1 projectile");
            assert_eq!(level_4.projectile_count(), 1, "Levels 1-4 should have 1 projectile");
            assert_eq!(level_5.projectile_count(), 2, "Levels 5-9 should have 2 projectiles");
            assert_eq!(level_9.projectile_count(), 2, "Levels 5-9 should have 2 projectiles");
            assert_eq!(level_10.projectile_count(), 3, "Level 10 should have 3 projectiles");
        }

        #[test]
        fn non_fireball_projectile_count_returns_zero() {
            let radiant_beam = radiant_beam_spell(5, 10.0);
            let thunder_strike = thunder_strike_spell(5, 10.0);

            assert_eq!(radiant_beam.projectile_count(), 0, "Radiant Beam should return 0 projectile count");
            assert_eq!(thunder_strike.projectile_count(), 0, "Thunder Strike should return 0 projectile count");
        }
    }
}
