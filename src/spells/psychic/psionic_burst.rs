//! Psionic Burst spell - An expanding wave of mental energy.
//!
//! A Psychic element spell (PsychicWave SpellType) that emanates a nova-style
//! expanding ring from the player, damaging all enemies it contacts as it
//! expands outward. Each enemy is only damaged once per burst.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Maximum radius the psionic burst will expand to
pub const PSIONIC_BURST_MAX_RADIUS: f32 = 10.0;

/// Duration in seconds for the burst to fully expand
pub const PSIONIC_BURST_EXPANSION_DURATION: f32 = 0.5;

/// Height of the visual effect above ground
pub const PSIONIC_BURST_VISUAL_HEIGHT: f32 = 0.3;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn psionic_burst_color() -> Color {
    Element::Psychic.color()
}

/// Component for the expanding psionic burst ring.
/// Tracks expansion state and which enemies have been hit.
#[derive(Component, Debug, Clone)]
pub struct PsionicBurstWave {
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
    /// Set of enemy entities already hit by this burst (prevents double damage)
    pub hit_enemies: HashSet<Entity>,
}

impl PsionicBurstWave {
    /// Create a new psionic burst at the given center position.
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: PSIONIC_BURST_MAX_RADIUS,
            expansion_rate: PSIONIC_BURST_MAX_RADIUS / PSIONIC_BURST_EXPANSION_DURATION,
            damage,
            hit_enemies: HashSet::new(),
        }
    }

    /// Create a psionic burst from a Spell component.
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

/// System that expands psionic burst waves over time.
pub fn psionic_burst_expansion_system(
    mut burst_query: Query<&mut PsionicBurstWave>,
    time: Res<Time>,
) {
    for mut burst in burst_query.iter_mut() {
        burst.expand(time.delta_secs());
    }
}

/// System that checks for enemy collisions with the expanding wave
/// and applies damage to enemies as the wave passes through them.
pub fn psionic_burst_collision_system(
    mut burst_query: Query<&mut PsionicBurstWave>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut burst in burst_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = burst.center.distance(enemy_pos);

            if burst.should_hit(enemy_entity, distance) {
                damage_events.write(DamageEvent::new(enemy_entity, burst.damage));
                burst.mark_hit(enemy_entity);
            }
        }
    }
}

/// System that despawns psionic bursts when they finish expanding.
pub fn psionic_burst_cleanup_system(
    mut commands: Commands,
    burst_query: Query<(Entity, &PsionicBurstWave)>,
) {
    for (entity, burst) in burst_query.iter() {
        if burst.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual scale of psionic bursts based on their current radius.
pub fn psionic_burst_visual_system(
    mut burst_query: Query<(&PsionicBurstWave, &mut Transform)>,
) {
    for (burst, mut transform) in burst_query.iter_mut() {
        // Scale the visual to match current radius
        transform.scale = Vec3::splat(burst.current_radius.max(0.1));
    }
}

/// Cast psionic burst (PsychicWave) spell - spawns an expanding wave of mental energy.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_psionic_burst(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_psionic_burst_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast psionic burst spell with explicit damage - spawns an expanding wave of mental energy.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_psionic_burst_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let burst = PsionicBurstWave::new(center, damage);
    let burst_pos = Vec3::new(spawn_position.x, PSIONIC_BURST_VISUAL_HEIGHT, spawn_position.z);

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

    mod psionic_burst_wave_tests {
        use super::*;

        #[test]
        fn test_psionic_burst_wave_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 25.0;
            let burst = PsionicBurstWave::new(center, damage);

            assert_eq!(burst.center, center);
            assert_eq!(burst.damage, damage);
            assert_eq!(burst.current_radius, 0.0);
            assert_eq!(burst.max_radius, PSIONIC_BURST_MAX_RADIUS);
            assert!(!burst.is_finished());
            assert!(burst.hit_enemies.is_empty());
        }

        #[test]
        fn test_psionic_burst_wave_from_spell() {
            let spell = Spell::new(SpellType::PsychicWave);
            let center = Vec2::new(5.0, 15.0);
            let burst = PsionicBurstWave::from_spell(center, &spell);

            assert_eq!(burst.center, center);
            assert_eq!(burst.damage, spell.damage());
        }

        #[test]
        fn test_psionic_burst_wave_expand() {
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);

            burst.expand(PSIONIC_BURST_EXPANSION_DURATION / 2.0);
            assert!(
                (burst.current_radius - PSIONIC_BURST_MAX_RADIUS / 2.0).abs() < 0.01,
                "Radius should be half of max after half duration"
            );

            burst.expand(PSIONIC_BURST_EXPANSION_DURATION / 2.0);
            assert!(
                (burst.current_radius - PSIONIC_BURST_MAX_RADIUS).abs() < 0.01,
                "Radius should be max after full duration"
            );
        }

        #[test]
        fn test_psionic_burst_wave_expand_caps_at_max() {
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);

            // Expand way past max
            burst.expand(PSIONIC_BURST_EXPANSION_DURATION * 10.0);

            assert_eq!(burst.current_radius, PSIONIC_BURST_MAX_RADIUS);
        }

        #[test]
        fn test_psionic_burst_wave_is_finished() {
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            assert!(!burst.is_finished());

            burst.current_radius = PSIONIC_BURST_MAX_RADIUS;
            assert!(burst.is_finished());
        }

        #[test]
        fn test_psionic_burst_wave_should_hit() {
            let burst = PsionicBurstWave {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                damage: 25.0,
                hit_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(burst.should_hit(entity, 3.0), "Should hit enemy within radius");
            assert!(burst.should_hit(entity, 5.0), "Should hit enemy at radius edge");
            assert!(!burst.should_hit(entity, 6.0), "Should not hit enemy outside radius");
        }

        #[test]
        fn test_psionic_burst_wave_should_hit_excludes_already_hit() {
            let mut burst = PsionicBurstWave {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                damage: 25.0,
                hit_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(burst.should_hit(entity, 3.0));

            burst.mark_hit(entity);
            assert!(!burst.should_hit(entity, 3.0), "Should not hit already-hit enemy");
        }

        #[test]
        fn test_psionic_burst_wave_mark_hit() {
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);

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
        fn test_psionic_burst_uses_psychic_element_color() {
            let color = psionic_burst_color();
            assert_eq!(color, Element::Psychic.color());
        }
    }

    mod psionic_burst_expansion_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_psionic_burst_expands_over_time() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PsionicBurstWave::new(Vec2::ZERO, 25.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(PSIONIC_BURST_EXPANSION_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(psionic_burst_expansion_system);

            let burst = app.world().get::<PsionicBurstWave>(entity).unwrap();
            assert!(
                (burst.current_radius - PSIONIC_BURST_MAX_RADIUS / 2.0).abs() < 0.1,
                "Radius should be approximately half after half duration: got {}",
                burst.current_radius
            );
        }

        #[test]
        fn test_psionic_burst_multiple_waves_expand_independently() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create two bursts with different starting radii
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PsionicBurstWave::new(Vec2::ZERO, 25.0),
            )).id();

            let mut burst2 = PsionicBurstWave::new(Vec2::new(10.0, 10.0), 20.0);
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

            let _ = app.world_mut().run_system_once(psionic_burst_expansion_system);

            let burst1 = app.world().get::<PsionicBurstWave>(entity1).unwrap();
            let burst2 = app.world().get::<PsionicBurstWave>(entity2).unwrap();

            // Both should have expanded but from different starting points
            assert!(burst1.current_radius > 0.0);
            assert!(burst2.current_radius > 3.0);
        }
    }

    mod psionic_burst_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_psionic_burst_damages_enemy_in_radius() {
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
            app.add_systems(Update, (psionic_burst_collision_system, count_damage_events).chain());

            // Create burst at origin with radius 5.0
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy within radius (XZ distance = 3)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_psionic_burst_no_damage_outside_radius() {
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
            app.add_systems(Update, (psionic_burst_collision_system, count_damage_events).chain());

            // Create burst at origin with radius 3.0
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = 3.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy outside radius (XZ distance = 5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_psionic_burst_damages_enemy_only_once() {
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
            app.add_systems(Update, (psionic_burst_collision_system, count_damage_events).chain());

            // Create burst
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy in radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Should only damage once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_psionic_burst_damages_multiple_enemies() {
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
            app.add_systems(Update, (psionic_burst_collision_system, count_damage_events).chain());

            // Create burst
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create 3 enemies in radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn test_psionic_burst_uses_xz_plane_ignores_y() {
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
            app.add_systems(Update, (psionic_burst_collision_system, count_damage_events).chain());

            // Create burst at origin
            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            ));

            // Create enemy close on XZ plane but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }
    }

    mod psionic_burst_cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_psionic_burst_despawns_when_finished() {
            let mut app = App::new();

            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = PSIONIC_BURST_MAX_RADIUS; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psionic_burst_cleanup_system);

            // Burst should be despawned
            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_psionic_burst_survives_before_finished() {
            let mut app = App::new();

            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = PSIONIC_BURST_MAX_RADIUS / 2.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psionic_burst_cleanup_system);

            // Burst should still exist
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod fire_psionic_burst_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_psionic_burst_spawns_wave() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicWave);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psionic_burst(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 burst
            let mut query = app.world_mut().query::<&PsionicBurstWave>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_psionic_burst_at_player_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicWave);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psionic_burst(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsionicBurstWave>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_psionic_burst_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicWave);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psionic_burst(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsionicBurstWave>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_psionic_burst_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicWave);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psionic_burst_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsionicBurstWave>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_psionic_burst_starts_at_zero_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PsychicWave);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_psionic_burst(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PsionicBurstWave>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.current_radius, 0.0);
            }
        }
    }

    mod psionic_burst_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_psionic_burst_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            burst.current_radius = 5.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psionic_burst_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(5.0));
        }

        #[test]
        fn test_psionic_burst_visual_minimum_scale() {
            let mut app = App::new();

            let burst = PsionicBurstWave::new(Vec2::ZERO, 25.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(psionic_burst_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }
}
