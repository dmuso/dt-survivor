//! Sanctify spell - Creates a sanctified zone that applies a damage vulnerability debuff.
//!
//! A Light element spell (Consecration SpellType) that creates a sanctified zone at a target
//! location. Enemies within the zone receive a SanctifiedDebuff component that causes them
//! to take increased damage from all sources. The debuff is removed when enemies leave the zone.

use bevy::prelude::*;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Sanctify spell
pub const SANCTIFY_ZONE_RADIUS: f32 = 5.0;
pub const SANCTIFY_ZONE_DURATION: f32 = 6.0;
pub const SANCTIFY_DAMAGE_MULTIPLIER: f32 = 1.5; // 50% more damage taken
pub const SANCTIFY_VISUAL_HEIGHT: f32 = 0.1;

/// Get the light element color for visual effects (white/gold)
pub fn sanctify_color() -> Color {
    Element::Light.color()
}

/// A sanctified zone that applies damage vulnerability to enemies inside.
/// Spawns at a target location and persists for a duration.
#[derive(Component, Debug, Clone)]
pub struct SanctifiedZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the sanctified zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Damage multiplier applied to enemies inside the zone
    pub damage_multiplier: f32,
}

impl SanctifiedZone {
    pub fn new(center: Vec2, radius: f32, duration: f32, damage_multiplier: f32) -> Self {
        Self {
            center,
            radius,
            duration: Timer::from_seconds(duration, TimerMode::Once),
            damage_multiplier,
        }
    }

    pub fn default_config(center: Vec2) -> Self {
        Self::new(
            center,
            SANCTIFY_ZONE_RADIUS,
            SANCTIFY_ZONE_DURATION,
            SANCTIFY_DAMAGE_MULTIPLIER,
        )
    }

    /// Check if the zone has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Check if a position (XZ plane) is inside the zone
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }
}

/// Sanctified debuff applied to enemies inside a SanctifiedZone.
/// Causes the enemy to take increased damage from all sources.
/// Tracks which zone(s) the enemy is inside for proper removal.
#[derive(Component, Debug, Clone)]
pub struct SanctifiedDebuff {
    /// Damage multiplier (1.5 = 50% more damage taken)
    pub damage_multiplier: f32,
    /// The zone entities this enemy is currently inside
    pub source_zones: Vec<Entity>,
}

impl SanctifiedDebuff {
    pub fn new(damage_multiplier: f32, source_zone: Entity) -> Self {
        Self {
            damage_multiplier,
            source_zones: vec![source_zone],
        }
    }

    /// Add a zone that is affecting this enemy
    pub fn add_zone(&mut self, zone: Entity) {
        if !self.source_zones.contains(&zone) {
            self.source_zones.push(zone);
        }
    }

    /// Remove a zone from the list of affecting zones
    pub fn remove_zone(&mut self, zone: Entity) {
        self.source_zones.retain(|&z| z != zone);
    }

    /// Check if the enemy is still affected by any zones
    pub fn has_active_zones(&self) -> bool {
        !self.source_zones.is_empty()
    }
}

impl Default for SanctifiedDebuff {
    fn default() -> Self {
        Self {
            damage_multiplier: SANCTIFY_DAMAGE_MULTIPLIER,
            source_zones: Vec::new(),
        }
    }
}

/// Spawns a sanctified zone at the target location when spell is cast.
pub fn spawn_sanctified_zone(
    commands: &mut Commands,
    _spell: &Spell,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    spawn_sanctified_zone_with_config(
        commands,
        target_pos,
        SANCTIFY_ZONE_RADIUS,
        SANCTIFY_ZONE_DURATION,
        SANCTIFY_DAMAGE_MULTIPLIER,
        game_meshes,
        game_materials,
    );
}

/// Spawns a sanctified zone with custom configuration.
pub fn spawn_sanctified_zone_with_config(
    commands: &mut Commands,
    target_pos: Vec2,
    radius: f32,
    duration: f32,
    damage_multiplier: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let zone = SanctifiedZone::new(target_pos, radius, duration, damage_multiplier);
    let zone_pos = Vec3::new(target_pos.x, SANCTIFY_VISUAL_HEIGHT, target_pos.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.radiant_beam.clone()),
            Transform::from_translation(zone_pos).with_scale(Vec3::splat(radius)),
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

/// System that ticks zone duration timers.
pub fn sanctified_zone_duration_system(
    time: Res<Time>,
    mut zone_query: Query<&mut SanctifiedZone>,
) {
    for mut zone in zone_query.iter_mut() {
        zone.tick(time.delta());
    }
}

/// System that applies SanctifiedDebuff to enemies entering zones
/// and removes it from enemies leaving zones.
pub fn sanctified_zone_debuff_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &SanctifiedZone)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut debuff_query: Query<&mut SanctifiedDebuff>,
) {
    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);

        // Track which zones this enemy is currently in
        let mut zones_enemy_is_in: Vec<(Entity, f32)> = Vec::new();

        for (zone_entity, zone) in zone_query.iter() {
            if zone.contains(enemy_pos) {
                zones_enemy_is_in.push((zone_entity, zone.damage_multiplier));
            }
        }

        if let Ok(mut debuff) = debuff_query.get_mut(enemy_entity) {
            // Enemy already has debuff - update zone list
            // First, remove zones the enemy is no longer in
            let zones_to_remove: Vec<Entity> = debuff
                .source_zones
                .iter()
                .filter(|&&z| !zones_enemy_is_in.iter().any(|(ze, _)| *ze == z))
                .copied()
                .collect();

            for zone in zones_to_remove {
                debuff.remove_zone(zone);
            }

            // Add any new zones
            for (zone_entity, _) in &zones_enemy_is_in {
                debuff.add_zone(*zone_entity);
            }

            // Remove debuff component if no longer in any zone
            if !debuff.has_active_zones() {
                commands.entity(enemy_entity).remove::<SanctifiedDebuff>();
            }
        } else if !zones_enemy_is_in.is_empty() {
            // Enemy doesn't have debuff but is in a zone - add it
            // Use the highest damage multiplier from all zones
            let max_multiplier = zones_enemy_is_in
                .iter()
                .map(|(_, m)| *m)
                .fold(f32::NEG_INFINITY, f32::max);
            let first_zone = zones_enemy_is_in[0].0;

            let mut debuff = SanctifiedDebuff::new(max_multiplier, first_zone);
            for (zone_entity, _) in zones_enemy_is_in.iter().skip(1) {
                debuff.add_zone(*zone_entity);
            }

            commands.entity(enemy_entity).insert(debuff);
        }
    }
}

/// System that despawns expired sanctified zones.
pub fn sanctified_zone_cleanup_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &SanctifiedZone)>,
) {
    for (entity, zone) in zone_query.iter() {
        if zone.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that removes SanctifiedDebuff from enemies when their source zones are despawned.
pub fn sanctified_debuff_cleanup_system(
    mut commands: Commands,
    zone_query: Query<Entity, With<SanctifiedZone>>,
    mut debuff_query: Query<(Entity, &mut SanctifiedDebuff)>,
) {
    let active_zones: Vec<Entity> = zone_query.iter().collect();

    for (enemy_entity, mut debuff) in debuff_query.iter_mut() {
        // Remove any zones that no longer exist
        debuff.source_zones.retain(|z| active_zones.contains(z));

        // Remove debuff if no active zones remain
        if !debuff.has_active_zones() {
            commands.entity(enemy_entity).remove::<SanctifiedDebuff>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod sanctified_zone_component_tests {
        use super::*;

        #[test]
        fn test_sanctified_zone_new() {
            let center = Vec2::new(5.0, 10.0);
            let zone = SanctifiedZone::new(center, 5.0, 6.0, 1.5);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, 5.0);
            assert_eq!(zone.damage_multiplier, 1.5);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_sanctified_zone_default_config() {
            let center = Vec2::new(10.0, 20.0);
            let zone = SanctifiedZone::default_config(center);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, SANCTIFY_ZONE_RADIUS);
            assert_eq!(zone.damage_multiplier, SANCTIFY_DAMAGE_MULTIPLIER);
        }

        #[test]
        fn test_sanctified_zone_is_expired() {
            let mut zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 1.0, 1.5);
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(1.1));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_sanctified_zone_tick() {
            let mut zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 2.0, 1.5);

            zone.tick(Duration::from_secs_f32(0.5));
            assert!(!zone.is_expired());

            zone.tick(Duration::from_secs_f32(1.6));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_sanctified_zone_contains_position_inside() {
            let zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5);

            assert!(zone.contains(Vec2::new(2.0, 0.0)));
            assert!(zone.contains(Vec2::new(0.0, 3.0)));
            assert!(zone.contains(Vec2::ZERO));
        }

        #[test]
        fn test_sanctified_zone_does_not_contain_outside() {
            let zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5);

            assert!(!zone.contains(Vec2::new(6.0, 0.0)));
            assert!(!zone.contains(Vec2::new(0.0, 7.0)));
            assert!(!zone.contains(Vec2::new(10.0, 10.0)));
        }

        #[test]
        fn test_sanctified_zone_contains_on_edge() {
            let zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5);

            assert!(zone.contains(Vec2::new(5.0, 0.0)));
            assert!(zone.contains(Vec2::new(0.0, 5.0)));
        }

        #[test]
        fn test_uses_light_element_color() {
            let color = sanctify_color();
            assert_eq!(color, Element::Light.color());
        }
    }

    mod sanctified_debuff_component_tests {
        use super::*;

        #[test]
        fn test_sanctified_debuff_new() {
            let zone_entity = Entity::from_bits(1);
            let debuff = SanctifiedDebuff::new(1.5, zone_entity);

            assert_eq!(debuff.damage_multiplier, 1.5);
            assert_eq!(debuff.source_zones.len(), 1);
            assert!(debuff.source_zones.contains(&zone_entity));
        }

        #[test]
        fn test_sanctified_debuff_default() {
            let debuff = SanctifiedDebuff::default();

            assert_eq!(debuff.damage_multiplier, SANCTIFY_DAMAGE_MULTIPLIER);
            assert!(debuff.source_zones.is_empty());
        }

        #[test]
        fn test_sanctified_debuff_add_zone() {
            let zone1 = Entity::from_bits(1);
            let zone2 = Entity::from_bits(2);
            let mut debuff = SanctifiedDebuff::new(1.5, zone1);

            debuff.add_zone(zone2);

            assert_eq!(debuff.source_zones.len(), 2);
            assert!(debuff.source_zones.contains(&zone1));
            assert!(debuff.source_zones.contains(&zone2));
        }

        #[test]
        fn test_sanctified_debuff_add_zone_no_duplicate() {
            let zone = Entity::from_bits(1);
            let mut debuff = SanctifiedDebuff::new(1.5, zone);

            debuff.add_zone(zone); // Try to add same zone again

            assert_eq!(debuff.source_zones.len(), 1);
        }

        #[test]
        fn test_sanctified_debuff_remove_zone() {
            let zone1 = Entity::from_bits(1);
            let zone2 = Entity::from_bits(2);
            let mut debuff = SanctifiedDebuff::new(1.5, zone1);
            debuff.add_zone(zone2);

            debuff.remove_zone(zone1);

            assert_eq!(debuff.source_zones.len(), 1);
            assert!(!debuff.source_zones.contains(&zone1));
            assert!(debuff.source_zones.contains(&zone2));
        }

        #[test]
        fn test_sanctified_debuff_has_active_zones() {
            let zone = Entity::from_bits(1);
            let mut debuff = SanctifiedDebuff::new(1.5, zone);

            assert!(debuff.has_active_zones());

            debuff.remove_zone(zone);
            assert!(!debuff.has_active_zones());
        }
    }

    mod spawn_sanctified_zone_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_spawn_sanctified_zone_creates_entity() {
            let mut app = setup_test_app();
            let target_pos = Vec2::new(15.0, 20.0);

            {
                let spell = Spell::new(SpellType::Consecration);
                let mut commands = app.world_mut().commands();
                spawn_sanctified_zone(
                    &mut commands,
                    &spell,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 1, "One zone should spawn");
        }

        #[test]
        fn test_spawn_sanctified_zone_at_target_position() {
            let mut app = setup_test_app();
            let target_pos = Vec2::new(15.0, 20.0);

            {
                let spell = Spell::new(SpellType::Consecration);
                let mut commands = app.world_mut().commands();
                spawn_sanctified_zone(
                    &mut commands,
                    &spell,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.center, target_pos);
            }
        }

        #[test]
        fn test_spawn_sanctified_zone_default_radius() {
            let mut app = setup_test_app();

            {
                let spell = Spell::new(SpellType::Consecration);
                let mut commands = app.world_mut().commands();
                spawn_sanctified_zone(
                    &mut commands,
                    &spell,
                    Vec2::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.radius, SANCTIFY_ZONE_RADIUS);
            }
        }

        #[test]
        fn test_spawn_sanctified_zone_default_damage_multiplier() {
            let mut app = setup_test_app();

            {
                let spell = Spell::new(SpellType::Consecration);
                let mut commands = app.world_mut().commands();
                spawn_sanctified_zone(
                    &mut commands,
                    &spell,
                    Vec2::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.damage_multiplier, SANCTIFY_DAMAGE_MULTIPLIER);
            }
        }

        #[test]
        fn test_spawn_sanctified_zone_with_custom_config() {
            let mut app = setup_test_app();
            let target_pos = Vec2::new(5.0, 5.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_sanctified_zone_with_config(
                    &mut commands,
                    target_pos,
                    10.0,  // custom radius
                    12.0,  // custom duration
                    2.0,   // custom multiplier
                    None,
                    None,
                );
            }
            app.update();

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.center, target_pos);
                assert_eq!(zone.radius, 10.0);
                assert_eq!(zone.damage_multiplier, 2.0);
            }
        }
    }

    mod sanctified_zone_duration_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_zone_duration_ticks() {
            let mut app = setup_test_app();

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 3.0, 1.5),
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(sanctified_zone_duration_system);

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            for zone in zone_query.iter(app.world()) {
                assert!(!zone.is_expired(), "Zone should not be expired after 1 second");
            }
        }

        #[test]
        fn test_zone_expires_after_duration() {
            let mut app = setup_test_app();

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 2.0, 1.5),
            ));

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.5));
            }

            let _ = app.world_mut().run_system_once(sanctified_zone_duration_system);

            let mut zone_query = app.world_mut().query::<&SanctifiedZone>();
            for zone in zone_query.iter(app.world()) {
                assert!(zone.is_expired(), "Zone should be expired after duration");
            }
        }
    }

    mod sanctified_zone_debuff_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_enemy_gains_debuff_when_entering_zone() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5),
            ));

            // Create enemy inside zone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);

            let debuff = app.world().get::<SanctifiedDebuff>(enemy_entity);
            assert!(debuff.is_some(), "Enemy inside zone should have SanctifiedDebuff");
            assert_eq!(debuff.unwrap().damage_multiplier, 1.5);
        }

        #[test]
        fn test_enemy_outside_zone_has_no_debuff() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5),
            ));

            // Create enemy outside zone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);

            let debuff = app.world().get::<SanctifiedDebuff>(enemy_entity);
            assert!(debuff.is_none(), "Enemy outside zone should not have SanctifiedDebuff");
        }

        #[test]
        fn test_enemy_loses_debuff_when_leaving_zone() {
            let mut app = setup_test_app();

            // Create zone at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5),
            ));

            // Create enemy inside zone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            // First update - enemy enters zone
            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);
            assert!(app.world().get::<SanctifiedDebuff>(enemy_entity).is_some());

            // Move enemy outside zone
            {
                let mut transform = app.world_mut().get_mut::<Transform>(enemy_entity).unwrap();
                transform.translation = Vec3::new(10.0, 0.375, 0.0);
            }

            // Second update - enemy leaves zone
            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);

            let debuff = app.world().get::<SanctifiedDebuff>(enemy_entity);
            assert!(debuff.is_none(), "Enemy should lose debuff after leaving zone");
        }

        #[test]
        fn test_enemy_in_multiple_zones() {
            let mut app = setup_test_app();

            // Create two overlapping zones
            let zone1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5),
            )).id();

            let zone2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
                SanctifiedZone::new(Vec2::new(3.0, 0.0), 5.0, 6.0, 1.5),
            )).id();

            // Create enemy in overlap area
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);

            let debuff = app.world().get::<SanctifiedDebuff>(enemy_entity).unwrap();
            assert_eq!(debuff.source_zones.len(), 2, "Debuff should track both zones");
            assert!(debuff.source_zones.contains(&zone1));
            assert!(debuff.source_zones.contains(&zone2));
        }

        #[test]
        fn test_debuff_multiplier_is_correct() {
            let mut app = setup_test_app();

            // Create zone with custom multiplier
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 2.0), // 100% more damage
            ));

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);

            let debuff = app.world().get::<SanctifiedDebuff>(enemy_entity).unwrap();
            assert_eq!(debuff.damage_multiplier, 2.0);
        }

        #[test]
        fn test_multiple_enemies_in_zone() {
            let mut app = setup_test_app();

            // Create zone
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 6.0, 1.5),
            ));

            // Create 3 enemies inside zone
            let enemies: Vec<Entity> = (0..3)
                .map(|i| {
                    app.world_mut().spawn((
                        Enemy { speed: 50.0, strength: 10.0 },
                        Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                    )).id()
                })
                .collect();

            let _ = app.world_mut().run_system_once(sanctified_zone_debuff_system);

            for enemy in enemies {
                assert!(
                    app.world().get::<SanctifiedDebuff>(enemy).is_some(),
                    "All enemies inside zone should have debuff"
                );
            }
        }
    }

    mod sanctified_zone_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_expired_zone_despawns() {
            let mut app = App::new();

            // Create expired zone
            let mut zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 0.0, 1.5);
            zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            zone.duration.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_cleanup_system);

            assert!(!app.world().entities().contains(entity), "Expired zone should despawn");
        }

        #[test]
        fn test_active_zone_survives() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 10.0, 1.5),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_cleanup_system);

            assert!(app.world().entities().contains(entity), "Active zone should survive");
        }

        #[test]
        fn test_multiple_zones_independent_cleanup() {
            let mut app = App::new();

            // Expired zone
            let mut expired_zone = SanctifiedZone::new(Vec2::ZERO, 5.0, 0.0, 1.5);
            expired_zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            expired_zone.duration.tick(Duration::from_secs(1));

            let expired_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                expired_zone,
            )).id();

            // Active zone
            let active_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                SanctifiedZone::new(Vec2::new(10.0, 10.0), 5.0, 10.0, 1.5),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_zone_cleanup_system);

            assert!(!app.world().entities().contains(expired_entity), "Expired zone should despawn");
            assert!(app.world().entities().contains(active_entity), "Active zone should survive");
        }
    }

    mod sanctified_debuff_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_debuff_removed_when_zone_despawned() {
            let mut app = App::new();

            // Create a zone entity (we'll track its ID but despawn it)
            let zone_entity = Entity::from_bits(12345);

            // Create enemy with debuff referencing the zone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                SanctifiedDebuff::new(1.5, zone_entity),
            )).id();

            // Run cleanup - zone doesn't exist so debuff should be removed
            let _ = app.world_mut().run_system_once(sanctified_debuff_cleanup_system);

            assert!(
                app.world().get::<SanctifiedDebuff>(enemy_entity).is_none(),
                "Debuff should be removed when source zone no longer exists"
            );
        }

        #[test]
        fn test_debuff_persists_when_zone_exists() {
            let mut app = App::new();

            // Create zone
            let zone_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 10.0, 1.5),
            )).id();

            // Create enemy with debuff
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                SanctifiedDebuff::new(1.5, zone_entity),
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_debuff_cleanup_system);

            assert!(
                app.world().get::<SanctifiedDebuff>(enemy_entity).is_some(),
                "Debuff should persist when source zone exists"
            );
        }

        #[test]
        fn test_debuff_survives_partial_zone_removal() {
            let mut app = App::new();

            // Create one real zone
            let zone1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SanctifiedZone::new(Vec2::ZERO, 5.0, 10.0, 1.5),
            )).id();

            // Fake zone ID that doesn't exist
            let zone2 = Entity::from_bits(99999);

            // Create enemy with debuff from both zones
            let mut debuff = SanctifiedDebuff::new(1.5, zone1);
            debuff.add_zone(zone2);

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                debuff,
            )).id();

            let _ = app.world_mut().run_system_once(sanctified_debuff_cleanup_system);

            let updated_debuff = app.world().get::<SanctifiedDebuff>(enemy_entity);
            assert!(updated_debuff.is_some(), "Debuff should survive with one valid zone");
            assert_eq!(updated_debuff.unwrap().source_zones.len(), 1);
            assert!(updated_debuff.unwrap().source_zones.contains(&zone1));
        }
    }
}
