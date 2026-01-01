use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::spell::SpellType;

use crate::enemies::components::*;
use crate::audio::plugin::*;
use crate::audio::plugin::SoundLimiter;
use crate::game::resources::{GameMeshes, GameMaterials};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::whisper::resources::SpellOrigin;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::inventory::resources::SpellList;
    use crate::spell::components::Spell;
    use crate::spells::psychic::echo_thought::LastSpellCast;
    use crate::whisper::resources::WhisperAttunement;

    mod spell_list_integration_tests {
        use super::*;

        #[test]
        fn spell_casting_uses_spell_list_resource() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            // Set up SpellList with a fireball
            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0; // Ready to fire
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let fireball_count = app.world_mut()
                .query::<&crate::spells::fire::fireball::FireballProjectile>()
                .iter(app.world())
                .count();
            assert_eq!(fireball_count, 1, "Fireball should spawn from SpellList");
        }

        #[test]
        fn spell_casting_iterates_all_5_spell_slots() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            // Set up SpellList with multiple spells
            let mut spell_list = SpellList::default();
            for spell_type in [SpellType::Fireball, SpellType::RadiantBeam, SpellType::ThunderStrike] {
                let mut spell = Spell::new(spell_type);
                spell.last_fired = -10.0;
                spell_list.equip(spell);
            }
            app.insert_resource(spell_list);

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            // All three spell types should have cast
            let fireball_count = app.world_mut()
                .query::<&crate::spells::fire::fireball::FireballProjectile>()
                .iter(app.world())
                .count();
            let beam_count = app.world_mut()
                .query::<&crate::spells::light::radiant_beam::RadiantBeam>()
                .iter(app.world())
                .count();
            let thunder_count = app.world_mut()
                .query::<&crate::spells::lightning::thunder_strike::ThunderStrikeMarker>()
                .iter(app.world())
                .count();

            assert!(fireball_count >= 1, "Fireball should cast from slot 0");
            assert!(beam_count >= 1, "Radiant beam should cast from slot 1");
            assert!(thunder_count >= 1, "Thunder strike should cast from slot 2");
        }

        #[test]
        fn empty_spell_slots_are_skipped() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            // Empty SpellList - no spells equipped
            app.init_resource::<SpellList>();

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            // Should not panic when iterating empty slots
            app.update();
        }

        #[test]
        fn cooldown_prevents_spell_casting() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = 1000.0; // Far in future, still on cooldown
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let fireball_count = app.world_mut()
                .query::<&crate::spells::fire::fireball::FireballProjectile>()
                .iter(app.world())
                .count();
            assert_eq!(fireball_count, 0, "Fireball should not spawn when on cooldown");
        }

        #[test]
        fn cooldown_resets_after_cast() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0;
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let spell_list = app.world().get_resource::<SpellList>().unwrap();
            let spell = spell_list.get_spell(0).unwrap();
            assert!(spell.last_fired >= 0.0, "last_fired should update after casting");
        }
    }

    mod whisper_attunement_tests {
        use super::*;
        use crate::spells::fire::fireball::FireballProjectile;

        #[test]
        fn attunement_bonus_applied_to_matching_element() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });

            // Fire attunement for fire spell
            app.insert_resource(WhisperAttunement::with_element(Element::Fire));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0;
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut query = app.world_mut().query::<&FireballProjectile>();
            let projectiles: Vec<_> = query.iter(app.world()).collect();
            assert_eq!(projectiles.len(), 1);

            // Fireball base damage 15.0 * level 1 * 1.25 = 18.75
            // With 10% fire attunement: 18.75 * 1.1 = 20.625
            let expected_damage = 15.0 * 1.0 * 1.25 * 1.1;
            assert!(
                (projectiles[0].damage - expected_damage).abs() < 0.01,
                "Expected damage {} with fire attunement, got {}",
                expected_damage,
                projectiles[0].damage
            );
        }

        #[test]
        fn no_attunement_uses_base_damage() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });

            // No attunement
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0;
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut query = app.world_mut().query::<&FireballProjectile>();
            let projectiles: Vec<_> = query.iter(app.world()).collect();
            assert_eq!(projectiles.len(), 1);

            // Fireball base damage 15.0 * level 1 * 1.25 = 18.75 (no attunement bonus)
            let expected_damage = 15.0 * 1.0 * 1.25;
            assert!(
                (projectiles[0].damage - expected_damage).abs() < 0.01,
                "Expected base damage {} without attunement, got {}",
                expected_damage,
                projectiles[0].damage
            );
        }

        #[test]
        fn mismatched_attunement_uses_base_damage() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });

            // Frost attunement for fire spell - no bonus
            app.insert_resource(WhisperAttunement::with_element(Element::Frost));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0;
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut query = app.world_mut().query::<&FireballProjectile>();
            let projectiles: Vec<_> = query.iter(app.world()).collect();
            assert_eq!(projectiles.len(), 1);

            // Fireball base damage 15.0 * level 1 * 1.25 = 18.75 (no frost bonus on fire)
            let expected_damage = 15.0 * 1.0 * 1.25;
            assert!(
                (projectiles[0].damage - expected_damage).abs() < 0.01,
                "Expected base damage {} with mismatched attunement, got {}",
                expected_damage,
                projectiles[0].damage
            );
        }
    }

    mod spells_disabled_tests {
        use super::*;

        #[test]
        fn spells_disabled_without_whisper() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin { position: None });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0;
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let fireball_count = app.world_mut()
                .query::<&crate::spells::fire::fireball::FireballProjectile>()
                .iter(app.world())
                .count();
            assert_eq!(fireball_count, 0, "No fireballs should spawn when Whisper not collected");
        }
    }

    mod targeting_tests {
        use super::*;

        #[test]
        fn spell_targets_from_closest_5_enemies() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -10.0;
            spell_list.equip(fireball);
            app.insert_resource(spell_list);

            // Create 10 enemies at different distances
            for i in 1..=10 {
                let distance = i as f32 * 20.0;
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(distance, 0.375, 0.0)),
                ));
            }

            app.init_resource::<Time>();
            app.update();

            // Should cast toward one of the 5 closest enemies
            let fireball_count = app.world_mut()
                .query::<&crate::spells::fire::fireball::FireballProjectile>()
                .iter(app.world())
                .count();
            assert!(fireball_count >= 1, "Fireball should be cast");
        }
    }

    mod radiant_beam_tests {
        use super::*;
        use crate::spells::light::radiant_beam::RadiantBeam;

        #[test]
        fn radiant_beam_spawns_with_correct_damage_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let beam = Spell {
                spell_type: SpellType::RadiantBeam,
                element: Element::Light,
                name: "Radiant Beam".to_string(),
                description: "A beam of light.".to_string(),
                level: 5,
                fire_rate: 0.1,
                base_damage: 10.0,
                last_fired: -10.0,
            };
            spell_list.equip(beam);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut beam_query = app.world_mut().query::<&RadiantBeam>();
            let beams: Vec<_> = beam_query.iter(app.world()).collect();
            assert_eq!(beams.len(), 1, "One radiant beam should be spawned");
            assert_eq!(beams[0].damage, 62.5, "Beam damage should be 62.5 (10.0 * 5 * 1.25)");
        }

        #[test]
        fn radiant_beam_with_light_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Light));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let beam = Spell {
                spell_type: SpellType::RadiantBeam,
                element: Element::Light,
                name: "Radiant Beam".to_string(),
                description: "A beam of light.".to_string(),
                level: 5,
                fire_rate: 0.1,
                base_damage: 10.0,
                last_fired: -10.0,
            };
            spell_list.equip(beam);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut beam_query = app.world_mut().query::<&RadiantBeam>();
            let beams: Vec<_> = beam_query.iter(app.world()).collect();
            assert_eq!(beams.len(), 1);
            // 10.0 * 5 * 1.25 * 1.1 = 68.75
            let expected = 10.0 * 5.0 * 1.25 * 1.1;
            assert!(
                (beams[0].damage - expected).abs() < 0.01,
                "Beam damage should be {} with light attunement, got {}",
                expected,
                beams[0].damage
            );
        }
    }

    mod thunder_strike_tests {
        use super::*;
        use crate::spells::lightning::thunder_strike::ThunderStrikeMarker;

        #[test]
        fn thunder_strike_spawns_with_correct_damage_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::ThunderStrike,
                element: Element::Lightning,
                name: "Thunder Strike".to_string(),
                description: "Lightning from above.".to_string(),
                level: 3,
                fire_rate: 2.0,
                base_damage: 30.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut marker_query = app.world_mut().query::<&ThunderStrikeMarker>();
            let markers: Vec<_> = marker_query.iter(app.world()).collect();
            assert_eq!(markers.len(), 1, "One thunder strike marker should be spawned");
            assert_eq!(markers[0].damage, 112.5, "Thunder strike damage should be 112.5 (30.0 * 3 * 1.25)");
        }

        #[test]
        fn thunder_strike_with_lightning_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Lightning));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::ThunderStrike,
                element: Element::Lightning,
                name: "Thunder Strike".to_string(),
                description: "Lightning from above.".to_string(),
                level: 3,
                fire_rate: 2.0,
                base_damage: 30.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut marker_query = app.world_mut().query::<&ThunderStrikeMarker>();
            let markers: Vec<_> = marker_query.iter(app.world()).collect();
            assert_eq!(markers.len(), 1);
            // 30.0 * 3 * 1.25 * 1.1 = 123.75
            let expected = 30.0 * 3.0 * 1.25 * 1.1;
            assert!(
                (markers[0].damage - expected).abs() < 0.01,
                "Thunder strike damage should be {} with lightning attunement, got {}",
                expected,
                markers[0].damage
            );
        }
    }

    mod ashfall_tests {
        use super::*;

        #[test]
        fn ashfall_spawns_zone_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Ashfall,
                element: Element::Fire,
                name: "Ashfall".to_string(),
                description: "Embers rain down over an area.".to_string(),
                level: 2,
                fire_rate: 4.0, // 0.25 shots/sec = 4 sec cooldown
                base_damage: 18.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&crate::spells::fire::ashfall::AshfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1, "One ashfall zone should be spawned");
            // 18.0 * 2 * 1.25 * 0.2 (damage ratio) = 9.0
            let expected_ember_damage = 18.0 * 2.0 * 1.25 * 0.2;
            assert!(
                (zones[0].damage_per_ember - expected_ember_damage).abs() < 0.1,
                "Ashfall ember damage should be {}, got {}",
                expected_ember_damage,
                zones[0].damage_per_ember
            );
        }

        #[test]
        fn ashfall_with_fire_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Fire));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Ashfall,
                element: Element::Fire,
                name: "Ashfall".to_string(),
                description: "Embers rain down over an area.".to_string(),
                level: 2,
                fire_rate: 4.0,
                base_damage: 18.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&crate::spells::fire::ashfall::AshfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1);
            // 18.0 * 2 * 1.25 * 1.1 * 0.2 = 9.9
            let expected = 18.0 * 2.0 * 1.25 * 1.1 * 0.2;
            assert!(
                (zones[0].damage_per_ember - expected).abs() < 0.1,
                "Ashfall damage should be {} with fire attunement, got {}",
                expected,
                zones[0].damage_per_ember
            );
        }

        #[test]
        fn ashfall_spawns_ahead_of_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(0.0, 3.0, 0.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Ashfall);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Enemy is to the right (positive X)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&crate::spells::fire::ashfall::AshfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1);
            // Zone should be spawned ahead of origin in direction of enemy
            assert!(zones[0].center.x > 0.0, "Zone should be ahead in X direction");
        }
    }

    mod plague_cloud_tests {
        use super::*;
        use crate::spells::poison::poison_cloud::PoisonCloudProjectile;

        #[test]
        fn plague_cloud_spawns_projectile_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::PlagueCloud,
                element: Element::Poison,
                name: "Plague Cloud".to_string(),
                description: "Arcing poison cloud.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 25.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut projectile_query = app.world_mut().query::<&PoisonCloudProjectile>();
            let projectiles: Vec<_> = projectile_query.iter(app.world()).collect();
            assert_eq!(projectiles.len(), 1, "One poison cloud projectile should be spawned");
            // 25.0 * 2 * 1.25 = 62.5
            assert_eq!(projectiles[0].damage, 62.5, "Plague cloud damage should be 62.5 (25.0 * 2 * 1.25)");
        }

        #[test]
        fn plague_cloud_with_poison_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Poison));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::PlagueCloud,
                element: Element::Poison,
                name: "Plague Cloud".to_string(),
                description: "Arcing poison cloud.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 25.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut projectile_query = app.world_mut().query::<&PoisonCloudProjectile>();
            let projectiles: Vec<_> = projectile_query.iter(app.world()).collect();
            assert_eq!(projectiles.len(), 1);
            // 25.0 * 2 * 1.25 * 1.1 = 68.75
            let expected = 25.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (projectiles[0].damage - expected).abs() < 0.01,
                "Plague cloud damage should be {} with poison attunement, got {}",
                expected,
                projectiles[0].damage
            );
        }

        #[test]
        fn plague_cloud_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::PlagueCloud);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 10.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut projectile_query = app.world_mut().query::<&PoisonCloudProjectile>();
            let projectiles: Vec<_> = projectile_query.iter(app.world()).collect();
            assert_eq!(projectiles.len(), 1);
            // Projectile should start from origin position on XZ plane
            assert_eq!(projectiles[0].start_pos, Vec2::new(5.0, 10.0));
        }
    }

    mod chain_lightning_tests {
        use super::*;
        use crate::spells::lightning::chain_lightning::ChainLightningBolt;

        #[test]
        fn chain_lightning_spawns_bolt_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::ChainLightning,
                element: Element::Lightning,
                name: "Chain Lightning".to_string(),
                description: "Arcing lightning.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 15.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut bolt_query = app.world_mut().query::<&ChainLightningBolt>();
            let bolts: Vec<_> = bolt_query.iter(app.world()).collect();
            assert_eq!(bolts.len(), 1, "One chain lightning bolt should be spawned");
            // 15.0 * 2 * 1.25 = 37.5
            assert_eq!(bolts[0].current_damage, 37.5, "Chain lightning damage should be 37.5 (15.0 * 2 * 1.25)");
        }

        #[test]
        fn chain_lightning_with_lightning_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Lightning));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::ChainLightning,
                element: Element::Lightning,
                name: "Chain Lightning".to_string(),
                description: "Arcing lightning.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 15.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut bolt_query = app.world_mut().query::<&ChainLightningBolt>();
            let bolts: Vec<_> = bolt_query.iter(app.world()).collect();
            assert_eq!(bolts.len(), 1);
            // 15.0 * 2 * 1.25 * 1.1 = 41.25
            let expected = 15.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (bolts[0].current_damage - expected).abs() < 0.01,
                "Chain lightning damage should be {} with lightning attunement, got {}",
                expected,
                bolts[0].current_damage
            );
        }

        #[test]
        fn chain_lightning_targets_enemy_entity() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::ChainLightning);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            )).id();

            app.init_resource::<Time>();
            app.update();

            let mut bolt_query = app.world_mut().query::<&ChainLightningBolt>();
            let bolts: Vec<_> = bolt_query.iter(app.world()).collect();
            assert_eq!(bolts.len(), 1);
            // Bolt should target the enemy entity
            assert_eq!(bolts[0].target, enemy_entity);
        }
    }

    mod toxic_spray_tests {
        use super::*;
        use crate::spells::poison::venom_spray::VenomSprayCone;

        #[test]
        fn toxic_spray_spawns_cone_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::ToxicSpray,
                element: Element::Poison,
                name: "Toxic Spray".to_string(),
                description: "Cone of poison.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 14.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut cone_query = app.world_mut().query::<&VenomSprayCone>();
            let cones: Vec<_> = cone_query.iter(app.world()).collect();
            assert_eq!(cones.len(), 1, "One venom spray cone should be spawned");
            // 14.0 * 2 * 1.25 = 35.0
            assert_eq!(cones[0].base_damage, 35.0, "Toxic spray damage should be 35.0 (14.0 * 2 * 1.25)");
        }

        #[test]
        fn toxic_spray_with_poison_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Poison));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::ToxicSpray,
                element: Element::Poison,
                name: "Toxic Spray".to_string(),
                description: "Cone of poison.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 14.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut cone_query = app.world_mut().query::<&VenomSprayCone>();
            let cones: Vec<_> = cone_query.iter(app.world()).collect();
            assert_eq!(cones.len(), 1);
            // 14.0 * 2 * 1.25 * 1.1 = 38.5
            let expected = 14.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (cones[0].base_damage - expected).abs() < 0.01,
                "Toxic spray damage should be {} with poison attunement, got {}",
                expected,
                cones[0].base_damage
            );
        }

        #[test]
        fn toxic_spray_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::ToxicSpray);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 10.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut cone_query = app.world_mut().query::<&VenomSprayCone>();
            let cones: Vec<_> = cone_query.iter(app.world()).collect();
            assert_eq!(cones.len(), 1);
            // Cone should be centered at origin position on XZ plane
            assert_eq!(cones[0].origin, Vec2::new(5.0, 10.0));
        }
    }

    mod frost_nova_tests {
        use super::*;
        use crate::spells::frost::glacial_pulse::GlacialPulseWave;

        #[test]
        fn frost_nova_spawns_glacial_pulse_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::FrostNova,
                element: Element::Frost,
                name: "Frost Nova".to_string(),
                description: "Expanding ring of frost.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 15.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut pulse_query = app.world_mut().query::<&GlacialPulseWave>();
            let pulses: Vec<_> = pulse_query.iter(app.world()).collect();
            assert_eq!(pulses.len(), 1, "One glacial pulse should be spawned");
            // 15.0 * 2 * 1.25 = 37.5
            assert_eq!(pulses[0].damage, 37.5, "Frost nova damage should be 37.5 (15.0 * 2 * 1.25)");
        }

        #[test]
        fn frost_nova_with_frost_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Frost));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::FrostNova,
                element: Element::Frost,
                name: "Frost Nova".to_string(),
                description: "Expanding ring of frost.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 15.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut pulse_query = app.world_mut().query::<&GlacialPulseWave>();
            let pulses: Vec<_> = pulse_query.iter(app.world()).collect();
            assert_eq!(pulses.len(), 1);
            // 15.0 * 2 * 1.25 * 1.1 = 41.25
            let expected = 15.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (pulses[0].damage - expected).abs() < 0.01,
                "Frost nova damage should be {} with frost attunement, got {}",
                expected,
                pulses[0].damage
            );
        }

        #[test]
        fn frost_nova_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::FrostNova);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut pulse_query = app.world_mut().query::<&GlacialPulseWave>();
            let pulses: Vec<_> = pulse_query.iter(app.world()).collect();
            assert_eq!(pulses.len(), 1);
            // Pulse should be centered at origin position on XZ plane
            assert_eq!(pulses[0].center, Vec2::new(5.0, 10.0));
        }
    }

    mod miasma_tests {
        use super::*;
        use crate::spells::poison::toxic_glob::ToxicGlobProjectile;

        #[test]
        fn miasma_spawns_toxic_glob_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Miasma,
                element: Element::Poison,
                name: "Miasma".to_string(),
                description: "Toxic glob.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 6.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut glob_query = app.world_mut().query::<&ToxicGlobProjectile>();
            let globs: Vec<_> = glob_query.iter(app.world()).collect();
            assert_eq!(globs.len(), 1, "One toxic glob should be spawned");
            // 6.0 * 2 * 1.25 = 15.0
            assert_eq!(globs[0].damage, 15.0, "Miasma damage should be 15.0 (6.0 * 2 * 1.25)");
        }

        #[test]
        fn miasma_with_poison_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Poison));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Miasma,
                element: Element::Poison,
                name: "Miasma".to_string(),
                description: "Toxic glob.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 6.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut glob_query = app.world_mut().query::<&ToxicGlobProjectile>();
            let globs: Vec<_> = glob_query.iter(app.world()).collect();
            assert_eq!(globs.len(), 1);
            // 6.0 * 2 * 1.25 * 1.1 = 16.5
            let expected = 6.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (globs[0].damage - expected).abs() < 0.01,
                "Miasma damage should be {} with poison attunement, got {}",
                expected,
                globs[0].damage
            );
        }

        #[test]
        fn miasma_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Miasma);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 10.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut glob_query = app.world_mut().query::<(&Transform, &ToxicGlobProjectile)>();
            let globs: Vec<_> = glob_query.iter(app.world()).collect();
            assert_eq!(globs.len(), 1);
            // Glob should spawn at origin position
            assert_eq!(globs[0].0.translation, origin_pos);
        }

        #[test]
        fn miasma_targets_enemy_direction() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Miasma);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Enemy in +X direction
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut glob_query = app.world_mut().query::<&ToxicGlobProjectile>();
            let globs: Vec<_> = glob_query.iter(app.world()).collect();
            assert_eq!(globs.len(), 1);
            // Glob should face +X direction
            assert!(
                globs[0].direction.x > 0.9,
                "Glob should face toward enemy (+X), got direction {:?}",
                globs[0].direction
            );
        }
    }

    mod combustion_tests {
        use super::*;
        use crate::spells::fire::ember_swarm::{EmberSwarmController, EmberWisp};

        #[test]
        fn combustion_spawns_ember_swarm_from_spell_list() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Combustion,
                element: Element::Fire,
                name: "Combustion".to_string(),
                description: "Ember swarm.".to_string(),
                level: 2,
                fire_rate: 1.0,
                base_damage: 22.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            // Should spawn 1 controller
            let mut controller_query = app.world_mut().query::<&EmberSwarmController>();
            let controllers: Vec<_> = controller_query.iter(app.world()).collect();
            assert_eq!(controllers.len(), 1, "One ember swarm controller should be spawned");

            // Should spawn 5-8 wisps
            let mut wisp_query = app.world_mut().query::<&EmberWisp>();
            let wisp_count = wisp_query.iter(app.world()).count();
            assert!(wisp_count >= 5 && wisp_count <= 8, "Expected 5-8 wisps, got {}", wisp_count);
        }

        #[test]
        fn combustion_with_fire_attunement() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Fire));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Combustion,
                element: Element::Fire,
                name: "Combustion".to_string(),
                description: "Ember swarm.".to_string(),
                level: 2,
                fire_rate: 1.0,
                base_damage: 22.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            let mut wisp_query = app.world_mut().query::<&EmberWisp>();
            let wisps: Vec<_> = wisp_query.iter(app.world()).collect();
            assert!(!wisps.is_empty(), "Should have wisps");
            // 22.0 * 2 * 1.25 * 1.1 = 60.5
            let expected = 22.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (wisps[0].damage - expected).abs() < 0.01,
                "Combustion damage should be {} with fire attunement, got {}",
                expected,
                wisps[0].damage
            );
        }

        #[test]
        fn combustion_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Combustion);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 10.0)),
            ));

            app.update();

            // Controller should be at origin position
            let mut controller_query = app.world_mut().query::<(&EmberSwarmController, &Transform)>();
            let controllers: Vec<_> = controller_query.iter(app.world()).collect();
            assert_eq!(controllers.len(), 1);
            assert_eq!(controllers[0].1.translation, origin_pos);
        }
    }

    mod hellfire_tests {
        use super::*;
        use crate::spells::fire::inferno_pulse::InfernoPulseWave;

        #[test]
        fn hellfire_spawns_inferno_pulse_from_spell_list() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<crate::combat::DamageEvent>();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Hellfire,
                element: Element::Fire,
                name: "Hellfire".to_string(),
                description: "Infernal pulse.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 18.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            // Should spawn a visual wave
            let mut wave_query = app.world_mut().query::<&InfernoPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1, "One inferno pulse wave should be spawned");
        }

        #[test]
        fn hellfire_with_fire_attunement() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<crate::combat::DamageEvent>();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Fire));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Hellfire,
                element: Element::Fire,
                name: "Hellfire".to_string(),
                description: "Infernal pulse.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 18.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            // Should still spawn a wave (verifies the spell fires with attunement)
            let mut wave_query = app.world_mut().query::<&InfernoPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1, "Inferno pulse should fire with fire attunement");
        }

        #[test]
        fn hellfire_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<crate::combat::DamageEvent>();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Hellfire);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 10.0)),
            ));

            app.update();

            let mut wave_query = app.world_mut().query::<&InfernoPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1);
            // Wave should be centered at origin position on XZ plane
            assert_eq!(waves[0].center, Vec2::new(5.0, 10.0));
        }
    }

    mod flashstep_tests {
        use super::*;
        use crate::combat::Health;
        use crate::spells::lightning::flashstep::FlashstepTeleport;

        #[test]
        fn flashstep_queues_teleport_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Flashstep,
                element: Element::Lightning,
                name: "Flashstep".to_string(),
                description: "Teleport with lightning burst.".to_string(),
                level: 1,
                fire_rate: 3.0,
                base_damage: 20.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Create player - needed for Flashstep
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::new(1.0, 0.0, 0.0), // Moving +X
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            // Create enemy to trigger spell casting
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            // Player should have FlashstepTeleport component queued
            let teleport = app.world().get::<FlashstepTeleport>(player_entity);
            assert!(teleport.is_some(), "Flashstep should queue a teleport on the player");

            let teleport = teleport.unwrap();
            // Destination should be in +X direction (player's movement direction)
            assert!(teleport.destination.x > teleport.origin.x, "Destination should be in +X direction");
        }

        #[test]
        fn flashstep_uses_movement_direction() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Flashstep);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Create player moving in +Z direction
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::new(0.0, 0.0, 1.0), // Moving +Z
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(5.0, 0.5, 5.0)),
            )).id();

            // Create enemy (needed for spell to fire)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 5.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // With movement in +Z, destination Y (which maps from Z) should be greater
            assert!(teleport.destination.y > teleport.origin.y, "Destination should be in +Z direction (Y in 2D)");
        }

        #[test]
        fn flashstep_with_lightning_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Lightning));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Flashstep,
                element: Element::Lightning,
                name: "Flashstep".to_string(),
                description: "Teleport with lightning burst.".to_string(),
                level: 2,
                fire_rate: 3.0,
                base_damage: 20.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::new(1.0, 0.0, 0.0),
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // 20.0 * 2 * 1.25 * 1.1 = 55.0
            let expected = 20.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (teleport.burst_damage - expected).abs() < 0.01,
                "Flashstep damage should be {} with lightning attunement, got {}",
                expected,
                teleport.burst_damage
            );
        }

        #[test]
        fn flashstep_targets_enemy_when_stationary() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Flashstep);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Create stationary player (no movement direction)
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO, // Stationary
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            // Create enemy in +X direction
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // Should teleport toward enemy (+X direction)
            assert!(
                teleport.destination.x > teleport.origin.x,
                "Stationary player should teleport toward enemy"
            );
        }
    }

    mod corrosive_pool_tests {
        use super::*;
        use crate::spells::poison::acid_rain::AcidRainZone;

        #[test]
        fn corrosive_pool_spawns_acid_rain_zone_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::CorrosivePool,
                element: Element::Poison,
                name: "Corrosive Pool".to_string(),
                description: "Creates a pool of acid.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 12.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&AcidRainZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1, "One acid rain zone should be spawned");
        }

        #[test]
        fn corrosive_pool_with_poison_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Poison));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::CorrosivePool,
                element: Element::Poison,
                name: "Corrosive Pool".to_string(),
                description: "Creates a pool of acid.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 12.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&AcidRainZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1, "Zone should fire with poison attunement");
            // 12.0 * 2 * 1.25 * 1.1 * 0.15 (damage ratio) = 4.95
            let expected_droplet_damage = 12.0 * 2.0 * 1.25 * 1.1 * 0.15;
            assert!(
                (zones[0].damage_per_droplet - expected_droplet_damage).abs() < 0.1,
                "Acid rain damage should be {} with poison attunement, got {}",
                expected_droplet_damage,
                zones[0].damage_per_droplet
            );
        }

        #[test]
        fn corrosive_pool_spawns_ahead_of_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(0.0, 3.0, 0.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::CorrosivePool);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Enemy is to the right (positive X)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&AcidRainZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1);
            // Zone should be spawned ahead of origin in direction of enemy
            assert!(zones[0].center.x > 0.0, "Zone should be ahead in X direction");
        }
    }

    mod dark_pulse_tests {
        use super::*;
        use crate::spells::dark::void_pulse::VoidPulseWave;

        #[test]
        fn dark_pulse_spawns_wave_from_spell_list() {
            let mut app = App::new();
            app.add_message::<crate::combat::DamageEvent>();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::DarkPulse,
                element: Element::Dark,
                name: "Dark Pulse".to_string(),
                description: "Releases a wave of dark energy.".to_string(),
                level: 2,
                fire_rate: 1.25, // 0.8 shots/sec
                base_damage: 20.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut wave_query = app.world_mut().query::<&VoidPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1, "One void pulse wave should be spawned");
            // 20.0 * 2 * 1.25 = 50.0
            assert_eq!(waves[0].damage, 50.0, "Dark pulse damage should be 50.0 (20.0 * 2 * 1.25)");
        }

        #[test]
        fn dark_pulse_with_dark_attunement() {
            let mut app = App::new();
            app.add_message::<crate::combat::DamageEvent>();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Dark));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::DarkPulse,
                element: Element::Dark,
                name: "Dark Pulse".to_string(),
                description: "Releases a wave of dark energy.".to_string(),
                level: 2,
                fire_rate: 1.25,
                base_damage: 20.0,
                last_fired: -10.0,
            };
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut wave_query = app.world_mut().query::<&VoidPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1);
            // 20.0 * 2 * 1.25 * 1.1 = 55.0
            let expected = 20.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (waves[0].damage - expected).abs() < 0.01,
                "Dark pulse damage should be {} with dark attunement, got {}",
                expected,
                waves[0].damage
            );
        }

        #[test]
        fn dark_pulse_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_message::<crate::combat::DamageEvent>();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(10.0, 3.0, 15.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::DarkPulse);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(20.0, 0.375, 15.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut wave_query = app.world_mut().query::<&VoidPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1);
            // Wave should be centered at origin position on XZ plane
            assert_eq!(waves[0].center, Vec2::new(10.0, 15.0));
        }
    }

    mod eclipse_tests {
        use super::*;
        use crate::spells::dark::nightfall::NightfallZone;

        #[test]
        fn eclipse_spawns_nightfall_zone_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Eclipse);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1, "One nightfall zone should be spawned");
        }

        #[test]
        fn eclipse_spawns_at_target_position() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(0.0, 3.0, 0.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Eclipse);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            // Enemy at (15, 0.375, 20) - zone should spawn at this XZ position
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 20.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1);
            // Zone should be centered at enemy XZ position
            assert_eq!(zones[0].center, Vec2::new(15.0, 20.0));
        }

        #[test]
        fn eclipse_with_dark_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Dark));
            app.init_resource::<LastSpellCast>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Eclipse);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            // Zone should still spawn with dark attunement
            assert_eq!(zones.len(), 1, "Eclipse should fire with dark attunement");
        }
    }
}

use crate::inventory::resources::SpellList;
use crate::whisper::resources::WhisperAttunement;

#[allow(clippy::too_many_arguments)]
pub fn spell_casting_system(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Option<Res<AssetServer>>,
    mut weapon_channel: Option<ResMut<AudioChannel<WeaponSoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
    spell_origin: Res<SpellOrigin>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
    enemy_query: Query<(Entity, &Transform, &Enemy)>,
    mut spell_list: ResMut<SpellList>,
    attunement: Res<WhisperAttunement>,
    mut damage_events: Option<MessageWriter<DamageEvent>>,
    player_query: Query<(Entity, &Transform, &Player)>,
    mut last_spell_cast: ResMut<crate::spells::psychic::echo_thought::LastSpellCast>,
) {
    let current_time = time.elapsed_secs();

    // Check if Whisper has been collected (spells enabled)
    let Some(origin_pos) = spell_origin.position else {
        return; // No Whisper = no spells
    };

    // Extract XZ plane position for targeting calculations
    let origin_xz = from_xz(origin_pos);

    // Find 5 closest enemies to the spell origin (Whisper)
    // Use XZ plane for distance calculation in 3D world
    let mut enemy_distances: Vec<(Entity, Vec2, f32)> = enemy_query
        .iter()
        .map(|(entity, transform, _)| {
            let pos = from_xz(transform.translation);
            let distance = origin_xz.distance(pos);
            (entity, pos, distance)
        })
        .collect();

    // Sort by distance and take first 5
    enemy_distances.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
    let closest_enemies: Vec<(Entity, Vec2)> = enemy_distances
        .into_iter()
        .take(5)
        .map(|(entity, pos, _)| (entity, pos))
        .collect();

    // If no enemies, don't cast
    if closest_enemies.is_empty() {
        return;
    }

    // Cast spells from all 5 slots in SpellList
    for slot in 0..5 {
        // Get spell from slot, skip empty slots
        let Some(spell) = spell_list.get_spell(slot) else {
            continue;
        };

        // Check cooldown
        if current_time - spell.last_fired < spell.effective_fire_rate() {
            continue;
        }

        // Select random target from 5 closest
        let mut rng = rand::thread_rng();
        let target_index = rng.gen_range(0..closest_enemies.len());
        let target_pos = closest_enemies[target_index].1;

        // Calculate damage with attunement multiplier
        let attunement_multiplier = attunement.damage_multiplier(spell.element);
        let final_damage = spell.damage() * attunement_multiplier;

        // Cast the spell based on type
        match &spell.spell_type {
            SpellType::Fireball => {
                crate::spells::fire::fireball::fire_fireball_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    asset_server.as_ref(),
                    weapon_channel.as_mut(),
                    sound_limiter.as_mut(),
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::RadiantBeam => {
                crate::spells::light::radiant_beam::fire_radiant_beam_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );

                // Play radiant beam sound effect
                if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
                    (asset_server.as_ref(), weapon_channel.as_mut(), sound_limiter.as_mut()) {
                    crate::audio::plugin::play_limited_sound_with_volume(
                        weapon_channel.as_mut(),
                        asset_server,
                        "sounds/72639__chipfork71__laser01rev.wav",
                        sound_limiter.as_mut(),
                        0.7,
                    );
                }
            }
            SpellType::ThunderStrike => {
                crate::spells::lightning::thunder_strike::fire_thunder_strike_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Ashfall => {
                crate::spells::fire::ashfall::spawn_ashfall_zone_with_damage(
                    &mut commands,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::PlagueCloud => {
                crate::spells::poison::poison_cloud::fire_poison_cloud_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::ChainLightning => {
                // Chain lightning targets a specific entity
                let target_entity = closest_enemies[target_index].0;
                crate::spells::lightning::chain_lightning::fire_chain_lightning_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_entity,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::IceShard => {
                crate::spells::frost::ice_shard::fire_ice_shard_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::GlacialSpike => {
                crate::spells::frost::ice_shards::fire_ice_shards_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::ToxicSpray => {
                crate::spells::poison::venom_spray::fire_venom_spray_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::FlameLance => {
                crate::spells::fire::cinder_shot::fire_cinder_shot_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Miasma => {
                crate::spells::poison::toxic_glob::fire_toxic_glob_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::FrostNova => {
                crate::spells::frost::glacial_pulse::fire_glacial_pulse_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Combustion => {
                crate::spells::fire::ember_swarm::fire_ember_swarm_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Overcharge => {
                crate::spells::lightning::overload::fire_overload_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Blizzard => {
                crate::spells::frost::frozen_orb::fire_frozen_orb_with_damage(
                    &mut commands,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::FrozenRay => {
                crate::spells::frost::ice_lance::fire_ice_lance_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Hellfire => {
                if let Some(ref mut events) = damage_events {
                    crate::spells::fire::inferno_pulse::fire_inferno_pulse_with_damage(
                        &mut commands,
                        spell,
                        final_damage,
                        origin_pos,
                        &enemy_query,
                        events,
                        game_meshes.as_deref(),
                        game_materials.as_deref(),
                    );
                }
            }
            SpellType::Electrocute => {
                crate::spells::lightning::ion_field::fire_ion_field_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Flashstep => {
                // Flashstep requires player entity and position
                if let Ok((player_entity, player_transform, player)) = player_query.single() {
                    // Use player's last movement direction, or direction toward nearest enemy if stationary
                    let direction = if player.last_movement_direction.length() > 0.1 {
                        // Convert 3D direction to 2D on XZ plane
                        Vec2::new(
                            player.last_movement_direction.x,
                            player.last_movement_direction.z,
                        ).normalize()
                    } else {
                        // Stationary: teleport toward nearest enemy
                        let player_pos_xz = from_xz(player_transform.translation);
                        (target_pos - player_pos_xz).normalize()
                    };

                    crate::spells::lightning::flashstep::fire_flashstep_with_damage(
                        &mut commands,
                        spell,
                        final_damage,
                        player_entity,
                        player_transform.translation,
                        direction,
                    );
                }
            }
            SpellType::CorrosivePool => {
                crate::spells::poison::acid_rain::spawn_acid_rain_zone_with_damage(
                    &mut commands,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Shatter => {
                crate::spells::frost::shatter::fire_shatter_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Radiance => {
                crate::spells::light::radiance::fire_radiance_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::SoulDrain => {
                crate::spells::dark::soul_drain::fire_soul_drain_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::DarkPulse => {
                crate::spells::dark::void_pulse::fire_void_pulse_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::MentalSpike => {
                crate::spells::psychic::mind_lash::fire_mind_lash_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::PsychicWave => {
                crate::spells::psychic::psionic_burst::fire_psionic_burst_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Dominate => {
                crate::spells::psychic::dominate::fire_dominate(
                    &mut commands,
                    spell,
                    origin_pos,
                );
            }
            SpellType::Telekinesis => {
                crate::spells::psychic::synapse_shock::fire_synapse_shock(
                    &mut commands,
                    spell,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::ChaosBolt => {
                crate::spells::chaos::chaos_bolt::fire_chaos_bolt_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Entropy => {
                crate::spells::chaos::entropy_field::fire_entropy_field_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Smite => {
                crate::spells::light::solar_flare::fire_solar_flare_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Eclipse => {
                crate::spells::dark::nightfall::fire_nightfall_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::DivineLight => {
                crate::spells::light::halo_shield::fire_halo_shield_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Confusion => {
                crate::spells::psychic::brainburn::fire_brainburn_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Judgment => {
                crate::spells::light::judgment::fire_judgment_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Hallucination => {
                crate::spells::psychic::echo_thought::fire_echo_thought(
                    &mut commands,
                    spell,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Paradox => {
                crate::spells::chaos::warp_rift::spawn_warp_rift_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::Consecration => {
                crate::spells::light::beacon::fire_beacon_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
                    target_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            SpellType::MindBlast => {
                crate::spells::psychic::mind_cage::fire_mind_cage(
                    &mut commands,
                    spell,
                    origin_pos,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
            }
            _ => {
                // Other spell types not implemented yet
            }
        }

        // Record last spell cast for Echo Thought (but not Hallucination itself to avoid infinite echoing)
        if spell.spell_type != SpellType::Hallucination {
            let direction = (target_pos - origin_xz).normalize_or_zero();
            last_spell_cast.record(spell.spell_type, origin_xz, direction, final_damage);
        }

        // Update last_fired time
        if let Some(spell_mut) = spell_list.get_spell_mut(slot) {
            spell_mut.last_fired = current_time;
        }
    }
}
