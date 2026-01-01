use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Ice Shard spell
pub const ICE_SHARD_SPEED: f32 = 25.0;
pub const ICE_SHARD_LIFETIME: f32 = 5.0;
pub const ICE_SHARD_COLLISION_RADIUS: f32 = 1.0;

/// Slow effect configuration
pub const SLOWED_DURATION: f32 = 2.0;
pub const SLOWED_SPEED_MULTIPLIER: f32 = 0.5; // 50% speed reduction

/// Get the frost element color for visual effects
pub fn ice_shard_color() -> Color {
    Element::Frost.color()
}

/// Marker component for ice shard projectiles
#[derive(Component, Debug, Clone)]
pub struct IceShardProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Damage dealt on hit
    pub damage: f32,
    /// Duration of slow effect to apply
    pub slow_duration: f32,
    /// Speed multiplier for slow effect (0.5 = 50% speed)
    pub slow_multiplier: f32,
}

impl IceShardProjectile {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, damage: f32) -> Self {
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            slow_duration: SLOWED_DURATION,
            slow_multiplier: SLOWED_SPEED_MULTIPLIER,
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(direction, ICE_SHARD_SPEED, ICE_SHARD_LIFETIME, spell.damage())
    }
}

/// Slowed debuff applied to enemies hit by frost spells.
/// Reduces movement speed for a duration.
#[derive(Component, Debug, Clone)]
pub struct SlowedDebuff {
    /// Remaining duration of the slow effect
    pub duration: Timer,
    /// Speed multiplier (0.5 = 50% of normal speed)
    pub speed_multiplier: f32,
}

impl SlowedDebuff {
    pub fn new(duration_secs: f32, speed_multiplier: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            speed_multiplier,
        }
    }

    /// Check if the slow effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Refresh the slow duration (for reapplying slow effect)
    pub fn refresh(&mut self, duration_secs: f32) {
        self.duration = Timer::from_seconds(duration_secs, TimerMode::Once);
    }
}

impl Default for SlowedDebuff {
    fn default() -> Self {
        Self::new(SLOWED_DURATION, SLOWED_SPEED_MULTIPLIER)
    }
}

/// Event fired when an ice shard collides with an enemy
#[derive(Message)]
pub struct IceShardEnemyCollisionEvent {
    pub ice_shard_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves ice shard projectiles
pub fn ice_shard_movement_system(
    mut ice_shard_query: Query<(&mut Transform, &IceShardProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, ice_shard) in ice_shard_query.iter_mut() {
        let movement = ice_shard.direction * ice_shard.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles ice shard lifetime
pub fn ice_shard_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut ice_shard_query: Query<(Entity, &mut IceShardProjectile)>,
) {
    for (entity, mut ice_shard) in ice_shard_query.iter_mut() {
        ice_shard.lifetime.tick(time.delta());

        if ice_shard.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects ice shard-enemy collisions and fires events
pub fn ice_shard_collision_detection(
    ice_shard_query: Query<(Entity, &Transform), With<IceShardProjectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<IceShardEnemyCollisionEvent>,
) {
    for (ice_shard_entity, ice_shard_transform) in ice_shard_query.iter() {
        let ice_shard_xz = Vec2::new(
            ice_shard_transform.translation.x,
            ice_shard_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = ice_shard_xz.distance(enemy_xz);

            if distance < ICE_SHARD_COLLISION_RADIUS {
                collision_events.write(IceShardEnemyCollisionEvent {
                    ice_shard_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per ice shard
            }
        }
    }
}

/// System that applies effects when ice shards collide with enemies
/// Sends DamageEvent and applies SlowedDebuff to enemies
pub fn ice_shard_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<IceShardEnemyCollisionEvent>,
    ice_shard_query: Query<&IceShardProjectile>,
    slowed_query: Query<&SlowedDebuff>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut ice_shards_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32, f32, f32)> = Vec::new();

    for event in collision_events.read() {
        ice_shards_to_despawn.insert(event.ice_shard_entity);

        // Get ice shard damage and slow values
        if let Ok(ice_shard) = ice_shard_query.get(event.ice_shard_entity) {
            effects_to_apply.push((
                event.enemy_entity,
                ice_shard.damage,
                ice_shard.slow_duration,
                ice_shard.slow_multiplier,
            ));
        }
    }

    // Despawn ice shards
    for ice_shard_entity in ice_shards_to_despawn {
        commands.entity(ice_shard_entity).try_despawn();
    }

    // Apply damage and slow effects
    for (enemy_entity, damage, slow_duration, slow_multiplier) in effects_to_apply {
        // Direct damage
        damage_events.write(DamageEvent::new(enemy_entity, damage));

        // Apply or refresh slow effect
        if slowed_query.get(enemy_entity).is_ok() {
            // Entity already has slow - refresh it (handled by the slow system)
            // We'll insert a new one which overwrites the old
        }
        commands.entity(enemy_entity).try_insert(SlowedDebuff::new(slow_duration, slow_multiplier));
    }
}

/// System that ticks slowed debuff timers and removes expired debuffs
pub fn slowed_debuff_system(
    mut commands: Commands,
    time: Res<Time>,
    mut slowed_query: Query<(Entity, &mut SlowedDebuff)>,
) {
    for (entity, mut slowed) in slowed_query.iter_mut() {
        slowed.tick(time.delta());

        if slowed.is_expired() {
            commands.entity(entity).remove::<SlowedDebuff>();
        }
    }
}

/// Cast ice shard spell - spawns projectiles with frost element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_ice_shard(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_ice_shard_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast ice shard spell with explicit damage - spawns projectiles with frost element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_ice_shard_with_damage(
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
    let spread_angle_rad = 15.0_f32.to_radians();

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

        let ice_shard = IceShardProjectile::new(direction, ICE_SHARD_SPEED, ICE_SHARD_LIFETIME, damage);

        // Spawn ice shard at Whisper's full 3D position
        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.ice_shard.clone()),
                Transform::from_translation(spawn_position),
                ice_shard,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                ice_shard,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod ice_shard_projectile_tests {
        use super::*;

        #[test]
        fn test_ice_shard_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let ice_shard = IceShardProjectile::new(direction, 25.0, 5.0, 15.0);

            assert_eq!(ice_shard.direction, direction);
            assert_eq!(ice_shard.speed, 25.0);
            assert_eq!(ice_shard.damage, 15.0);
            assert_eq!(ice_shard.slow_duration, SLOWED_DURATION);
            assert_eq!(ice_shard.slow_multiplier, SLOWED_SPEED_MULTIPLIER);
        }

        #[test]
        fn test_ice_shard_from_spell() {
            let spell = Spell::new(SpellType::IceShard);
            let direction = Vec2::new(0.0, 1.0);
            let ice_shard = IceShardProjectile::from_spell(direction, &spell);

            assert_eq!(ice_shard.direction, direction);
            assert_eq!(ice_shard.speed, ICE_SHARD_SPEED);
            assert_eq!(ice_shard.damage, spell.damage());
        }

        #[test]
        fn test_ice_shard_lifetime_timer() {
            let ice_shard = IceShardProjectile::new(Vec2::X, 25.0, 5.0, 15.0);
            assert_eq!(ice_shard.lifetime.duration(), Duration::from_secs_f32(5.0));
            assert!(!ice_shard.lifetime.is_finished());
        }

        #[test]
        fn test_ice_shard_uses_frost_element_color() {
            let color = ice_shard_color();
            assert_eq!(color, Element::Frost.color());
            assert_eq!(color, Color::srgb_u8(135, 206, 235));
        }
    }

    mod slowed_debuff_tests {
        use super::*;

        #[test]
        fn test_slowed_debuff_new() {
            let slowed = SlowedDebuff::new(3.0, 0.6);
            assert_eq!(slowed.speed_multiplier, 0.6);
            assert!(!slowed.is_expired());
        }

        #[test]
        fn test_slowed_debuff_default() {
            let slowed = SlowedDebuff::default();
            assert_eq!(slowed.speed_multiplier, SLOWED_SPEED_MULTIPLIER);
        }

        #[test]
        fn test_slowed_debuff_tick_expires() {
            let mut slowed = SlowedDebuff::new(1.0, 0.5);
            assert!(!slowed.is_expired());

            slowed.tick(Duration::from_secs_f32(1.1));
            assert!(slowed.is_expired());
        }

        #[test]
        fn test_slowed_debuff_tick_not_expired() {
            let mut slowed = SlowedDebuff::new(2.0, 0.5);
            slowed.tick(Duration::from_secs_f32(0.5));
            assert!(!slowed.is_expired());
        }

        #[test]
        fn test_slowed_debuff_refresh() {
            let mut slowed = SlowedDebuff::new(1.0, 0.5);
            slowed.tick(Duration::from_secs_f32(0.9));
            assert!(!slowed.is_expired());

            slowed.refresh(2.0);
            slowed.tick(Duration::from_secs_f32(1.5));
            assert!(!slowed.is_expired());
        }
    }

    mod slowed_debuff_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_slowed_debuff_system_ticks_duration() {
            let mut app = setup_test_app();

            // Spawn entity with slowed debuff
            let entity = app.world_mut().spawn(SlowedDebuff::new(2.0, 0.5)).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(slowed_debuff_system);

            // Debuff should still exist
            assert!(app.world().get::<SlowedDebuff>(entity).is_some());
        }

        #[test]
        fn test_slowed_debuff_system_removes_expired() {
            let mut app = setup_test_app();

            // Spawn entity with short slowed debuff
            let entity = app.world_mut().spawn(SlowedDebuff::new(0.5, 0.5)).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(slowed_debuff_system);

            // Debuff should be removed
            assert!(app.world().get::<SlowedDebuff>(entity).is_none());
        }

        #[test]
        fn test_slowed_debuff_system_preserves_other_components() {
            let mut app = setup_test_app();

            // Spawn entity with slowed debuff and other components
            let entity = app.world_mut().spawn((
                SlowedDebuff::new(0.5, 0.5),
                Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(slowed_debuff_system);

            // Entity should still exist with Transform
            assert!(app.world().entities().contains(entity));
            assert!(app.world().get::<Transform>(entity).is_some());
        }
    }

    mod ice_shard_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ice_shard_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create ice shard moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::new(1.0, 0.0), 100.0, 5.0, 15.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(ice_shard_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_ice_shard_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create ice shard moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::new(0.0, 1.0), 50.0, 5.0, 15.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(ice_shard_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }
    }

    mod ice_shard_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ice_shard_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::X, 100.0, 5.0, 15.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(6));
            }

            let _ = app.world_mut().run_system_once(ice_shard_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_ice_shard_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::X, 100.0, 5.0, 15.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(3));
            }

            let _ = app.world_mut().run_system_once(ice_shard_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod ice_shard_collision_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<IceShardEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<IceShardEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (ice_shard_collision_detection, count_collisions).chain());

            // Spawn ice shard at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
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
                mut events: MessageReader<IceShardEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (ice_shard_collision_detection, count_collisions).chain());

            // Spawn ice shard at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
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
        fn test_collision_effects_despawns_ice_shard() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (ice_shard_collision_detection, ice_shard_collision_effects).chain(),
            );

            let ice_shard_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Ice shard should be despawned
            assert!(!app.world().entities().contains(ice_shard_entity));
            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));
        }

        #[test]
        fn test_collision_effects_applies_slowed_debuff() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (ice_shard_collision_detection, ice_shard_collision_effects).chain(),
            );

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
            ));

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Enemy should have SlowedDebuff component
            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "Enemy should have SlowedDebuff after ice shard hit");
            assert_eq!(slowed.unwrap().speed_multiplier, SLOWED_SPEED_MULTIPLIER);
        }
    }

    mod fire_ice_shard_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_ice_shard_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::IceShard);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shard(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 ice shard (level 1)
            let mut query = app.world_mut().query::<&IceShardProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_ice_shard_spawns_multiple_at_higher_levels() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::IceShard);
            spell.level = 5; // Should spawn 2 projectiles
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shard(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2);
        }

        #[test]
        fn test_fire_ice_shard_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::IceShard);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shard(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardProjectile>();
            for ice_shard in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(ice_shard.direction.x > 0.9, "Ice shard should move toward target");
            }
        }

        #[test]
        fn test_fire_ice_shard_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::IceShard);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shard(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardProjectile>();
            for ice_shard in query.iter(app.world()) {
                assert_eq!(ice_shard.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_ice_shard_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::IceShard);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shard_with_damage(
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

            let mut query = app.world_mut().query::<&IceShardProjectile>();
            for ice_shard in query.iter(app.world()) {
                assert_eq!(ice_shard.damage, explicit_damage);
            }
        }
    }
}
