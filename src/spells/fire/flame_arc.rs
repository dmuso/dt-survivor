use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use rand::Rng;
use std::f32::consts::PI;

/// Configuration for Flame Arc spell
pub const FLAME_ARC_SPEED: f32 = 15.0;
pub const FLAME_ARC_LIFETIME: f32 = 3.0;
pub const FLAME_ARC_COLLISION_RADIUS: f32 = 1.0;
pub const FLAME_ARC_HEIGHT: f32 = 5.0; // Maximum arc height at midpoint
pub const FLAME_ARC_BASE_RANGE: f32 = 20.0; // Default horizontal range

/// Fragment configuration
pub const FRAGMENT_COUNT_MIN: u8 = 4;
pub const FRAGMENT_COUNT_MAX: u8 = 6;
pub const FRAGMENT_SPEED: f32 = 12.0;
pub const FRAGMENT_LIFETIME: f32 = 0.8;
pub const FRAGMENT_DAMAGE_RATIO: f32 = 0.35; // 35% of main projectile damage
pub const FRAGMENT_COLLISION_RADIUS: f32 = 0.5;

/// Main Flame Arc projectile that follows a parabolic arc trajectory
#[derive(Component, Debug, Clone)]
pub struct FlameArcProjectile {
    /// Damage dealt by the main projectile
    pub damage: f32,
    /// Number of fragments to spawn on impact (4-6)
    pub fragment_count: u8,
    /// Damage each fragment deals
    pub fragment_damage: f32,
}

impl FlameArcProjectile {
    pub fn new(damage: f32) -> Self {
        let mut rng = rand::thread_rng();
        let fragment_count = rng.gen_range(FRAGMENT_COUNT_MIN..=FRAGMENT_COUNT_MAX);
        Self {
            damage,
            fragment_count,
            fragment_damage: damage * FRAGMENT_DAMAGE_RATIO,
        }
    }

    pub fn with_fragment_count(damage: f32, fragment_count: u8) -> Self {
        Self {
            damage,
            fragment_count,
            fragment_damage: damage * FRAGMENT_DAMAGE_RATIO,
        }
    }
}

/// Tracks the arc trajectory from start to target position
#[derive(Component, Debug, Clone)]
pub struct FlameArcTrajectory {
    /// Starting position (XZ plane)
    pub start_pos: Vec2,
    /// Target position (XZ plane)
    pub target_pos: Vec2,
    /// Time elapsed in the arc
    pub elapsed: f32,
    /// Total duration of the arc
    pub duration: f32,
    /// Maximum height of the arc
    pub arc_height: f32,
    /// Starting Y position
    pub start_y: f32,
}

impl FlameArcTrajectory {
    pub fn new(start_pos: Vec2, target_pos: Vec2, start_y: f32) -> Self {
        let distance = start_pos.distance(target_pos);
        let duration = distance / FLAME_ARC_SPEED;
        Self {
            start_pos,
            target_pos,
            elapsed: 0.0,
            duration,
            arc_height: FLAME_ARC_HEIGHT,
            start_y,
        }
    }

    /// Returns the progress through the arc (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Returns true if the arc is complete
    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Calculate the current 3D position along the parabolic arc
    pub fn current_position(&self) -> Vec3 {
        let t = self.progress();

        // Linear interpolation for XZ position
        let xz = self.start_pos.lerp(self.target_pos, t);

        // Parabolic arc for Y: y = 4h * t * (1-t) where h is max height
        // This gives a parabola that starts at 0, peaks at t=0.5 with height h, ends at 0
        let arc_y = 4.0 * self.arc_height * t * (1.0 - t);
        let y = self.start_y + arc_y;

        Vec3::new(xz.x, y, xz.y)
    }
}

/// Fragment spawned when Flame Arc impacts
#[derive(Component, Debug, Clone)]
pub struct FlameFragment {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Damage dealt on hit
    pub damage: f32,
    /// Lifetime timer
    pub lifetime: Timer,
}

impl FlameFragment {
    pub fn new(direction: Vec2, damage: f32) -> Self {
        Self {
            direction: direction.normalize(),
            speed: FRAGMENT_SPEED,
            damage,
            lifetime: Timer::from_seconds(FRAGMENT_LIFETIME, TimerMode::Once),
        }
    }
}

/// Get the fire element color for visual effects
pub fn flame_arc_color() -> Color {
    Element::Fire.color()
}

/// Cast Flame Arc spell - spawns an arcing projectile toward target
#[allow(clippy::too_many_arguments)]
pub fn fire_flame_arc(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_flame_arc_with_damage(
        commands,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast Flame Arc spell with explicit damage
#[allow(clippy::too_many_arguments)]
pub fn fire_flame_arc_with_damage(
    commands: &mut Commands,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let spawn_xz = from_xz(spawn_position);
    let trajectory = FlameArcTrajectory::new(spawn_xz, target_pos, spawn_position.y);
    let projectile = FlameArcProjectile::new(damage);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.fireball.clone()),
            Transform::from_translation(spawn_position),
            projectile,
            trajectory,
        ));
    } else {
        commands.spawn((
            Transform::from_translation(spawn_position),
            projectile,
            trajectory,
        ));
    }
}

/// System that moves Flame Arc projectiles along their parabolic trajectory
pub fn flame_arc_movement_system(
    mut query: Query<(&mut Transform, &mut FlameArcTrajectory), With<FlameArcProjectile>>,
    time: Res<Time>,
) {
    for (mut transform, mut trajectory) in query.iter_mut() {
        trajectory.elapsed += time.delta_secs();
        transform.translation = trajectory.current_position();
    }
}

/// Message fired when a Flame Arc reaches its destination or hits an enemy
#[derive(Message, Debug, Clone)]
pub struct FlameArcImpactEvent {
    pub flame_arc_entity: Entity,
    pub impact_position: Vec3,
    pub fragment_count: u8,
    pub fragment_damage: f32,
    pub hit_enemy: Option<Entity>,
}

/// System that detects when Flame Arc reaches destination or hits an enemy
pub fn flame_arc_impact_detection(
    query: Query<(Entity, &Transform, &FlameArcProjectile, &FlameArcTrajectory)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut impact_events: MessageWriter<FlameArcImpactEvent>,
) {
    for (entity, transform, projectile, trajectory) in query.iter() {
        let arc_xz = Vec2::new(transform.translation.x, transform.translation.z);

        // Check for enemy collision
        let mut hit_enemy = None;
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            if arc_xz.distance(enemy_xz) < FLAME_ARC_COLLISION_RADIUS {
                hit_enemy = Some(enemy_entity);
                break;
            }
        }

        // Fire impact event if hit enemy or reached destination
        if hit_enemy.is_some() || trajectory.is_complete() {
            impact_events.write(FlameArcImpactEvent {
                flame_arc_entity: entity,
                impact_position: transform.translation,
                fragment_count: projectile.fragment_count,
                fragment_damage: projectile.fragment_damage,
                hit_enemy,
            });
        }
    }
}

/// System that handles Flame Arc impact - spawns fragments and applies damage
pub fn flame_arc_impact_effects(
    mut commands: Commands,
    mut impact_events: MessageReader<FlameArcImpactEvent>,
    query: Query<&FlameArcProjectile>,
    mut damage_events: MessageWriter<DamageEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for event in impact_events.read() {
        // Apply damage if hit enemy
        if let Some(enemy_entity) = event.hit_enemy {
            if let Ok(projectile) = query.get(event.flame_arc_entity) {
                damage_events.write(DamageEvent::new(enemy_entity, projectile.damage));
            }
        }

        // Spawn fragments
        spawn_flame_fragments(
            &mut commands,
            event.impact_position,
            event.fragment_count,
            event.fragment_damage,
            game_meshes.as_deref(),
            game_materials.as_deref(),
        );

        // Despawn the flame arc projectile
        commands.entity(event.flame_arc_entity).try_despawn();
    }
}

/// Spawns flame fragments in random directions from the impact point
pub fn spawn_flame_fragments(
    commands: &mut Commands,
    impact_position: Vec3,
    fragment_count: u8,
    fragment_damage: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let mut rng = rand::thread_rng();

    for _ in 0..fragment_count {
        // Random direction on XZ plane
        let angle = rng.gen_range(0.0..2.0 * PI);
        let direction = Vec2::new(angle.cos(), angle.sin());

        let fragment = FlameFragment::new(direction, fragment_damage);

        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.fireball.clone()),
                Transform::from_translation(impact_position),
                fragment,
            ));
        } else {
            commands.spawn((
                Transform::from_translation(impact_position),
                fragment,
            ));
        }
    }
}

/// System that moves flame fragments outward from explosion point
pub fn flame_fragment_movement_system(
    mut query: Query<(&mut Transform, &FlameFragment)>,
    time: Res<Time>,
) {
    for (mut transform, fragment) in query.iter_mut() {
        let movement = fragment.direction * fragment.speed * time.delta_secs();
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles fragment lifetime and despawning
pub fn flame_fragment_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FlameFragment)>,
) {
    for (entity, mut fragment) in query.iter_mut() {
        fragment.lifetime.tick(time.delta());
        if fragment.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Message fired when a flame fragment collides with an enemy
#[derive(Message, Debug, Clone)]
pub struct FlameFragmentCollisionEvent {
    pub fragment_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that detects flame fragment collisions with enemies
pub fn flame_fragment_collision_detection(
    query: Query<(Entity, &Transform), With<FlameFragment>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<FlameFragmentCollisionEvent>,
) {
    for (fragment_entity, fragment_transform) in query.iter() {
        let fragment_xz = Vec2::new(
            fragment_transform.translation.x,
            fragment_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );

            if fragment_xz.distance(enemy_xz) < FRAGMENT_COLLISION_RADIUS {
                collision_events.write(FlameFragmentCollisionEvent {
                    fragment_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per fragment
            }
        }
    }
}

/// System that applies damage when fragments hit enemies
pub fn flame_fragment_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<FlameFragmentCollisionEvent>,
    query: Query<&FlameFragment>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for event in collision_events.read() {
        if let Ok(fragment) = query.get(event.fragment_entity) {
            damage_events.write(DamageEvent::new(event.enemy_entity, fragment.damage));
        }
        commands.entity(event.fragment_entity).try_despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    mod flame_arc_projectile_tests {
        use super::*;

        #[test]
        fn test_flame_arc_projectile_new() {
            let projectile = FlameArcProjectile::new(100.0);
            assert_eq!(projectile.damage, 100.0);
            assert!(projectile.fragment_count >= FRAGMENT_COUNT_MIN);
            assert!(projectile.fragment_count <= FRAGMENT_COUNT_MAX);
            assert_eq!(projectile.fragment_damage, 100.0 * FRAGMENT_DAMAGE_RATIO);
        }

        #[test]
        fn test_flame_arc_projectile_with_fragment_count() {
            let projectile = FlameArcProjectile::with_fragment_count(50.0, 5);
            assert_eq!(projectile.damage, 50.0);
            assert_eq!(projectile.fragment_count, 5);
            assert_eq!(projectile.fragment_damage, 50.0 * FRAGMENT_DAMAGE_RATIO);
        }

        #[test]
        fn test_flame_arc_fragment_damage_ratio() {
            let projectile = FlameArcProjectile::new(200.0);
            // Fragment damage should be 35% of main damage
            assert_eq!(projectile.fragment_damage, 200.0 * 0.35);
        }

        #[test]
        fn test_flame_arc_uses_fire_element_color() {
            let color = flame_arc_color();
            assert_eq!(color, Element::Fire.color());
        }
    }

    mod flame_arc_trajectory_tests {
        use super::*;

        #[test]
        fn test_trajectory_new() {
            let start = Vec2::new(0.0, 0.0);
            let target = Vec2::new(20.0, 0.0);
            let trajectory = FlameArcTrajectory::new(start, target, 0.5);

            assert_eq!(trajectory.start_pos, start);
            assert_eq!(trajectory.target_pos, target);
            assert_eq!(trajectory.elapsed, 0.0);
            assert_eq!(trajectory.arc_height, FLAME_ARC_HEIGHT);
            assert_eq!(trajectory.start_y, 0.5);
        }

        #[test]
        fn test_trajectory_duration_based_on_distance() {
            let start = Vec2::new(0.0, 0.0);
            let target = Vec2::new(30.0, 0.0);
            let trajectory = FlameArcTrajectory::new(start, target, 0.0);

            // Duration = distance / speed = 30 / 15 = 2.0
            assert!((trajectory.duration - 2.0).abs() < 0.001);
        }

        #[test]
        fn test_trajectory_progress() {
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::ZERO,
                Vec2::new(15.0, 0.0), // 15 units = 1 second at speed 15
                0.0,
            );

            assert_eq!(trajectory.progress(), 0.0);

            trajectory.elapsed = 0.5; // Half way through
            assert!((trajectory.progress() - 0.5).abs() < 0.001);

            trajectory.elapsed = 1.0; // Complete
            assert!((trajectory.progress() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_trajectory_is_complete() {
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::ZERO,
                Vec2::new(15.0, 0.0),
                0.0,
            );

            assert!(!trajectory.is_complete());
            trajectory.elapsed = 0.5;
            assert!(!trajectory.is_complete());
            trajectory.elapsed = 1.0;
            assert!(trajectory.is_complete());
        }

        #[test]
        fn test_trajectory_current_position_at_start() {
            let trajectory = FlameArcTrajectory::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 0.0),
                0.5,
            );

            let pos = trajectory.current_position();
            assert!((pos.x - 0.0).abs() < 0.001);
            assert!((pos.y - 0.5).abs() < 0.001); // Start Y
            assert!((pos.z - 0.0).abs() < 0.001);
        }

        #[test]
        fn test_trajectory_current_position_at_end() {
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 0.0),
                0.5,
            );
            trajectory.elapsed = trajectory.duration;

            let pos = trajectory.current_position();
            assert!((pos.x - 20.0).abs() < 0.001);
            assert!((pos.y - 0.5).abs() < 0.001); // Back to start Y
            assert!((pos.z - 0.0).abs() < 0.001);
        }

        #[test]
        fn test_trajectory_hits_apex_at_midpoint() {
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 0.0),
                0.0,
            );
            trajectory.elapsed = trajectory.duration / 2.0;

            let pos = trajectory.current_position();
            // At midpoint (t=0.5), arc_y = 4 * h * 0.5 * 0.5 = h
            assert!((pos.y - FLAME_ARC_HEIGHT).abs() < 0.001);
        }

        #[test]
        fn test_trajectory_xz_interpolation() {
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 10.0),
                0.0,
            );
            trajectory.elapsed = trajectory.duration / 2.0;

            let pos = trajectory.current_position();
            // At midpoint, should be halfway to target
            assert!((pos.x - 10.0).abs() < 0.001);
            assert!((pos.z - 5.0).abs() < 0.001);
        }
    }

    mod flame_fragment_tests {
        use super::*;

        #[test]
        fn test_fragment_new() {
            let fragment = FlameFragment::new(Vec2::new(1.0, 0.0), 25.0);
            assert_eq!(fragment.direction, Vec2::new(1.0, 0.0));
            assert_eq!(fragment.speed, FRAGMENT_SPEED);
            assert_eq!(fragment.damage, 25.0);
            assert!(!fragment.lifetime.is_finished());
        }

        #[test]
        fn test_fragment_direction_normalized() {
            let fragment = FlameFragment::new(Vec2::new(3.0, 4.0), 10.0);
            // Should be normalized to length 1
            assert!((fragment.direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_fragment_lifetime_duration() {
            let fragment = FlameFragment::new(Vec2::X, 10.0);
            assert_eq!(
                fragment.lifetime.duration(),
                Duration::from_secs_f32(FRAGMENT_LIFETIME)
            );
        }
    }

    mod fire_flame_arc_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_flame_arc_spawns_projectile() {
            let mut app = setup_test_app();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(20.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_flame_arc_with_damage(
                    &mut commands,
                    100.0,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FlameArcProjectile>();
            assert_eq!(query.iter(app.world()).count(), 1);
        }

        #[test]
        fn test_fire_flame_arc_has_trajectory() {
            let mut app = setup_test_app();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(20.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_flame_arc_with_damage(
                    &mut commands,
                    100.0,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FlameArcTrajectory>();
            let trajectory = query.iter(app.world()).next().unwrap();
            assert_eq!(trajectory.target_pos, target_pos);
        }

        #[test]
        fn test_fire_flame_arc_direction_toward_target() {
            let mut app = setup_test_app();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(20.0, 10.0);

            {
                let mut commands = app.world_mut().commands();
                fire_flame_arc_with_damage(
                    &mut commands,
                    100.0,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FlameArcTrajectory>();
            let trajectory = query.iter(app.world()).next().unwrap();
            assert_eq!(trajectory.start_pos, Vec2::new(0.0, 0.0));
            assert_eq!(trajectory.target_pos, target_pos);
        }
    }

    mod flame_arc_movement_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_flame_arc_follows_parabola() {
            let mut app = setup_test_app();

            // Spawn a flame arc moving 15 units (1 second duration at speed 15)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameArcProjectile::with_fragment_count(100.0, 5),
                FlameArcTrajectory::new(
                    Vec2::new(0.0, 0.0),
                    Vec2::new(15.0, 0.0),
                    0.5,
                ),
            )).id();

            // Advance to midpoint (0.5 seconds)
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(flame_arc_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // At midpoint: x should be ~7.5, y should be at apex (0.5 + FLAME_ARC_HEIGHT)
            assert!((transform.translation.x - 7.5).abs() < 0.1);
            assert!((transform.translation.y - (0.5 + FLAME_ARC_HEIGHT)).abs() < 0.1);
        }

        #[test]
        fn test_flame_arc_reaches_target() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameArcProjectile::with_fragment_count(100.0, 5),
                FlameArcTrajectory::new(
                    Vec2::new(0.0, 0.0),
                    Vec2::new(15.0, 0.0),
                    0.5,
                ),
            )).id();

            // Advance past the full duration (1 second for 15 units at speed 15)
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.1));
            }

            let _ = app.world_mut().run_system_once(flame_arc_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Should be at target position
            assert!((transform.translation.x - 15.0).abs() < 0.1);
            assert!((transform.translation.y - 0.5).abs() < 0.1); // Back to start height
            assert!(transform.translation.z.abs() < 0.1);
        }
    }

    mod flame_arc_impact_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<FlameArcImpactEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_flame_arc_spawns_fragments_on_complete() {
            let mut app = setup_test_app();
            app.add_message::<FlameFragmentCollisionEvent>();

            // Spawn a flame arc that's already complete
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(15.0, 0.0),
                0.5,
            );
            trajectory.elapsed = trajectory.duration; // Mark as complete

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(15.0, 0.5, 0.0)),
                FlameArcProjectile::with_fragment_count(100.0, 5),
                trajectory,
            ));

            // Run impact detection and effects
            app.add_systems(Update, (
                flame_arc_impact_detection,
                flame_arc_impact_effects,
            ).chain());
            app.update();

            // Should have spawned 5 fragments
            let mut query = app.world_mut().query::<&FlameFragment>();
            assert_eq!(query.iter(app.world()).count(), 5);
        }

        #[test]
        fn test_flame_arc_damages_on_hit() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = setup_test_app();
            app.add_message::<FlameFragmentCollisionEvent>();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Spawn flame arc at enemy position
            let mut trajectory = FlameArcTrajectory::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(5.0, 0.0),
                0.5,
            );
            trajectory.elapsed = trajectory.duration * 0.99; // Almost complete

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(4.5, 0.5, 0.0)),
                FlameArcProjectile::with_fragment_count(100.0, 5),
                trajectory,
            ));

            // Spawn enemy at collision range
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(4.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.add_systems(Update, (
                flame_arc_impact_detection,
                flame_arc_impact_effects,
                count_damage,
            ).chain());
            app.update();

            // Should have sent a damage event for the direct hit
            assert!(counter.0.load(Ordering::SeqCst) >= 1);
        }
    }

    mod flame_fragment_movement_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fragment_moves_in_direction() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::new(1.0, 0.0), 10.0),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(flame_fragment_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!((transform.translation.x - FRAGMENT_SPEED).abs() < 0.1);
            assert_eq!(transform.translation.y, 0.5); // Y unchanged
        }

        #[test]
        fn test_fragment_scatter_different_directions() {
            let mut app = setup_test_app();

            // Spawn fragments in different directions
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::new(1.0, 0.0), 10.0),
            )).id();

            let entity2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::new(-1.0, 0.0), 10.0),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(flame_fragment_movement_system);

            let transform1 = app.world().get::<Transform>(entity1).unwrap();
            let transform2 = app.world().get::<Transform>(entity2).unwrap();

            // Should have moved in opposite directions
            assert!(transform1.translation.x > 0.0);
            assert!(transform2.translation.x < 0.0);
        }
    }

    mod flame_fragment_lifetime_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fragment_despawns_after_lifetime() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::X, 10.0),
            )).id();

            // Advance time past fragment lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(FRAGMENT_LIFETIME + 0.1));
            }

            let _ = app.world_mut().run_system_once(flame_fragment_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_fragment_survives_before_lifetime() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::X, 10.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(FRAGMENT_LIFETIME / 2.0));
            }

            let _ = app.world_mut().run_system_once(flame_fragment_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod flame_fragment_collision_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<FlameFragmentCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_fragment_collision_fires_event() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<FlameFragmentCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (flame_fragment_collision_detection, count_collisions).chain());

            // Spawn fragment at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::X, 10.0),
            ));

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.2, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_fragment_damages_enemy() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (
                flame_fragment_collision_detection,
                flame_fragment_collision_effects,
                count_damage,
            ).chain());

            // Spawn fragment
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::X, 25.0),
            ));

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.2, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_fragment_despawns_on_hit() {
            let mut app = setup_test_app();

            app.add_systems(Update, (
                flame_fragment_collision_detection,
                flame_fragment_collision_effects,
            ).chain());

            let fragment_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::X, 10.0),
            )).id();

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.2, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert!(!app.world().entities().contains(fragment_entity));
        }

        #[test]
        fn test_fragment_no_collision_when_far() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<FlameFragmentCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (flame_fragment_collision_detection, count_collisions).chain());

            // Spawn fragment at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlameFragment::new(Vec2::X, 10.0),
            ));

            // Spawn enemy far away
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }
    }
}
