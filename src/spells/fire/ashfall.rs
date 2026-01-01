//! Ashfall spell - Fire element area denial with falling ember particles.
//!
//! Embers rain down over an area dealing sustained damage. Creates a targeted zone
//! ahead of the player where falling ember particles spawn from above. Embers damage
//! enemies on contact as they fall through the zone.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Ashfall spell
pub const ASHFALL_ZONE_RADIUS: f32 = 3.5;
pub const ASHFALL_ZONE_DURATION: f32 = 4.0;
pub const ASHFALL_SPAWN_RATE: f32 = 0.15; // Seconds between ember spawns
pub const ASHFALL_SPAWN_HEIGHT: f32 = 5.0; // Height above zone where embers spawn
pub const ASHFALL_FALL_SPEED: f32 = 8.0;
pub const ASHFALL_DAMAGE_RATIO: f32 = 0.2; // 20% of spell damage per ember hit
pub const ASHFALL_EMBER_VISUAL_SCALE: f32 = 0.2;
pub const ASHFALL_GROUND_LEVEL: f32 = 0.0;
pub const ASHFALL_SPAWN_DISTANCE: f32 = 6.0; // Distance ahead of player

/// Get the fire element color for visual effects
pub fn ashfall_color() -> Color {
    Element::Fire.color()
}

/// The Ashfall zone that spawns falling embers.
#[derive(Component, Debug, Clone)]
pub struct AshfallZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the ember rain zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Timer between ember spawns
    pub spawn_timer: Timer,
    /// Damage per ember hit
    pub damage_per_ember: f32,
}

impl AshfallZone {
    pub fn new(center: Vec2, damage: f32) -> Self {
        let damage_per_ember = damage * ASHFALL_DAMAGE_RATIO;
        Self {
            center,
            radius: ASHFALL_ZONE_RADIUS,
            duration: Timer::from_seconds(ASHFALL_ZONE_DURATION, TimerMode::Once),
            spawn_timer: Timer::from_seconds(ASHFALL_SPAWN_RATE, TimerMode::Repeating),
            damage_per_ember,
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

    /// Check if ready to spawn a new ember
    pub fn should_spawn_ember(&self) -> bool {
        self.spawn_timer.just_finished()
    }
}

/// Individual falling ember particle.
#[derive(Component, Debug, Clone)]
pub struct FallingEmber {
    /// Falling speed in units per second
    pub fall_speed: f32,
    /// Damage dealt on contact with enemy
    pub damage: f32,
    /// Parent zone entity for reference
    pub zone: Entity,
}

impl FallingEmber {
    pub fn new(damage: f32, zone: Entity) -> Self {
        Self {
            fall_speed: ASHFALL_FALL_SPEED,
            damage,
            zone,
        }
    }
}

/// Spawns an Ashfall zone at a position ahead of the player.
pub fn spawn_ashfall_zone(
    commands: &mut Commands,
    spell: &Spell,
    origin_pos: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    spawn_ashfall_zone_with_damage(
        commands,
        spell.damage(),
        origin_pos,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Spawns an Ashfall zone with explicit damage value.
pub fn spawn_ashfall_zone_with_damage(
    commands: &mut Commands,
    damage: f32,
    origin_pos: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let origin_xz = from_xz(origin_pos);
    let direction = (target_pos - origin_xz).normalize_or_zero();
    let zone_center = origin_xz + direction * ASHFALL_SPAWN_DISTANCE;

    let zone = AshfallZone::new(zone_center, damage);
    let zone_pos = Vec3::new(zone_center.x, 0.1, zone_center.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.fireball.clone()),
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

/// System that ticks zone timers and spawns embers.
pub fn ashfall_spawn_embers_system(
    mut commands: Commands,
    time: Res<Time>,
    mut zone_query: Query<(Entity, &mut AshfallZone, &Transform)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (zone_entity, mut zone, _zone_transform) in zone_query.iter_mut() {
        zone.tick(time.delta());

        if zone.should_spawn_ember() {
            // Spawn ember at random position within zone radius
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let distance = rand::random::<f32>().sqrt() * zone.radius;
            let offset = Vec2::new(angle.cos() * distance, angle.sin() * distance);
            let ember_xz = zone.center + offset;

            let ember_pos = Vec3::new(ember_xz.x, ASHFALL_SPAWN_HEIGHT, ember_xz.y);

            let ember = FallingEmber::new(zone.damage_per_ember, zone_entity);

            if let (Some(meshes), Some(materials)) = (game_meshes.as_deref(), game_materials.as_deref()) {
                commands.spawn((
                    Mesh3d(meshes.bullet.clone()),
                    MeshMaterial3d(materials.fireball.clone()),
                    Transform::from_translation(ember_pos).with_scale(Vec3::splat(ASHFALL_EMBER_VISUAL_SCALE)),
                    ember,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(ember_pos),
                    ember,
                ));
            }
        }
    }
}

/// System that moves falling embers downward.
pub fn ashfall_move_embers_system(
    time: Res<Time>,
    mut ember_query: Query<(&FallingEmber, &mut Transform)>,
) {
    for (ember, mut transform) in ember_query.iter_mut() {
        transform.translation.y -= ember.fall_speed * time.delta_secs();
    }
}

/// System that checks for ember collisions with enemies.
pub fn ashfall_ember_collision_system(
    mut commands: Commands,
    ember_query: Query<(Entity, &FallingEmber, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (ember_entity, ember, ember_transform) in ember_query.iter() {
        let ember_pos = from_xz(ember_transform.translation);
        let ember_y = ember_transform.translation.y;

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            // Enemy hitbox height check - embers pass through at enemy level
            let enemy_height = 0.75; // Approximate enemy height
            let enemy_y = enemy_transform.translation.y;

            // Check if ember is at enemy height level
            if ember_y > enemy_y && ember_y < enemy_y + enemy_height {
                // Check XZ distance (using small collision radius)
                let distance = ember_pos.distance(enemy_pos);
                if distance < 1.0 {
                    // Deal damage
                    damage_events.write(DamageEvent::with_element(
                        enemy_entity,
                        ember.damage,
                        Element::Fire,
                    ));

                    // Despawn ember on hit
                    commands.entity(ember_entity).despawn();
                    break;
                }
            }
        }
    }
}

/// System that despawns embers that reach ground level.
pub fn ashfall_cleanup_embers_system(
    mut commands: Commands,
    ember_query: Query<(Entity, &Transform), With<FallingEmber>>,
) {
    for (entity, transform) in ember_query.iter() {
        if transform.translation.y <= ASHFALL_GROUND_LEVEL {
            commands.entity(entity).despawn();
        }
    }
}

/// System that despawns expired Ashfall zones.
pub fn ashfall_cleanup_zone_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &AshfallZone)>,
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

    mod ashfall_zone_component_tests {
        use super::*;

        #[test]
        fn test_ashfall_zone_new() {
            let center = Vec2::new(5.0, 10.0);
            let zone = AshfallZone::new(center, 30.0);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, ASHFALL_ZONE_RADIUS);
            assert_eq!(zone.damage_per_ember, 30.0 * ASHFALL_DAMAGE_RATIO);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_ashfall_zone_with_radius() {
            let zone = AshfallZone::new(Vec2::ZERO, 30.0).with_radius(10.0);
            assert_eq!(zone.radius, 10.0);
        }

        #[test]
        fn test_ashfall_zone_with_duration() {
            let zone = AshfallZone::new(Vec2::ZERO, 30.0).with_duration(10.0);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_ashfall_zone_is_expired() {
            let mut zone = AshfallZone::new(Vec2::ZERO, 30.0);
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(ASHFALL_ZONE_DURATION + 0.1));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_ashfall_zone_should_spawn_ember() {
            let mut zone = AshfallZone::new(Vec2::ZERO, 30.0);
            assert!(!zone.should_spawn_ember());

            zone.tick(Duration::from_secs_f32(ASHFALL_SPAWN_RATE + 0.01));
            assert!(zone.should_spawn_ember());
        }

        #[test]
        fn test_ashfall_uses_fire_element_color() {
            let color = ashfall_color();
            assert_eq!(color, Element::Fire.color());
        }
    }

    mod falling_ember_tests {
        use super::*;

        #[test]
        fn test_falling_ember_new() {
            let zone_entity = Entity::from_bits(1);
            let ember = FallingEmber::new(15.0, zone_entity);

            assert_eq!(ember.fall_speed, ASHFALL_FALL_SPEED);
            assert_eq!(ember.damage, 15.0);
            assert_eq!(ember.zone, zone_entity);
        }
    }

    mod spawn_ashfall_zone_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_ashfall_zone_spawns_ahead_of_player() {
            let mut app = setup_test_app();
            let origin_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target to the right

            {
                let mut commands = app.world_mut().commands();
                spawn_ashfall_zone_with_damage(
                    &mut commands,
                    30.0,
                    origin_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&AshfallZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 1, "One zone should spawn");

            for zone in zone_query.iter(app.world()) {
                // Zone should be ahead of origin in direction of target
                assert!(zone.center.x > 0.0, "Zone should be ahead in X direction");
                // Zone should be approximately ASHFALL_SPAWN_DISTANCE away
                let distance = zone.center.distance(Vec2::ZERO);
                assert!(
                    (distance - ASHFALL_SPAWN_DISTANCE).abs() < 0.5,
                    "Zone should be at spawn distance, got {}",
                    distance
                );
            }
        }

        #[test]
        fn test_ashfall_zone_has_correct_radius() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                spawn_ashfall_zone_with_damage(
                    &mut commands,
                    30.0,
                    Vec3::ZERO,
                    Vec2::new(10.0, 0.0),
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&AshfallZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.radius, ASHFALL_ZONE_RADIUS);
            }
        }

        #[test]
        fn test_spawn_ashfall_zone_uses_spell_damage() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Fireball); // Using Fireball as placeholder
            let expected_ember_damage = spell.damage() * ASHFALL_DAMAGE_RATIO;

            {
                let mut commands = app.world_mut().commands();
                spawn_ashfall_zone(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    Vec2::new(10.0, 0.0),
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&AshfallZone>();
            for zone in zone_query.iter(app.world()) {
                assert!(
                    (zone.damage_per_ember - expected_ember_damage).abs() < 0.01,
                    "Expected ember damage {}, got {}",
                    expected_ember_damage,
                    zone.damage_per_ember
                );
            }
        }
    }

    mod ember_spawn_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_spawn_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_falling_embers_spawn_in_zone() {
            let mut app = setup_spawn_test_app();

            // Create zone at origin with large radius for easy testing
            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AshfallZone::new(Vec2::ZERO, 30.0).with_radius(10.0),
            )).id();

            // Advance time to trigger spawn
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ASHFALL_SPAWN_RATE + 0.01));
            }

            let _ = app.world_mut().run_system_once(ashfall_spawn_embers_system);

            // Should have spawned an ember
            let mut ember_query = app.world_mut().query::<(&FallingEmber, &Transform)>();
            let count = ember_query.iter(app.world()).count();
            assert!(count >= 1, "Should spawn at least one ember");

            for (ember, transform) in ember_query.iter(app.world()) {
                // Verify ember references correct zone
                assert_eq!(ember.zone, zone_entity);

                // Verify ember spawns at correct height
                assert!(
                    (transform.translation.y - ASHFALL_SPAWN_HEIGHT).abs() < 0.1,
                    "Ember should spawn at height {}, got {}",
                    ASHFALL_SPAWN_HEIGHT,
                    transform.translation.y
                );

                // Verify ember is within zone radius
                let ember_xz = from_xz(transform.translation);
                assert!(
                    ember_xz.length() <= 10.0,
                    "Ember should spawn within zone radius"
                );
            }
        }

        #[test]
        fn test_ashfall_ember_spawn_rate() {
            let mut app = setup_spawn_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AshfallZone::new(Vec2::ZERO, 30.0),
            ));

            // Run multiple spawn cycles
            for _ in 0..3 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(ASHFALL_SPAWN_RATE + 0.01));
                }
                let _ = app.world_mut().run_system_once(ashfall_spawn_embers_system);
            }

            let mut ember_query = app.world_mut().query::<&FallingEmber>();
            let count = ember_query.iter(app.world()).count();
            // Should have spawned 3 embers (one per cycle)
            assert!(count >= 3, "Should spawn embers at configured rate, got {}", count);
        }
    }

    mod ember_movement_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_falling_embers_move_downward() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let zone_entity = Entity::from_bits(1);
            let initial_y = ASHFALL_SPAWN_HEIGHT;

            let ember_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, initial_y, 0.0)),
                FallingEmber::new(15.0, zone_entity),
            )).id();

            // Advance time
            let delta_time = 0.5;
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(delta_time));
            }

            let _ = app.world_mut().run_system_once(ashfall_move_embers_system);

            let transform = app.world().get::<Transform>(ember_entity).unwrap();
            let expected_y = initial_y - ASHFALL_FALL_SPEED * delta_time;
            assert!(
                (transform.translation.y - expected_y).abs() < 0.1,
                "Ember should move down. Expected y={}, got y={}",
                expected_y,
                transform.translation.y
            );
        }
    }

    mod ember_collision_tests {
        use super::*;
        use bevy::app::App;

        fn setup_collision_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_falling_ember_damages_enemy() {
            let mut app = setup_collision_test_app();
            app.add_systems(Update, ashfall_ember_collision_system);

            let zone_entity = Entity::from_bits(1);
            let ember_damage = 15.0;

            // Create ember at enemy level
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)), // At enemy height
                FallingEmber::new(ember_damage, zone_entity),
            ));

            // Create enemy at same position
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            app.update();

            // Ember should be despawned
            let mut ember_query = app.world_mut().query::<&FallingEmber>();
            let count = ember_query.iter(app.world()).count();
            assert_eq!(count, 0, "Ember should despawn after hitting enemy");
        }

        #[test]
        fn test_falling_ember_despawns_at_ground() {
            let mut app = App::new();
            app.add_systems(Update, ashfall_cleanup_embers_system);

            let zone_entity = Entity::from_bits(1);

            // Create ember at ground level
            let ember_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, ASHFALL_GROUND_LEVEL, 0.0)),
                FallingEmber::new(15.0, zone_entity),
            )).id();

            app.update();

            // Ember should be despawned
            assert!(
                app.world().get_entity(ember_entity).is_err(),
                "Ember should despawn at ground level"
            );
        }

        #[test]
        fn test_ember_no_damage_when_above_enemy() {
            let mut app = setup_collision_test_app();
            app.add_systems(Update, ashfall_ember_collision_system);

            let zone_entity = Entity::from_bits(1);

            // Create ember high above enemy
            let ember_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
                FallingEmber::new(15.0, zone_entity),
            )).id();

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            app.update();

            // Ember should still exist (no collision)
            assert!(
                app.world().get_entity(ember_entity).is_ok(),
                "Ember should not hit enemy when above it"
            );
        }
    }

    mod zone_cleanup_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_ashfall_zone_expires() {
            let mut app = App::new();

            let mut zone = AshfallZone::new(Vec2::ZERO, 30.0);
            zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            zone.duration.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(ashfall_cleanup_zone_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Zone should despawn after duration"
            );
        }

        #[test]
        fn test_ashfall_zone_survives_before_expiry() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AshfallZone::new(Vec2::ZERO, 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(ashfall_cleanup_zone_system);

            assert!(
                app.world().get_entity(entity).is_ok(),
                "Zone should survive before expiry"
            );
        }
    }
}
