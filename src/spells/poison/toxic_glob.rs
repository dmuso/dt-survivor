//! Toxic Glob spell - Slow-moving projectile that bursts into poison pools.
//!
//! A Poison element spell (Miasma SpellType) that fires a slow-moving projectile.
//! On enemy hit or lifetime expiry, it bursts into 3-5 poison puddles that damage
//! enemies over time.

use std::collections::HashSet;
use bevy::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Toxic Glob spell
pub const TOXIC_GLOB_SPEED: f32 = 4.0; // Slow-moving projectile
pub const TOXIC_GLOB_LIFETIME: f32 = 3.0; // Seconds before auto-burst
pub const TOXIC_GLOB_COLLISION_RADIUS: f32 = 1.0; // Collision detection radius
pub const TOXIC_GLOB_MIN_PUDDLES: u32 = 3;
pub const TOXIC_GLOB_MAX_PUDDLES: u32 = 5;
pub const POISON_PUDDLE_RADIUS: f32 = 2.0;
pub const POISON_PUDDLE_DURATION: f32 = 4.0;
pub const POISON_PUDDLE_TICK_INTERVAL: f32 = 0.5;
pub const POISON_PUDDLE_DAMAGE_RATIO: f32 = 0.2; // 20% of spell damage per tick
pub const POISON_PUDDLE_SPREAD_RADIUS: f32 = 2.5; // How far puddles spread from burst point

/// Get the poison element color for visual effects
pub fn toxic_glob_color() -> Color {
    Element::Poison.color()
}

/// Slow-moving poison projectile that bursts into puddles.
#[derive(Component, Debug, Clone)]
pub struct ToxicGlobProjectile {
    /// Direction the glob is moving (normalized)
    pub direction: Vec2,
    /// Movement speed
    pub speed: f32,
    /// Lifetime timer (bursts when finished)
    pub lifetime: Timer,
    /// Collision radius for enemy detection
    pub collision_radius: f32,
    /// Damage for burst/puddles
    pub damage: f32,
    /// Number of puddles to spawn on burst
    pub puddle_count: u32,
}

impl ToxicGlobProjectile {
    pub fn new(direction: Vec2, damage: f32) -> Self {
        let mut rng = rand::thread_rng();
        let puddle_count = rng.gen_range(TOXIC_GLOB_MIN_PUDDLES..=TOXIC_GLOB_MAX_PUDDLES);
        Self {
            direction: direction.normalize_or_zero(),
            speed: TOXIC_GLOB_SPEED,
            lifetime: Timer::from_seconds(TOXIC_GLOB_LIFETIME, TimerMode::Once),
            collision_radius: TOXIC_GLOB_COLLISION_RADIUS,
            damage,
            puddle_count,
        }
    }

    pub fn with_puddle_count(mut self, count: u32) -> Self {
        self.puddle_count = count;
        self
    }

    /// Check if the projectile lifetime has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }

    /// Tick the lifetime timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.lifetime.tick(delta);
    }

    /// Check if a position is within collision radius
    pub fn collides_with(&self, glob_pos: Vec2, enemy_pos: Vec2) -> bool {
        glob_pos.distance(enemy_pos) <= self.collision_radius
    }
}

/// Persistent poison pool that damages enemies over time.
#[derive(Component, Debug, Clone)]
pub struct PoisonPuddle {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the damage zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Damage per tick
    pub tick_damage: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Set of enemies damaged this tick (prevents double damage)
    pub hit_this_tick: HashSet<Entity>,
}

impl PoisonPuddle {
    pub fn new(center: Vec2, base_damage: f32) -> Self {
        let tick_damage = base_damage * POISON_PUDDLE_DAMAGE_RATIO;
        Self {
            center,
            radius: POISON_PUDDLE_RADIUS,
            duration: Timer::from_seconds(POISON_PUDDLE_DURATION, TimerMode::Once),
            tick_damage,
            tick_timer: Timer::from_seconds(POISON_PUDDLE_TICK_INTERVAL, TimerMode::Repeating),
            hit_this_tick: HashSet::new(),
        }
    }

    /// Check if the puddle has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick both timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);

        // Reset hit tracking each tick
        if self.tick_timer.just_finished() {
            self.hit_this_tick.clear();
        }
    }

    /// Check if ready to apply damage
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }

    /// Check if an enemy is in range and hasn't been damaged this tick
    pub fn can_damage(&self, entity: Entity, enemy_pos: Vec2) -> bool {
        let distance = self.center.distance(enemy_pos);
        distance <= self.radius && !self.hit_this_tick.contains(&entity)
    }

    /// Mark an enemy as damaged this tick
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_this_tick.insert(entity);
    }
}

/// System that moves toxic glob projectiles
pub fn toxic_glob_movement_system(
    mut projectile_query: Query<(&mut Transform, &ToxicGlobProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, glob) in projectile_query.iter_mut() {
        let movement = Vec3::new(
            glob.direction.x * glob.speed * time.delta_secs(),
            0.0,
            glob.direction.y * glob.speed * time.delta_secs(),
        );
        transform.translation += movement;
    }
}

/// System that ticks toxic glob lifetime and handles timeout burst
pub fn toxic_glob_lifetime_system(
    mut commands: Commands,
    mut projectile_query: Query<(Entity, &Transform, &mut ToxicGlobProjectile)>,
    time: Res<Time>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (entity, transform, mut glob) in projectile_query.iter_mut() {
        glob.tick(time.delta());

        if glob.is_expired() {
            // Burst into puddles at current position
            let burst_pos = from_xz(transform.translation);
            spawn_puddles(
                &mut commands,
                burst_pos,
                glob.damage,
                glob.puddle_count,
                game_meshes.as_deref(),
                game_materials.as_deref(),
            );
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects enemy collision with toxic glob
pub fn toxic_glob_collision_system(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Transform, &ToxicGlobProjectile)>,
    enemy_query: Query<&Transform, With<Enemy>>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (entity, glob_transform, glob) in projectile_query.iter() {
        let glob_pos = from_xz(glob_transform.translation);

        for enemy_transform in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if glob.collides_with(glob_pos, enemy_pos) {
                // Burst into puddles at collision point
                spawn_puddles(
                    &mut commands,
                    glob_pos,
                    glob.damage,
                    glob.puddle_count,
                    game_meshes.as_deref(),
                    game_materials.as_deref(),
                );
                commands.entity(entity).despawn();
                break; // Only burst once
            }
        }
    }
}

/// System that applies damage to enemies in poison puddles
pub fn poison_puddle_damage_system(
    mut puddle_query: Query<&mut PoisonPuddle>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    time: Res<Time>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut puddle in puddle_query.iter_mut() {
        puddle.tick(time.delta());

        if puddle.should_damage() {
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);

                if puddle.can_damage(enemy_entity, enemy_pos) {
                    damage_events.write(DamageEvent::new(enemy_entity, puddle.tick_damage));
                    puddle.mark_hit(enemy_entity);
                }
            }
        }
    }
}

/// System that despawns expired poison puddles
pub fn poison_puddle_cleanup_system(
    mut commands: Commands,
    puddle_query: Query<(Entity, &PoisonPuddle)>,
) {
    for (entity, puddle) in puddle_query.iter() {
        if puddle.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn puddles in a spread pattern around the burst point
fn spawn_puddles(
    commands: &mut Commands,
    burst_pos: Vec2,
    damage: f32,
    puddle_count: u32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let mut rng = rand::thread_rng();

    for _ in 0..puddle_count {
        // Random offset from burst position
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let distance = rng.gen_range(0.0..POISON_PUDDLE_SPREAD_RADIUS);
        let offset = Vec2::new(angle.cos() * distance, angle.sin() * distance);
        let puddle_center = burst_pos + offset;

        let puddle = PoisonPuddle::new(puddle_center, damage);
        let puddle_pos = Vec3::new(puddle_center.x, 0.1, puddle_center.y);

        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.explosion.clone()),
                MeshMaterial3d(materials.poison_cloud.clone()),
                Transform::from_translation(puddle_pos).with_scale(Vec3::splat(puddle.radius)),
                puddle,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(puddle_pos),
                puddle,
            ));
        }
    }
}

/// Cast toxic glob spell - spawns a slow-moving projectile toward target.
#[allow(clippy::too_many_arguments)]
pub fn fire_toxic_glob(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_toxic_glob_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast toxic glob spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_toxic_glob_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let start_pos = from_xz(spawn_position);
    let direction = (target_pos - start_pos).normalize_or_zero();

    let projectile = ToxicGlobProjectile::new(direction, damage);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.poison_projectile.clone()),
            Transform::from_translation(spawn_position)
                .with_scale(Vec3::splat(projectile.collision_radius * 1.5)),
            projectile,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            projectile,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod toxic_glob_projectile_tests {
        use super::*;

        #[test]
        fn test_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let projectile = ToxicGlobProjectile::new(direction, 30.0)
                .with_puddle_count(4); // Fixed count for test

            assert_eq!(projectile.direction, direction);
            assert_eq!(projectile.speed, TOXIC_GLOB_SPEED);
            assert_eq!(projectile.damage, 30.0);
            assert_eq!(projectile.puddle_count, 4);
            assert!(!projectile.is_expired());
        }

        #[test]
        fn test_projectile_normalizes_direction() {
            let unnormalized = Vec2::new(3.0, 4.0);
            let projectile = ToxicGlobProjectile::new(unnormalized, 30.0);

            assert!((projectile.direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_projectile_is_expired() {
            let mut projectile = ToxicGlobProjectile::new(Vec2::X, 30.0);
            assert!(!projectile.is_expired());

            projectile.tick(Duration::from_secs_f32(TOXIC_GLOB_LIFETIME + 0.1));
            assert!(projectile.is_expired());
        }

        #[test]
        fn test_projectile_collides_within_radius() {
            let projectile = ToxicGlobProjectile::new(Vec2::X, 30.0);
            let glob_pos = Vec2::ZERO;
            let close_pos = Vec2::new(0.5, 0.0);

            assert!(projectile.collides_with(glob_pos, close_pos));
        }

        #[test]
        fn test_projectile_does_not_collide_outside_radius() {
            let projectile = ToxicGlobProjectile::new(Vec2::X, 30.0);
            let glob_pos = Vec2::ZERO;
            let far_pos = Vec2::new(10.0, 0.0);

            assert!(!projectile.collides_with(glob_pos, far_pos));
        }

        #[test]
        fn test_uses_poison_element_color() {
            let color = toxic_glob_color();
            assert_eq!(color, Element::Poison.color());
        }
    }

    mod poison_puddle_tests {
        use super::*;

        #[test]
        fn test_puddle_new() {
            let center = Vec2::new(5.0, 5.0);
            let puddle = PoisonPuddle::new(center, 30.0);

            assert_eq!(puddle.center, center);
            assert_eq!(puddle.radius, POISON_PUDDLE_RADIUS);
            assert_eq!(puddle.tick_damage, 30.0 * POISON_PUDDLE_DAMAGE_RATIO);
            assert!(!puddle.is_expired());
        }

        #[test]
        fn test_puddle_is_expired() {
            let mut puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            assert!(!puddle.is_expired());

            puddle.tick(Duration::from_secs_f32(POISON_PUDDLE_DURATION + 0.1));
            assert!(puddle.is_expired());
        }

        #[test]
        fn test_puddle_should_damage_on_tick() {
            let mut puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            assert!(!puddle.should_damage());

            puddle.tick(Duration::from_secs_f32(POISON_PUDDLE_TICK_INTERVAL + 0.01));
            assert!(puddle.should_damage());
        }

        #[test]
        fn test_puddle_can_damage_in_range() {
            let puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(1.0, 0.0);

            assert!(puddle.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_puddle_cannot_damage_out_of_range() {
            let puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let out_of_range_pos = Vec2::new(100.0, 0.0);

            assert!(!puddle.can_damage(entity, out_of_range_pos));
        }

        #[test]
        fn test_puddle_cannot_damage_already_hit() {
            let mut puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(1.0, 0.0);

            puddle.mark_hit(entity);
            assert!(!puddle.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_puddle_resets_hit_tracking_on_tick() {
            let mut puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);

            puddle.mark_hit(entity);
            assert!(puddle.hit_this_tick.contains(&entity));

            // Tick to next damage interval
            puddle.tick(Duration::from_secs_f32(POISON_PUDDLE_TICK_INTERVAL + 0.01));
            assert!(puddle.hit_this_tick.is_empty());
        }
    }

    mod movement_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_toxic_glob_moves_slowly() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ToxicGlobProjectile::new(Vec2::X, 30.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(toxic_glob_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Should move TOXIC_GLOB_SPEED units per second
            assert!(
                (transform.translation.x - TOXIC_GLOB_SPEED).abs() < 0.1,
                "Expected X ~{}, got {}",
                TOXIC_GLOB_SPEED,
                transform.translation.x
            );
        }

        #[test]
        fn test_toxic_glob_moves_in_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ToxicGlobProjectile::new(Vec2::new(0.0, 1.0), 30.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(toxic_glob_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Direction is (0, 1) in XZ plane, so Z should increase
            assert!(
                (transform.translation.z - TOXIC_GLOB_SPEED).abs() < 0.1,
                "Expected Z ~{}, got {}",
                TOXIC_GLOB_SPEED,
                transform.translation.z
            );
        }
    }

    mod lifetime_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_glob_despawns_on_timeout() {
            let mut app = setup_test_app();

            let mut glob = ToxicGlobProjectile::new(Vec2::X, 30.0)
                .with_puddle_count(3);
            glob.lifetime = Timer::from_seconds(0.0, TimerMode::Once);
            glob.lifetime.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                glob,
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(toxic_glob_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_glob_spawns_puddles_on_timeout() {
            let mut app = setup_test_app();

            let mut glob = ToxicGlobProjectile::new(Vec2::X, 30.0)
                .with_puddle_count(3);
            glob.lifetime = Timer::from_seconds(0.0, TimerMode::Once);
            glob.lifetime.tick(Duration::from_secs(1));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.5, 5.0)),
                glob,
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(toxic_glob_lifetime_system);

            let mut puddle_query = app.world_mut().query::<&PoisonPuddle>();
            let count = puddle_query.iter(app.world()).count();
            assert_eq!(count, 3, "Should spawn 3 puddles");
        }

        #[test]
        fn test_glob_survives_before_timeout() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ToxicGlobProjectile::new(Vec2::X, 30.0),
            )).id();

            // Small time advance
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(toxic_glob_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod collision_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_glob_bursts_on_enemy_collision() {
            let mut app = setup_test_app();

            let glob_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ToxicGlobProjectile::new(Vec2::X, 30.0).with_puddle_count(4),
            )).id();

            // Enemy at same position (collision)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(toxic_glob_collision_system);

            assert!(!app.world().entities().contains(glob_entity), "Glob should despawn on collision");
        }

        #[test]
        fn test_glob_spawns_puddles_on_enemy_collision() {
            let mut app = setup_test_app();

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ToxicGlobProjectile::new(Vec2::X, 30.0).with_puddle_count(4),
            ));

            // Enemy at same position (collision)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(toxic_glob_collision_system);

            let mut puddle_query = app.world_mut().query::<&PoisonPuddle>();
            let count = puddle_query.iter(app.world()).count();
            assert_eq!(count, 4, "Should spawn 4 puddles on collision");
        }

        #[test]
        fn test_glob_survives_without_collision() {
            let mut app = setup_test_app();

            let glob_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ToxicGlobProjectile::new(Vec2::X, 30.0),
            )).id();

            // Enemy far away
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(toxic_glob_collision_system);

            assert!(app.world().entities().contains(glob_entity), "Glob should survive without collision");
        }
    }

    mod puddle_damage_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_puddle_damages_enemies_in_range() {
            let mut app = setup_test_app();

            // Create puddle at origin
            let puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                puddle,
            ));

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(POISON_PUDDLE_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(poison_puddle_damage_system);

            // Check that enemy was marked as hit
            let mut puddle_query = app.world_mut().query::<&PoisonPuddle>();
            let puddle = puddle_query.single(app.world()).unwrap();
            assert!(!puddle.hit_this_tick.is_empty(), "Enemy should be marked as hit");
        }

        #[test]
        fn test_puddle_ignores_enemies_out_of_range() {
            let mut app = setup_test_app();

            // Create puddle at origin
            let puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                puddle,
            ));

            // Create enemy far away
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(POISON_PUDDLE_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(poison_puddle_damage_system);

            // Check that no enemy was marked as hit
            let mut puddle_query = app.world_mut().query::<&PoisonPuddle>();
            let puddle = puddle_query.single(app.world()).unwrap();
            assert!(puddle.hit_this_tick.is_empty(), "No enemy should be hit");
        }

        #[test]
        fn test_multiple_puddles_damage_independently() {
            let mut app = setup_test_app();

            // Create 2 puddles at different positions
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PoisonPuddle::new(Vec2::ZERO, 30.0),
            ));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
                PoisonPuddle::new(Vec2::new(10.0, 0.0), 30.0),
            ));

            // Create 2 enemies, one near each puddle
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
            ));
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.5, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(POISON_PUDDLE_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(poison_puddle_damage_system);

            // Each puddle should have hit one enemy
            let mut puddle_query = app.world_mut().query::<&PoisonPuddle>();
            for puddle in puddle_query.iter(app.world()) {
                assert_eq!(puddle.hit_this_tick.len(), 1, "Each puddle should hit one enemy");
            }
        }
    }

    mod puddle_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_puddle_despawns_when_expired() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let mut puddle = PoisonPuddle::new(Vec2::ZERO, 30.0);
            puddle.duration = Timer::from_seconds(0.0, TimerMode::Once);
            puddle.duration.tick(Duration::from_secs(1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                puddle,
            )).id();

            let _ = app.world_mut().run_system_once(poison_puddle_cleanup_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_puddle_survives_before_expiry() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PoisonPuddle::new(Vec2::ZERO, 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(poison_puddle_cleanup_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod fire_toxic_glob_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_toxic_glob_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Miasma);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_toxic_glob(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ToxicGlobProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_toxic_glob_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Miasma);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_toxic_glob(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ToxicGlobProjectile>();
            for projectile in query.iter(app.world()) {
                assert_eq!(projectile.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_toxic_glob_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Miasma);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_toxic_glob_with_damage(
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

            let mut query = app.world_mut().query::<&ToxicGlobProjectile>();
            for projectile in query.iter(app.world()) {
                assert_eq!(projectile.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_toxic_glob_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Miasma);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_toxic_glob(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ToxicGlobProjectile>();
            for projectile in query.iter(app.world()) {
                assert!(
                    projectile.direction.x > 0.9,
                    "Glob should face toward target (+X), got direction {:?}",
                    projectile.direction
                );
            }
        }
    }
}
