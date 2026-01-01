//! Void Pulse spell - An expanding wave of dark energy that weakens enemies.
//!
//! A Dark element spell (DarkPulse SpellType) that creates an expanding wave
//! emanating from the player. Enemies touched by the wave receive a WeakenedDebuff
//! that causes them to take increased damage from all sources. Each enemy is
//! only debuffed once per pulse.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Maximum radius the void pulse will expand to
pub const VOID_PULSE_MAX_RADIUS: f32 = 8.0;

/// Duration in seconds for the pulse to fully expand
pub const VOID_PULSE_EXPANSION_DURATION: f32 = 0.6;

/// Height of the visual effect above ground
pub const VOID_PULSE_VISUAL_HEIGHT: f32 = 0.2;

/// Duration of the weakened debuff in seconds
pub const WEAKENED_DEBUFF_DURATION: f32 = 4.0;

/// Damage multiplier for weakened enemies (1.25 = 25% more damage taken)
pub const WEAKENED_DAMAGE_MULTIPLIER: f32 = 1.25;

/// Get the dark element color for visual effects (purple)
pub fn void_pulse_color() -> Color {
    Element::Dark.color()
}

/// Component for the expanding void pulse ring.
/// Tracks expansion state and which enemies have been affected.
#[derive(Component, Debug, Clone)]
pub struct VoidPulseWave {
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
    /// Duration of the weakened debuff to apply
    pub debuff_duration: f32,
    /// Damage multiplier for the weakened debuff
    pub debuff_multiplier: f32,
    /// Set of enemy entities already affected by this pulse (prevents double application)
    pub affected_enemies: HashSet<Entity>,
}

impl VoidPulseWave {
    /// Create a new void pulse at the given center position.
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: VOID_PULSE_MAX_RADIUS,
            expansion_rate: VOID_PULSE_MAX_RADIUS / VOID_PULSE_EXPANSION_DURATION,
            damage,
            debuff_duration: WEAKENED_DEBUFF_DURATION,
            debuff_multiplier: WEAKENED_DAMAGE_MULTIPLIER,
            affected_enemies: HashSet::new(),
        }
    }

    /// Create a void pulse from a Spell component.
    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the pulse has finished expanding.
    pub fn is_finished(&self) -> bool {
        self.current_radius >= self.max_radius
    }

    /// Expand the wave by the given delta time.
    pub fn expand(&mut self, delta_secs: f32) {
        self.current_radius = (self.current_radius + self.expansion_rate * delta_secs)
            .min(self.max_radius);
    }

    /// Check if an enemy at the given distance should be affected.
    /// Returns true if enemy is within the current wave radius and hasn't been affected yet.
    pub fn should_affect(&self, entity: Entity, distance: f32) -> bool {
        distance <= self.current_radius && !self.affected_enemies.contains(&entity)
    }

    /// Mark an enemy as affected by this pulse.
    pub fn mark_affected(&mut self, entity: Entity) {
        self.affected_enemies.insert(entity);
    }
}

/// Weakened debuff applied to enemies touched by a void pulse.
/// Causes the enemy to take increased damage from all sources.
#[derive(Component, Debug, Clone)]
pub struct WeakenedDebuff {
    /// Timer tracking remaining debuff duration
    pub duration: Timer,
    /// Damage multiplier (1.25 = 25% more damage taken)
    pub damage_multiplier: f32,
}

impl WeakenedDebuff {
    /// Create a new weakened debuff with specified duration and multiplier.
    pub fn new(duration_secs: f32, damage_multiplier: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            damage_multiplier,
        }
    }

    /// Create a weakened debuff with default values.
    pub fn default_config() -> Self {
        Self::new(WEAKENED_DEBUFF_DURATION, WEAKENED_DAMAGE_MULTIPLIER)
    }

    /// Check if the debuff has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the debuff timer.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }
}

/// System that expands void pulse waves over time.
pub fn void_pulse_expansion_system(
    mut pulse_query: Query<&mut VoidPulseWave>,
    time: Res<Time>,
) {
    for mut pulse in pulse_query.iter_mut() {
        pulse.expand(time.delta_secs());
    }
}

/// System that checks for enemy collisions with the expanding wave,
/// applies damage, and adds WeakenedDebuff to affected enemies.
pub fn void_pulse_collision_system(
    mut commands: Commands,
    mut pulse_query: Query<&mut VoidPulseWave>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    existing_debuff_query: Query<&WeakenedDebuff>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut pulse in pulse_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = pulse.center.distance(enemy_pos);

            if pulse.should_affect(enemy_entity, distance) {
                // Apply damage with Dark element
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    pulse.damage,
                    Element::Dark,
                ));

                // Apply or refresh WeakenedDebuff
                if existing_debuff_query.get(enemy_entity).is_ok() {
                    // Enemy already has debuff - refresh it
                    commands.entity(enemy_entity).insert(
                        WeakenedDebuff::new(pulse.debuff_duration, pulse.debuff_multiplier)
                    );
                } else {
                    // Add new debuff
                    commands.entity(enemy_entity).insert(
                        WeakenedDebuff::new(pulse.debuff_duration, pulse.debuff_multiplier)
                    );
                }

                pulse.mark_affected(enemy_entity);
            }
        }
    }
}

/// System that despawns void pulses when they finish expanding.
pub fn void_pulse_cleanup_system(
    mut commands: Commands,
    pulse_query: Query<(Entity, &VoidPulseWave)>,
) {
    for (entity, pulse) in pulse_query.iter() {
        if pulse.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual scale of void pulses based on their current radius.
pub fn void_pulse_visual_system(
    mut pulse_query: Query<(&VoidPulseWave, &mut Transform)>,
) {
    for (pulse, mut transform) in pulse_query.iter_mut() {
        // Scale the visual to match current radius
        transform.scale = Vec3::splat(pulse.current_radius.max(0.1));
    }
}

/// System that ticks WeakenedDebuff timers and removes expired debuffs.
pub fn weakened_debuff_tick_system(
    mut commands: Commands,
    time: Res<Time>,
    mut debuff_query: Query<(Entity, &mut WeakenedDebuff)>,
) {
    for (entity, mut debuff) in debuff_query.iter_mut() {
        debuff.tick(time.delta());
        if debuff.is_expired() {
            commands.entity(entity).remove::<WeakenedDebuff>();
        }
    }
}

/// Cast void pulse (DarkPulse) spell - spawns an expanding wave of dark energy.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_void_pulse(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_void_pulse_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast void pulse spell with explicit damage - spawns an expanding wave of dark energy.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_void_pulse_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let pulse = VoidPulseWave::new(center, damage);
    let pulse_pos = Vec3::new(spawn_position.x, VOID_PULSE_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.explosion.clone()), // Dark purple material
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod void_pulse_wave_tests {
        use super::*;

        #[test]
        fn test_void_pulse_wave_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 20.0;
            let pulse = VoidPulseWave::new(center, damage);

            assert_eq!(pulse.center, center);
            assert_eq!(pulse.damage, damage);
            assert_eq!(pulse.current_radius, 0.0);
            assert_eq!(pulse.max_radius, VOID_PULSE_MAX_RADIUS);
            assert_eq!(pulse.debuff_duration, WEAKENED_DEBUFF_DURATION);
            assert_eq!(pulse.debuff_multiplier, WEAKENED_DAMAGE_MULTIPLIER);
            assert!(!pulse.is_finished());
            assert!(pulse.affected_enemies.is_empty());
        }

        #[test]
        fn test_void_pulse_wave_from_spell() {
            let spell = Spell::new(SpellType::DarkPulse);
            let center = Vec2::new(5.0, 15.0);
            let pulse = VoidPulseWave::from_spell(center, &spell);

            assert_eq!(pulse.center, center);
            assert_eq!(pulse.damage, spell.damage());
        }

        #[test]
        fn test_void_pulse_wave_expand() {
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);

            pulse.expand(VOID_PULSE_EXPANSION_DURATION / 2.0);
            assert!(
                (pulse.current_radius - VOID_PULSE_MAX_RADIUS / 2.0).abs() < 0.01,
                "Radius should be half of max after half duration"
            );

            pulse.expand(VOID_PULSE_EXPANSION_DURATION / 2.0);
            assert!(
                (pulse.current_radius - VOID_PULSE_MAX_RADIUS).abs() < 0.01,
                "Radius should be max after full duration"
            );
        }

        #[test]
        fn test_void_pulse_wave_expand_caps_at_max() {
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);

            // Expand way past max
            pulse.expand(VOID_PULSE_EXPANSION_DURATION * 10.0);

            assert_eq!(pulse.current_radius, VOID_PULSE_MAX_RADIUS);
        }

        #[test]
        fn test_void_pulse_wave_is_finished() {
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
            assert!(!pulse.is_finished());

            pulse.current_radius = VOID_PULSE_MAX_RADIUS;
            assert!(pulse.is_finished());
        }

        #[test]
        fn test_void_pulse_wave_should_affect() {
            let pulse = VoidPulseWave {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 8.0,
                expansion_rate: 13.33,
                damage: 20.0,
                debuff_duration: 4.0,
                debuff_multiplier: 1.25,
                affected_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(pulse.should_affect(entity, 3.0), "Should affect enemy within radius");
            assert!(pulse.should_affect(entity, 5.0), "Should affect enemy at radius edge");
            assert!(!pulse.should_affect(entity, 6.0), "Should not affect enemy outside radius");
        }

        #[test]
        fn test_void_pulse_wave_should_affect_excludes_already_affected() {
            let mut pulse = VoidPulseWave {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 8.0,
                expansion_rate: 13.33,
                damage: 20.0,
                debuff_duration: 4.0,
                debuff_multiplier: 1.25,
                affected_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(pulse.should_affect(entity, 3.0));

            pulse.mark_affected(entity);
            assert!(!pulse.should_affect(entity, 3.0), "Should not affect already-affected enemy");
        }

        #[test]
        fn test_void_pulse_wave_mark_affected() {
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);

            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);

            pulse.mark_affected(entity1);
            assert!(pulse.affected_enemies.contains(&entity1));
            assert!(!pulse.affected_enemies.contains(&entity2));

            pulse.mark_affected(entity2);
            assert!(pulse.affected_enemies.contains(&entity1));
            assert!(pulse.affected_enemies.contains(&entity2));
        }

        #[test]
        fn test_void_pulse_uses_dark_element_color() {
            let color = void_pulse_color();
            assert_eq!(color, Element::Dark.color());
        }
    }

    mod weakened_debuff_tests {
        use super::*;

        #[test]
        fn test_weakened_debuff_new() {
            let debuff = WeakenedDebuff::new(5.0, 1.5);

            assert_eq!(debuff.damage_multiplier, 1.5);
            assert!(!debuff.is_expired());
        }

        #[test]
        fn test_weakened_debuff_default_config() {
            let debuff = WeakenedDebuff::default_config();

            assert_eq!(debuff.damage_multiplier, WEAKENED_DAMAGE_MULTIPLIER);
            assert!(!debuff.is_expired());
        }

        #[test]
        fn test_weakened_debuff_tick_and_expire() {
            let mut debuff = WeakenedDebuff::new(1.0, 1.25);

            assert!(!debuff.is_expired());

            debuff.tick(Duration::from_secs_f32(0.5));
            assert!(!debuff.is_expired());

            debuff.tick(Duration::from_secs_f32(0.6));
            assert!(debuff.is_expired());
        }

        #[test]
        fn test_weakened_debuff_multiplier_is_correct() {
            let debuff = WeakenedDebuff::new(4.0, 1.5);
            assert_eq!(debuff.damage_multiplier, 1.5);
        }
    }

    mod void_pulse_expansion_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_void_pulse_expands_over_time() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VoidPulseWave::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(VOID_PULSE_EXPANSION_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(void_pulse_expansion_system);

            let pulse = app.world().get::<VoidPulseWave>(entity).unwrap();
            assert!(
                (pulse.current_radius - VOID_PULSE_MAX_RADIUS / 2.0).abs() < 0.1,
                "Radius should be approximately half after half duration: got {}",
                pulse.current_radius
            );
        }

        #[test]
        fn test_void_pulse_multiple_waves_expand_independently() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create two pulses with different starting radii
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VoidPulseWave::new(Vec2::ZERO, 20.0),
            )).id();

            let mut pulse2 = VoidPulseWave::new(Vec2::new(10.0, 10.0), 15.0);
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

            let _ = app.world_mut().run_system_once(void_pulse_expansion_system);

            let pulse1 = app.world().get::<VoidPulseWave>(entity1).unwrap();
            let pulse2 = app.world().get::<VoidPulseWave>(entity2).unwrap();

            // Both should have expanded but from different starting points
            assert!(pulse1.current_radius > 0.0);
            assert!(pulse2.current_radius > 3.0);
        }
    }

    mod void_pulse_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_void_pulse_damages_enemy_in_radius() {
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
            app.add_systems(Update, (void_pulse_collision_system, count_damage_events).chain());

            // Create pulse at origin with radius 5.0
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
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
        fn test_void_pulse_no_damage_outside_radius() {
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
            app.add_systems(Update, (void_pulse_collision_system, count_damage_events).chain());

            // Create pulse at origin with radius 3.0
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
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
        fn test_void_pulse_damages_enemy_only_once() {
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
            app.add_systems(Update, (void_pulse_collision_system, count_damage_events).chain());

            // Create pulse
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
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
        fn test_void_pulse_damages_multiple_enemies() {
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
            app.add_systems(Update, (void_pulse_collision_system, count_damage_events).chain());

            // Create pulse
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
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
        fn test_void_pulse_applies_weakened_debuff() {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, void_pulse_collision_system);

            // Create pulse
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
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

            let debuff = app.world().get::<WeakenedDebuff>(enemy_entity);
            assert!(debuff.is_some(), "Enemy should have WeakenedDebuff");
            assert_eq!(debuff.unwrap().damage_multiplier, WEAKENED_DAMAGE_MULTIPLIER);
        }

        #[test]
        fn test_void_pulse_uses_xz_plane_ignores_y() {
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
            app.add_systems(Update, (void_pulse_collision_system, count_damage_events).chain());

            // Create pulse at origin
            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
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

    mod void_pulse_cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_void_pulse_despawns_when_finished() {
            let mut app = App::new();

            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
            pulse.current_radius = VOID_PULSE_MAX_RADIUS; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(void_pulse_cleanup_system);

            // Pulse should be despawned
            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_void_pulse_survives_before_finished() {
            let mut app = App::new();

            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
            pulse.current_radius = VOID_PULSE_MAX_RADIUS / 2.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(void_pulse_cleanup_system);

            // Pulse should still exist
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod weakened_debuff_tick_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_weakened_debuff_removed_when_expired() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let mut debuff = WeakenedDebuff::new(0.5, 1.25);
            debuff.duration.tick(Duration::from_secs_f32(0.6)); // Force expired

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                debuff,
            )).id();

            let _ = app.world_mut().run_system_once(weakened_debuff_tick_system);

            assert!(
                app.world().get::<WeakenedDebuff>(enemy_entity).is_none(),
                "Expired debuff should be removed"
            );
        }

        #[test]
        fn test_weakened_debuff_persists_before_expiry() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let debuff = WeakenedDebuff::new(10.0, 1.25);

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                debuff,
            )).id();

            // Advance time but not past expiry
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(weakened_debuff_tick_system);

            assert!(
                app.world().get::<WeakenedDebuff>(enemy_entity).is_some(),
                "Debuff should persist before expiry"
            );
        }

        #[test]
        fn test_weakened_debuff_ticks_down() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let debuff = WeakenedDebuff::new(5.0, 1.25);

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                debuff,
            )).id();

            // Advance time by 2 seconds
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.0));
            }

            let _ = app.world_mut().run_system_once(weakened_debuff_tick_system);

            let updated_debuff = app.world().get::<WeakenedDebuff>(enemy_entity).unwrap();
            let remaining = updated_debuff.duration.remaining_secs();
            assert!(
                remaining < 4.0 && remaining > 2.5,
                "Debuff timer should have ticked down, remaining: {}",
                remaining
            );
        }
    }

    mod fire_void_pulse_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_void_pulse_spawns_wave() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DarkPulse);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_void_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 pulse
            let mut query = app.world_mut().query::<&VoidPulseWave>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_void_pulse_at_player_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DarkPulse);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_void_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VoidPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_void_pulse_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DarkPulse);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_void_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VoidPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_void_pulse_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DarkPulse);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_void_pulse_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VoidPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_void_pulse_starts_at_zero_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DarkPulse);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_void_pulse(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VoidPulseWave>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.current_radius, 0.0);
            }
        }
    }

    mod void_pulse_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_void_pulse_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
            pulse.current_radius = 5.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(void_pulse_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(5.0));
        }

        #[test]
        fn test_void_pulse_visual_minimum_scale() {
            let mut app = App::new();

            let pulse = VoidPulseWave::new(Vec2::ZERO, 20.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                pulse,
            )).id();

            let _ = app.world_mut().run_system_once(void_pulse_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }
}
