//! Synapse Shock spell - Stuns enemies with mental overload.
//!
//! A Psychic element spell (Telekinesis SpellType) that creates an AOE stun burst
//! around the player. Enemies caught in the burst are briefly stunned, unable to
//! move or attack until the effect expires.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Maximum radius the synapse shock burst will expand to
pub const SYNAPSE_SHOCK_MAX_RADIUS: f32 = 8.0;

/// Duration in seconds for the burst to fully expand
pub const SYNAPSE_SHOCK_EXPANSION_DURATION: f32 = 0.4;

/// Duration in seconds that enemies remain stunned
pub const SYNAPSE_SHOCK_STUN_DURATION: f32 = 2.0;

/// Height of the visual effect above ground
pub const SYNAPSE_SHOCK_VISUAL_HEIGHT: f32 = 0.3;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn synapse_shock_color() -> Color {
    Element::Psychic.color()
}

/// Component for the expanding synapse shock burst.
/// Tracks expansion state and which enemies have been stunned.
#[derive(Component, Debug, Clone)]
pub struct SynapseShockBurst {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Current radius of the expanding wave
    pub current_radius: f32,
    /// Maximum radius the wave will expand to
    pub max_radius: f32,
    /// Expansion rate in units per second
    pub expansion_rate: f32,
    /// Duration of the stun effect to apply
    pub stun_duration: f32,
    /// Set of enemy entities already stunned by this burst (prevents double stun)
    pub stunned_enemies: HashSet<Entity>,
}

impl SynapseShockBurst {
    /// Create a new synapse shock burst at the given center position.
    pub fn new(center: Vec2) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: SYNAPSE_SHOCK_MAX_RADIUS,
            expansion_rate: SYNAPSE_SHOCK_MAX_RADIUS / SYNAPSE_SHOCK_EXPANSION_DURATION,
            stun_duration: SYNAPSE_SHOCK_STUN_DURATION,
            stunned_enemies: HashSet::new(),
        }
    }

    /// Create a synapse shock burst with custom stun duration.
    pub fn with_stun_duration(center: Vec2, stun_duration: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: SYNAPSE_SHOCK_MAX_RADIUS,
            expansion_rate: SYNAPSE_SHOCK_MAX_RADIUS / SYNAPSE_SHOCK_EXPANSION_DURATION,
            stun_duration,
            stunned_enemies: HashSet::new(),
        }
    }

    /// Check if the burst has finished expanding.
    pub fn is_finished(&self) -> bool {
        self.current_radius >= self.max_radius
    }

    /// Expand the wave by the given delta time.
    pub fn expand(&mut self, delta_secs: f32) {
        self.current_radius = (self.current_radius + self.expansion_rate * delta_secs)
            .min(self.max_radius);
    }

    /// Check if an enemy at the given distance should be stunned.
    /// Returns true if enemy is within the current wave radius and hasn't been stunned yet.
    pub fn should_stun(&self, entity: Entity, distance: f32) -> bool {
        distance <= self.current_radius && !self.stunned_enemies.contains(&entity)
    }

    /// Mark an enemy as stunned by this burst.
    pub fn mark_stunned(&mut self, entity: Entity) {
        self.stunned_enemies.insert(entity);
    }
}

/// Component attached to enemies that are currently stunned.
/// Prevents movement and tracks remaining stun duration.
#[derive(Component, Debug, Clone)]
pub struct StunnedEnemy {
    /// Timer tracking remaining stun duration
    pub duration: Timer,
    /// Original enemy speed (to verify restoration)
    pub original_speed: f32,
}

impl StunnedEnemy {
    /// Create a new StunnedEnemy with specified duration.
    pub fn new(duration: f32, original_speed: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration, TimerMode::Once),
            original_speed,
        }
    }

    /// Check if the stun effect has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the stun timer.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }
}

/// Height above enemy where stun indicator is positioned
pub const STUN_INDICATOR_HEIGHT: f32 = 2.5;

/// Component for the visual stun indicator above stunned enemies.
/// Tracks which enemy this indicator belongs to.
#[derive(Component, Debug, Clone)]
pub struct StunIndicator {
    /// The entity ID of the stunned enemy this indicator is attached to
    pub enemy_entity: Entity,
}

impl StunIndicator {
    /// Create a new StunIndicator for the given enemy entity.
    pub fn new(enemy_entity: Entity) -> Self {
        Self { enemy_entity }
    }
}

/// System that expands synapse shock bursts over time.
pub fn synapse_shock_expansion_system(
    mut burst_query: Query<&mut SynapseShockBurst>,
    time: Res<Time>,
) {
    for mut burst in burst_query.iter_mut() {
        burst.expand(time.delta_secs());
    }
}

/// System that applies stun effect to enemies caught in the expanding wave.
pub fn synapse_shock_stun_application_system(
    mut commands: Commands,
    mut burst_query: Query<&mut SynapseShockBurst>,
    enemy_query: Query<(Entity, &Transform, &Enemy), Without<StunnedEnemy>>,
) {
    for mut burst in burst_query.iter_mut() {
        for (enemy_entity, enemy_transform, enemy) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = burst.center.distance(enemy_pos);

            if burst.should_stun(enemy_entity, distance) {
                commands.entity(enemy_entity).insert(StunnedEnemy::new(
                    burst.stun_duration,
                    enemy.speed,
                ));
                burst.mark_stunned(enemy_entity);
            }
        }
    }
}

/// System that ticks stun duration timers.
pub fn synapse_shock_stun_tick_system(
    time: Res<Time>,
    mut stunned_query: Query<&mut StunnedEnemy>,
) {
    for mut stunned in stunned_query.iter_mut() {
        stunned.tick(time.delta());
    }
}

/// System that removes stun effect when duration expires.
pub fn synapse_shock_cleanup_stun_system(
    mut commands: Commands,
    stunned_query: Query<(Entity, &StunnedEnemy)>,
) {
    for (entity, stunned) in stunned_query.iter() {
        if stunned.is_expired() {
            commands.entity(entity).remove::<StunnedEnemy>();
        }
    }
}

/// System that despawns synapse shock bursts when they finish expanding.
pub fn synapse_shock_cleanup_burst_system(
    mut commands: Commands,
    burst_query: Query<(Entity, &SynapseShockBurst)>,
) {
    for (entity, burst) in burst_query.iter() {
        if burst.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual scale of synapse shock bursts.
pub fn synapse_shock_visual_system(
    mut burst_query: Query<(&SynapseShockBurst, &mut Transform)>,
) {
    for (burst, mut transform) in burst_query.iter_mut() {
        // Scale the visual to match current radius
        transform.scale = Vec3::splat(burst.current_radius.max(0.1));
    }
}

/// System that spawns stun indicators above newly stunned enemies.
/// Creates a yellow visual marker above each stunned enemy that doesn't already have one.
pub fn spawn_stun_indicator_system(
    mut commands: Commands,
    stunned_query: Query<(Entity, &Transform), With<StunnedEnemy>>,
    indicator_query: Query<&StunIndicator>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    // Build a set of enemies that already have indicators
    let existing_indicators: std::collections::HashSet<Entity> = indicator_query
        .iter()
        .map(|indicator| indicator.enemy_entity)
        .collect();

    for (enemy_entity, enemy_transform) in stunned_query.iter() {
        // Skip if this enemy already has an indicator
        if existing_indicators.contains(&enemy_entity) {
            continue;
        }

        // Position the indicator above the enemy
        let indicator_pos = Vec3::new(
            enemy_transform.translation.x,
            enemy_transform.translation.y + STUN_INDICATOR_HEIGHT,
            enemy_transform.translation.z,
        );

        if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), game_materials.as_ref()) {
            // Spawn a visible yellow sphere indicator
            commands.spawn((
                Mesh3d(meshes.whisper_core.clone()), // Small sphere
                MeshMaterial3d(materials.thunder_strike.clone()), // Bright yellow with emissive
                Transform::from_translation(indicator_pos).with_scale(Vec3::splat(2.0)),
                StunIndicator::new(enemy_entity),
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(indicator_pos),
                StunIndicator::new(enemy_entity),
            ));
        }
    }
}

/// System that updates stun indicator positions to follow their associated enemies.
pub fn update_stun_indicator_position_system(
    mut indicator_query: Query<(&StunIndicator, &mut Transform)>,
    enemy_query: Query<&Transform, (With<StunnedEnemy>, Without<StunIndicator>)>,
) {
    for (indicator, mut indicator_transform) in indicator_query.iter_mut() {
        if let Ok(enemy_transform) = enemy_query.get(indicator.enemy_entity) {
            // Update indicator position to stay above enemy
            indicator_transform.translation.x = enemy_transform.translation.x;
            indicator_transform.translation.y = enemy_transform.translation.y + STUN_INDICATOR_HEIGHT;
            indicator_transform.translation.z = enemy_transform.translation.z;
        }
    }
}

/// System that removes stun indicators when their associated enemies are no longer stunned.
pub fn cleanup_stun_indicator_system(
    mut commands: Commands,
    indicator_query: Query<(Entity, &StunIndicator)>,
    stunned_query: Query<Entity, With<StunnedEnemy>>,
) {
    // Build a set of currently stunned enemies
    let stunned_enemies: std::collections::HashSet<Entity> = stunned_query.iter().collect();

    for (indicator_entity, indicator) in indicator_query.iter() {
        // Remove indicator if its enemy is no longer stunned
        if !stunned_enemies.contains(&indicator.enemy_entity) {
            commands.entity(indicator_entity).despawn();
        }
    }
}

/// Cast synapse shock spell - spawns an expanding stun wave.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_synapse_shock(
    commands: &mut Commands,
    _spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_synapse_shock_with_stun_duration(
        commands,
        spawn_position,
        SYNAPSE_SHOCK_STUN_DURATION,
        game_meshes,
        game_materials,
    );
}

/// Cast synapse shock spell with explicit stun duration.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_synapse_shock_with_stun_duration(
    commands: &mut Commands,
    spawn_position: Vec3,
    stun_duration: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let burst = SynapseShockBurst::with_stun_duration(center, stun_duration);
    let burst_pos = Vec3::new(spawn_position.x, SYNAPSE_SHOCK_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.psychic_aoe.clone()), // Transparent magenta AOE material
            Transform::from_translation(burst_pos).with_scale(Vec3::splat(0.1)),
            burst,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(burst_pos),
            burst,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod synapse_shock_burst_tests {
        use super::*;

        #[test]
        fn test_synapse_shock_burst_new() {
            let center = Vec2::new(10.0, 20.0);
            let burst = SynapseShockBurst::new(center);

            assert_eq!(burst.center, center);
            assert_eq!(burst.current_radius, 0.0);
            assert_eq!(burst.max_radius, SYNAPSE_SHOCK_MAX_RADIUS);
            assert_eq!(burst.stun_duration, SYNAPSE_SHOCK_STUN_DURATION);
            assert!(!burst.is_finished());
            assert!(burst.stunned_enemies.is_empty());
        }

        #[test]
        fn test_synapse_shock_burst_with_stun_duration() {
            let center = Vec2::new(5.0, 15.0);
            let custom_duration = 5.0;
            let burst = SynapseShockBurst::with_stun_duration(center, custom_duration);

            assert_eq!(burst.center, center);
            assert_eq!(burst.stun_duration, custom_duration);
        }

        #[test]
        fn test_synapse_shock_burst_expand() {
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);

            burst.expand(SYNAPSE_SHOCK_EXPANSION_DURATION / 2.0);
            assert!(
                (burst.current_radius - SYNAPSE_SHOCK_MAX_RADIUS / 2.0).abs() < 0.01,
                "Radius should be half of max after half duration"
            );

            burst.expand(SYNAPSE_SHOCK_EXPANSION_DURATION / 2.0);
            assert!(
                (burst.current_radius - SYNAPSE_SHOCK_MAX_RADIUS).abs() < 0.01,
                "Radius should be max after full duration"
            );
        }

        #[test]
        fn test_synapse_shock_burst_expand_caps_at_max() {
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);

            // Expand way past max
            burst.expand(SYNAPSE_SHOCK_EXPANSION_DURATION * 10.0);

            assert_eq!(burst.current_radius, SYNAPSE_SHOCK_MAX_RADIUS);
        }

        #[test]
        fn test_synapse_shock_burst_is_finished() {
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            assert!(!burst.is_finished());

            burst.current_radius = SYNAPSE_SHOCK_MAX_RADIUS;
            assert!(burst.is_finished());
        }

        #[test]
        fn test_synapse_shock_burst_should_stun() {
            let burst = SynapseShockBurst {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                stun_duration: 2.0,
                stunned_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(burst.should_stun(entity, 3.0), "Should stun enemy within radius");
            assert!(burst.should_stun(entity, 5.0), "Should stun enemy at radius edge");
            assert!(!burst.should_stun(entity, 6.0), "Should not stun enemy outside radius");
        }

        #[test]
        fn test_synapse_shock_burst_should_stun_excludes_already_stunned() {
            let mut burst = SynapseShockBurst {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                stun_duration: 2.0,
                stunned_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(burst.should_stun(entity, 3.0));

            burst.mark_stunned(entity);
            assert!(!burst.should_stun(entity, 3.0), "Should not stun already-stunned enemy");
        }

        #[test]
        fn test_synapse_shock_burst_mark_stunned() {
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);

            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);

            burst.mark_stunned(entity1);
            assert!(burst.stunned_enemies.contains(&entity1));
            assert!(!burst.stunned_enemies.contains(&entity2));

            burst.mark_stunned(entity2);
            assert!(burst.stunned_enemies.contains(&entity1));
            assert!(burst.stunned_enemies.contains(&entity2));
        }

        #[test]
        fn test_synapse_shock_uses_psychic_element_color() {
            let color = synapse_shock_color();
            assert_eq!(color, Element::Psychic.color());
        }
    }

    mod stunned_enemy_tests {
        use super::*;

        #[test]
        fn test_stunned_enemy_new() {
            let stunned = StunnedEnemy::new(2.0, 50.0);
            assert!(!stunned.is_expired());
            assert_eq!(stunned.original_speed, 50.0);
        }

        #[test]
        fn test_stunned_enemy_is_expired() {
            let mut stunned = StunnedEnemy::new(0.1, 50.0);
            assert!(!stunned.is_expired());

            stunned.tick(Duration::from_secs_f32(0.2));
            assert!(stunned.is_expired());
        }

        #[test]
        fn test_stunned_enemy_tick() {
            let mut stunned = StunnedEnemy::new(1.0, 50.0);

            stunned.tick(Duration::from_secs_f32(0.5));
            assert!(!stunned.is_expired());

            stunned.tick(Duration::from_secs_f32(0.5));
            assert!(stunned.is_expired());
        }
    }

    mod synapse_shock_expansion_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_synapse_shock_expands_over_time() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SynapseShockBurst::new(Vec2::ZERO),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SYNAPSE_SHOCK_EXPANSION_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(synapse_shock_expansion_system);

            let burst = app.world().get::<SynapseShockBurst>(entity).unwrap();
            assert!(
                (burst.current_radius - SYNAPSE_SHOCK_MAX_RADIUS / 2.0).abs() < 0.1,
                "Radius should be approximately half after half duration: got {}",
                burst.current_radius
            );
        }

        #[test]
        fn test_synapse_shock_multiple_bursts_expand_independently() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create two bursts with different starting radii
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SynapseShockBurst::new(Vec2::ZERO),
            )).id();

            let mut burst2 = SynapseShockBurst::new(Vec2::new(10.0, 10.0));
            burst2.current_radius = 3.0; // Pre-expanded
            let entity2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                burst2,
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(synapse_shock_expansion_system);

            let burst1 = app.world().get::<SynapseShockBurst>(entity1).unwrap();
            let burst2 = app.world().get::<SynapseShockBurst>(entity2).unwrap();

            // Both should have expanded but from different starting points
            assert!(burst1.current_radius > 0.0);
            assert!(burst2.current_radius > 3.0);
        }
    }

    mod synapse_shock_stun_application_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_synapse_shock_stuns_enemies_in_radius() {
            let mut app = App::new();

            // Create burst at origin with radius 5.0
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy within radius (XZ distance = 3)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_stun_application_system);

            assert!(
                app.world().get::<StunnedEnemy>(enemy_entity).is_some(),
                "Enemy within radius should be stunned"
            );
        }

        #[test]
        fn test_synapse_shock_no_stun_outside_radius() {
            let mut app = App::new();

            // Create burst at origin with radius 3.0
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 3.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy outside radius (XZ distance = 5)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_stun_application_system);

            assert!(
                app.world().get::<StunnedEnemy>(enemy_entity).is_none(),
                "Enemy outside radius should not be stunned"
            );
        }

        #[test]
        fn test_synapse_shock_stuns_enemy_only_once() {
            let mut app = App::new();

            // Create burst
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy in radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            // Run system once - enemy gets stunned
            let _ = app.world_mut().run_system_once(synapse_shock_stun_application_system);

            let stunned = app.world().get::<StunnedEnemy>(enemy_entity).unwrap();
            let original_speed = stunned.original_speed;
            assert_eq!(original_speed, 50.0);

            // Check burst has marked the enemy as stunned
            let mut burst_query = app.world_mut().query::<&SynapseShockBurst>();
            let burst = burst_query.iter(app.world()).next().unwrap();
            assert!(burst.stunned_enemies.contains(&enemy_entity));
        }

        #[test]
        fn test_synapse_shock_stuns_multiple_enemies() {
            let mut app = App::new();

            // Create burst
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create 3 enemies in radius
            let mut enemies = Vec::new();
            for i in 0..3 {
                let entity = app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                )).id();
                enemies.push(entity);
            }

            let _ = app.world_mut().run_system_once(synapse_shock_stun_application_system);

            for enemy in enemies {
                assert!(
                    app.world().get::<StunnedEnemy>(enemy).is_some(),
                    "All enemies in radius should be stunned"
                );
            }
        }

        #[test]
        fn test_synapse_shock_uses_xz_plane_ignores_y() {
            let mut app = App::new();

            // Create burst at origin
            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy close on XZ plane but far on Y - should still be stunned
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_stun_application_system);

            assert!(
                app.world().get::<StunnedEnemy>(enemy_entity).is_some(),
                "Y distance should be ignored"
            );
        }

        #[test]
        fn test_synapse_shock_stores_original_speed() {
            let mut app = App::new();

            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy with specific speed
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 75.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_stun_application_system);

            let stunned = app.world().get::<StunnedEnemy>(enemy_entity).unwrap();
            assert_eq!(stunned.original_speed, 75.0);
        }
    }

    mod synapse_shock_stun_tick_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_stun_duration_ticks_down() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn(
                StunnedEnemy::new(1.0, 50.0),
            ).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(synapse_shock_stun_tick_system);

            let stunned = app.world().get::<StunnedEnemy>(entity).unwrap();
            assert!(!stunned.is_expired());

            // Advance more time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(synapse_shock_stun_tick_system);

            let stunned = app.world().get::<StunnedEnemy>(entity).unwrap();
            assert!(stunned.is_expired());
        }
    }

    mod synapse_shock_cleanup_stun_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_stun_removed_when_expired() {
            let mut app = App::new();

            // Create enemy with expired stun
            let mut expired_stun = StunnedEnemy::new(0.1, 50.0);
            expired_stun.tick(Duration::from_secs_f32(0.2));

            let entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                expired_stun,
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_cleanup_stun_system);

            // StunnedEnemy component should be removed
            assert!(
                app.world().get::<StunnedEnemy>(entity).is_none(),
                "StunnedEnemy should be removed when expired"
            );
            // Enemy should still exist
            assert!(
                app.world().get::<Enemy>(entity).is_some(),
                "Enemy should still exist after stun ends"
            );
        }

        #[test]
        fn test_stun_not_removed_if_active() {
            let mut app = App::new();

            // Create enemy with active stun
            let entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                StunnedEnemy::new(10.0, 50.0), // Long duration
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_cleanup_stun_system);

            // StunnedEnemy component should still exist
            assert!(
                app.world().get::<StunnedEnemy>(entity).is_some(),
                "StunnedEnemy should remain while active"
            );
        }
    }

    mod synapse_shock_cleanup_burst_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_burst_despawns_when_finished() {
            let mut app = App::new();

            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = SYNAPSE_SHOCK_MAX_RADIUS; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_cleanup_burst_system);

            // Burst should be despawned
            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_burst_survives_before_finished() {
            let mut app = App::new();

            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = SYNAPSE_SHOCK_MAX_RADIUS / 2.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_cleanup_burst_system);

            // Burst should still exist
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod fire_synapse_shock_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_synapse_shock_spawns_burst() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Telekinesis);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_synapse_shock(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 burst
            let mut query = app.world_mut().query::<&SynapseShockBurst>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_synapse_shock_at_player_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Telekinesis);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_synapse_shock(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SynapseShockBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_synapse_shock_with_custom_stun_duration() {
            let mut app = setup_test_app();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let custom_duration = 5.0;

            {
                let mut commands = app.world_mut().commands();
                fire_synapse_shock_with_stun_duration(
                    &mut commands,
                    spawn_pos,
                    custom_duration,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SynapseShockBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.stun_duration, custom_duration);
            }
        }

        #[test]
        fn test_fire_synapse_shock_starts_at_zero_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Telekinesis);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_synapse_shock(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SynapseShockBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.current_radius, 0.0);
            }
        }
    }

    mod synapse_shock_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut burst = SynapseShockBurst::new(Vec2::ZERO);
            burst.current_radius = 5.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(5.0));
        }

        #[test]
        fn test_visual_minimum_scale() {
            let mut app = App::new();

            let burst = SynapseShockBurst::new(Vec2::ZERO);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(synapse_shock_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }

    mod stun_indicator_tests {
        use super::*;

        #[test]
        fn test_stun_indicator_new() {
            let enemy_entity = Entity::from_bits(42);
            let indicator = StunIndicator::new(enemy_entity);
            assert_eq!(indicator.enemy_entity, enemy_entity);
        }

        #[test]
        fn test_stun_indicator_height_constant() {
            assert!(STUN_INDICATOR_HEIGHT > 0.0, "Height should be positive");
            assert_eq!(STUN_INDICATOR_HEIGHT, 2.5);
        }
    }

    mod spawn_stun_indicator_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_spawn_indicator_for_stunned_enemy() {
            let mut app = App::new();

            // Create a stunned enemy
            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.375, 10.0)),
                StunnedEnemy::new(2.0, 50.0),
            )).id();

            let _ = app.world_mut().run_system_once(spawn_stun_indicator_system);

            // Verify an indicator was spawned
            let mut indicator_query = app.world_mut().query::<&StunIndicator>();
            let indicators: Vec<_> = indicator_query.iter(app.world()).collect();
            assert_eq!(indicators.len(), 1, "Should spawn one indicator");
            assert_eq!(indicators[0].enemy_entity, enemy_entity);
        }

        #[test]
        fn test_spawn_indicator_position_above_enemy() {
            let mut app = App::new();

            // Create a stunned enemy at a specific position
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.375, 20.0)),
                StunnedEnemy::new(2.0, 50.0),
            ));

            let _ = app.world_mut().run_system_once(spawn_stun_indicator_system);

            // Verify indicator is positioned above enemy
            let mut query = app.world_mut().query::<(&StunIndicator, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation.x, 10.0);
                assert_eq!(transform.translation.y, 0.375 + STUN_INDICATOR_HEIGHT);
                assert_eq!(transform.translation.z, 20.0);
            }
        }

        #[test]
        fn test_no_duplicate_indicators() {
            let mut app = App::new();

            // Create a stunned enemy
            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.375, 10.0)),
                StunnedEnemy::new(2.0, 50.0),
            )).id();

            // Run the system twice
            let _ = app.world_mut().run_system_once(spawn_stun_indicator_system);
            let _ = app.world_mut().run_system_once(spawn_stun_indicator_system);

            // Should still only have one indicator
            let mut indicator_query = app.world_mut().query::<&StunIndicator>();
            let indicators: Vec<_> = indicator_query.iter(app.world()).collect();
            assert_eq!(indicators.len(), 1, "Should not create duplicate indicators");
            assert_eq!(indicators[0].enemy_entity, enemy_entity);
        }

        #[test]
        fn test_spawn_multiple_indicators_for_multiple_enemies() {
            let mut app = App::new();

            // Create multiple stunned enemies
            for i in 0..3 {
                app.world_mut().spawn((
                    Transform::from_translation(Vec3::new(i as f32 * 5.0, 0.375, 0.0)),
                    StunnedEnemy::new(2.0, 50.0),
                ));
            }

            let _ = app.world_mut().run_system_once(spawn_stun_indicator_system);

            // Should have 3 indicators
            let mut indicator_query = app.world_mut().query::<&StunIndicator>();
            let count = indicator_query.iter(app.world()).count();
            assert_eq!(count, 3, "Should spawn indicator for each stunned enemy");
        }
    }

    mod update_stun_indicator_position_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_indicator_follows_enemy_movement() {
            let mut app = App::new();

            // Create stunned enemy at initial position
            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
                StunnedEnemy::new(2.0, 50.0),
            )).id();

            // Create indicator for that enemy
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.375 + STUN_INDICATOR_HEIGHT, 0.0)),
                StunIndicator::new(enemy_entity),
            ));

            // Move the enemy
            app.world_mut().entity_mut(enemy_entity)
                .get_mut::<Transform>()
                .unwrap()
                .translation = Vec3::new(10.0, 0.375, 20.0);

            let _ = app.world_mut().run_system_once(update_stun_indicator_position_system);

            // Verify indicator moved with enemy
            let mut query = app.world_mut().query::<(&StunIndicator, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation.x, 10.0);
                assert_eq!(transform.translation.y, 0.375 + STUN_INDICATOR_HEIGHT);
                assert_eq!(transform.translation.z, 20.0);
            }
        }

        #[test]
        fn test_indicator_with_missing_enemy_unchanged() {
            let mut app = App::new();

            // Create indicator for non-existent enemy
            let fake_enemy = Entity::from_bits(9999);
            let indicator_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 5.0, 5.0)),
                StunIndicator::new(fake_enemy),
            )).id();

            let _ = app.world_mut().run_system_once(update_stun_indicator_position_system);

            // Indicator position should be unchanged
            let transform = app.world().get::<Transform>(indicator_entity).unwrap();
            assert_eq!(transform.translation, Vec3::new(5.0, 5.0, 5.0));
        }
    }

    mod cleanup_stun_indicator_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_indicator_removed_when_stun_ends() {
            let mut app = App::new();

            // Create an entity that WAS stunned (no longer has StunnedEnemy component)
            let former_enemy = app.world_mut().spawn(
                Transform::from_translation(Vec3::new(5.0, 0.375, 10.0)),
            ).id();

            // Create indicator that still references this enemy
            let indicator_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.375 + STUN_INDICATOR_HEIGHT, 10.0)),
                StunIndicator::new(former_enemy),
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_stun_indicator_system);

            // Indicator should be despawned
            assert!(
                app.world().get_entity(indicator_entity).is_err(),
                "Indicator should be removed when enemy is no longer stunned"
            );
        }

        #[test]
        fn test_indicator_remains_while_stunned() {
            let mut app = App::new();

            // Create a stunned enemy
            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.375, 10.0)),
                StunnedEnemy::new(10.0, 50.0), // Long stun
            )).id();

            // Create indicator for that enemy
            let indicator_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.375 + STUN_INDICATOR_HEIGHT, 10.0)),
                StunIndicator::new(enemy_entity),
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_stun_indicator_system);

            // Indicator should still exist
            assert!(
                app.world().get_entity(indicator_entity).is_ok(),
                "Indicator should remain while enemy is still stunned"
            );
        }

        #[test]
        fn test_cleanup_multiple_indicators() {
            let mut app = App::new();

            // Create mix of stunned and non-stunned enemies
            let stunned_enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                StunnedEnemy::new(10.0, 50.0),
            )).id();

            let unstunned_enemy = app.world_mut().spawn(
                Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ).id();

            // Create indicators for both
            let indicator1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, STUN_INDICATOR_HEIGHT, 0.0)),
                StunIndicator::new(stunned_enemy),
            )).id();

            let indicator2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, STUN_INDICATOR_HEIGHT, 0.0)),
                StunIndicator::new(unstunned_enemy),
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_stun_indicator_system);

            // Only the indicator for the stunned enemy should remain
            assert!(
                app.world().get_entity(indicator1).is_ok(),
                "Indicator for stunned enemy should remain"
            );
            assert!(
                app.world().get_entity(indicator2).is_err(),
                "Indicator for non-stunned enemy should be removed"
            );
        }
    }
}
