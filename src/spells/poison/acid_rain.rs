//! Acid Rain spell - Poison element area denial with falling toxic droplets.
//!
//! Toxic droplets rain down over a wide area dealing damage and applying poison
//! to enemies. Creates a targeted zone where acid droplets spawn from above.
//! Each droplet damages enemies on contact and applies poison DOT.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::poison::venom_spray::PoisonStack;

/// Default configuration for Acid Rain spell
pub const ACID_RAIN_ZONE_RADIUS: f32 = 4.5;
pub const ACID_RAIN_ZONE_DURATION: f32 = 5.0;
pub const ACID_RAIN_SPAWN_RATE: f32 = 0.12; // Seconds between droplet spawns
pub const ACID_RAIN_SPAWN_HEIGHT: f32 = 6.0; // Height above zone where droplets spawn
pub const ACID_RAIN_FALL_SPEED: f32 = 10.0;
pub const ACID_RAIN_DAMAGE_RATIO: f32 = 0.15; // 15% of spell damage per droplet hit
pub const ACID_RAIN_DROPLET_VISUAL_SCALE: f32 = 0.15;
pub const ACID_RAIN_GROUND_LEVEL: f32 = 0.0;
pub const ACID_RAIN_SPAWN_DISTANCE: f32 = 6.0; // Distance ahead of player
pub const ACID_RAIN_POISON_DURATION: f32 = 3.0; // Duration of poison DOT
pub const ACID_RAIN_POISON_DAMAGE: f32 = 2.0; // Poison damage per tick

/// Get the poison element color for visual effects
pub fn acid_rain_color() -> Color {
    Element::Poison.color()
}

/// The Acid Rain zone that spawns falling toxic droplets.
#[derive(Component, Debug, Clone)]
pub struct AcidRainZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the acid rain zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Timer between droplet spawns
    pub spawn_timer: Timer,
    /// Damage per droplet hit
    pub damage_per_droplet: f32,
}

impl AcidRainZone {
    pub fn new(center: Vec2, damage: f32) -> Self {
        let damage_per_droplet = damage * ACID_RAIN_DAMAGE_RATIO;
        Self {
            center,
            radius: ACID_RAIN_ZONE_RADIUS,
            duration: Timer::from_seconds(ACID_RAIN_ZONE_DURATION, TimerMode::Once),
            spawn_timer: Timer::from_seconds(ACID_RAIN_SPAWN_RATE, TimerMode::Repeating),
            damage_per_droplet,
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = Timer::from_seconds(duration, TimerMode::Once);
        self
    }

    /// Check if the zone has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick both timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.spawn_timer.tick(delta);
    }

    /// Check if ready to spawn a new droplet
    pub fn should_spawn_droplet(&self) -> bool {
        self.spawn_timer.just_finished()
    }
}

/// Individual falling acid droplet.
#[derive(Component, Debug, Clone)]
pub struct AcidDroplet {
    /// Falling speed in units per second
    pub fall_speed: f32,
    /// Damage dealt on contact with enemy
    pub damage: f32,
    /// Parent zone entity for reference
    pub zone: Entity,
    /// Poison duration to apply on hit
    pub poison_duration: f32,
    /// Poison damage per tick
    pub poison_damage: f32,
}

impl AcidDroplet {
    pub fn new(damage: f32, zone: Entity) -> Self {
        Self {
            fall_speed: ACID_RAIN_FALL_SPEED,
            damage,
            zone,
            poison_duration: ACID_RAIN_POISON_DURATION,
            poison_damage: ACID_RAIN_POISON_DAMAGE,
        }
    }
}

/// Spawns an Acid Rain zone at a position ahead of the player.
pub fn spawn_acid_rain_zone(
    commands: &mut Commands,
    spell: &Spell,
    origin_pos: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    spawn_acid_rain_zone_with_damage(
        commands,
        spell.damage(),
        origin_pos,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Spawns an Acid Rain zone with explicit damage value.
pub fn spawn_acid_rain_zone_with_damage(
    commands: &mut Commands,
    damage: f32,
    origin_pos: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let origin_xz = from_xz(origin_pos);
    let direction = (target_pos - origin_xz).normalize_or_zero();
    let zone_center = origin_xz + direction * ACID_RAIN_SPAWN_DISTANCE;

    let zone = AcidRainZone::new(zone_center, damage);
    let zone_pos = Vec3::new(zone_center.x, 0.1, zone_center.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.poison_cloud.clone()),
            Transform::from_translation(zone_pos).with_scale(Vec3::splat(zone.radius * 0.5)),
            zone,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(zone_pos),
            zone,
        ));
    }
}

/// System that ticks zone timers and spawns droplets.
pub fn acid_rain_spawn_droplets_system(
    mut commands: Commands,
    time: Res<Time>,
    mut zone_query: Query<(Entity, &mut AcidRainZone, &Transform)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (zone_entity, mut zone, _zone_transform) in zone_query.iter_mut() {
        zone.tick(time.delta());

        if zone.should_spawn_droplet() {
            // Spawn droplet at random position within zone radius
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let distance = rand::random::<f32>().sqrt() * zone.radius;
            let offset = Vec2::new(angle.cos() * distance, angle.sin() * distance);
            let droplet_xz = zone.center + offset;

            let droplet_pos = Vec3::new(droplet_xz.x, ACID_RAIN_SPAWN_HEIGHT, droplet_xz.y);

            let droplet = AcidDroplet::new(zone.damage_per_droplet, zone_entity);

            if let (Some(meshes), Some(materials)) = (game_meshes.as_deref(), game_materials.as_deref()) {
                commands.spawn((
                    Mesh3d(meshes.bullet.clone()),
                    MeshMaterial3d(materials.poison_cloud.clone()),
                    Transform::from_translation(droplet_pos).with_scale(Vec3::splat(ACID_RAIN_DROPLET_VISUAL_SCALE)),
                    droplet,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(droplet_pos),
                    droplet,
                ));
            }
        }
    }
}

/// System that moves falling droplets downward.
pub fn acid_rain_move_droplets_system(
    time: Res<Time>,
    mut droplet_query: Query<(&AcidDroplet, &mut Transform)>,
) {
    for (droplet, mut transform) in droplet_query.iter_mut() {
        transform.translation.y -= droplet.fall_speed * time.delta_secs();
    }
}

/// System that checks for droplet collisions with enemies.
pub fn acid_rain_droplet_collision_system(
    mut commands: Commands,
    droplet_query: Query<(Entity, &AcidDroplet, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut poison_query: Query<&mut PoisonStack>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (droplet_entity, droplet, droplet_transform) in droplet_query.iter() {
        let droplet_pos = from_xz(droplet_transform.translation);
        let droplet_y = droplet_transform.translation.y;

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            // Enemy hitbox height check - droplets pass through at enemy level
            let enemy_height = 0.75; // Approximate enemy height
            let enemy_y = enemy_transform.translation.y;

            // Check if droplet is at enemy height level
            if droplet_y > enemy_y && droplet_y < enemy_y + enemy_height {
                // Check XZ distance (using small collision radius)
                let distance = droplet_pos.distance(enemy_pos);
                if distance < 1.0 {
                    // Deal direct damage
                    damage_events.write(DamageEvent::with_element(
                        enemy_entity,
                        droplet.damage,
                        Element::Poison,
                    ));

                    // Apply or refresh poison stack
                    if let Ok(mut existing_stack) = poison_query.get_mut(enemy_entity) {
                        existing_stack.add_stack();
                    } else {
                        commands.entity(enemy_entity).insert(PoisonStack::new());
                    }

                    // Despawn droplet on hit
                    commands.entity(droplet_entity).despawn();
                    break;
                }
            }
        }
    }
}

/// System that despawns droplets that reach ground level.
pub fn acid_rain_cleanup_droplets_system(
    mut commands: Commands,
    droplet_query: Query<(Entity, &Transform), With<AcidDroplet>>,
) {
    for (entity, transform) in droplet_query.iter() {
        if transform.translation.y <= ACID_RAIN_GROUND_LEVEL {
            commands.entity(entity).despawn();
        }
    }
}

/// System that despawns expired Acid Rain zones.
pub fn acid_rain_cleanup_zone_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &AcidRainZone)>,
) {
    for (entity, zone) in zone_query.iter() {
        if zone.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod acid_rain_zone_component_tests {
        use super::*;

        #[test]
        fn test_acid_rain_zone_new() {
            let center = Vec2::new(5.0, 10.0);
            let zone = AcidRainZone::new(center, 30.0);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, ACID_RAIN_ZONE_RADIUS);
            assert_eq!(zone.damage_per_droplet, 30.0 * ACID_RAIN_DAMAGE_RATIO);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_acid_rain_zone_with_radius() {
            let zone = AcidRainZone::new(Vec2::ZERO, 30.0).with_radius(10.0);
            assert_eq!(zone.radius, 10.0);
        }

        #[test]
        fn test_acid_rain_zone_with_duration() {
            let zone = AcidRainZone::new(Vec2::ZERO, 30.0).with_duration(10.0);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_acid_rain_zone_is_expired() {
            let mut zone = AcidRainZone::new(Vec2::ZERO, 30.0);
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(ACID_RAIN_ZONE_DURATION + 0.1));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_acid_rain_zone_should_spawn_droplet() {
            let mut zone = AcidRainZone::new(Vec2::ZERO, 30.0);
            assert!(!zone.should_spawn_droplet());

            zone.tick(Duration::from_secs_f32(ACID_RAIN_SPAWN_RATE + 0.01));
            assert!(zone.should_spawn_droplet());
        }

        #[test]
        fn test_acid_rain_uses_poison_element_color() {
            let color = acid_rain_color();
            assert_eq!(color, Element::Poison.color());
        }
    }

    mod acid_droplet_tests {
        use super::*;

        #[test]
        fn test_acid_droplet_new() {
            let zone_entity = Entity::from_bits(1);
            let droplet = AcidDroplet::new(15.0, zone_entity);

            assert_eq!(droplet.fall_speed, ACID_RAIN_FALL_SPEED);
            assert_eq!(droplet.damage, 15.0);
            assert_eq!(droplet.zone, zone_entity);
            assert_eq!(droplet.poison_duration, ACID_RAIN_POISON_DURATION);
            assert_eq!(droplet.poison_damage, ACID_RAIN_POISON_DAMAGE);
        }
    }

    mod spawn_acid_rain_zone_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_acid_rain_zone_spawns_ahead_of_player() {
            let mut app = setup_test_app();
            let origin_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target to the right

            {
                let mut commands = app.world_mut().commands();
                spawn_acid_rain_zone_with_damage(
                    &mut commands,
                    30.0,
                    origin_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&AcidRainZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 1, "One zone should spawn");

            for zone in zone_query.iter(app.world()) {
                // Zone should be ahead of origin in direction of target
                assert!(zone.center.x > 0.0, "Zone should be ahead in X direction");
                // Zone should be approximately ACID_RAIN_SPAWN_DISTANCE away
                let distance = zone.center.distance(Vec2::ZERO);
                assert!(
                    (distance - ACID_RAIN_SPAWN_DISTANCE).abs() < 0.5,
                    "Zone should be at spawn distance, got {}",
                    distance
                );
            }
        }

        #[test]
        fn test_acid_rain_zone_has_correct_radius() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                spawn_acid_rain_zone_with_damage(
                    &mut commands,
                    30.0,
                    Vec3::ZERO,
                    Vec2::new(10.0, 0.0),
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&AcidRainZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.radius, ACID_RAIN_ZONE_RADIUS);
            }
        }

        #[test]
        fn test_spawn_acid_rain_zone_uses_spell_damage() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::CorrosivePool);
            let expected_droplet_damage = spell.damage() * ACID_RAIN_DAMAGE_RATIO;

            {
                let mut commands = app.world_mut().commands();
                spawn_acid_rain_zone(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    Vec2::new(10.0, 0.0),
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&AcidRainZone>();
            for zone in zone_query.iter(app.world()) {
                assert!(
                    (zone.damage_per_droplet - expected_droplet_damage).abs() < 0.01,
                    "Expected droplet damage {}, got {}",
                    expected_droplet_damage,
                    zone.damage_per_droplet
                );
            }
        }
    }

    mod droplet_spawn_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_spawn_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_acid_droplets_spawn_in_zone() {
            let mut app = setup_spawn_test_app();

            // Create zone at origin with large radius for easy testing
            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AcidRainZone::new(Vec2::ZERO, 30.0).with_radius(10.0),
            )).id();

            // Advance time to trigger spawn
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ACID_RAIN_SPAWN_RATE + 0.01));
            }

            let _ = app.world_mut().run_system_once(acid_rain_spawn_droplets_system);

            // Should have spawned a droplet
            let mut droplet_query = app.world_mut().query::<(&AcidDroplet, &Transform)>();
            let count = droplet_query.iter(app.world()).count();
            assert!(count >= 1, "Should spawn at least one droplet");

            for (droplet, transform) in droplet_query.iter(app.world()) {
                // Verify droplet references correct zone
                assert_eq!(droplet.zone, zone_entity);

                // Verify droplet spawns at correct height
                assert!(
                    (transform.translation.y - ACID_RAIN_SPAWN_HEIGHT).abs() < 0.1,
                    "Droplet should spawn at height {}, got {}",
                    ACID_RAIN_SPAWN_HEIGHT,
                    transform.translation.y
                );

                // Verify droplet is within zone radius
                let droplet_xz = from_xz(transform.translation);
                assert!(
                    droplet_xz.length() <= 10.0,
                    "Droplet should spawn within zone radius"
                );
            }
        }

        #[test]
        fn test_acid_rain_droplet_spawn_rate() {
            let mut app = setup_spawn_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AcidRainZone::new(Vec2::ZERO, 30.0),
            ));

            // Run multiple spawn cycles
            for _ in 0..3 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(ACID_RAIN_SPAWN_RATE + 0.01));
                }
                let _ = app.world_mut().run_system_once(acid_rain_spawn_droplets_system);
            }

            let mut droplet_query = app.world_mut().query::<&AcidDroplet>();
            let count = droplet_query.iter(app.world()).count();
            // Should have spawned 3 droplets (one per cycle)
            assert!(count >= 3, "Should spawn droplets at configured rate, got {}", count);
        }

        #[test]
        fn test_acid_rain_droplet_random_positions() {
            let mut app = setup_spawn_test_app();

            // Create zone at origin with small radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AcidRainZone::new(Vec2::ZERO, 30.0).with_radius(5.0),
            ));

            // Spawn several droplets
            for _ in 0..10 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(ACID_RAIN_SPAWN_RATE + 0.01));
                }
                let _ = app.world_mut().run_system_once(acid_rain_spawn_droplets_system);
            }

            let mut droplet_query = app.world_mut().query::<&Transform>();
            let positions: Vec<Vec2> = droplet_query
                .iter(app.world())
                .map(|t| from_xz(t.translation))
                .collect();

            // All positions should be within radius
            for pos in &positions {
                assert!(pos.length() <= 5.0, "All droplets should be within zone radius");
            }
        }

        #[test]
        fn test_acid_rain_respects_radius() {
            let mut app = setup_spawn_test_app();
            let radius = 3.0;

            // Create zone with specific radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AcidRainZone::new(Vec2::ZERO, 30.0).with_radius(radius),
            ));

            // Spawn several droplets
            for _ in 0..20 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(ACID_RAIN_SPAWN_RATE + 0.01));
                }
                let _ = app.world_mut().run_system_once(acid_rain_spawn_droplets_system);
            }

            let mut droplet_query = app.world_mut().query::<&Transform>();
            for transform in droplet_query.iter(app.world()) {
                let droplet_xz = from_xz(transform.translation);
                assert!(
                    droplet_xz.length() <= radius,
                    "Droplet at {:?} exceeds zone radius {}",
                    droplet_xz,
                    radius
                );
            }
        }
    }

    mod droplet_movement_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_acid_droplets_move_downward() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let zone_entity = Entity::from_bits(1);
            let initial_y = ACID_RAIN_SPAWN_HEIGHT;

            let droplet_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, initial_y, 0.0)),
                AcidDroplet::new(15.0, zone_entity),
            )).id();

            // Advance time
            let delta_time = 0.5;
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(delta_time));
            }

            let _ = app.world_mut().run_system_once(acid_rain_move_droplets_system);

            let transform = app.world().get::<Transform>(droplet_entity).unwrap();
            let expected_y = initial_y - ACID_RAIN_FALL_SPEED * delta_time;
            assert!(
                (transform.translation.y - expected_y).abs() < 0.1,
                "Droplet should move down. Expected y={}, got y={}",
                expected_y,
                transform.translation.y
            );
        }
    }

    mod droplet_collision_tests {
        use super::*;
        use bevy::app::App;

        fn setup_collision_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_acid_droplet_damages_enemy() {
            let mut app = setup_collision_test_app();
            app.add_systems(Update, acid_rain_droplet_collision_system);

            let zone_entity = Entity::from_bits(1);
            let droplet_damage = 15.0;

            // Create droplet at enemy level
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)), // At enemy height
                AcidDroplet::new(droplet_damage, zone_entity),
            ));

            // Create enemy at same position
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            app.update();

            // Droplet should be despawned
            let mut droplet_query = app.world_mut().query::<&AcidDroplet>();
            let count = droplet_query.iter(app.world()).count();
            assert_eq!(count, 0, "Droplet should despawn after hitting enemy");
        }

        #[test]
        fn test_acid_droplet_applies_poison() {
            let mut app = setup_collision_test_app();
            app.add_systems(Update, acid_rain_droplet_collision_system);

            let zone_entity = Entity::from_bits(1);

            // Create droplet at enemy level
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)),
                AcidDroplet::new(15.0, zone_entity),
            ));

            // Create enemy at same position
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should have PoisonStack
            let stack = app.world().get::<PoisonStack>(enemy_entity);
            assert!(stack.is_some(), "Enemy should have PoisonStack after being hit");
        }

        #[test]
        fn test_acid_droplet_despawns_at_ground() {
            let mut app = App::new();
            app.add_systems(Update, acid_rain_cleanup_droplets_system);

            let zone_entity = Entity::from_bits(1);

            // Create droplet at ground level
            let droplet_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, ACID_RAIN_GROUND_LEVEL, 0.0)),
                AcidDroplet::new(15.0, zone_entity),
            )).id();

            app.update();

            // Droplet should be despawned
            assert!(
                app.world().get_entity(droplet_entity).is_err(),
                "Droplet should despawn at ground level"
            );
        }

        #[test]
        fn test_droplet_no_damage_when_above_enemy() {
            let mut app = setup_collision_test_app();
            app.add_systems(Update, acid_rain_droplet_collision_system);

            let zone_entity = Entity::from_bits(1);

            // Create droplet high above enemy
            let droplet_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
                AcidDroplet::new(15.0, zone_entity),
            )).id();

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            app.update();

            // Droplet should still exist (no collision)
            assert!(
                app.world().get_entity(droplet_entity).is_ok(),
                "Droplet should not hit enemy when above it"
            );
        }
    }

    mod zone_cleanup_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_acid_rain_zone_expires() {
            let mut app = App::new();

            let mut zone = AcidRainZone::new(Vec2::ZERO, 30.0);
            zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            zone.duration.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(acid_rain_cleanup_zone_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Zone should despawn after duration"
            );
        }

        #[test]
        fn test_acid_rain_zone_survives_before_expiry() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AcidRainZone::new(Vec2::ZERO, 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(acid_rain_cleanup_zone_system);

            assert!(
                app.world().get_entity(entity).is_ok(),
                "Zone should survive before expiry"
            );
        }

        #[test]
        fn test_acid_rain_despawns_after_duration() {
            let mut app = App::new();

            let mut zone = AcidRainZone::new(Vec2::ZERO, 30.0);
            zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            zone.duration.tick(Duration::from_secs(1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(acid_rain_cleanup_zone_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Zone should despawn when duration finished"
            );
        }
    }
}
