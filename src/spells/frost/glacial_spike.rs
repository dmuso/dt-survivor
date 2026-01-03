//! Glacial Spike spell - A massive spike of ice erupts from the ground.
//!
//! A Frost element spell that spawns an ice spike at the target location.
//! The spike erupts from the ground, damaging and slowing enemies caught
//! in its collision radius.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;
use crate::spells::frost::ice_shard::SlowedDebuff;

/// Collision radius for the spike (enemies within this distance take damage)
pub const GLACIAL_SPIKE_COLLISION_RADIUS: f32 = 1.5;

/// Maximum height of the spike when fully erupted
pub const GLACIAL_SPIKE_MAX_HEIGHT: f32 = 4.0;

/// Time for spike to fully erupt from ground (seconds)
pub const GLACIAL_SPIKE_ERUPTION_TIME: f32 = 0.3;

/// Time the spike lingers at full height before despawning (seconds)
pub const GLACIAL_SPIKE_LINGER_TIME: f32 = 0.5;

/// Slow effect duration when hitting enemies
pub const GLACIAL_SPIKE_SLOW_DURATION: f32 = 2.0;

/// Slow effect multiplier (0.4 = 40% of normal speed)
pub const GLACIAL_SPIKE_SLOW_MULTIPLIER: f32 = 0.4;

/// Get the frost element color for visual effects
pub fn glacial_spike_color() -> Color {
    Element::Frost.color()
}

/// Glacial Spike component - represents a spike erupting from the ground.
#[derive(Component, Debug, Clone)]
pub struct GlacialSpike {
    /// Center position on XZ plane where spike erupts
    pub center: Vec2,
    /// Damage dealt to enemies caught in the eruption
    pub damage: f32,
    /// Collision radius for damage
    pub collision_radius: f32,
    /// Maximum height of the spike
    pub max_height: f32,
    /// Timer for eruption animation
    pub eruption_timer: Timer,
    /// Timer for lingering after full eruption
    pub linger_timer: Timer,
    /// Current phase of the spike
    pub phase: GlacialSpikePhase,
    /// Whether damage has been applied
    pub damage_applied: bool,
    /// Slow effect duration
    pub slow_duration: f32,
    /// Slow effect multiplier
    pub slow_multiplier: f32,
}

/// Phase of the glacial spike lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlacialSpikePhase {
    /// Spike is rising from the ground
    Erupting,
    /// Spike is at full height, lingering
    Lingering,
    /// Spike should be despawned
    Expired,
}

impl GlacialSpike {
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            damage,
            collision_radius: GLACIAL_SPIKE_COLLISION_RADIUS,
            max_height: GLACIAL_SPIKE_MAX_HEIGHT,
            eruption_timer: Timer::from_seconds(GLACIAL_SPIKE_ERUPTION_TIME, TimerMode::Once),
            linger_timer: Timer::from_seconds(GLACIAL_SPIKE_LINGER_TIME, TimerMode::Once),
            phase: GlacialSpikePhase::Erupting,
            damage_applied: false,
            slow_duration: GLACIAL_SPIKE_SLOW_DURATION,
            slow_multiplier: GLACIAL_SPIKE_SLOW_MULTIPLIER,
        }
    }

    /// Get the current height based on eruption progress (0.0 to max_height)
    pub fn current_height(&self) -> f32 {
        match self.phase {
            GlacialSpikePhase::Erupting => {
                let progress = self.eruption_timer.fraction();
                self.max_height * progress
            }
            GlacialSpikePhase::Lingering | GlacialSpikePhase::Expired => self.max_height,
        }
    }

    /// Check if the spike has fully erupted
    pub fn is_fully_erupted(&self) -> bool {
        self.eruption_timer.is_finished()
    }

    /// Check if the spike has expired
    pub fn is_expired(&self) -> bool {
        self.phase == GlacialSpikePhase::Expired
    }
}

/// System that animates the spike erupting from the ground and updates phase.
pub fn glacial_spike_eruption_system(
    time: Res<Time>,
    mut spike_query: Query<(&mut GlacialSpike, &mut Transform)>,
) {
    for (mut spike, mut transform) in spike_query.iter_mut() {
        match spike.phase {
            GlacialSpikePhase::Erupting => {
                spike.eruption_timer.tick(time.delta());

                // Scale the spike based on eruption progress
                let height = spike.current_height();
                // Scale Y to animate rising, keep XZ at base collision radius
                transform.scale = Vec3::new(
                    spike.collision_radius,
                    height.max(0.1), // Minimum height to prevent zero scale
                    spike.collision_radius,
                );

                // Adjust Y position so spike emerges from ground (bottom at ground level)
                transform.translation.y = height / 2.0;

                if spike.is_fully_erupted() {
                    spike.phase = GlacialSpikePhase::Lingering;
                }
            }
            GlacialSpikePhase::Lingering => {
                spike.linger_timer.tick(time.delta());
                if spike.linger_timer.is_finished() {
                    spike.phase = GlacialSpikePhase::Expired;
                }
            }
            GlacialSpikePhase::Expired => {
                // Will be cleaned up by cleanup system
            }
        }
    }
}

/// System that applies damage and slow to enemies caught in the spike's collision radius.
/// Damage is applied when the spike is at or past 50% eruption.
pub fn glacial_spike_collision_system(
    mut commands: Commands,
    mut spike_query: Query<&mut GlacialSpike>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut spike in spike_query.iter_mut() {
        if spike.damage_applied {
            continue;
        }

        // Only deal damage once spike is at least 50% erupted
        let eruption_progress = spike.eruption_timer.fraction();
        if eruption_progress < 0.5 {
            continue;
        }

        // Apply damage to all enemies within collision radius
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = spike.center.distance(enemy_pos);

            if distance <= spike.collision_radius {
                damage_events.write(DamageEvent::new(enemy_entity, spike.damage));

                // Apply slow effect
                commands.entity(enemy_entity).try_insert(SlowedDebuff::new(
                    spike.slow_duration,
                    spike.slow_multiplier,
                ));
            }
        }

        spike.damage_applied = true;
    }
}

/// System that despawns expired spikes.
pub fn glacial_spike_cleanup_system(
    mut commands: Commands,
    spike_query: Query<(Entity, &GlacialSpike)>,
) {
    for (entity, spike) in spike_query.iter() {
        if spike.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast glacial spike spell - spawns a spike at the target location.
/// `spawn_position` is Whisper's full 3D position (unused for ground-based effect).
/// `target_pos` is the target on XZ plane where the spike will erupt.
#[allow(clippy::too_many_arguments)]
pub fn fire_glacial_spike(
    commands: &mut Commands,
    spell: &Spell,
    _spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_glacial_spike_with_damage(
        commands,
        spell,
        spell.damage(),
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast glacial spike spell with explicit damage.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
/// `target_pos` is the target on XZ plane where the spike will erupt.
#[allow(clippy::too_many_arguments)]
pub fn fire_glacial_spike_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let spike = GlacialSpike::new(target_pos, damage);

    // Spike starts at ground level with minimal height (will animate up)
    let initial_pos = to_xz(target_pos);
    let initial_scale = Vec3::new(GLACIAL_SPIKE_COLLISION_RADIUS, 0.1, GLACIAL_SPIKE_COLLISION_RADIUS);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.glacial_spike.clone()),
            MeshMaterial3d(materials.glacial_spike.clone()),
            Transform::from_translation(initial_pos).with_scale(initial_scale),
            spike,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(initial_pos).with_scale(initial_scale),
            spike,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod glacial_spike_component_tests {
        use super::*;

        #[test]
        fn test_glacial_spike_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 30.0;
            let spike = GlacialSpike::new(center, damage);

            assert_eq!(spike.center, center);
            assert_eq!(spike.damage, damage);
            assert_eq!(spike.collision_radius, GLACIAL_SPIKE_COLLISION_RADIUS);
            assert_eq!(spike.max_height, GLACIAL_SPIKE_MAX_HEIGHT);
            assert_eq!(spike.phase, GlacialSpikePhase::Erupting);
            assert!(!spike.damage_applied);
        }

        #[test]
        fn test_glacial_spike_current_height_at_start() {
            let spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            // At start, eruption progress is 0, so height should be 0
            assert_eq!(spike.current_height(), 0.0);
        }

        #[test]
        fn test_glacial_spike_current_height_at_full_eruption() {
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            // Tick timer to completion
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            spike.phase = GlacialSpikePhase::Lingering;

            assert_eq!(spike.current_height(), GLACIAL_SPIKE_MAX_HEIGHT);
        }

        #[test]
        fn test_glacial_spike_is_fully_erupted() {
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            assert!(!spike.is_fully_erupted());

            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            assert!(spike.is_fully_erupted());
        }

        #[test]
        fn test_glacial_spike_is_expired() {
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            assert!(!spike.is_expired());

            spike.phase = GlacialSpikePhase::Expired;
            assert!(spike.is_expired());
        }

        #[test]
        fn test_uses_frost_element_color() {
            let color = glacial_spike_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod glacial_spike_eruption_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_spike_scales_during_eruption() {
            let mut app = setup_test_app();

            let spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spike,
            )).id();

            // Advance time halfway through eruption
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME / 2.0));
            }

            app.world_mut().run_system_once(glacial_spike_eruption_system).unwrap();

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Y scale should be approximately half of max height
            let expected_height = GLACIAL_SPIKE_MAX_HEIGHT / 2.0;
            assert!((transform.scale.y - expected_height).abs() < 0.1);
        }

        #[test]
        fn test_spike_transitions_to_lingering() {
            let mut app = setup_test_app();

            let spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spike,
            )).id();

            // Advance past eruption time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.01));
            }

            app.world_mut().run_system_once(glacial_spike_eruption_system).unwrap();

            let spike = app.world().get::<GlacialSpike>(entity).unwrap();
            assert_eq!(spike.phase, GlacialSpikePhase::Lingering);
        }

        #[test]
        fn test_spike_transitions_to_expired() {
            let mut app = setup_test_app();

            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.phase = GlacialSpikePhase::Lingering;
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spike,
            )).id();

            // Advance past linger time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(GLACIAL_SPIKE_LINGER_TIME + 0.01));
            }

            app.world_mut().run_system_once(glacial_spike_eruption_system).unwrap();

            let spike = app.world().get::<GlacialSpike>(entity).unwrap();
            assert_eq!(spike.phase, GlacialSpikePhase::Expired);
        }

        #[test]
        fn test_spike_y_position_adjusts_during_eruption() {
            let mut app = setup_test_app();

            let spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spike,
            )).id();

            // Advance past eruption
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.01));
            }

            app.world_mut().run_system_once(glacial_spike_eruption_system).unwrap();

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Y position should be half of max height (center of spike)
            let expected_y = GLACIAL_SPIKE_MAX_HEIGHT / 2.0;
            assert!((transform.translation.y - expected_y).abs() < 0.1);
        }
    }

    mod glacial_spike_collision_system_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_damage_applied_to_enemies_in_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (glacial_spike_collision_system, count_damage_events).chain());

            // Create spike at origin, 50% erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME / 2.0));
            app.world_mut().spawn(spike);

            // Create enemy within collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_no_damage_before_50_percent_eruption() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (glacial_spike_collision_system, count_damage_events).chain());

            // Create spike at origin, only 25% erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME / 4.0));
            app.world_mut().spawn(spike);

            // Create enemy within collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_no_damage_outside_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (glacial_spike_collision_system, count_damage_events).chain());

            // Create spike at origin, fully erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            app.world_mut().spawn(spike);

            // Create enemy outside collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_damage_applied_only_once() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (glacial_spike_collision_system, count_damage_events).chain());

            // Create spike, fully erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            app.world_mut().spawn(spike);

            // Create enemy in radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_slow_effect_applied() {
            let mut app = setup_test_app();

            app.add_systems(Update, glacial_spike_collision_system);

            // Create spike, fully erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            app.world_mut().spawn(spike);

            // Create enemy in radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            )).id();

            app.update();

            // Enemy should have SlowedDebuff
            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "Enemy should have SlowedDebuff");
            assert_eq!(slowed.unwrap().speed_multiplier, GLACIAL_SPIKE_SLOW_MULTIPLIER);
        }

        #[test]
        fn test_multiple_enemies_in_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (glacial_spike_collision_system, count_damage_events).chain());

            // Create spike, fully erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            app.world_mut().spawn(spike);

            // Create 3 enemies in radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 * 0.4, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn test_uses_xz_plane_ignores_y() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (glacial_spike_collision_system, count_damage_events).chain());

            // Create spike, fully erupted
            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.eruption_timer.tick(Duration::from_secs_f32(GLACIAL_SPIKE_ERUPTION_TIME + 0.1));
            app.world_mut().spawn(spike);

            // Create enemy close on XZ but far on Y
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 100.0, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }
    }

    mod glacial_spike_cleanup_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_expired_spike_despawns() {
            let mut app = setup_test_app();

            let mut spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            spike.phase = GlacialSpikePhase::Expired;

            let entity = app.world_mut().spawn(spike).id();

            app.world_mut().run_system_once(glacial_spike_cleanup_system).unwrap();

            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_non_expired_spike_survives() {
            let mut app = setup_test_app();

            let spike = GlacialSpike::new(Vec2::ZERO, 30.0);
            let entity = app.world_mut().spawn(spike).id();

            app.world_mut().run_system_once(glacial_spike_cleanup_system).unwrap();

            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod fire_glacial_spike_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_glacial_spike_spawns_spike() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 5.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_spike(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialSpike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_glacial_spike_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_spike(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&GlacialSpike, &Transform)>();
            for (spike, transform) in query.iter(app.world()) {
                assert_eq!(spike.center, target_pos);
                // X and Z should match target, Y starts at ground level
                assert!((transform.translation.x - target_pos.x).abs() < 0.01);
                assert!((transform.translation.z - target_pos.y).abs() < 0.01);
            }
        }

        #[test]
        fn test_fire_glacial_spike_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let expected_damage = spell.damage();
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_spike(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialSpike>();
            for spike in query.iter(app.world()) {
                assert_eq!(spike.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_glacial_spike_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let explicit_damage = 100.0;
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_spike_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&GlacialSpike>();
            for spike in query.iter(app.world()) {
                assert_eq!(spike.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_glacial_spike_initial_scale() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_glacial_spike(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&Transform>();
            for transform in query.iter(app.world()) {
                // Initial scale should be collision radius on X/Z, minimal on Y
                assert_eq!(transform.scale.x, GLACIAL_SPIKE_COLLISION_RADIUS);
                assert_eq!(transform.scale.z, GLACIAL_SPIKE_COLLISION_RADIUS);
                assert!((transform.scale.y - 0.1).abs() < 0.01);
            }
        }
    }
}
