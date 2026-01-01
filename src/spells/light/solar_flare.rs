//! Solar Flare spell - A bright explosion that damages and blinds enemies.
//!
//! A Light element spell (Smite SpellType) that fires a projectile which explodes
//! on enemy contact or at max range. The explosion deals damage and applies a
//! BlindedDebuff to affected enemies, causing them to move randomly for a short duration.

use std::collections::HashSet;
use bevy::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Solar Flare spell
pub const SOLAR_FLARE_SPEED: f32 = 18.0;
pub const SOLAR_FLARE_MAX_RANGE: f32 = 15.0;
pub const SOLAR_FLARE_COLLISION_RADIUS: f32 = 1.0;
pub const SOLAR_FLARE_EXPLOSION_RADIUS: f32 = 4.0;
pub const SOLAR_FLARE_VISUAL_HEIGHT: f32 = 0.5;

/// Blind debuff configuration
pub const BLIND_DURATION: f32 = 2.5;
pub const BLIND_DIRECTION_CHANGE_INTERVAL: f32 = 0.3;

/// Get the light element color for visual effects (white/gold)
pub fn solar_flare_color() -> Color {
    Element::Light.color()
}

/// Solar Flare projectile component.
#[derive(Component, Debug, Clone)]
pub struct SolarFlareProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Starting position for max range calculation
    pub start_position: Vec2,
    /// Maximum travel distance
    pub max_range: f32,
    /// Base damage dealt on explosion
    pub damage: f32,
    /// Explosion radius
    pub explosion_radius: f32,
    /// Duration of the blind effect
    pub blind_duration: f32,
}

impl SolarFlareProjectile {
    /// Create a new solar flare projectile.
    pub fn new(start_position: Vec2, direction: Vec2, damage: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed: SOLAR_FLARE_SPEED,
            start_position,
            max_range: SOLAR_FLARE_MAX_RANGE,
            damage,
            explosion_radius: SOLAR_FLARE_EXPLOSION_RADIUS,
            blind_duration: BLIND_DURATION,
        }
    }

    /// Calculate distance traveled from start position.
    pub fn distance_traveled(&self, current_position: Vec2) -> f32 {
        self.start_position.distance(current_position)
    }

    /// Check if projectile has exceeded max range.
    pub fn is_at_max_range(&self, current_position: Vec2) -> bool {
        self.distance_traveled(current_position) >= self.max_range
    }
}

/// Solar Flare explosion component - created when projectile explodes.
#[derive(Component, Debug, Clone)]
pub struct SolarFlareExplosion {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Explosion radius
    pub radius: f32,
    /// Damage to deal
    pub damage: f32,
    /// Duration of the blind effect to apply
    pub blind_duration: f32,
    /// Set of enemies already hit (prevents double damage)
    pub hit_enemies: HashSet<Entity>,
    /// Lifetime timer for cleanup
    pub lifetime: Timer,
}

impl SolarFlareExplosion {
    pub fn new(center: Vec2, radius: f32, damage: f32, blind_duration: f32) -> Self {
        Self {
            center,
            radius,
            damage,
            blind_duration,
            hit_enemies: HashSet::new(),
            lifetime: Timer::from_seconds(0.1, TimerMode::Once), // Quick lifetime, just for hit detection
        }
    }

    /// Check if a position is within the explosion radius.
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }

    /// Check if an enemy has already been hit.
    pub fn has_hit(&self, entity: Entity) -> bool {
        self.hit_enemies.contains(&entity)
    }

    /// Mark an enemy as hit.
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }

    /// Check if explosion should be cleaned up.
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }

    /// Tick the lifetime.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.lifetime.tick(delta);
    }
}

/// Blinded debuff - causes enemies to move in random directions.
#[derive(Component, Debug, Clone)]
pub struct BlindedDebuff {
    /// Remaining duration
    pub duration: Timer,
    /// Current random movement direction
    pub random_direction: Vec2,
    /// Timer for changing direction
    pub direction_change_timer: Timer,
}

impl BlindedDebuff {
    pub fn new(duration_secs: f32) -> Self {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let direction = Vec2::new(angle.cos(), angle.sin());

        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            random_direction: direction,
            direction_change_timer: Timer::from_seconds(
                BLIND_DIRECTION_CHANGE_INTERVAL,
                TimerMode::Repeating,
            ),
        }
    }

    pub fn default_config() -> Self {
        Self::new(BLIND_DURATION)
    }

    /// Check if the debuff has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the debuff timers.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.direction_change_timer.tick(delta);

        // Change direction periodically
        if self.direction_change_timer.just_finished() {
            let mut rng = rand::thread_rng();
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            self.random_direction = Vec2::new(angle.cos(), angle.sin());
        }
    }

    /// Get the current random movement direction.
    pub fn get_direction(&self) -> Vec2 {
        self.random_direction
    }
}

/// Event fired when a Solar Flare projectile needs to explode.
#[derive(Message)]
pub struct SolarFlareExplosionEvent {
    pub position: Vec2,
    pub damage: f32,
    pub radius: f32,
    pub blind_duration: f32,
}

/// System that moves Solar Flare projectiles.
pub fn solar_flare_movement_system(
    mut projectile_query: Query<(&mut Transform, &SolarFlareProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, projectile) in projectile_query.iter_mut() {
        let movement = Vec3::new(
            projectile.direction.x * projectile.speed * time.delta_secs(),
            0.0,
            projectile.direction.y * projectile.speed * time.delta_secs(),
        );
        transform.translation += movement;
    }
}

/// System that checks for projectile collision with enemies or max range.
pub fn solar_flare_collision_system(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Transform, &SolarFlareProjectile)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut explosion_events: MessageWriter<SolarFlareExplosionEvent>,
) {
    for (projectile_entity, projectile_transform, projectile) in projectile_query.iter() {
        let projectile_xz = from_xz(projectile_transform.translation);
        let mut should_explode = false;

        // Check for enemy collision
        for (_enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = from_xz(enemy_transform.translation);
            let distance = projectile_xz.distance(enemy_xz);

            if distance <= SOLAR_FLARE_COLLISION_RADIUS {
                should_explode = true;
                break;
            }
        }

        // Check for max range
        if !should_explode && projectile.is_at_max_range(projectile_xz) {
            should_explode = true;
        }

        if should_explode {
            // Fire explosion event
            explosion_events.write(SolarFlareExplosionEvent {
                position: projectile_xz,
                damage: projectile.damage,
                radius: projectile.explosion_radius,
                blind_duration: projectile.blind_duration,
            });

            // Despawn the projectile
            commands.entity(projectile_entity).despawn();
        }
    }
}

/// System that spawns explosions from explosion events.
pub fn solar_flare_spawn_explosion_system(
    mut commands: Commands,
    mut explosion_events: MessageReader<SolarFlareExplosionEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for event in explosion_events.read() {
        let explosion = SolarFlareExplosion::new(
            event.position,
            event.radius,
            event.damage,
            event.blind_duration,
        );

        let explosion_pos = Vec3::new(event.position.x, SOLAR_FLARE_VISUAL_HEIGHT, event.position.y);

        if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), game_materials.as_ref()) {
            commands.spawn((
                Mesh3d(meshes.explosion.clone()),
                MeshMaterial3d(materials.radiant_beam.clone()),
                Transform::from_translation(explosion_pos).with_scale(Vec3::splat(event.radius)),
                explosion,
            ));
        } else {
            // Fallback for tests
            commands.spawn((
                Transform::from_translation(explosion_pos),
                explosion,
            ));
        }
    }
}

/// System that applies damage and blind to enemies in explosions.
pub fn solar_flare_explosion_damage_system(
    mut commands: Commands,
    mut explosion_query: Query<&mut SolarFlareExplosion>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut explosion in explosion_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = from_xz(enemy_transform.translation);

            if explosion.contains(enemy_xz) && !explosion.has_hit(enemy_entity) {
                // Apply damage
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    explosion.damage,
                    Element::Light,
                ));

                // Apply blind debuff
                commands
                    .entity(enemy_entity)
                    .try_insert(BlindedDebuff::new(explosion.blind_duration));

                explosion.mark_hit(enemy_entity);
            }
        }
    }
}

/// System that cleans up expired explosions.
pub fn solar_flare_explosion_cleanup_system(
    mut commands: Commands,
    mut explosion_query: Query<(Entity, &mut SolarFlareExplosion)>,
    time: Res<Time>,
) {
    for (entity, mut explosion) in explosion_query.iter_mut() {
        explosion.tick(time.delta());

        if explosion.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that ticks blind debuffs and removes expired ones.
pub fn blinded_debuff_tick_system(
    mut commands: Commands,
    mut blinded_query: Query<(Entity, &mut BlindedDebuff)>,
    time: Res<Time>,
) {
    for (entity, mut blinded) in blinded_query.iter_mut() {
        blinded.tick(time.delta());

        if blinded.is_expired() {
            commands.entity(entity).remove::<BlindedDebuff>();
        }
    }
}

/// Cast Solar Flare (Smite) spell - spawns a projectile that explodes on impact.
#[allow(clippy::too_many_arguments)]
pub fn fire_solar_flare(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_solar_flare_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast Solar Flare spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_solar_flare_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let spawn_xz = from_xz(spawn_position);
    let direction = (target_pos - spawn_xz).normalize_or_zero();

    let projectile = SolarFlareProjectile::new(spawn_xz, direction, damage);
    let projectile_pos = Vec3::new(spawn_position.x, SOLAR_FLARE_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.radiant_beam.clone()),
            Transform::from_translation(projectile_pos),
            projectile,
        ));
    } else {
        // Fallback for tests
        commands.spawn((
            Transform::from_translation(projectile_pos),
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

    mod solar_flare_projectile_tests {
        use super::*;

        #[test]
        fn test_solar_flare_projectile_new() {
            let start_pos = Vec2::new(5.0, 10.0);
            let direction = Vec2::new(1.0, 0.0);
            let damage = 25.0;
            let projectile = SolarFlareProjectile::new(start_pos, direction, damage);

            assert_eq!(projectile.start_position, start_pos);
            assert_eq!(projectile.direction, direction.normalize());
            assert_eq!(projectile.damage, damage);
            assert_eq!(projectile.speed, SOLAR_FLARE_SPEED);
            assert_eq!(projectile.max_range, SOLAR_FLARE_MAX_RANGE);
            assert_eq!(projectile.explosion_radius, SOLAR_FLARE_EXPLOSION_RADIUS);
            assert_eq!(projectile.blind_duration, BLIND_DURATION);
        }

        #[test]
        fn test_solar_flare_projectile_normalizes_direction() {
            let start_pos = Vec2::ZERO;
            let direction = Vec2::new(3.0, 4.0); // Length = 5
            let projectile = SolarFlareProjectile::new(start_pos, direction, 20.0);

            assert!((projectile.direction.length() - 1.0).abs() < 0.001);
            assert!((projectile.direction.x - 0.6).abs() < 0.001);
            assert!((projectile.direction.y - 0.8).abs() < 0.001);
        }

        #[test]
        fn test_solar_flare_projectile_handles_zero_direction() {
            let start_pos = Vec2::ZERO;
            let direction = Vec2::ZERO;
            let projectile = SolarFlareProjectile::new(start_pos, direction, 20.0);

            assert_eq!(projectile.direction, Vec2::ZERO);
        }

        #[test]
        fn test_solar_flare_projectile_distance_traveled() {
            let start_pos = Vec2::ZERO;
            let projectile = SolarFlareProjectile::new(start_pos, Vec2::X, 20.0);

            let current_pos = Vec2::new(5.0, 0.0);
            assert!((projectile.distance_traveled(current_pos) - 5.0).abs() < 0.001);
        }

        #[test]
        fn test_solar_flare_projectile_is_at_max_range() {
            let start_pos = Vec2::ZERO;
            let projectile = SolarFlareProjectile::new(start_pos, Vec2::X, 20.0);

            let within_range = Vec2::new(10.0, 0.0);
            assert!(!projectile.is_at_max_range(within_range));

            let at_max_range = Vec2::new(SOLAR_FLARE_MAX_RANGE, 0.0);
            assert!(projectile.is_at_max_range(at_max_range));

            let beyond_range = Vec2::new(20.0, 0.0);
            assert!(projectile.is_at_max_range(beyond_range));
        }

        #[test]
        fn test_solar_flare_uses_light_element_color() {
            let color = solar_flare_color();
            assert_eq!(color, Element::Light.color());
        }
    }

    mod solar_flare_explosion_tests {
        use super::*;

        #[test]
        fn test_solar_flare_explosion_new() {
            let center = Vec2::new(10.0, 20.0);
            let radius = 5.0;
            let damage = 30.0;
            let blind_duration = 3.0;
            let explosion = SolarFlareExplosion::new(center, radius, damage, blind_duration);

            assert_eq!(explosion.center, center);
            assert_eq!(explosion.radius, radius);
            assert_eq!(explosion.damage, damage);
            assert_eq!(explosion.blind_duration, blind_duration);
            assert!(explosion.hit_enemies.is_empty());
            assert!(!explosion.is_expired());
        }

        #[test]
        fn test_solar_flare_explosion_contains() {
            let explosion = SolarFlareExplosion::new(Vec2::ZERO, 5.0, 20.0, 2.0);

            assert!(explosion.contains(Vec2::new(2.0, 0.0)));
            assert!(explosion.contains(Vec2::new(0.0, 3.0)));
            assert!(explosion.contains(Vec2::new(5.0, 0.0))); // Edge
            assert!(explosion.contains(Vec2::ZERO)); // Center
            assert!(!explosion.contains(Vec2::new(6.0, 0.0)));
            assert!(!explosion.contains(Vec2::new(10.0, 10.0)));
        }

        #[test]
        fn test_solar_flare_explosion_hit_tracking() {
            let mut explosion = SolarFlareExplosion::new(Vec2::ZERO, 5.0, 20.0, 2.0);
            let entity = Entity::from_bits(1);

            assert!(!explosion.has_hit(entity));

            explosion.mark_hit(entity);
            assert!(explosion.has_hit(entity));
        }

        #[test]
        fn test_solar_flare_explosion_expires() {
            let mut explosion = SolarFlareExplosion::new(Vec2::ZERO, 5.0, 20.0, 2.0);

            assert!(!explosion.is_expired());

            explosion.tick(Duration::from_secs_f32(0.2));
            assert!(explosion.is_expired());
        }
    }

    mod blinded_debuff_tests {
        use super::*;

        #[test]
        fn test_blinded_debuff_new() {
            let debuff = BlindedDebuff::new(3.0);

            assert!(!debuff.is_expired());
            // Direction should be normalized
            assert!((debuff.random_direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_blinded_debuff_default_config() {
            let debuff = BlindedDebuff::default_config();

            assert!(!debuff.is_expired());
        }

        #[test]
        fn test_blinded_debuff_expires() {
            let mut debuff = BlindedDebuff::new(1.0);

            assert!(!debuff.is_expired());

            debuff.tick(Duration::from_secs_f32(0.5));
            assert!(!debuff.is_expired());

            debuff.tick(Duration::from_secs_f32(0.6));
            assert!(debuff.is_expired());
        }

        #[test]
        fn test_blinded_debuff_get_direction() {
            let debuff = BlindedDebuff::new(3.0);
            let direction = debuff.get_direction();

            assert!((direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_blinded_debuff_changes_direction() {
            let mut debuff = BlindedDebuff::new(5.0);
            let initial_direction = debuff.random_direction;

            // Tick past the direction change interval multiple times
            // Direction is random so we can't guarantee it changes,
            // but the tick should work without error
            debuff.tick(Duration::from_secs_f32(BLIND_DIRECTION_CHANGE_INTERVAL + 0.01));

            // Direction might or might not be different (it's random)
            // Just verify the tick works and direction is still normalized
            assert!((debuff.random_direction.length() - 1.0).abs() < 0.001);
            let _ = initial_direction; // Avoid unused warning
        }
    }

    mod solar_flare_movement_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_solar_flare_moves_in_direction() {
            let mut app = setup_test_app();

            let start_pos = Vec2::ZERO;
            let direction = Vec2::new(1.0, 0.0);
            let projectile = SolarFlareProjectile::new(start_pos, direction, 20.0);

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                projectile,
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(solar_flare_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!(
                (transform.translation.x - SOLAR_FLARE_SPEED).abs() < 0.1,
                "Should move at SOLAR_FLARE_SPEED in X direction"
            );
            assert!(transform.translation.z.abs() < 0.01);
        }

        #[test]
        fn test_solar_flare_moves_on_xz_plane() {
            let mut app = setup_test_app();

            let start_pos = Vec2::ZERO;
            let direction = Vec2::new(0.0, 1.0); // Should move in Z direction
            let projectile = SolarFlareProjectile::new(start_pos, direction, 20.0);

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)), // Start at Y=1
                projectile,
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(solar_flare_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!(
                (transform.translation.y - 1.0).abs() < 0.01,
                "Y position should not change"
            );
            assert!(
                transform.translation.z > 0.0,
                "Should move in positive Z direction"
            );
        }
    }

    mod solar_flare_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_message::<SolarFlareExplosionEvent>();
            app
        }

        #[test]
        fn test_solar_flare_explodes_on_enemy_collision() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct ExplosionEventCounter(Arc<AtomicUsize>);

            fn count_events(
                mut events: MessageReader<SolarFlareExplosionEvent>,
                counter: Res<ExplosionEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = ExplosionEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_collision_system, count_events).chain());

            // Create projectile at origin
            let projectile = SolarFlareProjectile::new(Vec2::ZERO, Vec2::X, 20.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                projectile,
            ));

            // Create enemy within collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.5, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Should fire explosion event");
        }

        #[test]
        fn test_solar_flare_explodes_at_max_range() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct ExplosionEventCounter(Arc<AtomicUsize>);

            fn count_events(
                mut events: MessageReader<SolarFlareExplosionEvent>,
                counter: Res<ExplosionEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = ExplosionEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_collision_system, count_events).chain());

            // Create projectile that has traveled max range
            let projectile = SolarFlareProjectile::new(Vec2::ZERO, Vec2::X, 20.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(SOLAR_FLARE_MAX_RANGE, 0.5, 0.0)),
                projectile,
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Should fire explosion event at max range");
        }

        #[test]
        fn test_solar_flare_no_explosion_before_collision_or_max_range() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct ExplosionEventCounter(Arc<AtomicUsize>);

            fn count_events(
                mut events: MessageReader<SolarFlareExplosionEvent>,
                counter: Res<ExplosionEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = ExplosionEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_collision_system, count_events).chain());

            // Create projectile mid-flight
            let projectile = SolarFlareProjectile::new(Vec2::ZERO, Vec2::X, 20.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.5, 0.0)),
                projectile,
            ));

            // Create enemy outside collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Should not explode yet");
        }

        #[test]
        fn test_solar_flare_despawns_on_explosion() {
            let mut app = setup_test_app();
            app.add_systems(Update, solar_flare_collision_system);

            // Create projectile at max range
            let projectile = SolarFlareProjectile::new(Vec2::ZERO, Vec2::X, 20.0);
            let projectile_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(SOLAR_FLARE_MAX_RANGE, 0.5, 0.0)),
                projectile,
            )).id();

            app.update();

            assert!(
                app.world().get_entity(projectile_entity).is_err(),
                "Projectile should despawn after explosion"
            );
        }
    }

    mod solar_flare_explosion_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_explosion_damages_enemies_in_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_explosion_damage_system, count_damage).chain());

            // Create explosion at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 2.0),
            ));

            // Create enemy within explosion radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_explosion_applies_blind_debuff() {
            let mut app = setup_test_app();
            app.add_systems(Update, solar_flare_explosion_damage_system);

            // Create explosion
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 3.0),
            ));

            // Create enemy within explosion radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            app.update();

            let debuff = app.world().get::<BlindedDebuff>(enemy_entity);
            assert!(debuff.is_some(), "Enemy should have BlindedDebuff");
        }

        #[test]
        fn test_explosion_does_not_damage_enemies_outside_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_explosion_damage_system, count_damage).chain());

            // Create explosion at origin with radius 5
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 2.0),
            ));

            // Create enemy outside explosion radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_explosion_damages_enemy_only_once() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_explosion_damage_system, count_damage).chain());

            // Create explosion
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 2.0),
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Should only damage once");
        }

        #[test]
        fn test_explosion_damages_multiple_enemies() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, (solar_flare_explosion_damage_system, count_damage).chain());

            // Create explosion
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 2.0),
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

    mod blinded_debuff_tick_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_blinded_debuff_removed_when_expired() {
            let mut app = setup_test_app();

            let mut debuff = BlindedDebuff::new(0.5);
            debuff.duration.tick(Duration::from_secs_f32(0.6)); // Force expired

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                debuff,
            )).id();

            let _ = app.world_mut().run_system_once(blinded_debuff_tick_system);

            assert!(
                app.world().get::<BlindedDebuff>(enemy_entity).is_none(),
                "Expired debuff should be removed"
            );
        }

        #[test]
        fn test_blinded_debuff_persists_before_expiry() {
            let mut app = setup_test_app();

            let debuff = BlindedDebuff::new(10.0);

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                debuff,
            )).id();

            // Advance time but not past expiry
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(blinded_debuff_tick_system);

            assert!(
                app.world().get::<BlindedDebuff>(enemy_entity).is_some(),
                "Debuff should persist before expiry"
            );
        }
    }

    mod fire_solar_flare_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_solar_flare_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Smite);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_solar_flare(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SolarFlareProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn one projectile");
        }

        #[test]
        fn test_fire_solar_flare_correct_direction() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Smite);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_solar_flare(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SolarFlareProjectile>();
            for projectile in query.iter(app.world()) {
                assert!(
                    (projectile.direction.x - 1.0).abs() < 0.01,
                    "Direction should be towards target"
                );
            }
        }

        #[test]
        fn test_fire_solar_flare_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Smite);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_solar_flare_with_damage(
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

            let mut query = app.world_mut().query::<&SolarFlareProjectile>();
            for projectile in query.iter(app.world()) {
                assert_eq!(projectile.damage, explicit_damage);
            }
        }
    }

    mod solar_flare_explosion_cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_expired_explosion_despawns() {
            let mut app = setup_test_app();

            let mut explosion = SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 2.0);
            explosion.tick(Duration::from_secs_f32(0.2)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                explosion,
            )).id();

            let _ = app.world_mut().run_system_once(solar_flare_explosion_cleanup_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Expired explosion should despawn"
            );
        }

        #[test]
        fn test_active_explosion_survives() {
            let mut app = setup_test_app();

            let explosion = SolarFlareExplosion::new(Vec2::ZERO, 5.0, 25.0, 2.0);

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                explosion,
            )).id();

            // Don't advance time
            let _ = app.world_mut().run_system_once(solar_flare_explosion_cleanup_system);

            assert!(
                app.world().get_entity(entity).is_ok(),
                "Active explosion should survive"
            );
        }
    }
}
