//! Warp Rift spell - Creates a space tear that pulls enemies toward its center.
//!
//! A Chaos element spell (Paradox SpellType) that tears space at a target location,
//! creating a rift with a strong gravitational pull effect on enemies. Enemies near
//! the center take damage. The pull strength falls off with distance.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, Velocity};
use crate::spell::components::Spell;

/// Default configuration for Warp Rift spell
pub const WARP_RIFT_PULL_RADIUS: f32 = 8.0;
pub const WARP_RIFT_DAMAGE_RADIUS: f32 = 2.0;
pub const WARP_RIFT_PULL_STRENGTH: f32 = 150.0;
pub const WARP_RIFT_DURATION: f32 = 4.0;
pub const WARP_RIFT_DAMAGE_TICK_INTERVAL: f32 = 0.5;
pub const WARP_RIFT_VISUAL_HEIGHT: f32 = 0.5;

/// Get the chaos element color for visual effects (magenta/pink)
pub fn warp_rift_color() -> Color {
    Element::Chaos.color()
}

/// Component for the Warp Rift zone.
/// Creates a rift at a location that pulls enemies toward its center and damages
/// those close to the core.
#[derive(Component, Debug, Clone)]
pub struct WarpRift {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the pull effect
    pub pull_radius: f32,
    /// Radius of the damage zone (center)
    pub damage_radius: f32,
    /// Strength of the pull force
    pub pull_strength: f32,
    /// Remaining duration of the rift
    pub duration: Timer,
    /// Damage dealt per tick to enemies in damage_radius
    pub damage_per_tick: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
}

impl WarpRift {
    /// Create a new warp rift at the given center position.
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            pull_radius: WARP_RIFT_PULL_RADIUS,
            damage_radius: WARP_RIFT_DAMAGE_RADIUS,
            pull_strength: WARP_RIFT_PULL_STRENGTH,
            duration: Timer::from_seconds(WARP_RIFT_DURATION, TimerMode::Once),
            damage_per_tick: damage,
            tick_timer: Timer::from_seconds(WARP_RIFT_DAMAGE_TICK_INTERVAL, TimerMode::Repeating),
        }
    }

    /// Create a warp rift from a Spell component.
    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the rift has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the rift timers.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);
    }

    /// Check if damage should be applied this frame.
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }

    /// Check if an entity at the given position is within the pull radius.
    pub fn is_in_pull_range(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.pull_radius
    }

    /// Check if an entity at the given position is within the damage radius.
    pub fn is_in_damage_range(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.damage_radius
    }

    /// Calculate the pull force for an entity at the given position.
    /// Returns a velocity vector pointing toward the rift center.
    /// Pull strength falls off with distance from center.
    pub fn calculate_pull(&self, position: Vec2) -> Vec2 {
        let distance = self.center.distance(position);
        if distance <= 0.01 || distance > self.pull_radius {
            return Vec2::ZERO;
        }

        // Direction toward center
        let direction = (self.center - position).normalize();

        // Pull strength falls off with distance (stronger when closer to edge, weaker at center)
        // Using inverse falloff: strength is stronger closer to the rift
        let normalized_distance = distance / self.pull_radius;
        // Inverse: closer = stronger pull. At edge (1.0) = base strength, at center = very strong
        let falloff = 1.0 / normalized_distance.max(0.1);
        let clamped_falloff = falloff.min(10.0); // Cap maximum pull strength

        direction * self.pull_strength * clamped_falloff * 0.1
    }
}

/// System that ticks warp rift timers.
pub fn warp_rift_tick_system(
    mut rift_query: Query<&mut WarpRift>,
    time: Res<Time>,
) {
    for mut rift in rift_query.iter_mut() {
        rift.tick(time.delta());
    }
}

/// System that applies pull force to enemies within the rift's pull radius.
pub fn warp_rift_pull_system(
    rift_query: Query<&WarpRift>,
    mut enemy_query: Query<(&Transform, &mut Velocity), With<Enemy>>,
) {
    for rift in rift_query.iter() {
        for (transform, mut velocity) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(transform.translation);

            if rift.is_in_pull_range(enemy_pos) {
                let pull = rift.calculate_pull(enemy_pos);
                // Add pull force to enemy velocity
                velocity.0 += pull;
            }
        }
    }
}

/// System that damages enemies within the rift's damage radius.
pub fn warp_rift_damage_system(
    rift_query: Query<&WarpRift>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for rift in rift_query.iter() {
        if !rift.should_damage() {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if rift.is_in_damage_range(enemy_pos) {
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    rift.damage_per_tick,
                    Element::Chaos,
                ));
            }
        }
    }
}

/// System that despawns warp rifts when their duration expires.
pub fn warp_rift_cleanup_system(
    mut commands: Commands,
    rift_query: Query<(Entity, &WarpRift)>,
) {
    for (entity, rift) in rift_query.iter() {
        if rift.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast warp rift spell - spawns a zone at target location.
#[allow(clippy::too_many_arguments)]
pub fn spawn_warp_rift(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    spawn_warp_rift_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast warp rift spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn spawn_warp_rift_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    _spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let rift = WarpRift::new(target_pos, damage);
    let rift_pos = Vec3::new(target_pos.x, WARP_RIFT_VISUAL_HEIGHT, target_pos.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.chaos_aoe.clone()), // Transparent chaos AOE material
            Transform::from_translation(rift_pos).with_scale(Vec3::splat(rift.pull_radius)),
            rift,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(rift_pos),
            rift,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod warp_rift_component_tests {
        use super::*;

        #[test]
        fn test_warp_rift_spawns_at_correct_location() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 25.0;
            let rift = WarpRift::new(center, damage);

            assert_eq!(rift.center, center);
            assert_eq!(rift.damage_per_tick, damage);
            assert_eq!(rift.pull_radius, WARP_RIFT_PULL_RADIUS);
            assert_eq!(rift.damage_radius, WARP_RIFT_DAMAGE_RADIUS);
            assert!(!rift.is_expired());
        }

        #[test]
        fn test_warp_rift_has_correct_radii() {
            let rift = WarpRift::new(Vec2::ZERO, 20.0);
            assert_eq!(rift.pull_radius, WARP_RIFT_PULL_RADIUS);
            assert_eq!(rift.damage_radius, WARP_RIFT_DAMAGE_RADIUS);
            assert!(rift.damage_radius < rift.pull_radius);
        }

        #[test]
        fn test_warp_rift_pulls_enemies_in_range() {
            let rift = WarpRift::new(Vec2::ZERO, 20.0);

            // Enemy at edge of pull radius
            let enemy_pos = Vec2::new(WARP_RIFT_PULL_RADIUS - 1.0, 0.0);
            assert!(rift.is_in_pull_range(enemy_pos));

            let pull = rift.calculate_pull(enemy_pos);
            assert!(pull.length() > 0.0, "Should have pull force");
            assert!(pull.x < 0.0, "Pull should point toward center (negative x)");
        }

        #[test]
        fn test_warp_rift_pull_strength_scales_with_distance() {
            let rift = WarpRift::new(Vec2::ZERO, 20.0);

            // Enemy closer to center
            let close_pos = Vec2::new(2.0, 0.0);
            let close_pull = rift.calculate_pull(close_pos);

            // Enemy further from center
            let far_pos = Vec2::new(6.0, 0.0);
            let far_pull = rift.calculate_pull(far_pos);

            // Closer enemies should have stronger pull (inverse falloff)
            assert!(
                close_pull.length() > far_pull.length(),
                "Closer enemies should have stronger pull: close={}, far={}",
                close_pull.length(),
                far_pull.length()
            );
        }

        #[test]
        fn test_warp_rift_damages_enemies_at_center() {
            let rift = WarpRift::new(Vec2::ZERO, 20.0);

            // Enemy at center
            let center_pos = Vec2::new(0.5, 0.0);
            assert!(rift.is_in_damage_range(center_pos));

            // Enemy at edge of damage radius
            let edge_pos = Vec2::new(WARP_RIFT_DAMAGE_RADIUS, 0.0);
            assert!(rift.is_in_damage_range(edge_pos));
        }

        #[test]
        fn test_warp_rift_does_not_pull_outside_radius() {
            let rift = WarpRift::new(Vec2::ZERO, 20.0);

            // Enemy outside pull radius
            let outside_pos = Vec2::new(WARP_RIFT_PULL_RADIUS + 1.0, 0.0);
            assert!(!rift.is_in_pull_range(outside_pos));

            let pull = rift.calculate_pull(outside_pos);
            assert_eq!(pull, Vec2::ZERO);
        }

        #[test]
        fn test_warp_rift_duration_expires() {
            let mut rift = WarpRift::new(Vec2::ZERO, 20.0);
            assert!(!rift.is_expired());

            // Tick past duration
            rift.tick(Duration::from_secs_f32(WARP_RIFT_DURATION + 0.1));
            assert!(rift.is_expired());
        }

        #[test]
        fn test_warp_rift_despawns_after_duration() {
            let mut app = bevy::app::App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create a rift that's already expired
            let mut rift = WarpRift::new(Vec2::ZERO, 20.0);
            rift.duration.tick(Duration::from_secs_f32(WARP_RIFT_DURATION + 0.1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                rift,
            )).id();

            let _ = app.world_mut().run_system_once(warp_rift_cleanup_system);

            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_fast_enemies_can_escape_pull() {
            let rift = WarpRift::new(Vec2::ZERO, 20.0);

            // Enemy at edge of pull radius
            let enemy_pos = Vec2::new(7.0, 0.0);
            let pull = rift.calculate_pull(enemy_pos);

            // A fast enemy with speed > pull strength could escape
            let fast_enemy_speed = 200.0; // Higher than typical pull force
            let escape_velocity = Vec2::new(1.0, 0.0) * fast_enemy_speed;

            // Combined velocity would move away from rift if escape > pull
            let net_velocity = escape_velocity + pull;
            assert!(
                net_velocity.x > 0.0,
                "Fast enemy should be able to escape: net velocity x = {}",
                net_velocity.x
            );
        }

        #[test]
        fn test_warp_rift_from_spell() {
            let spell = Spell::new(SpellType::Paradox);
            let center = Vec2::new(5.0, 15.0);
            let rift = WarpRift::from_spell(center, &spell);

            assert_eq!(rift.center, center);
            assert_eq!(rift.damage_per_tick, spell.damage());
        }

        #[test]
        fn test_warp_rift_uses_chaos_element_color() {
            let color = warp_rift_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod warp_rift_tick_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_warp_rift_tick_updates_timer() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                WarpRift::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(warp_rift_tick_system);

            let rift = app.world().get::<WarpRift>(entity).unwrap();
            assert!(
                rift.duration.elapsed_secs() > 0.9,
                "Duration timer should have ticked"
            );
        }

        #[test]
        fn test_warp_rift_tick_triggers_should_damage() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                WarpRift::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(WARP_RIFT_DAMAGE_TICK_INTERVAL));
            }

            let _ = app.world_mut().run_system_once(warp_rift_tick_system);

            let rift = app.world().get::<WarpRift>(entity).unwrap();
            assert!(rift.should_damage(), "should_damage should be true after tick interval");
        }
    }

    mod warp_rift_pull_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_warp_rift_applies_pull_to_enemies() {
            let mut app = App::new();

            // Create rift at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                WarpRift::new(Vec2::ZERO, 20.0),
            ));

            // Create enemy within pull radius with initial velocity
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                Velocity::new(Vec2::ZERO),
            )).id();

            let _ = app.world_mut().run_system_once(warp_rift_pull_system);

            let velocity = app.world().get::<Velocity>(enemy).unwrap();
            assert!(velocity.0.x < 0.0, "Enemy should be pulled toward center (negative x)");
        }

        #[test]
        fn test_warp_rift_does_not_pull_enemies_outside_radius() {
            let mut app = App::new();

            // Create rift at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                WarpRift::new(Vec2::ZERO, 20.0),
            ));

            // Create enemy outside pull radius
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
                Velocity::new(Vec2::ZERO),
            )).id();

            let _ = app.world_mut().run_system_once(warp_rift_pull_system);

            let velocity = app.world().get::<Velocity>(enemy).unwrap();
            assert_eq!(velocity.0, Vec2::ZERO, "Enemy outside radius should not be pulled");
        }
    }

    mod warp_rift_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_warp_rift_damages_enemies_in_damage_zone() {
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
            app.add_systems(Update, (warp_rift_damage_system, count_damage_events).chain());

            // Create rift that should damage
            let mut rift = WarpRift::new(Vec2::ZERO, 20.0);
            rift.tick_timer.tick(Duration::from_secs_f32(WARP_RIFT_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                rift,
            ));

            // Create enemy within damage radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_warp_rift_ignores_enemies_outside_damage_radius() {
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
            app.add_systems(Update, (warp_rift_damage_system, count_damage_events).chain());

            // Create rift that should damage
            let mut rift = WarpRift::new(Vec2::ZERO, 20.0);
            rift.tick_timer.tick(Duration::from_secs_f32(WARP_RIFT_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                rift,
            ));

            // Create enemy in pull radius but outside damage radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_warp_rift_no_damage_before_tick() {
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
            app.add_systems(Update, (warp_rift_damage_system, count_damage_events).chain());

            // Create rift that hasn't ticked yet
            let rift = WarpRift::new(Vec2::ZERO, 20.0);

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                rift,
            ));

            // Create enemy within damage radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }
    }

    mod warp_rift_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_warp_rift_survives_before_expiry() {
            let mut app = App::new();

            let rift = WarpRift::new(Vec2::ZERO, 20.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                rift,
            )).id();

            let _ = app.world_mut().run_system_once(warp_rift_cleanup_system);

            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod spawn_warp_rift_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_spawn_warp_rift_spawns_zone() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_warp_rift(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&WarpRift>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_spawn_warp_rift_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(15.0, 25.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_warp_rift(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&WarpRift>();
            for rift in query.iter(app.world()) {
                assert_eq!(rift.center, target_pos);
            }
        }

        #[test]
        fn test_spawn_warp_rift_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_warp_rift(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&WarpRift>();
            for rift in query.iter(app.world()) {
                assert_eq!(rift.damage_per_tick, expected_damage);
            }
        }

        #[test]
        fn test_spawn_warp_rift_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let explicit_damage = 150.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                spawn_warp_rift_with_damage(
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

            let mut query = app.world_mut().query::<&WarpRift>();
            for rift in query.iter(app.world()) {
                assert_eq!(rift.damage_per_tick, explicit_damage);
            }
        }
    }
}
