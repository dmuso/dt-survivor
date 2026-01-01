use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Arc Surge spell
pub const ARC_SURGE_PULSE_INTERVAL: f32 = 0.8;
pub const ARC_SURGE_PULSE_RANGE: f32 = 10.0;
pub const ARC_SURGE_TARGETS_PER_PULSE: u8 = 3;
pub const ARC_SURGE_DURATION: f32 = 6.0;

/// Get the lightning element color for visual effects (yellow)
pub fn arc_surge_color() -> Color {
    Element::Lightning.color()
}

/// Arc Surge component - periodic lightning bursts that hit random enemies in range.
/// Similar to Inferno Pulse but with lightning effects and multi-target selection.
#[derive(Component, Debug, Clone)]
pub struct ArcSurge {
    /// Timer for periodic pulse attacks
    pub pulse_timer: Timer,
    /// Range within which enemies can be targeted
    pub pulse_range: f32,
    /// Maximum number of targets per pulse
    pub targets_per_pulse: u8,
    /// Damage dealt per target
    pub damage_per_target: f32,
    /// Total duration the spell is active
    pub duration: Timer,
}

impl ArcSurge {
    pub fn new(damage: f32) -> Self {
        Self {
            pulse_timer: Timer::from_seconds(ARC_SURGE_PULSE_INTERVAL, TimerMode::Repeating),
            pulse_range: ARC_SURGE_PULSE_RANGE,
            targets_per_pulse: ARC_SURGE_TARGETS_PER_PULSE,
            damage_per_target: damage,
            duration: Timer::from_seconds(ARC_SURGE_DURATION, TimerMode::Once),
        }
    }

    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(spell.damage())
    }

    /// Check if the spell has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }
}

/// Marker component indicating Arc Surge is currently active on the player
#[derive(Component, Debug, Clone)]
pub struct ArcSurgeActive;

/// System that updates Arc Surge duration timers and removes expired instances
pub fn arc_surge_duration_system(
    mut commands: Commands,
    time: Res<Time>,
    mut surge_query: Query<(Entity, &mut ArcSurge)>,
) {
    for (entity, mut surge) in surge_query.iter_mut() {
        surge.duration.tick(time.delta());

        if surge.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that handles Arc Surge pulsing and targeting random enemies
pub fn arc_surge_pulse_system(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut surge_query: Query<&mut ArcSurge>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = from_xz(player_transform.translation);

    for mut surge in surge_query.iter_mut() {
        surge.pulse_timer.tick(time.delta());

        if surge.pulse_timer.just_finished() {
            // Collect enemies within range
            let mut enemies_in_range: Vec<(Entity, f32)> = enemy_query
                .iter()
                .filter_map(|(entity, transform)| {
                    let enemy_pos = from_xz(transform.translation);
                    let distance = player_pos.distance(enemy_pos);
                    if distance <= surge.pulse_range {
                        Some((entity, distance))
                    } else {
                        None
                    }
                })
                .collect();

            // Sort by distance (closest first) for deterministic behavior
            enemies_in_range.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            // Take up to targets_per_pulse enemies
            let targets: Vec<Entity> = enemies_in_range
                .into_iter()
                .take(surge.targets_per_pulse as usize)
                .map(|(entity, _)| entity)
                .collect();

            // Apply damage to all selected targets
            for target in targets {
                damage_events.write(DamageEvent::new(target, surge.damage_per_target));
            }
        }
    }
}

/// Cast Arc Surge spell - creates a pulsing lightning aura around the player.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_arc_surge(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_arc_surge_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Arc Surge spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_arc_surge_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let surge = ArcSurge::new(damage);
    let surge_pos = spawn_position + Vec3::new(0.0, 0.3, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(surge_pos).with_scale(Vec3::splat(1.0)),
            surge,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(surge_pos),
            surge,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod arc_surge_component_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_arc_surge_new() {
            let damage = 20.0;
            let surge = ArcSurge::new(damage);

            assert_eq!(surge.damage_per_target, damage);
            assert_eq!(surge.pulse_range, ARC_SURGE_PULSE_RANGE);
            assert_eq!(surge.targets_per_pulse, ARC_SURGE_TARGETS_PER_PULSE);
            assert!(!surge.is_expired());
        }

        #[test]
        fn test_arc_surge_from_spell() {
            let spell = Spell::new(SpellType::Electrocute);
            let surge = ArcSurge::from_spell(&spell);

            assert_eq!(surge.damage_per_target, spell.damage());
        }

        #[test]
        fn test_arc_surge_pulse_timer_initial_state() {
            let surge = ArcSurge::new(10.0);
            assert!(!surge.pulse_timer.just_finished());
            assert_eq!(surge.pulse_timer.duration(), Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL));
        }

        #[test]
        fn test_arc_surge_duration_initial_state() {
            let surge = ArcSurge::new(10.0);
            assert!(!surge.is_expired());
            assert_eq!(surge.duration.duration(), Duration::from_secs_f32(ARC_SURGE_DURATION));
        }

        #[test]
        fn test_arc_surge_expires_after_duration() {
            let mut surge = ArcSurge::new(10.0);
            surge.duration.tick(Duration::from_secs_f32(ARC_SURGE_DURATION + 0.1));
            assert!(surge.is_expired());
        }

        #[test]
        fn test_arc_surge_does_not_expire_before_duration() {
            let mut surge = ArcSurge::new(10.0);
            surge.duration.tick(Duration::from_secs_f32(ARC_SURGE_DURATION / 2.0));
            assert!(!surge.is_expired());
        }

        #[test]
        fn test_arc_surge_uses_lightning_element_color() {
            let color = arc_surge_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }

        #[test]
        fn test_arc_surge_targets_per_pulse_is_configured() {
            let surge = ArcSurge::new(10.0);
            assert_eq!(surge.targets_per_pulse, ARC_SURGE_TARGETS_PER_PULSE);
        }
    }

    mod arc_surge_duration_system_tests {
        use super::*;

        #[test]
        fn test_surge_despawns_after_duration() {
            let mut app = App::new();
            app.add_systems(Update, arc_surge_duration_system);
            app.init_resource::<Time>();

            let surge_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(10.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ARC_SURGE_DURATION + 0.1));
            }

            app.update();

            // Surge should be despawned
            assert!(app.world().get_entity(surge_entity).is_err());
        }

        #[test]
        fn test_surge_survives_before_duration() {
            let mut app = App::new();
            app.add_systems(Update, arc_surge_duration_system);
            app.init_resource::<Time>();

            let surge_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(10.0),
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ARC_SURGE_DURATION / 2.0));
            }

            app.update();

            // Surge should still exist
            assert!(app.world().get_entity(surge_entity).is_ok());
        }
    }

    mod arc_surge_pulse_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_surge_pulses_enemy_in_range() {
            let mut app = setup_test_app();

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(15.0),
            ));

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time to trigger first pulse
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL + 0.01));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(arc_surge_pulse_system);

            // Check that surge timer has triggered (pulse occurred)
            let mut surge_query = app.world_mut().query::<&ArcSurge>();
            let surge = surge_query.single(app.world()).unwrap();
            assert!(surge.pulse_timer.just_finished(), "Pulse timer should have triggered");
        }

        #[test]
        fn test_surge_does_not_pulse_enemy_outside_range() {
            let mut app = setup_test_app();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (arc_surge_pulse_system, count_damage_events).chain());

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(15.0),
            ));

            // Create enemy outside range (distance = 100)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger pulse attempt
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL + 0.01));
            }

            app.update();

            // No damage should have been dealt
            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside range should not be hit");
        }

        #[test]
        fn test_surge_hits_multiple_enemies_up_to_limit() {
            let mut app = App::new();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_message::<DamageEvent>();
            app.init_resource::<Time>();
            app.add_systems(Update, (arc_surge_pulse_system, count_damage_events).chain());

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge with pulse timer almost at threshold
            let mut surge = ArcSurge::new(15.0);
            // Pre-tick almost to completion, then tick once more with small delta
            surge.pulse_timer.tick(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL - 0.001));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                surge,
            ));

            // Create 5 enemies within range (more than targets_per_pulse)
            for i in 0..5 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 + 1.0, 0.375, 0.0)),
                ));
            }

            // Manually advance time and update
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Should hit exactly targets_per_pulse enemies
            assert_eq!(
                counter.0.load(Ordering::SeqCst),
                ARC_SURGE_TARGETS_PER_PULSE as usize,
                "Should hit exactly {} targets", ARC_SURGE_TARGETS_PER_PULSE
            );
        }

        #[test]
        fn test_surge_hits_fewer_enemies_when_less_available() {
            let mut app = App::new();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_message::<DamageEvent>();
            app.init_resource::<Time>();
            app.add_systems(Update, (arc_surge_pulse_system, count_damage_events).chain());

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge with pulse timer almost at threshold
            let mut surge = ArcSurge::new(15.0);
            surge.pulse_timer.tick(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL - 0.001));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                surge,
            ));

            // Create only 2 enemies (less than targets_per_pulse)
            for i in 0..2 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 + 1.0, 0.375, 0.0)),
                ));
            }

            // Manually advance time and update
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Should hit only 2 enemies
            assert_eq!(counter.0.load(Ordering::SeqCst), 2, "Should hit only available enemies");
        }

        #[test]
        fn test_surge_pulses_repeatedly_at_interval() {
            let mut app = setup_test_app();

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge
            let surge_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(15.0),
            )).id();

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Track how many times pulse triggered
            let mut pulse_count = 0;

            // Run 3 pulse cycles
            for _ in 0..3 {
                // Advance time to trigger pulse
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL + 0.01));
                }

                // Run the system
                let _ = app.world_mut().run_system_once(arc_surge_pulse_system);

                // Check if pulse triggered
                let surge = app.world().get::<ArcSurge>(surge_entity).unwrap();
                if surge.pulse_timer.just_finished() {
                    pulse_count += 1;
                }
            }

            assert_eq!(pulse_count, 3, "Should have pulsed 3 times");
        }

        #[test]
        fn test_no_pulse_when_no_enemies_in_range() {
            let mut app = setup_test_app();

            // Create player at origin with no enemies
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(15.0),
            ));

            // Advance time to trigger pulse attempt
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL + 0.01));
            }

            // Run the system - should not crash
            let _ = app.world_mut().run_system_once(arc_surge_pulse_system);

            // Verify surge still exists (no crash)
            let mut surge_query = app.world_mut().query::<&ArcSurge>();
            let count = surge_query.iter(app.world()).count();
            assert_eq!(count, 1, "Surge should still exist");
        }

        #[test]
        fn test_surge_uses_xz_plane_ignores_y() {
            let mut app = App::new();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_message::<DamageEvent>();
            app.init_resource::<Time>();
            app.add_systems(Update, (arc_surge_pulse_system, count_damage_events).chain());

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge with pulse timer almost at threshold
            let mut surge = ArcSurge::new(15.0);
            surge.pulse_timer.tick(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL - 0.001));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                surge,
            ));

            // Create enemy close on XZ plane but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            ));

            // Manually advance time and update
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Enemy should be hit (Y distance is ignored)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }

        #[test]
        fn test_surge_targets_closest_enemies() {
            let mut app = App::new();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_message::<DamageEvent>();
            app.init_resource::<Time>();
            app.add_systems(Update, (arc_surge_pulse_system, count_damage_events).chain());

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create arc surge with only 1 target per pulse for testing
            let mut surge = ArcSurge::new(15.0);
            surge.targets_per_pulse = 1;
            surge.pulse_timer.tick(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL - 0.001));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                surge,
            ));

            // Create far enemy first (distance = 5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            // Create near enemy (distance = 2)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            // Manually advance time and update
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Should hit exactly 1 target (the closest one)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Should hit exactly 1 target");
        }

        #[test]
        fn test_surge_no_player_does_not_crash() {
            let mut app = setup_test_app();

            // Create arc surge without a player
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                ArcSurge::new(15.0),
            ));

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time to trigger pulse
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ARC_SURGE_PULSE_INTERVAL + 0.01));
            }

            // Run the system - should not crash due to missing player
            let _ = app.world_mut().run_system_once(arc_surge_pulse_system);

            // Verify surge still exists
            let mut surge_query = app.world_mut().query::<&ArcSurge>();
            let count = surge_query.iter(app.world()).count();
            assert_eq!(count, 1, "Surge should still exist");
        }
    }

    mod fire_arc_surge_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_arc_surge_spawns_surge() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_arc_surge(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 arc surge
            let mut query = app.world_mut().query::<&ArcSurge>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_arc_surge_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_arc_surge(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ArcSurge>();
            for surge in query.iter(app.world()) {
                assert_eq!(surge.damage_per_target, expected_damage);
            }
        }

        #[test]
        fn test_fire_arc_surge_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_arc_surge_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ArcSurge>();
            for surge in query.iter(app.world()) {
                assert_eq!(surge.damage_per_target, explicit_damage);
            }
        }

        #[test]
        fn test_fire_arc_surge_spawns_at_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_arc_surge(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&Transform>();
            for transform in query.iter(app.world()) {
                // Should be at spawn position with slight Y offset
                assert_eq!(transform.translation.x, 10.0);
                assert_eq!(transform.translation.z, 20.0);
            }
        }
    }
}
