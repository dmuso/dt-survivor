//! Purify spell (Light) - Cleanses debuffs from the player and damages nearby enemies.
//!
//! An instant burst spell centered on the player that removes negative status effects
//! (debuffs) from the player while dealing radiant damage to all enemies within range.
//! The spell triggers automatically when the cooldown is ready.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

// Import debuff types that Purify can cleanse
use crate::spells::frost::ice_shard::SlowedDebuff;
use crate::spells::light::solar_flare::BlindedDebuff;
use crate::spells::poison::venom_spray::PoisonStack;
use crate::spells::fire::fireball::BurnEffect;
use crate::spells::dark::void_pulse::WeakenedDebuff;

/// Default configuration for Purify spell
pub const PURIFY_RADIUS: f32 = 6.0;
pub const PURIFY_COOLDOWN: f32 = 2.5;
pub const PURIFY_VISUAL_HEIGHT: f32 = 0.5;

/// Get the light element color for visual effects (white/gold)
pub fn purify_color() -> Color {
    Element::Light.color()
}

/// Purify caster component - attached to the player when Purify spell is active.
/// Manages cooldown and triggers burst effects.
#[derive(Component, Debug, Clone)]
pub struct PurifyCaster {
    /// Cooldown between purify bursts
    pub cooldown: Timer,
    /// Radius within which enemies take damage
    pub radius: f32,
    /// Damage dealt per burst
    pub damage: f32,
}

impl PurifyCaster {
    /// Creates a new PurifyCaster with the given damage.
    pub fn new(damage: f32) -> Self {
        Self {
            cooldown: Timer::from_seconds(PURIFY_COOLDOWN, TimerMode::Repeating),
            radius: PURIFY_RADIUS,
            damage,
        }
    }

    /// Creates a PurifyCaster from a Spell component.
    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(spell.damage())
    }

    /// Check if the caster is ready to fire a burst.
    pub fn is_ready(&self) -> bool {
        self.cooldown.just_finished()
    }
}

/// Purify burst visual effect component.
/// Short-lived entity that represents the radiant burst effect.
#[derive(Component, Debug, Clone)]
pub struct PurifyBurst {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the burst
    pub radius: f32,
    /// Lifetime timer for cleanup
    pub lifetime: Timer,
}

impl PurifyBurst {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self {
            center,
            radius,
            lifetime: Timer::from_seconds(0.2, TimerMode::Once),
        }
    }

    /// Check if the burst has expired.
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }

    /// Tick the lifetime.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.lifetime.tick(delta);
    }
}

/// System that updates purify caster cooldown timers.
pub fn update_purify_cooldown(
    time: Res<Time>,
    mut caster_query: Query<&mut PurifyCaster>,
) {
    for mut caster in caster_query.iter_mut() {
        caster.cooldown.tick(time.delta());
    }
}

/// System that triggers purify bursts when the cooldown is ready.
/// Deals damage to all enemies in range and cleanses debuffs from the player.
pub fn trigger_purify_burst(
    mut commands: Commands,
    caster_query: Query<(&PurifyCaster, &Transform)>,
    player_query: Query<Entity, With<Player>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (caster, caster_transform) in caster_query.iter() {
        if !caster.is_ready() {
            continue;
        }

        let caster_pos = from_xz(caster_transform.translation);

        // Deal damage to all enemies in range
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = caster_pos.distance(enemy_pos);

            if distance <= caster.radius {
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    caster.damage,
                    Element::Light,
                ));
            }
        }

        // Cleanse debuffs from the player
        if let Some(player_entity) = player_query.iter().next() {
            // Remove all cleansable debuffs
            commands.entity(player_entity)
                .remove::<SlowedDebuff>()
                .remove::<BlindedDebuff>()
                .remove::<PoisonStack>()
                .remove::<BurnEffect>()
                .remove::<WeakenedDebuff>();
        }

        // Spawn visual burst effect
        commands.spawn((
            Transform::from_translation(caster_transform.translation),
            PurifyBurst::new(caster_pos, caster.radius),
        ));
    }
}

/// System that cleans up expired purify burst visuals.
pub fn cleanup_purify_bursts(
    mut commands: Commands,
    mut burst_query: Query<(Entity, &mut PurifyBurst)>,
    time: Res<Time>,
) {
    for (entity, mut burst) in burst_query.iter_mut() {
        burst.tick(time.delta());

        if burst.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Purify spell - spawns a caster component on the player.
/// The caster will automatically trigger bursts when the cooldown is ready.
pub fn fire_purify(
    commands: &mut Commands,
    spell: &Spell,
    player_entity: Entity,
) {
    fire_purify_with_damage(commands, spell, spell.damage(), player_entity);
}

/// Cast Purify spell with explicit damage.
pub fn fire_purify_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    player_entity: Entity,
) {
    commands.entity(player_entity).insert(PurifyCaster::new(damage));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;
    use crate::combat::Health;

    mod purify_caster_tests {
        use super::*;

        #[test]
        fn test_purify_caster_new() {
            let damage = 25.0;
            let caster = PurifyCaster::new(damage);

            assert_eq!(caster.damage, damage);
            assert_eq!(caster.radius, PURIFY_RADIUS);
            assert!(!caster.is_ready(), "New caster should not be ready");
        }

        #[test]
        fn test_purify_caster_from_spell() {
            let spell = Spell::new(SpellType::Purify);
            let caster = PurifyCaster::from_spell(&spell);

            assert_eq!(caster.damage, spell.damage());
            assert_eq!(caster.radius, PURIFY_RADIUS);
        }

        #[test]
        fn test_purify_caster_is_ready_after_cooldown() {
            let mut caster = PurifyCaster::new(10.0);

            // Tick past cooldown
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            assert!(caster.is_ready(), "Caster should be ready after cooldown");
        }

        #[test]
        fn test_purify_caster_not_ready_before_cooldown() {
            let mut caster = PurifyCaster::new(10.0);

            // Tick but not past cooldown
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN / 2.0));

            assert!(!caster.is_ready(), "Caster should not be ready before cooldown");
        }

        #[test]
        fn test_purify_uses_light_element_color() {
            let color = purify_color();
            assert_eq!(color, Element::Light.color());
        }
    }

    mod purify_burst_tests {
        use super::*;

        #[test]
        fn test_purify_burst_new() {
            let center = Vec2::new(5.0, 10.0);
            let radius = 6.0;
            let burst = PurifyBurst::new(center, radius);

            assert_eq!(burst.center, center);
            assert_eq!(burst.radius, radius);
            assert!(!burst.is_expired());
        }

        #[test]
        fn test_purify_burst_expires() {
            let mut burst = PurifyBurst::new(Vec2::ZERO, 5.0);

            assert!(!burst.is_expired());

            burst.tick(Duration::from_secs_f32(0.3));
            assert!(burst.is_expired());
        }
    }

    mod update_purify_cooldown_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_cooldown_ticks() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(PurifyCaster::new(10.0)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(update_purify_cooldown);

            let caster = app.world().get::<PurifyCaster>(entity).unwrap();
            assert!(caster.cooldown.elapsed_secs() > 0.0);
        }

        #[test]
        fn test_cooldown_fires_after_duration() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(PurifyCaster::new(10.0)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));
            }

            let _ = app.world_mut().run_system_once(update_purify_cooldown);

            let caster = app.world().get::<PurifyCaster>(entity).unwrap();
            assert!(caster.is_ready());
        }
    }

    mod trigger_purify_burst_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_purify_damages_enemies_in_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (trigger_purify_burst, count_damage).chain());

            // Create ready caster at origin
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Should damage enemy in radius");
        }

        #[test]
        fn test_purify_does_not_damage_enemies_outside_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (trigger_purify_burst, count_damage).chain());

            // Create ready caster at origin
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create enemy outside radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(20.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Should not damage enemy outside radius");
        }

        #[test]
        fn test_purify_damages_multiple_enemies() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (trigger_purify_burst, count_damage).chain());

            // Create ready caster
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create 3 enemies within radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3, "Should damage all enemies in radius");
        }

        #[test]
        fn test_purify_does_not_fire_before_cooldown() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (trigger_purify_burst, count_damage).chain());

            // Create non-ready caster (cooldown not finished)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PurifyCaster::new(20.0),
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Should not fire before cooldown");
        }

        #[test]
        fn test_purify_spawns_visual_burst() {
            let mut app = setup_test_app();
            app.add_systems(Update, trigger_purify_burst);

            // Create ready caster
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            app.update();

            let mut burst_query = app.world_mut().query::<&PurifyBurst>();
            let count = burst_query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn one visual burst");
        }

        #[test]
        fn test_purify_cleanses_slowed_debuff() {
            let mut app = setup_test_app();
            app.add_systems(Update, trigger_purify_burst);

            // Create player with SlowedDebuff
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::ZERO),
                SlowedDebuff::new(5.0, 0.5),
            )).id();

            // Create ready caster
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            app.update();

            // Player should no longer have SlowedDebuff
            assert!(
                app.world().get::<SlowedDebuff>(player_entity).is_none(),
                "SlowedDebuff should be cleansed"
            );
        }

        #[test]
        fn test_purify_cleanses_blinded_debuff() {
            let mut app = setup_test_app();
            app.add_systems(Update, trigger_purify_burst);

            // Create player with BlindedDebuff
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::ZERO),
                BlindedDebuff::new(5.0),
            )).id();

            // Create ready caster
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            app.update();

            // Player should no longer have BlindedDebuff
            assert!(
                app.world().get::<BlindedDebuff>(player_entity).is_none(),
                "BlindedDebuff should be cleansed"
            );
        }

        #[test]
        fn test_purify_works_without_debuffs() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (trigger_purify_burst, count_damage).chain());

            // Create player without debuffs
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create ready caster
            let mut caster = PurifyCaster::new(20.0);
            caster.cooldown.tick(Duration::from_secs_f32(PURIFY_COOLDOWN + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            // Should still deal damage even without debuffs to cleanse
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }
    }

    mod cleanup_purify_bursts_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_expired_burst_despawns() {
            let mut app = setup_test_app();

            let mut burst = PurifyBurst::new(Vec2::ZERO, 5.0);
            burst.tick(Duration::from_secs_f32(0.3)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_purify_bursts);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Expired burst should despawn"
            );
        }

        #[test]
        fn test_active_burst_survives() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PurifyBurst::new(Vec2::ZERO, 5.0),
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_purify_bursts);

            assert!(
                app.world().get_entity(entity).is_ok(),
                "Active burst should survive"
            );
        }
    }

    mod fire_purify_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_purify_adds_caster_to_player() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Purify);
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::ZERO),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_purify(&mut commands, &spell, player_entity);
            }
            app.update();

            // Player should have PurifyCaster component
            assert!(
                app.world().get::<PurifyCaster>(player_entity).is_some(),
                "Player should have PurifyCaster component"
            );
        }

        #[test]
        fn test_fire_purify_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Purify);
            let expected_damage = spell.damage();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::ZERO),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_purify(&mut commands, &spell, player_entity);
            }
            app.update();

            let caster = app.world().get::<PurifyCaster>(player_entity).unwrap();
            assert_eq!(caster.damage, expected_damage);
        }

        #[test]
        fn test_fire_purify_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Purify);
            let explicit_damage = 999.0;

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::ZERO),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_purify_with_damage(&mut commands, &spell, explicit_damage, player_entity);
            }
            app.update();

            let caster = app.world().get::<PurifyCaster>(player_entity).unwrap();
            assert_eq!(caster.damage, explicit_damage);
        }
    }
}
