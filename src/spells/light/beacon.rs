//! Beacon spell (Light) - Light source that draws enemies while damaging them.
//!
//! Creates a bright beacon at a target location that acts as a taunt, attracting
//! enemies toward it. Enemies near the beacon take continuous damage.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;

/// Default radius within which enemies are attracted to the beacon
pub const BEACON_ATTRACT_RADIUS: f32 = 12.0;

/// Default radius within which enemies take damage
pub const BEACON_DAMAGE_RADIUS: f32 = 3.0;

/// Default damage tick interval in seconds
pub const BEACON_DAMAGE_INTERVAL: f32 = 0.5;

/// Default beacon duration in seconds
pub const BEACON_DURATION: f32 = 5.0;

/// Visual beacon height
pub const BEACON_HEIGHT: f32 = 5.0;

/// Visual beacon base radius
pub const BEACON_BASE_RADIUS: f32 = 1.5;

/// Get the light element color for visual effects (white/gold)
pub fn beacon_color() -> Color {
    Element::Light.color()
}

/// Beacon component - creates a light source that draws enemies while damaging them.
#[derive(Component, Debug, Clone)]
pub struct Beacon {
    /// Position on the XZ plane where the beacon is placed
    pub position: Vec2,
    /// Radius within which enemies are attracted toward the beacon
    pub attract_radius: f32,
    /// Radius within which enemies take damage
    pub damage_radius: f32,
    /// Damage dealt per tick to enemies within damage radius
    pub damage_per_tick: f32,
    /// Timer controlling damage tick frequency
    pub tick_timer: Timer,
    /// Timer controlling beacon lifetime
    pub duration: Timer,
}

impl Beacon {
    /// Creates a new Beacon at the given position with default values.
    pub fn new(position: Vec2, damage: f32) -> Self {
        Self {
            position,
            attract_radius: BEACON_ATTRACT_RADIUS,
            damage_radius: BEACON_DAMAGE_RADIUS,
            damage_per_tick: damage,
            tick_timer: Timer::from_seconds(BEACON_DAMAGE_INTERVAL, TimerMode::Repeating),
            duration: Timer::from_seconds(BEACON_DURATION, TimerMode::Once),
        }
    }

    /// Creates a Beacon from a Spell component.
    pub fn from_spell(position: Vec2, spell: &Spell) -> Self {
        Self::new(position, spell.damage())
    }

    /// Check if the beacon is still active (not expired)
    pub fn is_active(&self) -> bool {
        !self.duration.is_finished()
    }

    /// Check if the beacon should deal damage this frame
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }
}

/// Marker component for enemies currently attracted to a beacon.
/// Enemies with this component have their movement overridden to move toward the beacon.
#[derive(Component, Debug, Clone)]
pub struct BeaconAttracted {
    /// The entity ID of the beacon this enemy is attracted to
    pub beacon_entity: Entity,
}

/// System to update beacon timers and despawn expired beacons.
pub fn update_beacon_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut beacon_query: Query<(Entity, &mut Beacon)>,
) {
    for (entity, mut beacon) in beacon_query.iter_mut() {
        beacon.tick_timer.tick(time.delta());
        beacon.duration.tick(time.delta());

        if !beacon.is_active() {
            commands.entity(entity).despawn();
        }
    }
}

/// System to attract enemies toward beacons.
/// Adds BeaconAttracted component to enemies within attract_radius.
#[allow(clippy::type_complexity)]
pub fn attract_enemies_to_beacon(
    mut commands: Commands,
    beacon_query: Query<(Entity, &Beacon)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<BeaconAttracted>)>,
) {
    for (beacon_entity, beacon) in beacon_query.iter() {
        if !beacon.is_active() {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = beacon.position.distance(enemy_pos);

            if distance <= beacon.attract_radius {
                commands.entity(enemy_entity).insert(BeaconAttracted {
                    beacon_entity,
                });
            }
        }
    }
}

/// System to apply beacon damage to enemies within damage radius.
pub fn apply_beacon_damage(
    beacon_query: Query<&Beacon>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for beacon in beacon_query.iter() {
        if !beacon.is_active() || !beacon.should_damage() {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = beacon.position.distance(enemy_pos);

            if distance <= beacon.damage_radius {
                damage_events.write(DamageEvent::new(enemy_entity, beacon.damage_per_tick));
            }
        }
    }
}

/// System to remove BeaconAttracted from enemies when their beacon despawns.
pub fn remove_beacon_attraction(
    mut commands: Commands,
    beacon_query: Query<Entity, With<Beacon>>,
    attracted_query: Query<(Entity, &BeaconAttracted)>,
) {
    for (enemy_entity, attracted) in attracted_query.iter() {
        // Check if the beacon still exists
        if beacon_query.get(attracted.beacon_entity).is_err() {
            commands.entity(enemy_entity).remove::<BeaconAttracted>();
        }
    }
}

/// System to override enemy movement toward the beacon.
/// Enemies with BeaconAttracted component move toward the beacon's position.
pub fn beacon_movement_override(
    time: Res<Time>,
    beacon_query: Query<&Beacon>,
    mut enemy_query: Query<(&BeaconAttracted, &Enemy, &mut Transform)>,
) {
    for (attracted, enemy, mut transform) in enemy_query.iter_mut() {
        if let Ok(beacon) = beacon_query.get(attracted.beacon_entity) {
            let current_pos = from_xz(transform.translation);
            let direction = (beacon.position - current_pos).normalize_or_zero();

            // Move toward beacon at enemy's speed
            let movement = direction * enemy.speed * time.delta_secs();
            transform.translation.x += movement.x;
            transform.translation.z += movement.y;
        }
    }
}

/// Cast Beacon spell - spawns a beacon at the target location.
/// `spawn_position` is Whisper's full 3D position (used for height reference).
/// `target_pos` is the target location on the XZ plane.
#[allow(clippy::too_many_arguments)]
pub fn fire_beacon(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_beacon_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast Beacon spell with explicit damage.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_beacon_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let beacon = Beacon::new(target_pos, damage);
    let beacon_pos = to_xz(target_pos) + Vec3::new(0.0, BEACON_HEIGHT / 2.0, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        // Spawn beacon with visual representation
        commands.spawn((
            Mesh3d(meshes.laser.clone()),
            MeshMaterial3d(materials.radiant_beam.clone()),
            Transform::from_translation(beacon_pos)
                .with_scale(Vec3::new(BEACON_BASE_RADIUS, BEACON_HEIGHT, BEACON_BASE_RADIUS)),
            beacon,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(Vec3::new(target_pos.x, spawn_position.y, target_pos.y)),
            beacon,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod beacon_component_tests {
        use super::*;

        #[test]
        fn test_beacon_creation() {
            let position = Vec2::new(10.0, 20.0);
            let damage = 15.0;
            let beacon = Beacon::new(position, damage);

            assert_eq!(beacon.position, position);
            assert_eq!(beacon.attract_radius, BEACON_ATTRACT_RADIUS);
            assert_eq!(beacon.damage_radius, BEACON_DAMAGE_RADIUS);
            assert_eq!(beacon.damage_per_tick, damage);
            assert!(beacon.is_active());
        }

        #[test]
        fn test_beacon_from_spell() {
            let spell = Spell::new(SpellType::Consecration);
            let position = Vec2::new(5.0, 5.0);
            let beacon = Beacon::from_spell(position, &spell);

            assert_eq!(beacon.position, position);
            assert_eq!(beacon.damage_per_tick, spell.damage());
        }

        #[test]
        fn test_beacon_is_active_initially() {
            let beacon = Beacon::new(Vec2::ZERO, 10.0);
            assert!(beacon.is_active(), "New beacon should be active");
        }

        #[test]
        fn test_beacon_expires_after_duration() {
            let mut beacon = Beacon::new(Vec2::ZERO, 10.0);

            // Tick past duration
            beacon.duration.tick(Duration::from_secs_f32(BEACON_DURATION + 1.0));

            assert!(!beacon.is_active(), "Beacon should be inactive after duration");
        }

        #[test]
        fn test_beacon_should_damage_on_timer() {
            let mut beacon = Beacon::new(Vec2::ZERO, 10.0);

            // Not ready initially
            assert!(!beacon.should_damage());

            // Tick past interval
            beacon.tick_timer.tick(Duration::from_secs_f32(BEACON_DAMAGE_INTERVAL + 0.1));

            assert!(beacon.should_damage());
        }

        #[test]
        fn test_beacon_uses_light_element_color() {
            let color = beacon_color();
            assert_eq!(color, Element::Light.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 255)); // White
        }
    }

    mod beacon_attracted_tests {
        use super::*;

        #[test]
        fn test_beacon_attracted_stores_entity() {
            // Test that BeaconAttracted properly stores and retrieves the entity
            let mut app = App::new();

            let beacon_entity = app.world_mut().spawn_empty().id();
            let attracted = BeaconAttracted { beacon_entity };

            assert_eq!(attracted.beacon_entity, beacon_entity);
        }
    }

    mod update_beacon_timers_tests {
        use super::*;

        #[test]
        fn test_beacon_despawns_after_duration() {
            let mut app = App::new();
            app.add_systems(Update, update_beacon_timers);
            app.init_resource::<Time>();

            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BEACON_DURATION + 1.0));
            }

            app.update();

            // Beacon should be despawned
            assert!(app.world().get_entity(beacon_entity).is_err());
        }

        #[test]
        fn test_beacon_survives_before_duration() {
            let mut app = App::new();
            app.add_systems(Update, update_beacon_timers);
            app.init_resource::<Time>();

            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BEACON_DURATION / 2.0));
            }

            app.update();

            // Beacon should still exist
            assert!(app.world().get_entity(beacon_entity).is_ok());
        }
    }

    mod attract_enemies_tests {
        use super::*;

        #[test]
        fn test_enemy_within_attract_radius_gets_attracted() {
            let mut app = App::new();
            app.add_systems(Update, attract_enemies_to_beacon);

            // Create beacon at origin
            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            )).id();

            // Create enemy within attract radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should have BeaconAttracted component
            let attracted = app.world().get::<BeaconAttracted>(enemy_entity);
            assert!(attracted.is_some(), "Enemy within attract radius should be attracted");
            assert_eq!(attracted.unwrap().beacon_entity, beacon_entity);
        }

        #[test]
        fn test_enemy_outside_attract_radius_not_affected() {
            let mut app = App::new();
            app.add_systems(Update, attract_enemies_to_beacon);

            // Create beacon at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            ));

            // Create enemy outside attract radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should NOT have BeaconAttracted component
            let attracted = app.world().get::<BeaconAttracted>(enemy_entity);
            assert!(attracted.is_none(), "Enemy outside attract radius should not be attracted");
        }

        #[test]
        fn test_multiple_enemies_can_be_attracted() {
            let mut app = App::new();
            app.add_systems(Update, attract_enemies_to_beacon);

            // Create beacon at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            ));

            // Create multiple enemies within attract radius
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 5.0)),
            )).id();

            app.update();

            // Both enemies should be attracted
            assert!(app.world().get::<BeaconAttracted>(enemy1).is_some());
            assert!(app.world().get::<BeaconAttracted>(enemy2).is_some());
        }
    }

    mod apply_beacon_damage_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_enemy_within_damage_radius_takes_damage() {
            let mut app = App::new();

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
            app.add_systems(Update, (apply_beacon_damage, count_damage_events).chain());

            // Create beacon with ready tick timer
            let mut beacon = Beacon::new(Vec2::ZERO, 10.0);
            beacon.tick_timer.tick(Duration::from_secs_f32(BEACON_DAMAGE_INTERVAL + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                beacon,
            ));

            // Create enemy within damage radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy within damage radius should take damage");
        }

        #[test]
        fn test_enemy_outside_damage_radius_no_damage() {
            let mut app = App::new();

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
            app.add_systems(Update, (apply_beacon_damage, count_damage_events).chain());

            // Create beacon with ready tick timer
            let mut beacon = Beacon::new(Vec2::ZERO, 10.0);
            beacon.tick_timer.tick(Duration::from_secs_f32(BEACON_DAMAGE_INTERVAL + 0.1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                beacon,
            ));

            // Create enemy outside damage radius but within attract radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside damage radius should not take damage");
        }

        #[test]
        fn test_no_damage_before_tick_timer() {
            let mut app = App::new();

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
            app.add_systems(Update, (apply_beacon_damage, count_damage_events).chain());

            // Create beacon with fresh tick timer (not ready)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            ));

            // Create enemy within damage radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "No damage before tick timer fires");
        }
    }

    mod remove_beacon_attraction_tests {
        use super::*;

        #[test]
        fn test_attraction_removed_when_beacon_despawns() {
            let mut app = App::new();
            app.add_systems(Update, remove_beacon_attraction);

            // Create beacon entity that we'll despawn
            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            )).id();

            // Create enemy with attraction to that beacon
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::ZERO),
                BeaconAttracted { beacon_entity },
            )).id();

            // Despawn the beacon
            app.world_mut().despawn(beacon_entity);

            app.update();

            // Enemy should no longer have BeaconAttracted
            assert!(app.world().get::<BeaconAttracted>(enemy_entity).is_none());
        }

        #[test]
        fn test_attraction_remains_while_beacon_exists() {
            let mut app = App::new();
            app.add_systems(Update, remove_beacon_attraction);

            // Create beacon
            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Beacon::new(Vec2::ZERO, 10.0),
            )).id();

            // Create enemy with attraction
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::ZERO),
                BeaconAttracted { beacon_entity },
            )).id();

            app.update();

            // Enemy should still have BeaconAttracted
            assert!(app.world().get::<BeaconAttracted>(enemy_entity).is_some());
        }
    }

    mod beacon_movement_override_tests {
        use super::*;

        #[test]
        fn test_attracted_enemy_moves_toward_beacon() {
            let mut app = App::new();
            app.add_systems(Update, beacon_movement_override);
            app.init_resource::<Time>();

            // Create beacon at (10, 0)
            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
                Beacon::new(Vec2::new(10.0, 0.0), 10.0),
            )).id();

            // Create attracted enemy at origin
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::ZERO),
                BeaconAttracted { beacon_entity },
            )).id();

            // Get initial position
            let initial_x = app.world().get::<Transform>(enemy_entity).unwrap().translation.x;

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Enemy should have moved toward beacon (positive X)
            let final_x = app.world().get::<Transform>(enemy_entity).unwrap().translation.x;
            assert!(final_x > initial_x, "Enemy should move toward beacon (x increased from {} to {})", initial_x, final_x);
        }

        #[test]
        fn test_attracted_enemy_movement_uses_enemy_speed() {
            let mut app = App::new();
            app.add_systems(Update, beacon_movement_override);
            app.init_resource::<Time>();

            let enemy_speed = 100.0;
            let delta_time = 0.1;

            // Create beacon at (100, 0)
            let beacon_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
                Beacon::new(Vec2::new(100.0, 0.0), 10.0),
            )).id();

            // Create attracted enemy at origin
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: enemy_speed, strength: 10.0 },
                Transform::from_translation(Vec3::ZERO),
                BeaconAttracted { beacon_entity },
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(delta_time));
            }

            app.update();

            // Expected movement: speed * delta_time = 100 * 0.1 = 10 units
            let expected_x = enemy_speed * delta_time;
            let final_x = app.world().get::<Transform>(enemy_entity).unwrap().translation.x;

            assert!(
                (final_x - expected_x).abs() < 0.1,
                "Enemy should move {} units, moved {}",
                expected_x, final_x
            );
        }
    }

    mod fire_beacon_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_beacon_spawns_beacon() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Consecration);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_beacon(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 beacon
            let mut query = app.world_mut().query::<&Beacon>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_beacon_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Consecration);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(15.0, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_beacon(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&Beacon>();
            for beacon in query.iter(app.world()) {
                assert_eq!(beacon.position, target_pos);
            }
        }

        #[test]
        fn test_fire_beacon_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Consecration);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::ZERO;
            let target_pos = Vec2::ZERO;

            {
                let mut commands = app.world_mut().commands();
                fire_beacon(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&Beacon>();
            for beacon in query.iter(app.world()) {
                assert_eq!(beacon.damage_per_tick, expected_damage);
            }
        }

        #[test]
        fn test_fire_beacon_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Consecration);
            let explicit_damage = 999.0;
            let spawn_pos = Vec3::ZERO;
            let target_pos = Vec2::ZERO;

            {
                let mut commands = app.world_mut().commands();
                fire_beacon_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&Beacon>();
            for beacon in query.iter(app.world()) {
                assert_eq!(beacon.damage_per_tick, explicit_damage);
            }
        }
    }
}
