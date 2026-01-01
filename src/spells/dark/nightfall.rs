//! Nightfall spell - A zone that increases Dark damage dealt within it.
//!
//! Creates a dark zone at a target location where darkness intensifies.
//! All Dark element spells deal bonus damage to enemies within the Nightfall zone.
//! This implements the Eclipse SpellType from the Dark element.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Nightfall spell
pub const NIGHTFALL_RADIUS: f32 = 8.0;
pub const NIGHTFALL_DURATION: f32 = 6.0;
pub const NIGHTFALL_DARK_DAMAGE_MULTIPLIER: f32 = 1.5; // 50% bonus dark damage
pub const NIGHTFALL_VISUAL_HEIGHT: f32 = 0.1;

/// Get the dark element color for visual effects (purple)
pub fn nightfall_color() -> Color {
    Element::Dark.color()
}

/// NightfallZone component - a zone of darkness that amplifies Dark damage.
/// Enemies within this zone take increased damage from Dark element spells.
#[derive(Component, Debug, Clone)]
pub struct NightfallZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the zone
    pub radius: f32,
    /// Timer tracking remaining duration
    pub duration: Timer,
    /// Damage multiplier for Dark spells in zone (e.g., 1.5 = 50% bonus)
    pub dark_damage_multiplier: f32,
}

impl NightfallZone {
    /// Create a new nightfall zone at the given center position.
    pub fn new(center: Vec2, radius: f32, duration_secs: f32, multiplier: f32) -> Self {
        Self {
            center,
            radius,
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            dark_damage_multiplier: multiplier,
        }
    }

    /// Create a nightfall zone with default configuration.
    pub fn default_config(center: Vec2) -> Self {
        Self::new(
            center,
            NIGHTFALL_RADIUS,
            NIGHTFALL_DURATION,
            NIGHTFALL_DARK_DAMAGE_MULTIPLIER,
        )
    }

    /// Check if a position (XZ plane) is inside the zone.
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }

    /// Check if the zone has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }
}

/// Marker component for enemies currently inside a Nightfall zone.
/// Added when enemy enters zone, removed when they leave or zone expires.
#[derive(Component, Debug, Clone)]
pub struct InNightfallZone {
    /// The multiplier to apply to Dark damage
    pub dark_damage_multiplier: f32,
}

/// System that updates NightfallZone duration timers.
pub fn nightfall_zone_duration_system(
    time: Res<Time>,
    mut zone_query: Query<&mut NightfallZone>,
) {
    for mut zone in zone_query.iter_mut() {
        zone.tick(time.delta());
    }
}

/// System that tracks which enemies are inside nightfall zones
/// and adds/removes the InNightfallZone marker component.
pub fn nightfall_zone_tracking_system(
    mut commands: Commands,
    zone_query: Query<&NightfallZone>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    marked_query: Query<Entity, With<InNightfallZone>>,
) {
    // First, check which enemies are in any nightfall zone
    let mut enemies_in_zones: Vec<(Entity, f32)> = Vec::new();

    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);

        // Check if enemy is in any nightfall zone
        let mut max_multiplier: Option<f32> = None;
        for zone in zone_query.iter() {
            if zone.contains(enemy_pos) {
                // Use highest multiplier if in multiple zones
                match max_multiplier {
                    Some(current) => {
                        if zone.dark_damage_multiplier > current {
                            max_multiplier = Some(zone.dark_damage_multiplier);
                        }
                    }
                    None => {
                        max_multiplier = Some(zone.dark_damage_multiplier);
                    }
                }
            }
        }

        if let Some(multiplier) = max_multiplier {
            enemies_in_zones.push((enemy_entity, multiplier));
        }
    }

    // Remove markers from enemies no longer in any zone
    for marked_entity in marked_query.iter() {
        let still_in_zone = enemies_in_zones.iter().any(|(e, _)| *e == marked_entity);
        if !still_in_zone {
            commands.entity(marked_entity).remove::<InNightfallZone>();
        }
    }

    // Add or update markers for enemies in zones
    for (enemy_entity, multiplier) in enemies_in_zones {
        commands.entity(enemy_entity).insert(InNightfallZone {
            dark_damage_multiplier: multiplier,
        });
    }
}

/// System that applies the Nightfall damage bonus to Dark element DamageEvents.
/// Reads InNightfallZone markers and modifies Dark damage accordingly.
pub fn nightfall_damage_bonus_system(
    marked_query: Query<&InNightfallZone>,
    mut damage_events: MessageReader<DamageEvent>,
    mut boosted_events: MessageWriter<DamageEvent>,
) {
    // Note: This system reads damage events and re-sends them with modified damage
    // if the target is in a nightfall zone and the damage is Dark element.
    // This pattern requires the original damage event to be consumed and a new one sent.
    //
    // However, the current architecture applies damage in apply_damage_system which
    // reads DamageEvent. To avoid double-damage, we need a different approach:
    // We modify the damage in-flight using a component-based multiplier check
    // in the apply_damage_system itself.
    //
    // For now, this system is a placeholder - the actual damage boost is applied
    // by checking InNightfallZone in the combat damage system.
    //
    // Alternative: Use a separate event channel for "pre-processed" damage.
    let _ = (&marked_query, &mut damage_events, &mut boosted_events);
}

/// System that despawns expired nightfall zones.
pub fn nightfall_zone_cleanup_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &NightfallZone)>,
) {
    for (entity, zone) in zone_query.iter() {
        if zone.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Nightfall (Eclipse) spell - spawns a dark zone at the target location.
/// `spawn_position` is Whisper's position, `target_position` is where zone should appear.
#[allow(clippy::too_many_arguments)]
pub fn fire_nightfall(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_position: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_nightfall_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Nightfall spell at the target location.
/// The zone spawns at the target position (where enemies are).
/// `damage` parameter is unused for this spell (zone provides damage multiplier, not direct damage).
#[allow(clippy::too_many_arguments)]
pub fn fire_nightfall_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    _damage: f32,
    _spawn_position: Vec3,
    target_position: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let zone = NightfallZone::default_config(target_position);
    let zone_pos = Vec3::new(target_position.x, NIGHTFALL_VISUAL_HEIGHT, target_position.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.explosion.clone()),
            Transform::from_translation(zone_pos).with_scale(Vec3::splat(NIGHTFALL_RADIUS)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;
    use bevy::ecs::system::RunSystemOnce;

    mod nightfall_zone_component_tests {
        use super::*;

        #[test]
        fn test_nightfall_zone_new() {
            let center = Vec2::new(10.0, 20.0);
            let zone = NightfallZone::new(center, 8.0, 6.0, 1.5);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, 8.0);
            assert_eq!(zone.dark_damage_multiplier, 1.5);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_nightfall_zone_default_config() {
            let center = Vec2::new(5.0, 15.0);
            let zone = NightfallZone::default_config(center);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, NIGHTFALL_RADIUS);
            assert_eq!(zone.dark_damage_multiplier, NIGHTFALL_DARK_DAMAGE_MULTIPLIER);
        }

        #[test]
        fn test_nightfall_zone_contains_inside() {
            let zone = NightfallZone::default_config(Vec2::ZERO);
            assert!(zone.contains(Vec2::new(3.0, 0.0)));
            assert!(zone.contains(Vec2::new(0.0, 5.0)));
            assert!(zone.contains(Vec2::ZERO));
        }

        #[test]
        fn test_nightfall_zone_contains_outside() {
            let zone = NightfallZone::default_config(Vec2::ZERO);
            assert!(!zone.contains(Vec2::new(10.0, 0.0)));
            assert!(!zone.contains(Vec2::new(0.0, 10.0)));
            assert!(!zone.contains(Vec2::new(7.0, 7.0)));
        }

        #[test]
        fn test_nightfall_zone_contains_on_edge() {
            let zone = NightfallZone::default_config(Vec2::ZERO);
            assert!(zone.contains(Vec2::new(NIGHTFALL_RADIUS, 0.0)));
            assert!(zone.contains(Vec2::new(0.0, NIGHTFALL_RADIUS)));
        }

        #[test]
        fn test_nightfall_zone_tick_and_expire() {
            let mut zone = NightfallZone::new(Vec2::ZERO, 8.0, 1.0, 1.5);
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(0.5));
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(0.6));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_nightfall_uses_dark_element_color() {
            let color = nightfall_color();
            assert_eq!(color, Element::Dark.color());
        }
    }

    mod nightfall_zone_duration_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_zone_duration_ticks_down() {
            let mut app = setup_test_app();

            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::new(Vec2::ZERO, 8.0, 5.0, 1.5),
            )).id();

            // Advance time by 2 seconds
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.0));
            }

            let _ = app.world_mut().run_system_once(nightfall_zone_duration_system);

            let zone = app.world().get::<NightfallZone>(zone_entity).unwrap();
            let remaining = zone.duration.remaining_secs();
            assert!(
                remaining < 4.0 && remaining > 2.5,
                "Zone timer should have ticked down, remaining: {}",
                remaining
            );
        }

        #[test]
        fn test_zone_expires_after_duration() {
            let mut app = setup_test_app();

            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::new(Vec2::ZERO, 8.0, 1.0, 1.5),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.5));
            }

            let _ = app.world_mut().run_system_once(nightfall_zone_duration_system);

            let zone = app.world().get::<NightfallZone>(zone_entity).unwrap();
            assert!(zone.is_expired());
        }
    }

    mod nightfall_zone_tracking_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_enemy_in_zone_gets_marker() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::default_config(Vec2::ZERO),
            ));

            // Create enemy inside zone (XZ distance = 3)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);

            assert!(
                app.world().get::<InNightfallZone>(enemy_entity).is_some(),
                "Enemy in zone should have InNightfallZone marker"
            );
        }

        #[test]
        fn test_enemy_outside_zone_no_marker() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::default_config(Vec2::ZERO),
            ));

            // Create enemy outside zone (XZ distance = 15)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(15.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);

            assert!(
                app.world().get::<InNightfallZone>(enemy_entity).is_none(),
                "Enemy outside zone should not have marker"
            );
        }

        #[test]
        fn test_marker_removed_when_enemy_leaves_zone() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::default_config(Vec2::ZERO),
            ));

            // Create enemy inside zone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // First tick - should add marker
            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);
            assert!(app.world().get::<InNightfallZone>(enemy_entity).is_some());

            // Move enemy outside zone
            app.world_mut().entity_mut(enemy_entity)
                .get_mut::<Transform>()
                .unwrap()
                .translation = Vec3::new(20.0, 0.375, 0.0);

            // Second tick - should remove marker
            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);
            assert!(
                app.world().get::<InNightfallZone>(enemy_entity).is_none(),
                "Marker should be removed when enemy leaves zone"
            );
        }

        #[test]
        fn test_multiple_zones_use_highest_multiplier() {
            let mut app = setup_test_app();

            // Create zone with 1.5x multiplier
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::new(Vec2::ZERO, 10.0, 6.0, 1.5),
            ));

            // Create overlapping zone with 2.0x multiplier
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
                NightfallZone::new(Vec2::new(3.0, 0.0), 10.0, 6.0, 2.0),
            ));

            // Create enemy in both zones
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);

            let marker = app.world().get::<InNightfallZone>(enemy_entity).unwrap();
            assert_eq!(
                marker.dark_damage_multiplier, 2.0,
                "Should use highest multiplier from overlapping zones"
            );
        }

        #[test]
        fn test_uses_xz_plane_ignores_y() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::default_config(Vec2::ZERO),
            ));

            // Create enemy close on XZ but far on Y - should still be in zone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);

            assert!(
                app.world().get::<InNightfallZone>(enemy_entity).is_some(),
                "Y distance should be ignored for zone containment"
            );
        }

        #[test]
        fn test_multiple_enemies_tracked_independently() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                NightfallZone::default_config(Vec2::ZERO),
            ));

            // Create enemy inside zone
            let enemy_inside = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // Create enemy outside zone
            let enemy_outside = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(20.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_tracking_system);

            assert!(app.world().get::<InNightfallZone>(enemy_inside).is_some());
            assert!(app.world().get::<InNightfallZone>(enemy_outside).is_none());
        }
    }

    mod nightfall_zone_cleanup_system_tests {
        use super::*;

        #[test]
        fn test_expired_zone_despawned() {
            let mut app = App::new();

            let mut zone = NightfallZone::new(Vec2::ZERO, 8.0, 0.5, 1.5);
            zone.duration.tick(Duration::from_secs_f32(1.0)); // Force expired

            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_cleanup_system);

            assert!(
                app.world().get_entity(zone_entity).is_err(),
                "Expired zone should be despawned"
            );
        }

        #[test]
        fn test_active_zone_survives() {
            let mut app = App::new();

            let zone = NightfallZone::new(Vec2::ZERO, 8.0, 10.0, 1.5);
            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(nightfall_zone_cleanup_system);

            assert!(
                app.world().get_entity(zone_entity).is_ok(),
                "Active zone should survive cleanup"
            );
        }
    }

    mod fire_nightfall_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_nightfall_spawns_zone() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Eclipse);
            let spawn_pos = Vec3::new(0.0, 3.0, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_nightfall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            let zones: Vec<_> = zone_query.iter(app.world()).collect();
            assert_eq!(zones.len(), 1, "One nightfall zone should be spawned");
        }

        #[test]
        fn test_fire_nightfall_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Eclipse);
            let spawn_pos = Vec3::new(0.0, 3.0, 0.0);
            let target_pos = Vec2::new(15.0, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_nightfall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.center, target_pos, "Zone should spawn at target position");
            }
        }

        #[test]
        fn test_fire_nightfall_has_correct_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Eclipse);
            let spawn_pos = Vec3::new(0.0, 3.0, 0.0);
            let target_pos = Vec2::new(10.0, 10.0);

            {
                let mut commands = app.world_mut().commands();
                fire_nightfall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.radius, NIGHTFALL_RADIUS);
            }
        }

        #[test]
        fn test_fire_nightfall_has_correct_multiplier() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Eclipse);
            let spawn_pos = Vec3::new(0.0, 3.0, 0.0);
            let target_pos = Vec2::new(10.0, 10.0);

            {
                let mut commands = app.world_mut().commands();
                fire_nightfall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&NightfallZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.dark_damage_multiplier, NIGHTFALL_DARK_DAMAGE_MULTIPLIER);
            }
        }

        #[test]
        fn test_zone_visual_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Eclipse);
            let spawn_pos = Vec3::new(0.0, 3.0, 0.0);
            let target_pos = Vec2::new(12.0, 18.0);

            {
                let mut commands = app.world_mut().commands();
                fire_nightfall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&NightfallZone, &Transform)>();
            for (zone, transform) in query.iter(app.world()) {
                // Visual position should be at target XZ with low Y
                assert_eq!(transform.translation.x, zone.center.x);
                assert_eq!(transform.translation.z, zone.center.y);
                assert_eq!(transform.translation.y, NIGHTFALL_VISUAL_HEIGHT);
            }
        }
    }

    mod in_nightfall_zone_tests {
        use super::*;

        #[test]
        fn test_in_nightfall_zone_stores_multiplier() {
            let marker = InNightfallZone {
                dark_damage_multiplier: 1.75,
            };
            assert_eq!(marker.dark_damage_multiplier, 1.75);
        }
    }
}
