//! Cinder Shot spell - Fast piercing fire projectile that weakens enemies.
//!
//! A Fire element spell (FlameLance SpellType) that fires a fast projectile
//! toward enemies. The projectile pierces through enemies (doesn't stop on hit)
//! and applies a Weakened debuff that causes enemies to take increased damage.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Cinder Shot spell
pub const CINDER_SHOT_SPEED: f32 = 28.0;
pub const CINDER_SHOT_LIFETIME: f32 = 4.0;
pub const CINDER_SHOT_COLLISION_RADIUS: f32 = 0.8;
pub const CINDER_SHOT_SPREAD_ANGLE: f32 = 10.0; // degrees

/// Weakened debuff configuration
pub const WEAKENED_DURATION: f32 = 3.0;
pub const WEAKENED_DAMAGE_MULTIPLIER: f32 = 1.25; // 25% more damage taken

/// Get the fire element color for visual effects
pub fn cinder_shot_color() -> Color {
    Element::Fire.color()
}

/// Marker component for cinder shot projectiles.
/// Pierces through enemies and applies WeakenedDebuff on hit.
#[derive(Component, Debug, Clone)]
pub struct CinderShotProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Damage dealt on hit
    pub damage: f32,
    /// Duration of weakened effect to apply
    pub weakened_duration: f32,
    /// Damage multiplier for weakened effect
    pub weakened_multiplier: f32,
    /// Set of enemies already hit by this projectile (for piercing)
    pub hit_enemies: HashSet<Entity>,
}

impl CinderShotProjectile {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, damage: f32) -> Self {
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            weakened_duration: WEAKENED_DURATION,
            weakened_multiplier: WEAKENED_DAMAGE_MULTIPLIER,
            hit_enemies: HashSet::new(),
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(direction, CINDER_SHOT_SPEED, CINDER_SHOT_LIFETIME, spell.damage())
    }

    /// Check if this projectile can damage the given entity (not already hit)
    pub fn can_damage(&self, entity: Entity) -> bool {
        !self.hit_enemies.contains(&entity)
    }

    /// Mark an entity as hit by this projectile
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }
}

/// Weakened debuff applied to enemies hit by Cinder Shot.
/// Causes the enemy to take increased damage from all sources.
#[derive(Component, Debug, Clone)]
pub struct WeakenedDebuff {
    /// Remaining duration of the weakened effect
    pub duration: Timer,
    /// Damage multiplier (1.25 = 25% more damage taken)
    pub damage_multiplier: f32,
}

impl WeakenedDebuff {
    pub fn new(duration_secs: f32, damage_multiplier: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            damage_multiplier,
        }
    }

    /// Check if the weakened effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Refresh the weakened duration (for reapplying effect)
    pub fn refresh(&mut self, duration_secs: f32) {
        self.duration = Timer::from_seconds(duration_secs, TimerMode::Once);
    }
}

impl Default for WeakenedDebuff {
    fn default() -> Self {
        Self::new(WEAKENED_DURATION, WEAKENED_DAMAGE_MULTIPLIER)
    }
}

/// Event fired when a cinder shot collides with an enemy
#[derive(Message)]
pub struct CinderShotEnemyCollisionEvent {
    pub cinder_shot_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves cinder shot projectiles
pub fn cinder_shot_movement_system(
    mut cinder_shot_query: Query<(&mut Transform, &CinderShotProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, cinder_shot) in cinder_shot_query.iter_mut() {
        let movement = cinder_shot.direction * cinder_shot.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles cinder shot lifetime
pub fn cinder_shot_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut cinder_shot_query: Query<(Entity, &mut CinderShotProjectile)>,
) {
    for (entity, mut cinder_shot) in cinder_shot_query.iter_mut() {
        cinder_shot.lifetime.tick(time.delta());

        if cinder_shot.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects cinder shot-enemy collisions and fires events.
/// Unlike non-piercing projectiles, this fires events for each enemy hit
/// but only if the enemy hasn't been hit by this projectile before.
pub fn cinder_shot_collision_detection(
    cinder_shot_query: Query<(Entity, &Transform, &CinderShotProjectile)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<CinderShotEnemyCollisionEvent>,
) {
    for (cinder_shot_entity, cinder_shot_transform, cinder_shot) in cinder_shot_query.iter() {
        let cinder_shot_xz = Vec2::new(
            cinder_shot_transform.translation.x,
            cinder_shot_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            // Skip if already hit this enemy
            if !cinder_shot.can_damage(enemy_entity) {
                continue;
            }

            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = cinder_shot_xz.distance(enemy_xz);

            if distance < CINDER_SHOT_COLLISION_RADIUS {
                collision_events.write(CinderShotEnemyCollisionEvent {
                    cinder_shot_entity,
                    enemy_entity,
                });
                // Don't break! Piercing projectile can hit multiple enemies per frame
            }
        }
    }
}

/// System that applies effects when cinder shots collide with enemies.
/// Sends DamageEvent and applies WeakenedDebuff to enemies.
/// Does NOT despawn the projectile (piercing behavior).
pub fn cinder_shot_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<CinderShotEnemyCollisionEvent>,
    mut cinder_shot_query: Query<&mut CinderShotProjectile>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut effects_to_apply: Vec<(Entity, Entity, f32, f32, f32)> = Vec::new();

    for event in collision_events.read() {
        // Get cinder shot damage and weakened values
        if let Ok(cinder_shot) = cinder_shot_query.get(event.cinder_shot_entity) {
            // Only process if we haven't already hit this enemy
            if cinder_shot.can_damage(event.enemy_entity) {
                effects_to_apply.push((
                    event.cinder_shot_entity,
                    event.enemy_entity,
                    cinder_shot.damage,
                    cinder_shot.weakened_duration,
                    cinder_shot.weakened_multiplier,
                ));
            }
        }
    }

    // Apply damage and weakened effects, mark enemies as hit
    for (cinder_shot_entity, enemy_entity, damage, weakened_duration, weakened_multiplier) in effects_to_apply {
        // Mark enemy as hit by this projectile
        if let Ok(mut cinder_shot) = cinder_shot_query.get_mut(cinder_shot_entity) {
            cinder_shot.mark_hit(enemy_entity);
        }

        // Direct damage
        damage_events.write(DamageEvent::new(enemy_entity, damage));

        // Apply or refresh weakened effect
        commands.entity(enemy_entity).try_insert(WeakenedDebuff::new(weakened_duration, weakened_multiplier));
    }
}

/// System that ticks weakened debuff timers and removes expired debuffs
pub fn weakened_debuff_system(
    mut commands: Commands,
    time: Res<Time>,
    mut weakened_query: Query<(Entity, &mut WeakenedDebuff)>,
) {
    for (entity, mut weakened) in weakened_query.iter_mut() {
        weakened.tick(time.delta());

        if weakened.is_expired() {
            commands.entity(entity).remove::<WeakenedDebuff>();
        }
    }
}

/// Cast cinder shot spell - spawns projectiles with fire element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_cinder_shot(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_cinder_shot_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast cinder shot spell with explicit damage - spawns projectiles with fire element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_cinder_shot_with_damage(
    commands: &mut Commands,
    spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let base_direction = (target_pos - spawn_xz).normalize();

    // Get projectile count based on spell level
    let projectile_count = spell.projectile_count();
    let spread_angle_rad = CINDER_SHOT_SPREAD_ANGLE.to_radians();

    // Create projectiles in a spread pattern centered around the target direction
    for i in 0..projectile_count {
        let angle_offset = if projectile_count == 1 {
            0.0
        } else {
            let half_spread = (projectile_count - 1) as f32 / 2.0;
            (i as f32 - half_spread) * spread_angle_rad
        };

        let cos_offset = angle_offset.cos();
        let sin_offset = angle_offset.sin();
        let direction = Vec2::new(
            base_direction.x * cos_offset - base_direction.y * sin_offset,
            base_direction.x * sin_offset + base_direction.y * cos_offset,
        );

        let cinder_shot = CinderShotProjectile::new(direction, CINDER_SHOT_SPEED, CINDER_SHOT_LIFETIME, damage);

        // Spawn cinder shot at Whisper's full 3D position
        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.fireball.clone()),
                Transform::from_translation(spawn_position),
                cinder_shot,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                cinder_shot,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod cinder_shot_projectile_tests {
        use super::*;

        #[test]
        fn test_cinder_shot_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let cinder_shot = CinderShotProjectile::new(direction, 28.0, 4.0, 25.0);

            assert_eq!(cinder_shot.direction, direction);
            assert_eq!(cinder_shot.speed, 28.0);
            assert_eq!(cinder_shot.damage, 25.0);
            assert_eq!(cinder_shot.weakened_duration, WEAKENED_DURATION);
            assert_eq!(cinder_shot.weakened_multiplier, WEAKENED_DAMAGE_MULTIPLIER);
            assert!(cinder_shot.hit_enemies.is_empty());
        }

        #[test]
        fn test_cinder_shot_from_spell() {
            let spell = Spell::new(SpellType::FlameLance);
            let direction = Vec2::new(0.0, 1.0);
            let cinder_shot = CinderShotProjectile::from_spell(direction, &spell);

            assert_eq!(cinder_shot.direction, direction);
            assert_eq!(cinder_shot.speed, CINDER_SHOT_SPEED);
            assert_eq!(cinder_shot.damage, spell.damage());
        }

        #[test]
        fn test_cinder_shot_lifetime_timer() {
            let cinder_shot = CinderShotProjectile::new(Vec2::X, 28.0, 4.0, 25.0);
            assert_eq!(cinder_shot.lifetime.duration(), Duration::from_secs_f32(4.0));
            assert!(!cinder_shot.lifetime.is_finished());
        }

        #[test]
        fn test_cinder_shot_uses_fire_element_color() {
            let color = cinder_shot_color();
            assert_eq!(color, Element::Fire.color());
        }

        #[test]
        fn test_cinder_shot_can_damage_new_enemy() {
            let cinder_shot = CinderShotProjectile::new(Vec2::X, 28.0, 4.0, 25.0);
            let enemy = Entity::from_bits(1);
            assert!(cinder_shot.can_damage(enemy));
        }

        #[test]
        fn test_cinder_shot_cannot_damage_hit_enemy() {
            let mut cinder_shot = CinderShotProjectile::new(Vec2::X, 28.0, 4.0, 25.0);
            let enemy = Entity::from_bits(1);

            cinder_shot.mark_hit(enemy);
            assert!(!cinder_shot.can_damage(enemy));
        }

        #[test]
        fn test_cinder_shot_can_damage_different_enemy_after_hit() {
            let mut cinder_shot = CinderShotProjectile::new(Vec2::X, 28.0, 4.0, 25.0);
            let enemy1 = Entity::from_bits(1);
            let enemy2 = Entity::from_bits(2);

            cinder_shot.mark_hit(enemy1);
            assert!(!cinder_shot.can_damage(enemy1));
            assert!(cinder_shot.can_damage(enemy2));
        }

        #[test]
        fn test_cinder_shot_mark_hit_adds_to_set() {
            let mut cinder_shot = CinderShotProjectile::new(Vec2::X, 28.0, 4.0, 25.0);
            let enemy = Entity::from_bits(1);

            assert!(!cinder_shot.hit_enemies.contains(&enemy));
            cinder_shot.mark_hit(enemy);
            assert!(cinder_shot.hit_enemies.contains(&enemy));
        }
    }

    mod weakened_debuff_tests {
        use super::*;

        #[test]
        fn test_weakened_debuff_new() {
            let weakened = WeakenedDebuff::new(3.0, 1.25);
            assert_eq!(weakened.damage_multiplier, 1.25);
            assert!(!weakened.is_expired());
        }

        #[test]
        fn test_weakened_debuff_default() {
            let weakened = WeakenedDebuff::default();
            assert_eq!(weakened.damage_multiplier, WEAKENED_DAMAGE_MULTIPLIER);
        }

        #[test]
        fn test_weakened_debuff_tick_expires() {
            let mut weakened = WeakenedDebuff::new(1.0, 1.25);
            assert!(!weakened.is_expired());

            weakened.tick(Duration::from_secs_f32(1.1));
            assert!(weakened.is_expired());
        }

        #[test]
        fn test_weakened_debuff_tick_not_expired() {
            let mut weakened = WeakenedDebuff::new(2.0, 1.25);
            weakened.tick(Duration::from_secs_f32(0.5));
            assert!(!weakened.is_expired());
        }

        #[test]
        fn test_weakened_debuff_refresh() {
            let mut weakened = WeakenedDebuff::new(1.0, 1.25);
            weakened.tick(Duration::from_secs_f32(0.9));
            assert!(!weakened.is_expired());

            weakened.refresh(2.0);
            weakened.tick(Duration::from_secs_f32(1.5));
            assert!(!weakened.is_expired());
        }
    }

    mod weakened_debuff_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_weakened_debuff_system_ticks_duration() {
            let mut app = setup_test_app();

            // Spawn entity with weakened debuff
            let entity = app.world_mut().spawn(WeakenedDebuff::new(2.0, 1.25)).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(weakened_debuff_system);

            // Debuff should still exist
            assert!(app.world().get::<WeakenedDebuff>(entity).is_some());
        }

        #[test]
        fn test_weakened_debuff_system_removes_expired() {
            let mut app = setup_test_app();

            // Spawn entity with short weakened debuff
            let entity = app.world_mut().spawn(WeakenedDebuff::new(0.5, 1.25)).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(weakened_debuff_system);

            // Debuff should be removed
            assert!(app.world().get::<WeakenedDebuff>(entity).is_none());
        }

        #[test]
        fn test_weakened_debuff_system_preserves_other_components() {
            let mut app = setup_test_app();

            // Spawn entity with weakened debuff and other components
            let entity = app.world_mut().spawn((
                WeakenedDebuff::new(0.5, 1.25),
                Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(weakened_debuff_system);

            // Entity should still exist with Transform
            assert!(app.world().entities().contains(entity));
            assert!(app.world().get::<Transform>(entity).is_some());
        }
    }

    mod cinder_shot_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_cinder_shot_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create cinder shot moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::new(1.0, 0.0), 100.0, 4.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(cinder_shot_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_cinder_shot_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create cinder shot moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::new(0.0, 1.0), 50.0, 4.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(cinder_shot_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }

        #[test]
        fn test_cinder_shot_fast_speed() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create cinder shot with default fast speed
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                CinderShotProjectile::new(Vec2::new(1.0, 0.0), CINDER_SHOT_SPEED, 4.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(cinder_shot_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Should travel CINDER_SHOT_SPEED units in 1 second
            assert!((transform.translation.x - CINDER_SHOT_SPEED).abs() < 0.01);
        }
    }

    mod cinder_shot_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_cinder_shot_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 100.0, 4.0, 25.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(5));
            }

            let _ = app.world_mut().run_system_once(cinder_shot_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_cinder_shot_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 100.0, 4.0, 25.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(2));
            }

            let _ = app.world_mut().run_system_once(cinder_shot_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod cinder_shot_collision_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<CinderShotEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<CinderShotEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (cinder_shot_collision_detection, count_collisions).chain());

            // Spawn cinder shot at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            ));

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_collision_detection_no_event_when_far() {
            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<CinderShotEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (cinder_shot_collision_detection, count_collisions).chain());

            // Spawn cinder shot at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            ));

            // Spawn enemy far away
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_cinder_shot_pierces_does_not_despawn_on_hit() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (cinder_shot_collision_detection, cinder_shot_collision_effects).chain(),
            );

            let cinder_shot_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            )).id();

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // Cinder shot should NOT be despawned (piercing behavior)
            assert!(app.world().entities().contains(cinder_shot_entity));
        }

        #[test]
        fn test_collision_effects_applies_weakened_debuff() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (cinder_shot_collision_detection, cinder_shot_collision_effects).chain(),
            );

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            ));

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Enemy should have WeakenedDebuff component
            let weakened = app.world().get::<WeakenedDebuff>(enemy_entity);
            assert!(weakened.is_some(), "Enemy should have WeakenedDebuff after cinder shot hit");
            assert_eq!(weakened.unwrap().damage_multiplier, WEAKENED_DAMAGE_MULTIPLIER);
        }

        #[test]
        fn test_collision_effects_marks_enemy_as_hit() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (cinder_shot_collision_detection, cinder_shot_collision_effects).chain(),
            );

            let cinder_shot_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Enemy should be marked as hit in the projectile
            let cinder_shot = app.world().get::<CinderShotProjectile>(cinder_shot_entity).unwrap();
            assert!(cinder_shot.hit_enemies.contains(&enemy_entity));
        }

        #[test]
        fn test_no_double_hit_same_enemy() {
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

            let mut app = setup_test_app();

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(
                Update,
                (cinder_shot_collision_detection, cinder_shot_collision_effects, count_damage).chain(),
            );

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            // Run multiple updates - enemy should only be damaged once
            app.update();
            app.update();
            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy should only be damaged once");
        }

        #[test]
        fn test_can_hit_multiple_different_enemies() {
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

            let mut app = setup_test_app();

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(
                Update,
                (cinder_shot_collision_detection, cinder_shot_collision_effects, count_damage).chain(),
            );

            // Spawn cinder shot at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                CinderShotProjectile::new(Vec2::X, 20.0, 4.0, 25.0),
            ));

            // Spawn two enemies within collision radius (close together)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.3, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // Both enemies should be damaged
            assert_eq!(counter.0.load(Ordering::SeqCst), 2, "Both enemies should be damaged");
        }
    }

    mod fire_cinder_shot_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_cinder_shot_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FlameLance);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_cinder_shot(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 cinder shot (level 1)
            let mut query = app.world_mut().query::<&CinderShotProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_cinder_shot_spawns_multiple_at_higher_levels() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::FlameLance);
            spell.level = 5; // Should spawn 2 projectiles
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_cinder_shot(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&CinderShotProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2);
        }

        #[test]
        fn test_fire_cinder_shot_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FlameLance);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_cinder_shot(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&CinderShotProjectile>();
            for cinder_shot in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(cinder_shot.direction.x > 0.9, "Cinder shot should move toward target");
            }
        }

        #[test]
        fn test_fire_cinder_shot_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FlameLance);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_cinder_shot(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&CinderShotProjectile>();
            for cinder_shot in query.iter(app.world()) {
                assert_eq!(cinder_shot.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_cinder_shot_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FlameLance);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_cinder_shot_with_damage(
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

            let mut query = app.world_mut().query::<&CinderShotProjectile>();
            for cinder_shot in query.iter(app.world()) {
                assert_eq!(cinder_shot.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_cinder_shot_projectile_has_empty_hit_list() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FlameLance);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_cinder_shot(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&CinderShotProjectile>();
            for cinder_shot in query.iter(app.world()) {
                assert!(cinder_shot.hit_enemies.is_empty(), "New projectile should have empty hit list");
            }
        }
    }
}
