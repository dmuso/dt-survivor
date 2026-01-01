//! Psychic Scream spell - A large AOE burst that damages and disorients enemies.
//!
//! A Psychic element spell (PsychicShatter SpellType) that emanates a massive
//! shockwave from the player, damaging all enemies within a wide radius and
//! applying a disorientation effect that causes them to move erratically.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use rand::Rng;
use std::collections::HashSet;

/// Radius of the psychic scream AOE burst
pub const PSYCHIC_SCREAM_RADIUS: f32 = 12.0;

/// Duration in seconds for the scream to expand to full radius
pub const PSYCHIC_SCREAM_EXPANSION_DURATION: f32 = 0.4;

/// Duration of the disorientation effect in seconds
pub const DISORIENTATION_DURATION: f32 = 3.0;

/// Magnitude of random jitter applied to disoriented enemies
pub const DISORIENTATION_JITTER: f32 = 2.0;

/// Time between direction changes while disoriented
pub const DISORIENTATION_JITTER_INTERVAL: f32 = 0.2;

/// Height of the visual effect above ground
pub const PSYCHIC_SCREAM_VISUAL_HEIGHT: f32 = 0.3;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn psychic_scream_color() -> Color {
    Element::Psychic.color()
}

/// Component for the expanding psychic scream burst.
/// Tracks expansion state and which enemies have been hit.
#[derive(Component, Debug, Clone)]
pub struct PsychicScreamBurst {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Current radius of the expanding wave
    pub current_radius: f32,
    /// Maximum radius the wave will expand to
    pub max_radius: f32,
    /// Expansion rate in units per second
    pub expansion_rate: f32,
    /// Damage to deal to enemies as the wave passes through them
    pub damage: f32,
    /// Duration of disorientation to apply to affected enemies
    pub disorientation_duration: f32,
    /// Set of enemy entities already hit by this burst (prevents double damage)
    pub hit_enemies: HashSet<Entity>,
}

impl PsychicScreamBurst {
    /// Create a new psychic scream burst at the given center position.
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: PSYCHIC_SCREAM_RADIUS,
            expansion_rate: PSYCHIC_SCREAM_RADIUS / PSYCHIC_SCREAM_EXPANSION_DURATION,
            damage,
            disorientation_duration: DISORIENTATION_DURATION,
            hit_enemies: HashSet::new(),
        }
    }

    /// Create a psychic scream burst from a Spell component.
    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
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

    /// Check if an enemy at the given distance should be hit.
    /// Returns true if enemy is within the current wave radius and hasn't been hit yet.
    pub fn should_hit(&self, entity: Entity, distance: f32) -> bool {
        distance <= self.current_radius && !self.hit_enemies.contains(&entity)
    }

    /// Mark an enemy as hit by this burst.
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }
}

/// Component applied to enemies affected by the psychic scream.
/// Causes them to move erratically with random jitter.
#[derive(Component, Debug, Clone)]
pub struct DisorientedEnemy {
    /// Timer tracking remaining disorientation duration
    pub duration: Timer,
    /// Magnitude of random movement applied
    pub jitter: f32,
    /// Timer for periodic direction changes
    pub jitter_timer: Timer,
    /// Current jitter offset to apply to movement
    pub current_offset: Vec2,
}

impl DisorientedEnemy {
    /// Create a new disorientation effect with the given duration.
    pub fn new(duration: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration, TimerMode::Once),
            jitter: DISORIENTATION_JITTER,
            jitter_timer: Timer::from_seconds(DISORIENTATION_JITTER_INTERVAL, TimerMode::Repeating),
            current_offset: Vec2::ZERO,
        }
    }

    /// Check if the disorientation effect has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the disorientation timers.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.jitter_timer.tick(delta);
    }

    /// Check if it's time to change jitter direction.
    pub fn should_change_direction(&self) -> bool {
        self.jitter_timer.just_finished()
    }

    /// Generate a new random jitter offset.
    pub fn randomize_offset(&mut self) {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        self.current_offset = Vec2::new(angle.cos(), angle.sin()) * self.jitter;
    }

    /// Refresh the disorientation duration with a new timer.
    pub fn refresh(&mut self, duration: f32) {
        self.duration = Timer::from_seconds(duration, TimerMode::Once);
    }
}

/// System that expands psychic scream bursts over time.
pub fn psychic_scream_expansion_system(
    mut burst_query: Query<&mut PsychicScreamBurst>,
    time: Res<Time>,
) {
    for mut burst in burst_query.iter_mut() {
        burst.expand(time.delta_secs());
    }
}

/// System that checks for enemy collisions with the expanding wave
/// and applies damage and disorientation to enemies as the wave passes through them.
pub fn psychic_scream_collision_system(
    mut commands: Commands,
    mut burst_query: Query<&mut PsychicScreamBurst>,
    mut enemy_query: Query<(Entity, &Transform, Option<&mut DisorientedEnemy>), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut burst in burst_query.iter_mut() {
        for (enemy_entity, enemy_transform, existing_disorientation) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = burst.center.distance(enemy_pos);

            if burst.should_hit(enemy_entity, distance) {
                // Apply damage
                damage_events.write(DamageEvent::new(enemy_entity, burst.damage));

                // Apply or refresh disorientation
                if let Some(mut disoriented) = existing_disorientation {
                    disoriented.refresh(burst.disorientation_duration);
                } else {
                    commands.entity(enemy_entity).try_insert(
                        DisorientedEnemy::new(burst.disorientation_duration)
                    );
                }

                burst.mark_hit(enemy_entity);
            }
        }
    }
}

/// System that updates disoriented enemy movement with random jitter.
pub fn update_disoriented_enemies_system(
    mut query: Query<&mut DisorientedEnemy>,
    time: Res<Time>,
) {
    for mut disoriented in query.iter_mut() {
        disoriented.tick(time.delta());

        if disoriented.should_change_direction() {
            disoriented.randomize_offset();
        }
    }
}

/// System that applies jitter movement to disoriented enemies.
/// This modifies enemy movement to be erratic.
pub fn apply_disoriented_movement_system(
    mut query: Query<(&DisorientedEnemy, &mut Transform)>,
    time: Res<Time>,
) {
    for (disoriented, mut transform) in query.iter_mut() {
        // Apply jitter offset as additional movement
        let offset_3d = Vec3::new(
            disoriented.current_offset.x,
            0.0,
            disoriented.current_offset.y,
        );
        transform.translation += offset_3d * time.delta_secs();
    }
}

/// System that removes expired disorientation effects from enemies.
pub fn cleanup_disorientation_system(
    mut commands: Commands,
    query: Query<(Entity, &DisorientedEnemy)>,
) {
    for (entity, disoriented) in query.iter() {
        if disoriented.is_expired() {
            commands.entity(entity).remove::<DisorientedEnemy>();
        }
    }
}

/// System that despawns psychic scream bursts when they finish expanding.
pub fn psychic_scream_cleanup_system(
    mut commands: Commands,
    burst_query: Query<(Entity, &PsychicScreamBurst)>,
) {
    for (entity, burst) in burst_query.iter() {
        if burst.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual scale of psychic scream bursts based on their current radius.
pub fn psychic_scream_visual_system(
    mut burst_query: Query<(&PsychicScreamBurst, &mut Transform)>,
) {
    for (burst, mut transform) in burst_query.iter_mut() {
        // Scale the visual to match current radius
        transform.scale = Vec3::splat(burst.current_radius.max(0.1));
    }
}

/// Cast psychic scream (PsychicShatter) spell - spawns a large AOE burst of mental energy.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_psychic_scream(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_psychic_scream_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast psychic scream spell with explicit damage - spawns a large AOE burst of mental energy.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_psychic_scream_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let burst = PsychicScreamBurst::new(center, damage);
    let burst_pos = Vec3::new(spawn_position.x, PSYCHIC_SCREAM_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.powerup.clone()), // Magenta/pink material for psychic
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

    mod psychic_scream_burst_tests {
        use super::*;

        #[test]
        fn test_psychic_scream_burst_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 35.0;
            let burst = PsychicScreamBurst::new(center, damage);

            assert_eq!(burst.center, center);
            assert_eq!(burst.damage, damage);
            assert_eq!(burst.current_radius, 0.0);
            assert_eq!(burst.max_radius, PSYCHIC_SCREAM_RADIUS);
            assert!(!burst.is_finished());
            assert!(burst.hit_enemies.is_empty());
        }

        #[test]
        fn test_psychic_scream_burst_from_spell() {
            let spell = Spell::new(SpellType::PsychicShatter);
            let center = Vec2::new(5.0, 15.0);
            let burst = PsychicScreamBurst::from_spell(center, &spell);

            assert_eq!(burst.center, center);
            assert_eq!(burst.damage, spell.damage());
        }

        #[test]
        fn test_psychic_scream_burst_expand() {
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);

            burst.expand(PSYCHIC_SCREAM_EXPANSION_DURATION / 2.0);
            assert!(
                (burst.current_radius - PSYCHIC_SCREAM_RADIUS / 2.0).abs() < 0.01,
                "Radius should be half of max after half duration"
            );

            burst.expand(PSYCHIC_SCREAM_EXPANSION_DURATION / 2.0);
            assert!(
                (burst.current_radius - PSYCHIC_SCREAM_RADIUS).abs() < 0.01,
                "Radius should be max after full duration"
            );
        }

        #[test]
        fn test_psychic_scream_burst_expand_caps_at_max() {
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);

            // Expand way past max
            burst.expand(PSYCHIC_SCREAM_EXPANSION_DURATION * 10.0);

            assert_eq!(burst.current_radius, PSYCHIC_SCREAM_RADIUS);
        }

        #[test]
        fn test_psychic_scream_burst_is_finished() {
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            assert!(!burst.is_finished());

            burst.current_radius = PSYCHIC_SCREAM_RADIUS;
            assert!(burst.is_finished());
        }

        #[test]
        fn test_psychic_scream_burst_should_hit() {
            let burst = PsychicScreamBurst {
                center: Vec2::ZERO,
                current_radius: 8.0,
                max_radius: 12.0,
                expansion_rate: 30.0,
                damage: 35.0,
                disorientation_duration: 3.0,
                hit_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(burst.should_hit(entity, 5.0), "Should hit enemy within radius");
            assert!(burst.should_hit(entity, 8.0), "Should hit enemy at radius edge");
            assert!(!burst.should_hit(entity, 9.0), "Should not hit enemy outside radius");
        }

        #[test]
        fn test_psychic_scream_burst_should_hit_excludes_already_hit() {
            let mut burst = PsychicScreamBurst {
                center: Vec2::ZERO,
                current_radius: 8.0,
                max_radius: 12.0,
                expansion_rate: 30.0,
                damage: 35.0,
                disorientation_duration: 3.0,
                hit_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(burst.should_hit(entity, 5.0));

            burst.mark_hit(entity);
            assert!(!burst.should_hit(entity, 5.0), "Should not hit already-hit enemy");
        }

        #[test]
        fn test_psychic_scream_burst_mark_hit() {
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);

            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);

            burst.mark_hit(entity1);
            assert!(burst.hit_enemies.contains(&entity1));
            assert!(!burst.hit_enemies.contains(&entity2));

            burst.mark_hit(entity2);
            assert!(burst.hit_enemies.contains(&entity1));
            assert!(burst.hit_enemies.contains(&entity2));
        }

        #[test]
        fn test_psychic_scream_uses_psychic_element_color() {
            let color = psychic_scream_color();
            assert_eq!(color, Element::Psychic.color());
        }

        #[test]
        fn test_psychic_scream_large_radius() {
            // Verify psychic scream has a large radius compared to other AOE spells
            assert!(
                PSYCHIC_SCREAM_RADIUS >= 10.0,
                "Psychic Scream should have a large AOE radius (at least 10 units)"
            );
        }
    }

    mod disoriented_enemy_tests {
        use super::*;

        #[test]
        fn test_disoriented_enemy_new() {
            let disoriented = DisorientedEnemy::new(3.0);

            assert_eq!(disoriented.jitter, DISORIENTATION_JITTER);
            assert_eq!(disoriented.current_offset, Vec2::ZERO);
            assert!(!disoriented.is_expired());
        }

        #[test]
        fn test_disoriented_enemy_is_expired() {
            let mut disoriented = DisorientedEnemy::new(0.1);
            assert!(!disoriented.is_expired());

            disoriented.tick(Duration::from_secs_f32(0.2));
            assert!(disoriented.is_expired());
        }

        #[test]
        fn test_disoriented_enemy_tick() {
            let mut disoriented = DisorientedEnemy::new(1.0);

            disoriented.tick(Duration::from_secs_f32(0.5));
            assert!(!disoriented.is_expired());

            disoriented.tick(Duration::from_secs_f32(0.5));
            assert!(disoriented.is_expired());
        }

        #[test]
        fn test_disoriented_enemy_refresh() {
            let mut disoriented = DisorientedEnemy::new(1.0);
            disoriented.tick(Duration::from_secs_f32(0.9));

            // About to expire, but refresh
            disoriented.refresh(2.0);

            assert!(!disoriented.is_expired());
        }

        #[test]
        fn test_disoriented_enemy_should_change_direction() {
            let mut disoriented = DisorientedEnemy::new(5.0);

            // Should not change direction immediately
            assert!(!disoriented.should_change_direction());

            // Tick past jitter interval
            disoriented.tick(Duration::from_secs_f32(DISORIENTATION_JITTER_INTERVAL));

            // Now should change direction
            assert!(disoriented.should_change_direction());
        }

        #[test]
        fn test_disoriented_enemy_randomize_offset() {
            let mut disoriented = DisorientedEnemy::new(5.0);
            assert_eq!(disoriented.current_offset, Vec2::ZERO);

            disoriented.randomize_offset();

            // After randomizing, offset should have magnitude approximately equal to jitter
            let magnitude = disoriented.current_offset.length();
            assert!(
                (magnitude - DISORIENTATION_JITTER).abs() < 0.01,
                "Offset magnitude should equal jitter value"
            );
        }

        #[test]
        fn test_disoriented_movement_is_erratic() {
            // Test that multiple randomizations produce different offsets
            let mut disoriented = DisorientedEnemy::new(5.0);
            let mut offsets = Vec::new();

            for _ in 0..5 {
                disoriented.randomize_offset();
                offsets.push(disoriented.current_offset);
            }

            // Check that at least some offsets are different (highly likely with randomness)
            let unique_offsets: HashSet<_> = offsets.iter()
                .map(|o| (o.x.to_bits(), o.y.to_bits()))
                .collect();

            // With 5 samples, we should have at least 2 different offsets
            // (probability of all same is astronomically low)
            assert!(
                unique_offsets.len() >= 2,
                "Multiple randomizations should produce varied offsets"
            );
        }

        #[test]
        fn test_disorientation_duration_expires() {
            let mut disoriented = DisorientedEnemy::new(DISORIENTATION_DURATION);

            // Tick through the full duration
            disoriented.tick(Duration::from_secs_f32(DISORIENTATION_DURATION));

            assert!(disoriented.is_expired());
        }

        #[test]
        fn test_disorientation_restores_normal_movement() {
            // The cleanup system should remove DisorientedEnemy when expired
            // This test verifies the expiration check works
            let mut disoriented = DisorientedEnemy::new(0.1);
            disoriented.tick(Duration::from_secs_f32(0.2));

            assert!(
                disoriented.is_expired(),
                "Disorientation should be marked expired after duration"
            );
        }
    }

    mod psychic_scream_expansion_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_psychic_scream_expands_over_time() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PsychicScreamBurst::new(Vec2::ZERO, 35.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(PSYCHIC_SCREAM_EXPANSION_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(psychic_scream_expansion_system);

            let burst = app.world().get::<PsychicScreamBurst>(entity).unwrap();
            assert!(
                (burst.current_radius - PSYCHIC_SCREAM_RADIUS / 2.0).abs() < 0.5,
                "Radius should be approximately half after half duration: got {}",
                burst.current_radius
            );
        }

        #[test]
        fn test_psychic_scream_multiple_bursts_expand_independently() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create two bursts with different starting radii
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PsychicScreamBurst::new(Vec2::ZERO, 35.0),
            )).id();

            let mut burst2 = PsychicScreamBurst::new(Vec2::new(10.0, 10.0), 30.0);
            burst2.current_radius = 5.0; // Pre-expanded
            let entity2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                burst2,
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(psychic_scream_expansion_system);

            let burst1 = app.world().get::<PsychicScreamBurst>(entity1).unwrap();
            let burst2 = app.world().get::<PsychicScreamBurst>(entity2).unwrap();

            // Both should have expanded but from different starting points
            assert!(burst1.current_radius > 0.0);
            assert!(burst2.current_radius > 5.0);
        }
    }

    mod psychic_scream_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_psychic_scream_damages_enemy_in_radius() {
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
            app.add_systems(Update, (psychic_scream_collision_system, count_damage_events).chain());

            // Create burst at origin with radius 8.0
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 8.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy within radius (XZ distance = 5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_psychic_scream_no_damage_outside_radius() {
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
            app.add_systems(Update, (psychic_scream_collision_system, count_damage_events).chain());

            // Create burst at origin with radius 5.0
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy outside radius (XZ distance = 10)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_psychic_scream_damages_enemy_only_once() {
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
            app.add_systems(Update, (psychic_scream_collision_system, count_damage_events).chain());

            // Create burst
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 8.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy in radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Should only damage once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_psychic_scream_damages_multiple_enemies() {
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
            app.add_systems(Update, (psychic_scream_collision_system, count_damage_events).chain());

            // Create burst
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 10.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create 5 enemies in radius
            for i in 0..5 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 * 2.0, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 5);
        }

        #[test]
        fn test_psychic_scream_disorients_enemies() {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, psychic_scream_collision_system);

            // Create burst at origin
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 8.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy in range
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should have DisorientedEnemy component
            let disoriented = app.world().get::<DisorientedEnemy>(enemy);
            assert!(disoriented.is_some(), "Enemy should have DisorientedEnemy component");
        }
    }

    mod psychic_scream_cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_psychic_scream_despawns_when_finished() {
            let mut app = App::new();

            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = PSYCHIC_SCREAM_RADIUS; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psychic_scream_cleanup_system);

            // Burst should be despawned
            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_psychic_scream_survives_before_finished() {
            let mut app = App::new();

            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = PSYCHIC_SCREAM_RADIUS / 2.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psychic_scream_cleanup_system);

            // Burst should still exist
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod cleanup_disorientation_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_cleanup_removes_expired_disorientation() {
            let mut app = App::new();

            // Create enemy with expired disorientation
            let mut disoriented = DisorientedEnemy::new(0.0);
            disoriented.duration.tick(Duration::from_secs(1)); // Force expire

            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                disoriented,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_disorientation_system);

            // DisorientedEnemy should be removed
            assert!(app.world().get::<DisorientedEnemy>(enemy).is_none());
            // Enemy should still exist
            assert!(app.world().get::<Enemy>(enemy).is_some());
        }

        #[test]
        fn test_cleanup_keeps_active_disorientation() {
            let mut app = App::new();

            // Create enemy with active disorientation
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                DisorientedEnemy::new(5.0), // Long duration
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_disorientation_system);

            // DisorientedEnemy should still be present
            assert!(app.world().get::<DisorientedEnemy>(enemy).is_some());
        }
    }

    mod fire_psychic_scream_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_psychic_scream_spawns_burst() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicShatter);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psychic_scream(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 burst
            let mut query = app.world_mut().query::<&PsychicScreamBurst>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_psychic_scream_at_player_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicShatter);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psychic_scream(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsychicScreamBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_psychic_scream_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicShatter);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psychic_scream(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsychicScreamBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_psychic_scream_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicShatter);
            let explicit_damage = 150.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psychic_scream_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsychicScreamBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_psychic_scream_starts_at_zero_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicShatter);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psychic_scream(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsychicScreamBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.current_radius, 0.0);
            }
        }

        #[test]
        fn test_psychic_scream_creates_burst() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicShatter);

            {
                let mut commands = app.world_mut().commands();
                fire_psychic_scream(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsychicScreamBurst>();
            assert_eq!(query.iter(app.world()).count(), 1);
        }
    }

    mod psychic_scream_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_psychic_scream_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 6.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psychic_scream_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(6.0));
        }

        #[test]
        fn test_psychic_scream_visual_minimum_scale() {
            let mut app = App::new();

            let burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psychic_scream_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }

    mod psychic_scream_enemies_outside_radius_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_psychic_scream_enemies_outside_radius_not_affected() {
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
            app.add_systems(Update, (psychic_scream_collision_system, count_damage_events).chain());

            // Create burst with small current radius
            let mut burst = PsychicScreamBurst::new(Vec2::ZERO, 35.0);
            burst.current_radius = 3.0; // Small radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy inside radius
            let inside_enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)), // Distance = 2
            )).id();

            // Create enemy outside radius
            let outside_enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)), // Distance = 10
            )).id();

            app.update();

            // Only inside enemy should be damaged
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);

            // Only inside enemy should be disoriented
            assert!(app.world().get::<DisorientedEnemy>(inside_enemy).is_some());
            assert!(app.world().get::<DisorientedEnemy>(outside_enemy).is_none());
        }
    }
}
