//! Black Spiral spell - A rotating vortex of dark energy that pulls enemies inward.
//!
//! A Dark element spell (VoidRift SpellType) that creates a swirling vortex at a
//! target location. The vortex continuously pulls nearby enemies toward its center
//! while dealing damage over time to those caught within. Excellent for crowd control
//! and grouping enemies together.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, Velocity};
use crate::spell::components::Spell;

/// Default configuration for Black Spiral spell
pub const BLACK_SPIRAL_PULL_RADIUS: f32 = 6.0;
pub const BLACK_SPIRAL_PULL_STRENGTH: f32 = 120.0;
pub const BLACK_SPIRAL_DAMAGE_RATE: f32 = 15.0; // Damage per second
pub const BLACK_SPIRAL_DURATION: f32 = 5.0;
pub const BLACK_SPIRAL_VISUAL_HEIGHT: f32 = 0.3;
pub const BLACK_SPIRAL_DAMAGE_TICK_INTERVAL: f32 = 0.25;
pub const BLACK_SPIRAL_ROTATION_SPEED: f32 = 3.0; // Radians per second

/// Get the dark element color for visual effects (purple)
pub fn black_spiral_color() -> Color {
    Element::Dark.color()
}

/// Component for the Black Spiral vortex.
/// Creates a rotating vortex that pulls enemies inward while dealing damage.
#[derive(Component, Debug, Clone)]
pub struct BlackSpiral {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius within which enemies are pulled
    pub pull_radius: f32,
    /// Force applied toward center
    pub pull_strength: f32,
    /// Damage per tick to enemies in radius
    pub damage_per_tick: f32,
    /// Remaining duration of the vortex
    pub duration: Timer,
    /// Timer between damage ticks
    pub damage_timer: Timer,
    /// Current rotation angle for visual effect (radians)
    pub rotation_angle: f32,
}

impl BlackSpiral {
    /// Create a new black spiral at the given center position.
    pub fn new(center: Vec2, damage: f32) -> Self {
        // Calculate damage per tick based on damage rate and tick interval
        let damage_per_tick = damage * BLACK_SPIRAL_DAMAGE_TICK_INTERVAL;
        Self {
            center,
            pull_radius: BLACK_SPIRAL_PULL_RADIUS,
            pull_strength: BLACK_SPIRAL_PULL_STRENGTH,
            damage_per_tick,
            duration: Timer::from_seconds(BLACK_SPIRAL_DURATION, TimerMode::Once),
            damage_timer: Timer::from_seconds(BLACK_SPIRAL_DAMAGE_TICK_INTERVAL, TimerMode::Repeating),
            rotation_angle: 0.0,
        }
    }

    /// Create a black spiral from a Spell component.
    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the vortex has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the vortex timers and rotation.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.damage_timer.tick(delta);
        self.rotation_angle += BLACK_SPIRAL_ROTATION_SPEED * delta.as_secs_f32();
    }

    /// Check if damage should be applied this frame.
    pub fn should_damage(&self) -> bool {
        self.damage_timer.just_finished()
    }

    /// Check if an entity at the given position is within the pull radius.
    pub fn is_in_pull_range(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.pull_radius
    }

    /// Calculate the pull force for an entity at the given position.
    /// Returns a velocity vector pointing toward the vortex center.
    /// Pull strength is constant regardless of distance (per design choice).
    pub fn calculate_pull(&self, position: Vec2) -> Vec2 {
        let distance = self.center.distance(position);
        if distance <= 0.01 || distance > self.pull_radius {
            return Vec2::ZERO;
        }

        // Direction toward center
        let direction = (self.center - position).normalize();

        // Constant pull strength within radius
        direction * self.pull_strength
    }
}

/// System that ticks black spiral timers and rotation.
pub fn black_spiral_tick_system(
    mut spiral_query: Query<&mut BlackSpiral>,
    time: Res<Time>,
) {
    for mut spiral in spiral_query.iter_mut() {
        spiral.tick(time.delta());
    }
}

/// System that applies pull force to enemies within the spiral's pull radius.
pub fn black_spiral_pull_system(
    spiral_query: Query<&BlackSpiral>,
    mut enemy_query: Query<(&Transform, &mut Velocity), With<Enemy>>,
) {
    for spiral in spiral_query.iter() {
        for (transform, mut velocity) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(transform.translation);

            if spiral.is_in_pull_range(enemy_pos) {
                let pull = spiral.calculate_pull(enemy_pos);
                // Add pull force to enemy velocity
                velocity.0 += pull;
            }
        }
    }
}

/// System that damages enemies within the spiral's radius.
pub fn black_spiral_damage_system(
    spiral_query: Query<&BlackSpiral>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for spiral in spiral_query.iter() {
        if !spiral.should_damage() {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if spiral.is_in_pull_range(enemy_pos) {
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    spiral.damage_per_tick,
                    Element::Dark,
                ));
            }
        }
    }
}

/// System that despawns black spirals when their duration expires.
pub fn black_spiral_cleanup_system(
    mut commands: Commands,
    spiral_query: Query<(Entity, &BlackSpiral)>,
) {
    for (entity, spiral) in spiral_query.iter() {
        if spiral.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual rotation of black spirals.
pub fn black_spiral_visual_system(
    mut spiral_query: Query<(&BlackSpiral, &mut Transform)>,
) {
    for (spiral, mut transform) in spiral_query.iter_mut() {
        // Rotate the visual around the Y axis
        transform.rotation = Quat::from_rotation_y(spiral.rotation_angle);
    }
}

/// Cast black spiral spell - spawns a vortex at target location.
#[allow(clippy::too_many_arguments)]
pub fn spawn_black_spiral(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    spawn_black_spiral_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast black spiral spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn spawn_black_spiral_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    _spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let spiral = BlackSpiral::new(target_pos, damage);
    let spiral_pos = Vec3::new(target_pos.x, BLACK_SPIRAL_VISUAL_HEIGHT, target_pos.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.explosion.clone()), // Dark purple material
            Transform::from_translation(spiral_pos).with_scale(Vec3::splat(spiral.pull_radius)),
            spiral,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spiral_pos),
            spiral,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod black_spiral_component_tests {
        use super::*;

        #[test]
        fn test_black_spiral_spawns_at_correct_location() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 25.0;
            let spiral = BlackSpiral::new(center, damage);

            assert_eq!(spiral.center, center);
            assert_eq!(spiral.pull_radius, BLACK_SPIRAL_PULL_RADIUS);
            assert!(!spiral.is_expired());
        }

        #[test]
        fn test_black_spiral_has_correct_radius() {
            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            assert_eq!(spiral.pull_radius, BLACK_SPIRAL_PULL_RADIUS);
        }

        #[test]
        fn test_enemies_within_radius_are_pulled_toward_center() {
            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);

            // Enemy within pull radius
            let enemy_pos = Vec2::new(BLACK_SPIRAL_PULL_RADIUS - 1.0, 0.0);
            assert!(spiral.is_in_pull_range(enemy_pos));

            let pull = spiral.calculate_pull(enemy_pos);
            assert!(pull.length() > 0.0, "Should have pull force");
            assert!(pull.x < 0.0, "Pull should point toward center (negative x)");
        }

        #[test]
        fn test_enemies_outside_radius_are_unaffected() {
            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);

            // Enemy outside pull radius
            let outside_pos = Vec2::new(BLACK_SPIRAL_PULL_RADIUS + 1.0, 0.0);
            assert!(!spiral.is_in_pull_range(outside_pos));

            let pull = spiral.calculate_pull(outside_pos);
            assert_eq!(pull, Vec2::ZERO);
        }

        #[test]
        fn test_pull_strength_applies_correct_force() {
            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);

            // Enemy at a position within radius
            let enemy_pos = Vec2::new(3.0, 0.0);
            let pull = spiral.calculate_pull(enemy_pos);

            // Pull should be constant strength in direction of center
            assert!((pull.length() - BLACK_SPIRAL_PULL_STRENGTH).abs() < 0.01);
            assert!(pull.x < 0.0, "Pull should point toward center");
        }

        #[test]
        fn test_damage_applies_per_second_correctly() {
            let damage = 20.0;
            let spiral = BlackSpiral::new(Vec2::ZERO, damage);

            // Damage per tick should be scaled by tick interval
            let expected_damage_per_tick = damage * BLACK_SPIRAL_DAMAGE_TICK_INTERVAL;
            assert_eq!(spiral.damage_per_tick, expected_damage_per_tick);
        }

        #[test]
        fn test_vortex_expires_after_duration() {
            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            assert!(!spiral.is_expired());

            // Tick past duration
            spiral.tick(Duration::from_secs_f32(BLACK_SPIRAL_DURATION + 0.1));
            assert!(spiral.is_expired());
        }

        #[test]
        fn test_multiple_enemies_can_be_pulled_simultaneously() {
            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);

            // Multiple enemies at different positions
            let positions = vec![
                Vec2::new(2.0, 0.0),
                Vec2::new(0.0, 3.0),
                Vec2::new(-2.0, -2.0),
            ];

            for pos in positions {
                assert!(spiral.is_in_pull_range(pos));
                let pull = spiral.calculate_pull(pos);
                assert!(pull.length() > 0.0, "Each enemy should be pulled");
            }
        }

        #[test]
        fn test_vortex_cleanup_removes_all_associated_entities() {
            let mut app = bevy::app::App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create a spiral that's already expired
            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            spiral.duration.tick(Duration::from_secs_f32(BLACK_SPIRAL_DURATION + 0.1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            )).id();

            let _ = app.world_mut().run_system_once(black_spiral_cleanup_system);

            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_black_spiral_from_spell() {
            let spell = Spell::new(SpellType::VoidRift);
            let center = Vec2::new(5.0, 15.0);
            let spiral = BlackSpiral::from_spell(center, &spell);

            assert_eq!(spiral.center, center);
            // Damage per tick should be spell damage * tick interval
            let expected_damage_per_tick = spell.damage() * BLACK_SPIRAL_DAMAGE_TICK_INTERVAL;
            assert_eq!(spiral.damage_per_tick, expected_damage_per_tick);
        }

        #[test]
        fn test_black_spiral_uses_dark_element_color() {
            let color = black_spiral_color();
            assert_eq!(color, Element::Dark.color());
        }

        #[test]
        fn test_rotation_angle_updates() {
            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            assert_eq!(spiral.rotation_angle, 0.0);

            spiral.tick(Duration::from_secs_f32(1.0));
            assert!((spiral.rotation_angle - BLACK_SPIRAL_ROTATION_SPEED).abs() < 0.01);
        }
    }

    mod black_spiral_tick_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_black_spiral_tick_updates_timer() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlackSpiral::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(black_spiral_tick_system);

            let spiral = app.world().get::<BlackSpiral>(entity).unwrap();
            assert!(
                spiral.duration.elapsed_secs() > 0.9,
                "Duration timer should have ticked"
            );
        }

        #[test]
        fn test_black_spiral_tick_triggers_should_damage() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlackSpiral::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BLACK_SPIRAL_DAMAGE_TICK_INTERVAL));
            }

            let _ = app.world_mut().run_system_once(black_spiral_tick_system);

            let spiral = app.world().get::<BlackSpiral>(entity).unwrap();
            assert!(spiral.should_damage(), "should_damage should be true after tick interval");
        }
    }

    mod black_spiral_pull_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_black_spiral_applies_pull_to_enemies() {
            let mut app = App::new();

            // Create spiral at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlackSpiral::new(Vec2::ZERO, 20.0),
            ));

            // Create enemy within pull radius with initial velocity
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                Velocity::new(Vec2::ZERO),
            )).id();

            let _ = app.world_mut().run_system_once(black_spiral_pull_system);

            let velocity = app.world().get::<Velocity>(enemy).unwrap();
            assert!(velocity.0.x < 0.0, "Enemy should be pulled toward center (negative x)");
        }

        #[test]
        fn test_black_spiral_does_not_pull_enemies_outside_radius() {
            let mut app = App::new();

            // Create spiral at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BlackSpiral::new(Vec2::ZERO, 20.0),
            ));

            // Create enemy outside pull radius
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
                Velocity::new(Vec2::ZERO),
            )).id();

            let _ = app.world_mut().run_system_once(black_spiral_pull_system);

            let velocity = app.world().get::<Velocity>(enemy).unwrap();
            assert_eq!(velocity.0, Vec2::ZERO, "Enemy outside radius should not be pulled");
        }
    }

    mod black_spiral_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_black_spiral_damages_enemies_in_radius() {
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
            app.add_systems(Update, (black_spiral_damage_system, count_damage_events).chain());

            // Create spiral that should damage
            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            spiral.damage_timer.tick(Duration::from_secs_f32(BLACK_SPIRAL_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_black_spiral_ignores_enemies_outside_radius() {
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
            app.add_systems(Update, (black_spiral_damage_system, count_damage_events).chain());

            // Create spiral that should damage
            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            spiral.damage_timer.tick(Duration::from_secs_f32(BLACK_SPIRAL_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            ));

            // Create enemy outside radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_black_spiral_no_damage_before_tick() {
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
            app.add_systems(Update, (black_spiral_damage_system, count_damage_events).chain());

            // Create spiral that hasn't ticked yet
            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_black_spiral_damages_multiple_enemies() {
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
            app.add_systems(Update, (black_spiral_damage_system, count_damage_events).chain());

            // Create spiral that should damage
            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            spiral.damage_timer.tick(Duration::from_secs_f32(BLACK_SPIRAL_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            ));

            // Create 3 enemies within radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }
    }

    mod black_spiral_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_black_spiral_survives_before_expiry() {
            let mut app = App::new();

            let spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            )).id();

            let _ = app.world_mut().run_system_once(black_spiral_cleanup_system);

            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod spawn_black_spiral_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_spawn_black_spiral_spawns_vortex() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::VoidRift);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_black_spiral(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&BlackSpiral>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_spawn_black_spiral_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::VoidRift);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(15.0, 25.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_black_spiral(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&BlackSpiral>();
            for spiral in query.iter(app.world()) {
                assert_eq!(spiral.center, target_pos);
            }
        }

        #[test]
        fn test_spawn_black_spiral_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::VoidRift);
            let expected_damage_per_tick = spell.damage() * BLACK_SPIRAL_DAMAGE_TICK_INTERVAL;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_black_spiral(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&BlackSpiral>();
            for spiral in query.iter(app.world()) {
                assert_eq!(spiral.damage_per_tick, expected_damage_per_tick);
            }
        }

        #[test]
        fn test_spawn_black_spiral_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::VoidRift);
            let explicit_damage = 150.0;
            let expected_damage_per_tick = explicit_damage * BLACK_SPIRAL_DAMAGE_TICK_INTERVAL;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_black_spiral_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&BlackSpiral>();
            for spiral in query.iter(app.world()) {
                assert_eq!(spiral.damage_per_tick, expected_damage_per_tick);
            }
        }
    }

    mod black_spiral_visual_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_black_spiral_visual_rotates() {
            let mut app = App::new();

            let mut spiral = BlackSpiral::new(Vec2::ZERO, 20.0);
            spiral.rotation_angle = std::f32::consts::PI / 2.0;

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                spiral,
            )).id();

            let _ = app.world_mut().run_system_once(black_spiral_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Check rotation is around Y axis
            let expected_rotation = Quat::from_rotation_y(std::f32::consts::PI / 2.0);
            assert!(
                transform.rotation.abs_diff_eq(expected_rotation, 0.001),
                "Transform should rotate around Y axis"
            );
        }
    }
}
