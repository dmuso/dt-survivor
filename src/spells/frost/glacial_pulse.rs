//! Glacial Pulse spell - Expanding frost wave that slows and weakens enemies.
//!
//! A Frost element spell (FrostNova SpellType) that creates an expanding ring
//! of icy energy centered on the player. Enemies caught in the wave take damage
//! and receive both Slowed (reduced movement) and Weakened (increased damage taken) debuffs.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::fire::cinder_shot::WeakenedDebuff;
use crate::spells::frost::ice_shard::SlowedDebuff;

/// Default configuration for Glacial Pulse spell
pub const GLACIAL_PULSE_MAX_RADIUS: f32 = 10.0;
pub const GLACIAL_PULSE_EXPANSION_DURATION: f32 = 0.5;
pub const GLACIAL_PULSE_VISUAL_HEIGHT: f32 = 0.2;

/// Debuff configuration for Glacial Pulse
pub const GLACIAL_PULSE_SLOW_DURATION: f32 = 3.0;
pub const GLACIAL_PULSE_SLOW_MULTIPLIER: f32 = 0.4; // 60% speed reduction
pub const GLACIAL_PULSE_WEAKEN_DURATION: f32 = 4.0;
pub const GLACIAL_PULSE_WEAKEN_MULTIPLIER: f32 = 1.3; // 30% more damage taken

/// Get the frost element color for visual effects
pub fn glacial_pulse_color() -> Color {
    Element::Frost.color()
}

/// Component for the expanding glacial pulse ring.
/// Tracks expansion state and which enemies have been hit.
#[derive(Component, Debug, Clone)]
pub struct GlacialPulseWave {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Current radius of the ring
    pub current_radius: f32,
    /// Maximum radius the ring will expand to
    pub max_radius: f32,
    /// Expansion rate in units per second
    pub expansion_rate: f32,
    /// Damage to deal to enemies as the ring passes through them
    pub damage: f32,
    /// Set of enemy entities already hit by this pulse (prevents double damage)
    pub hit_enemies: HashSet<Entity>,
    /// Duration of slow effect to apply
    pub slow_duration: f32,
    /// Speed multiplier for slow effect
    pub slow_multiplier: f32,
    /// Duration of weaken effect to apply
    pub weaken_duration: f32,
    /// Damage multiplier for weaken effect
    pub weaken_multiplier: f32,
}

impl GlacialPulseWave {
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: GLACIAL_PULSE_MAX_RADIUS,
            expansion_rate: GLACIAL_PULSE_MAX_RADIUS / GLACIAL_PULSE_EXPANSION_DURATION,
            damage,
            hit_enemies: HashSet::new(),
            slow_duration: GLACIAL_PULSE_SLOW_DURATION,
            slow_multiplier: GLACIAL_PULSE_SLOW_MULTIPLIER,
            weaken_duration: GLACIAL_PULSE_WEAKEN_DURATION,
            weaken_multiplier: GLACIAL_PULSE_WEAKEN_MULTIPLIER,
        }
    }

    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the pulse has finished expanding
    pub fn is_finished(&self) -> bool {
        self.current_radius >= self.max_radius
    }

    /// Expand the ring by the given delta time
    pub fn expand(&mut self, delta_secs: f32) {
        self.current_radius = (self.current_radius + self.expansion_rate * delta_secs)
            .min(self.max_radius);
    }

    /// Check if an enemy at the given distance should be hit.
    /// Returns true if enemy is within the current ring radius and hasn't been hit yet.
    pub fn should_hit(&self, entity: Entity, distance: f32) -> bool {
        distance <= self.current_radius && !self.hit_enemies.contains(&entity)
    }

    /// Mark an enemy as hit
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }
}

/// System that expands glacial pulse waves over time
pub fn glacial_pulse_expansion_system(
    mut pulse_query: Query<&mut GlacialPulseWave>,
    time: Res<Time>,
) {
    for mut pulse in pulse_query.iter_mut() {
        pulse.expand(time.delta_secs());
    }
}

/// System that checks for enemy collisions with the expanding ring
/// and applies damage and debuffs to enemies as the ring passes through them
pub fn glacial_pulse_collision_system(
    mut commands: Commands,
    mut pulse_query: Query<&mut GlacialPulseWave>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut pulse in pulse_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = pulse.center.distance(enemy_pos);

            if pulse.should_hit(enemy_entity, distance) {
                // Apply damage
                damage_events.write(DamageEvent::new(enemy_entity, pulse.damage));

                // Apply SlowedDebuff
                commands.entity(enemy_entity).try_insert(
                    SlowedDebuff::new(pulse.slow_duration, pulse.slow_multiplier)
                );

                // Apply WeakenedDebuff
                commands.entity(enemy_entity).try_insert(
                    WeakenedDebuff::new(pulse.weaken_duration, pulse.weaken_multiplier)
                );

                pulse.mark_hit(enemy_entity);
            }
        }
    }
}

/// System that despawns glacial pulses when they finish expanding
pub fn glacial_pulse_cleanup_system(
    mut commands: Commands,
    pulse_query: Query<(Entity, &GlacialPulseWave)>,
) {
    for (entity, pulse) in pulse_query.iter() {
        if pulse.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast glacial pulse (Frost Nova) spell - spawns an expanding ring of frost.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_glacial_pulse(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_glacial_pulse_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast glacial pulse (Frost Nova) spell with explicit damage - spawns an expanding ring of frost.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_glacial_pulse_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let pulse = GlacialPulseWave::new(center, damage);
    let pulse_pos = Vec3::new(spawn_position.x, GLACIAL_PULSE_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.glacial_pulse.clone()),
            Transform::from_translation(pulse_pos).with_scale(Vec3::splat(0.1)),
            pulse,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(pulse_pos),
            pulse,
        ));
    }
}

/// System that updates the visual scale of glacial pulses based on their current radius
pub fn glacial_pulse_visual_system(
    mut pulse_query: Query<(&GlacialPulseWave, &mut Transform)>,
) {
    for (pulse, mut transform) in pulse_query.iter_mut() {
        // Scale the visual to match current radius
        transform.scale = Vec3::splat(pulse.current_radius.max(0.1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod glacial_pulse_wave_tests {
        use super::*;

        #[test]
        fn test_glacial_pulse_wave_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 30.0;
            let pulse = GlacialPulseWave::new(center, damage);

            assert_eq!(pulse.center, center);
            assert_eq!(pulse.damage, damage);
            assert_eq!(pulse.current_radius, 0.0);
            assert_eq!(pulse.max_radius, GLACIAL_PULSE_MAX_RADIUS);
            assert!(!pulse.is_finished());
            assert!(pulse.hit_enemies.is_empty());
            assert_eq!(pulse.slow_duration, GLACIAL_PULSE_SLOW_DURATION);
            assert_eq!(pulse.slow_multiplier, GLACIAL_PULSE_SLOW_MULTIPLIER);
            assert_eq!(pulse.weaken_duration, GLACIAL_PULSE_WEAKEN_DURATION);
            assert_eq!(pulse.weaken_multiplier, GLACIAL_PULSE_WEAKEN_MULTIPLIER);
        }

        #[test]
        fn test_glacial_pulse_wave_from_spell() {
            let spell = Spell::new(SpellType::FrostNova);
            let center = Vec2::new(5.0, 15.0);
            let pulse = GlacialPulseWave::from_spell(center, &spell);

            assert_eq!(pulse.center, center);
            assert_eq!(pulse.damage, spell.damage());
        }

        #[test]
        fn test_glacial_pulse_wave_expand() {
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);

            pulse.expand(GLACIAL_PULSE_EXPANSION_DURATION / 2.0);
            assert!(
                (pulse.current_radius - GLACIAL_PULSE_MAX_RADIUS / 2.0).abs() < 0.01,
                "Radius should be half of max after half duration"
            );

            pulse.expand(GLACIAL_PULSE_EXPANSION_DURATION / 2.0);
            assert!(
                (pulse.current_radius - GLACIAL_PULSE_MAX_RADIUS).abs() < 0.01,
                "Radius should be max after full duration"
            );
        }

        #[test]
        fn test_glacial_pulse_wave_expand_caps_at_max() {
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);

            // Expand way past max
            pulse.expand(GLACIAL_PULSE_EXPANSION_DURATION * 10.0);

            assert_eq!(pulse.current_radius, GLACIAL_PULSE_MAX_RADIUS);
        }

        #[test]
        fn test_glacial_pulse_wave_is_finished() {
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            assert!(!pulse.is_finished());

            pulse.current_radius = GLACIAL_PULSE_MAX_RADIUS;
            assert!(pulse.is_finished());
        }

        #[test]
        fn test_glacial_pulse_wave_should_hit() {
            let pulse = GlacialPulseWave {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                damage: 30.0,
                hit_enemies: HashSet::new(),
                slow_duration: GLACIAL_PULSE_SLOW_DURATION,
                slow_multiplier: GLACIAL_PULSE_SLOW_MULTIPLIER,
                weaken_duration: GLACIAL_PULSE_WEAKEN_DURATION,
                weaken_multiplier: GLACIAL_PULSE_WEAKEN_MULTIPLIER,
            };

            let entity = Entity::from_bits(1);
            assert!(pulse.should_hit(entity, 3.0), "Should hit enemy within radius");
            assert!(pulse.should_hit(entity, 5.0), "Should hit enemy at radius edge");
            assert!(!pulse.should_hit(entity, 6.0), "Should not hit enemy outside radius");
        }

        #[test]
        fn test_glacial_pulse_wave_should_hit_excludes_already_hit() {
            let mut pulse = GlacialPulseWave {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                damage: 30.0,
                hit_enemies: HashSet::new(),
                slow_duration: GLACIAL_PULSE_SLOW_DURATION,
                slow_multiplier: GLACIAL_PULSE_SLOW_MULTIPLIER,
                weaken_duration: GLACIAL_PULSE_WEAKEN_DURATION,
                weaken_multiplier: GLACIAL_PULSE_WEAKEN_MULTIPLIER,
            };

            let entity = Entity::from_bits(1);
            assert!(pulse.should_hit(entity, 3.0));

            pulse.mark_hit(entity);
            assert!(!pulse.should_hit(entity, 3.0), "Should not hit already-hit enemy");
        }

        #[test]
        fn test_glacial_pulse_wave_mark_hit() {
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);

            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);

            pulse.mark_hit(entity1);
            assert!(pulse.hit_enemies.contains(&entity1));
            assert!(!pulse.hit_enemies.contains(&entity2));

            pulse.mark_hit(entity2);
            assert!(pulse.hit_enemies.contains(&entity1));
            assert!(pulse.hit_enemies.contains(&entity2));
        }

        #[test]
        fn test_glacial_pulse_uses_frost_element_color() {
            let color = glacial_pulse_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod glacial_pulse_expansion_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_glacial_pulse_expands_over_time() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                GlacialPulseWave::new(Vec2::ZERO, 30.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(GLACIAL_PULSE_EXPANSION_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(glacial_pulse_expansion_system);

            let pulse = app.world().get::<GlacialPulseWave>(entity).unwrap();
            assert!(
                (pulse.current_radius - GLACIAL_PULSE_MAX_RADIUS / 2.0).abs() < 0.1,
                "Radius should be approximately half after half duration: got {}",
                pulse.current_radius
            );
        }

        #[test]
        fn test_glacial_pulse_multiple_rings_expand_independently() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create two pulses with different starting radii
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                GlacialPulseWave::new(Vec2::ZERO, 30.0),
            )).id();

            let mut pulse2 = GlacialPulseWave::new(Vec2::new(10.0, 10.0), 20.0);
            pulse2.current_radius = 3.0; // Pre-expanded
            let entity2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                pulse2,
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(glacial_pulse_expansion_system);

            let pulse1 = app.world().get::<GlacialPulseWave>(entity1).unwrap();
            let pulse2 = app.world().get::<GlacialPulseWave>(entity2).unwrap();

            // Both should have expanded but from different starting points
            assert!(pulse1.current_radius > 0.0);
            assert!(pulse2.current_radius > 3.0);
        }
    }

    mod glacial_pulse_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_glacial_pulse_damages_enemy_in_radius() {
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
            app.add_systems(Update, (glacial_pulse_collision_system, count_damage_events).chain());

            // Create pulse at origin with radius 5.0
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
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
        fn test_glacial_pulse_no_damage_outside_radius() {
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
            app.add_systems(Update, (glacial_pulse_collision_system, count_damage_events).chain());

            // Create pulse at origin with radius 3.0
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 3.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
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
        fn test_glacial_pulse_damages_enemy_only_once() {
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
            app.add_systems(Update, (glacial_pulse_collision_system, count_damage_events).chain());

            // Create pulse
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
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
        fn test_glacial_pulse_damages_multiple_enemies() {
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
            app.add_systems(Update, (glacial_pulse_collision_system, count_damage_events).chain());

            // Create pulse
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
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
        fn test_glacial_pulse_applies_slowed_debuff() {
            let mut app = App::new();

            app.add_message::<DamageEvent>();
            app.add_systems(Update, glacial_pulse_collision_system);

            // Create pulse
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            ));

            // Create enemy in radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should have SlowedDebuff
            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "Enemy should have SlowedDebuff after glacial pulse hit");
            assert_eq!(slowed.unwrap().speed_multiplier, GLACIAL_PULSE_SLOW_MULTIPLIER);
        }

        #[test]
        fn test_glacial_pulse_applies_weakened_debuff() {
            let mut app = App::new();

            app.add_message::<DamageEvent>();
            app.add_systems(Update, glacial_pulse_collision_system);

            // Create pulse
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            ));

            // Create enemy in radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should have WeakenedDebuff
            let weakened = app.world().get::<WeakenedDebuff>(enemy_entity);
            assert!(weakened.is_some(), "Enemy should have WeakenedDebuff after glacial pulse hit");
            assert_eq!(weakened.unwrap().damage_multiplier, GLACIAL_PULSE_WEAKEN_MULTIPLIER);
        }

        #[test]
        fn test_glacial_pulse_uses_xz_plane_ignores_y() {
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
            app.add_systems(Update, (glacial_pulse_collision_system, count_damage_events).chain());

            // Create pulse at origin
            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
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

    mod glacial_pulse_cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_glacial_pulse_despawns_when_finished() {
            let mut app = App::new();

            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = GLACIAL_PULSE_MAX_RADIUS; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(glacial_pulse_cleanup_system);

            // Pulse should be despawned
            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_glacial_pulse_survives_before_finished() {
            let mut app = App::new();

            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = GLACIAL_PULSE_MAX_RADIUS / 2.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(glacial_pulse_cleanup_system);

            // Pulse should still exist
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod fire_glacial_pulse_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_glacial_pulse_spawns_wave() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrostNova);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 pulse
            let mut query = app.world_mut().query::<&GlacialPulseWave>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_glacial_pulse_at_player_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrostNova);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_glacial_pulse_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrostNova);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_glacial_pulse_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrostNova);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_pulse_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_glacial_pulse_starts_at_zero_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrostNova);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.current_radius, 0.0);
            }
        }

        #[test]
        fn test_fire_glacial_pulse_has_debuff_settings() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrostNova);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.slow_duration, GLACIAL_PULSE_SLOW_DURATION);
                assert_eq!(pulse.slow_multiplier, GLACIAL_PULSE_SLOW_MULTIPLIER);
                assert_eq!(pulse.weaken_duration, GLACIAL_PULSE_WEAKEN_DURATION);
                assert_eq!(pulse.weaken_multiplier, GLACIAL_PULSE_WEAKEN_MULTIPLIER);
            }
        }
    }

    mod glacial_pulse_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_glacial_pulse_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            pulse.current_radius = 5.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(glacial_pulse_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(5.0));
        }

        #[test]
        fn test_glacial_pulse_visual_minimum_scale() {
            let mut app = App::new();

            let pulse = GlacialPulseWave::new(Vec2::ZERO, 30.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(glacial_pulse_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }
}
