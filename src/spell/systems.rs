use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rand::Rng;
use crate::spell::SpellType;

use crate::enemies::components::*;
use crate::audio::plugin::*;
use crate::audio::plugin::SoundLimiter;
use crate::game::resources::{GameMeshes, GameMaterials};
use crate::movement::components::from_xz;
use crate::whisper::resources::SpellOrigin;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::inventory::resources::SpellList;
    use crate::spell::components::Spell;
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

    mod inferno_tests {
        use super::*;
        use crate::spells::fire::fire_nova::FireNovaRing;

        #[test]
        fn inferno_spawns_fire_nova_from_spell_list() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.init_resource::<WhisperAttunement>();

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Inferno,
                element: Element::Fire,
                name: "Inferno".to_string(),
                description: "Expanding ring of flames.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 20.0,
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

            let mut nova_query = app.world_mut().query::<&FireNovaRing>();
            let novas: Vec<_> = nova_query.iter(app.world()).collect();
            assert_eq!(novas.len(), 1, "One fire nova should be spawned");
            // 20.0 * 2 * 1.25 = 50.0
            assert_eq!(novas[0].damage, 50.0, "Inferno damage should be 50.0 (20.0 * 2 * 1.25)");
        }

        #[test]
        fn inferno_with_fire_attunement() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });
            app.insert_resource(WhisperAttunement::with_element(Element::Fire));

            let mut spell_list = SpellList::default();
            let spell = Spell {
                spell_type: SpellType::Inferno,
                element: Element::Fire,
                name: "Inferno".to_string(),
                description: "Expanding ring of flames.".to_string(),
                level: 2,
                fire_rate: 2.0,
                base_damage: 20.0,
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

            let mut nova_query = app.world_mut().query::<&FireNovaRing>();
            let novas: Vec<_> = nova_query.iter(app.world()).collect();
            assert_eq!(novas.len(), 1);
            // 20.0 * 2 * 1.25 * 1.1 = 55.0
            let expected = 20.0 * 2.0 * 1.25 * 1.1;
            assert!(
                (novas[0].damage - expected).abs() < 0.01,
                "Inferno damage should be {} with fire attunement, got {}",
                expected,
                novas[0].damage
            );
        }

        #[test]
        fn inferno_spawns_at_spell_origin() {
            let mut app = App::new();
            app.add_systems(Update, spell_casting_system);

            let origin_pos = Vec3::new(5.0, 3.0, 10.0);
            app.insert_resource(SpellOrigin {
                position: Some(origin_pos),
            });
            app.init_resource::<WhisperAttunement>();

            let mut spell_list = SpellList::default();
            let mut spell = Spell::new(SpellType::Inferno);
            spell.last_fired = -10.0;
            spell_list.equip(spell);
            app.insert_resource(spell_list);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut nova_query = app.world_mut().query::<&FireNovaRing>();
            let novas: Vec<_> = nova_query.iter(app.world()).collect();
            assert_eq!(novas.len(), 1);
            // Nova should be centered at origin position on XZ plane
            assert_eq!(novas[0].center, Vec2::new(5.0, 10.0));
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
            SpellType::Inferno => {
                crate::spells::fire::fire_nova::fire_fire_nova_with_damage(
                    &mut commands,
                    spell,
                    final_damage,
                    origin_pos,
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
            _ => {
                // Other spell types not implemented yet
            }
        }

        // Update last_fired time
        if let Some(spell_mut) = spell_list.get_spell_mut(slot) {
            spell_mut.last_fired = current_time;
        }
    }
}
