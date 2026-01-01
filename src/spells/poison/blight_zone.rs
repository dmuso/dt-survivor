//! Blight Zone spell - Area denial poison spell with damage over time.
//!
//! A Poison element spell (Blight SpellType) that creates a contaminated zone
//! at a target location. Enemies within the zone take poison damage over time.
//! The zone persists for a duration and then despawns.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Blight Zone spell
pub const BLIGHT_ZONE_RADIUS: f32 = 4.0;
pub const BLIGHT_ZONE_DURATION: f32 = 5.0;
pub const BLIGHT_ZONE_TICK_INTERVAL: f32 = 0.5;
pub const BLIGHT_ZONE_TICK_DAMAGE_RATIO: f32 = 0.15; // 15% of spell damage per tick

/// Get the poison element color for visual effects
pub fn blight_zone_color() -> Color {
    Element::Poison.color()
}

/// A contaminated zone that damages enemies over time.
/// Spawns at a target location and persists for a duration.
#[derive(Component, Debug, Clone)]
pub struct BlightZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the damage zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Damage per tick
    pub tick_damage: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Set of enemies damaged this tick (prevents double damage)
    pub hit_this_tick: HashSet<Entity>,
}

impl BlightZone {
    pub fn new(center: Vec2, damage: f32) -> Self {
        let tick_damage = damage * BLIGHT_ZONE_TICK_DAMAGE_RATIO;
        Self {
            center,
            radius: BLIGHT_ZONE_RADIUS,
            duration: Timer::from_seconds(BLIGHT_ZONE_DURATION, TimerMode::Once),
            tick_damage,
            tick_timer: Timer::from_seconds(BLIGHT_ZONE_TICK_INTERVAL, TimerMode::Repeating),
            hit_this_tick: HashSet::new(),
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
        self.tick_timer.tick(delta);

        // Reset hit tracking each tick
        if self.tick_timer.just_finished() {
            self.hit_this_tick.clear();
        }
    }

    /// Check if ready to apply damage
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }

    /// Check if an enemy is in range and hasn't been damaged this tick
    pub fn can_damage(&self, entity: Entity, enemy_pos: Vec2) -> bool {
        let distance = self.center.distance(enemy_pos);
        distance <= self.radius && !self.hit_this_tick.contains(&entity)
    }

    /// Mark an enemy as damaged this tick
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_this_tick.insert(entity);
    }
}

/// Spawns a blight zone at the target location when spell is cast.
pub fn spawn_blight_zone(
    commands: &mut Commands,
    spell: &Spell,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    spawn_blight_zone_with_damage(
        commands,
        spell.damage(),
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Spawns a blight zone with explicit damage value.
pub fn spawn_blight_zone_with_damage(
    commands: &mut Commands,
    damage: f32,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let zone = BlightZone::new(target_pos, damage);
    let zone_pos = Vec3::new(target_pos.x, 0.1, target_pos.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.poison_cloud.clone()),
            Transform::from_translation(zone_pos).with_scale(Vec3::splat(zone.radius)),
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

/// System that applies damage to enemies in blight zones.
pub fn blight_zone_damage_system(
    mut zone_query: Query<&mut BlightZone>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    time: Res<Time>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut zone in zone_query.iter_mut() {
        zone.tick(time.delta());

        if zone.should_damage() {
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);

                if zone.can_damage(enemy_entity, enemy_pos) {
                    damage_events.write(DamageEvent::with_element(
                        enemy_entity,
                        zone.tick_damage,
                        Element::Poison,
                    ));
                    zone.mark_hit(enemy_entity);
                }
            }
        }
    }
}

/// System that despawns expired blight zones.
pub fn blight_zone_cleanup_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &BlightZone)>,
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

    mod blight_zone_component_tests {
        use super::*;

        #[test]
        fn test_blight_zone_new() {
            let center = Vec2::new(5.0, 10.0);
            let zone = BlightZone::new(center, 30.0);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, BLIGHT_ZONE_RADIUS);
            assert_eq!(zone.tick_damage, 30.0 * BLIGHT_ZONE_TICK_DAMAGE_RATIO);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_blight_zone_with_radius() {
            let zone = BlightZone::new(Vec2::ZERO, 30.0).with_radius(10.0);
            assert_eq!(zone.radius, 10.0);
        }

        #[test]
        fn test_blight_zone_with_duration() {
            let zone = BlightZone::new(Vec2::ZERO, 30.0).with_duration(10.0);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_blight_zone_is_expired() {
            let mut zone = BlightZone::new(Vec2::ZERO, 30.0);
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(BLIGHT_ZONE_DURATION + 0.1));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_blight_zone_should_damage() {
            let mut zone = BlightZone::new(Vec2::ZERO, 30.0);
            assert!(!zone.should_damage());

            zone.tick(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            assert!(zone.should_damage());
        }

        #[test]
        fn test_blight_zone_can_damage_in_range() {
            let zone = BlightZone::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(2.0, 0.0); // Within default radius

            assert!(zone.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_blight_zone_no_damage_outside() {
            let zone = BlightZone::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let out_of_range_pos = Vec2::new(100.0, 0.0);

            assert!(!zone.can_damage(entity, out_of_range_pos));
        }

        #[test]
        fn test_blight_zone_cannot_damage_already_hit() {
            let mut zone = BlightZone::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(2.0, 0.0);

            zone.mark_hit(entity);
            assert!(!zone.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_blight_zone_resets_hit_tracking_on_tick() {
            let mut zone = BlightZone::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);

            zone.mark_hit(entity);
            assert!(zone.hit_this_tick.contains(&entity));

            zone.tick(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            assert!(zone.hit_this_tick.is_empty());
        }

        #[test]
        fn test_uses_poison_element_color() {
            let color = blight_zone_color();
            assert_eq!(color, Element::Poison.color());
        }
    }

    mod spawn_blight_zone_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_blight_zone_spawns_at_target() {
            let mut app = setup_test_app();
            let target_pos = Vec2::new(15.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_blight_zone_with_damage(
                    &mut commands,
                    30.0,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&BlightZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 1, "One zone should spawn");

            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.center, target_pos);
            }
        }

        #[test]
        fn test_spawn_blight_zone_uses_spell_damage() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Blight);
            let expected_tick_damage = spell.damage() * BLIGHT_ZONE_TICK_DAMAGE_RATIO;
            let target_pos = Vec2::new(5.0, 5.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_blight_zone(
                    &mut commands,
                    &spell,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&BlightZone>();
            for zone in zone_query.iter(app.world()) {
                assert!(
                    (zone.tick_damage - expected_tick_damage).abs() < 0.01,
                    "Expected tick damage {}, got {}",
                    expected_tick_damage,
                    zone.tick_damage
                );
            }
        }

        #[test]
        fn test_blight_zone_has_visible_radius_boundary() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                spawn_blight_zone_with_damage(
                    &mut commands,
                    30.0,
                    Vec2::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            // Verify the zone was created with a transform that could be rendered
            let mut query = app.world_mut().query::<(&BlightZone, &Transform)>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }
    }

    mod damage_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_damage_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_blight_zone_damages_enemies_inside() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Check that enemy was marked as hit
            let mut zone_query = app.world_mut().query::<&BlightZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert!(!zone.hit_this_tick.is_empty(), "Enemy should have been marked as hit");
        }

        #[test]
        fn test_blight_zone_no_damage_outside() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy far outside range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Check that no enemy was marked as hit
            let mut zone_query = app.world_mut().query::<&BlightZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert!(zone.hit_this_tick.is_empty(), "No enemy should have been hit");
        }

        #[test]
        fn test_blight_zone_damages_enemies_entering() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy outside zone initially
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            )).id();

            // First tick - enemy is outside
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }
            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Verify no damage initially
            {
                let mut zone_query = app.world_mut().query::<&BlightZone>();
                let zone = zone_query.single(app.world()).unwrap();
                assert!(zone.hit_this_tick.is_empty(), "Enemy should not be hit yet");
            }

            // Move enemy into zone
            {
                let mut transform = app.world_mut().get_mut::<Transform>(enemy).unwrap();
                transform.translation = Vec3::new(2.0, 0.375, 0.0);
            }

            // Second tick - enemy is inside
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }
            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Verify damage now
            let mut zone_query = app.world_mut().query::<&BlightZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert!(!zone.hit_this_tick.is_empty(), "Enemy should be hit after entering zone");
        }

        #[test]
        fn test_blight_zone_stops_damage_on_exit() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy inside zone
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            // First tick - enemy is inside
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }
            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Verify damage initially
            {
                let mut zone_query = app.world_mut().query::<&BlightZone>();
                let zone = zone_query.single(app.world()).unwrap();
                assert!(!zone.hit_this_tick.is_empty(), "Enemy should be hit while inside");
            }

            // Move enemy out of zone
            {
                let mut transform = app.world_mut().get_mut::<Transform>(enemy).unwrap();
                transform.translation = Vec3::new(100.0, 0.375, 0.0);
            }

            // Second tick - enemy is outside
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }
            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Verify no damage after exit
            let mut zone_query = app.world_mut().query::<&BlightZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert!(zone.hit_this_tick.is_empty(), "Enemy should not be hit after leaving zone");
        }

        #[test]
        fn test_blight_zone_tick_rate_correct() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy in range
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            // Run 3 tick cycles
            let mut total_hits = 0;
            for _ in 0..3 {
                // Advance time to trigger tick
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
                }

                // Run the system
                let _ = app.world_mut().run_system_once(blight_zone_damage_system);

                // Count hits and clear hit tracking
                let mut zone_query = app.world_mut().query::<&mut BlightZone>();
                let mut zone = zone_query.single_mut(app.world_mut()).unwrap();
                if zone.hit_this_tick.contains(&enemy_entity) {
                    total_hits += 1;
                }
                zone.hit_this_tick.clear();
            }

            assert_eq!(total_hits, 3, "Enemy should have been hit 3 times over 3 tick cycles");
        }

        #[test]
        fn test_multiple_enemies_in_zone() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            ));

            // Create 3 enemies in range
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 * 0.5, 0.375, 0.0)),
                ));
            }

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLIGHT_ZONE_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(blight_zone_damage_system);

            // Check that all 3 enemies were hit
            let mut zone_query = app.world_mut().query::<&BlightZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert_eq!(zone.hit_this_tick.len(), 3, "All 3 enemies should have been marked as hit");
        }
    }

    mod cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_blight_zone_despawns_after_duration() {
            let mut app = App::new();

            let mut zone = BlightZone::new(Vec2::ZERO, 30.0);
            zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            zone.duration.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(blight_zone_cleanup_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_blight_zone_survives_before_expiry() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlightZone::new(Vec2::ZERO, 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(blight_zone_cleanup_system);

            assert!(app.world().entities().contains(entity));
        }

        #[test]
        fn test_multiple_zones_independent_cleanup() {
            let mut app = App::new();

            // Expired zone
            let mut expired_zone = BlightZone::new(Vec2::new(0.0, 0.0), 30.0);
            expired_zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            expired_zone.duration.tick(Duration::from_secs(1));

            let expired_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                expired_zone,
            )).id();

            // Active zone
            let active_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                BlightZone::new(Vec2::new(10.0, 10.0), 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(blight_zone_cleanup_system);

            assert!(!app.world().entities().contains(expired_entity), "Expired zone should despawn");
            assert!(app.world().entities().contains(active_entity), "Active zone should survive");
        }
    }
}
