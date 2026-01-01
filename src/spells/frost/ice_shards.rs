//! Ice Shards spell - Multiple ice fragments fired in a cone pattern.
//!
//! A Frost element spell (GlacialSpike SpellType) that fires 5-7 ice shards
//! in a 60-degree cone toward the target direction. Each shard damages
//! independently, allowing for multi-hit potential on grouped enemies.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::frost::ice_shard::SlowedDebuff;

/// Default configuration for Ice Shards spell
pub const ICE_SHARDS_SPEED: f32 = 20.0;
pub const ICE_SHARDS_LIFETIME: f32 = 3.0;
pub const ICE_SHARDS_COLLISION_RADIUS: f32 = 0.8;
pub const ICE_SHARDS_CONE_ANGLE: f32 = 60.0; // degrees
pub const ICE_SHARDS_MIN_COUNT: u32 = 5;
pub const ICE_SHARDS_MAX_COUNT: u32 = 7;

// Re-use slow effect configuration from ice_shard module for consistency
use crate::spells::frost::ice_shard::{SLOWED_DURATION, SLOWED_SPEED_MULTIPLIER};

/// Get the frost element color for visual effects
pub fn ice_shards_color() -> Color {
    Element::Frost.color()
}

/// Individual ice shard projectile component.
/// Each shard travels in its own direction within the cone spread.
#[derive(Component, Debug, Clone)]
pub struct IceShardFragment {
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

impl IceShardFragment {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, damage: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            slow_duration: SLOWED_DURATION,
            slow_multiplier: SLOWED_SPEED_MULTIPLIER,
        }
    }
}

/// Event fired when an ice shard fragment collides with an enemy
#[derive(Message)]
pub struct IceShardFragmentCollisionEvent {
    pub fragment_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves ice shard fragments
pub fn ice_shards_movement_system(
    mut fragment_query: Query<(&mut Transform, &IceShardFragment)>,
    time: Res<Time>,
) {
    for (mut transform, fragment) in fragment_query.iter_mut() {
        let movement = fragment.direction * fragment.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles ice shard fragment lifetime
pub fn ice_shards_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut fragment_query: Query<(Entity, &mut IceShardFragment)>,
) {
    for (entity, mut fragment) in fragment_query.iter_mut() {
        fragment.lifetime.tick(time.delta());

        if fragment.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects ice shard fragment-enemy collisions and fires events
pub fn ice_shards_collision_detection(
    fragment_query: Query<(Entity, &Transform), With<IceShardFragment>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<IceShardFragmentCollisionEvent>,
) {
    for (fragment_entity, fragment_transform) in fragment_query.iter() {
        let fragment_xz = Vec2::new(
            fragment_transform.translation.x,
            fragment_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = fragment_xz.distance(enemy_xz);

            if distance < ICE_SHARDS_COLLISION_RADIUS {
                collision_events.write(IceShardFragmentCollisionEvent {
                    fragment_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per fragment
            }
        }
    }
}

/// System that applies effects when ice shard fragments collide with enemies
/// Sends DamageEvent and applies SlowedDebuff to enemies
pub fn ice_shards_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<IceShardFragmentCollisionEvent>,
    fragment_query: Query<&IceShardFragment>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut fragments_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32, f32, f32)> = Vec::new();

    for event in collision_events.read() {
        fragments_to_despawn.insert(event.fragment_entity);

        // Get fragment damage and slow values
        if let Ok(fragment) = fragment_query.get(event.fragment_entity) {
            effects_to_apply.push((
                event.enemy_entity,
                fragment.damage,
                fragment.slow_duration,
                fragment.slow_multiplier,
            ));
        }
    }

    // Despawn fragments
    for fragment_entity in fragments_to_despawn {
        commands.entity(fragment_entity).try_despawn();
    }

    // Apply damage and slow effects
    for (enemy_entity, damage, slow_duration, slow_multiplier) in effects_to_apply {
        // Direct damage
        damage_events.write(DamageEvent::new(enemy_entity, damage));

        // Apply or refresh slow effect
        commands.entity(enemy_entity).try_insert(SlowedDebuff::new(slow_duration, slow_multiplier));
    }
}

/// Calculate how many shards to spawn (5-7 based on spell level)
fn calculate_shard_count(spell: &Spell) -> u32 {
    // Base 5 shards, +1 at level 4, +1 at level 8 (max 7)
    let bonus = spell.level / 4;
    (ICE_SHARDS_MIN_COUNT + bonus).min(ICE_SHARDS_MAX_COUNT)
}

/// Cast ice shards spell - spawns multiple projectiles in a cone pattern
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_ice_shards(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_ice_shards_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast ice shards spell with explicit damage - spawns multiple projectiles in a cone pattern
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_ice_shards_with_damage(
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
    let base_direction = (target_pos - spawn_xz).normalize_or_zero();

    // Get shard count based on spell level
    let shard_count = calculate_shard_count(spell);
    let cone_angle_rad = ICE_SHARDS_CONE_ANGLE.to_radians();
    let half_cone = cone_angle_rad / 2.0;

    // Create shards evenly distributed across the cone
    for i in 0..shard_count {
        // Calculate angle offset within cone
        // Spread shards evenly from -half_cone to +half_cone
        let angle_offset = if shard_count == 1 {
            0.0
        } else {
            let t = i as f32 / (shard_count - 1) as f32; // 0.0 to 1.0
            -half_cone + t * cone_angle_rad
        };

        // Rotate base direction by angle offset
        let cos_offset = angle_offset.cos();
        let sin_offset = angle_offset.sin();
        let direction = Vec2::new(
            base_direction.x * cos_offset - base_direction.y * sin_offset,
            base_direction.x * sin_offset + base_direction.y * cos_offset,
        );

        let fragment = IceShardFragment::new(
            direction,
            ICE_SHARDS_SPEED,
            ICE_SHARDS_LIFETIME,
            damage,
        );

        // Spawn fragment at Whisper's full 3D position
        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.ice_shard.clone()),
                Transform::from_translation(spawn_position),
                fragment,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                fragment,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod ice_shard_fragment_tests {
        use super::*;

        #[test]
        fn test_ice_shard_fragment_new() {
            let direction = Vec2::new(1.0, 0.0);
            let fragment = IceShardFragment::new(direction, 20.0, 3.0, 25.0);

            assert_eq!(fragment.direction, direction);
            assert_eq!(fragment.speed, 20.0);
            assert_eq!(fragment.damage, 25.0);
            assert_eq!(fragment.slow_duration, SLOWED_DURATION);
            assert_eq!(fragment.slow_multiplier, SLOWED_SPEED_MULTIPLIER);
        }

        #[test]
        fn test_ice_shard_fragment_normalizes_direction() {
            let unnormalized = Vec2::new(3.0, 4.0);
            let fragment = IceShardFragment::new(unnormalized, 20.0, 3.0, 25.0);

            assert!((fragment.direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_ice_shard_fragment_lifetime_timer() {
            let fragment = IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0);
            assert_eq!(fragment.lifetime.duration(), Duration::from_secs_f32(3.0));
            assert!(!fragment.lifetime.is_finished());
        }

        #[test]
        fn test_ice_shards_uses_frost_element_color() {
            let color = ice_shards_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod shard_count_tests {
        use super::*;

        #[test]
        fn test_shard_count_level_1_is_5() {
            let spell = Spell::new(SpellType::GlacialSpike);
            assert_eq!(calculate_shard_count(&spell), 5);
        }

        #[test]
        fn test_shard_count_level_4_is_6() {
            let mut spell = Spell::new(SpellType::GlacialSpike);
            spell.level = 4;
            assert_eq!(calculate_shard_count(&spell), 6);
        }

        #[test]
        fn test_shard_count_level_8_is_7() {
            let mut spell = Spell::new(SpellType::GlacialSpike);
            spell.level = 8;
            assert_eq!(calculate_shard_count(&spell), 7);
        }

        #[test]
        fn test_shard_count_level_10_caps_at_7() {
            let mut spell = Spell::new(SpellType::GlacialSpike);
            spell.level = 10;
            assert_eq!(calculate_shard_count(&spell), 7);
        }
    }

    mod ice_shards_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ice_shards_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create fragment moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::new(1.0, 0.0), 100.0, 3.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(ice_shards_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_ice_shards_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create fragment moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::new(0.0, 1.0), 50.0, 3.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(ice_shards_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }
    }

    mod ice_shards_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ice_shard_fragment_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 100.0, 3.0, 25.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(4));
            }

            let _ = app.world_mut().run_system_once(ice_shards_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_ice_shard_fragment_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 100.0, 3.0, 25.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(2));
            }

            let _ = app.world_mut().run_system_once(ice_shards_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod ice_shards_collision_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<IceShardFragmentCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<IceShardFragmentCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (ice_shards_collision_detection, count_collisions).chain());

            // Spawn fragment at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0),
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
                mut events: MessageReader<IceShardFragmentCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (ice_shards_collision_detection, count_collisions).chain());

            // Spawn fragment at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0),
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
        fn test_collision_effects_despawns_fragment() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (ice_shards_collision_detection, ice_shards_collision_effects).chain(),
            );

            let fragment_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Fragment should be despawned
            assert!(!app.world().entities().contains(fragment_entity));
            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));
        }

        #[test]
        fn test_collision_effects_applies_slowed_debuff() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (ice_shards_collision_detection, ice_shards_collision_effects).chain(),
            );

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0),
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

        #[test]
        fn test_multiple_fragments_can_hit_same_enemy() {
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

            let mut app = setup_test_app();

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(
                Update,
                (ice_shards_collision_detection, ice_shards_collision_effects, count_damage_events).chain(),
            );

            // Spawn multiple fragments at slightly different positions but all hitting same enemy
            for i in 0..3 {
                app.world_mut().spawn((
                    Transform::from_translation(Vec3::new(0.1 * i as f32, 0.5, 0.0)),
                    IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0),
                ));
            }

            // Spawn enemy within collision radius of all fragments
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.1, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // All 3 fragments should deal damage independently
            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }
    }

    mod fire_ice_shards_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_ice_shards_spawns_5_fragments_at_level_1() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 5);
        }

        #[test]
        fn test_fire_ice_shards_spawns_6_fragments_at_level_4() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::GlacialSpike);
            spell.level = 4;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 6);
        }

        #[test]
        fn test_fire_ice_shards_spawns_7_fragments_at_level_8() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::GlacialSpike);
            spell.level = 8;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 7);
        }

        #[test]
        fn test_fire_ice_shards_cone_spread_within_60_degrees() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let base_direction = Vec2::new(1.0, 0.0);
            let half_cone = (ICE_SHARDS_CONE_ANGLE / 2.0).to_radians();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            for fragment in query.iter(app.world()) {
                // Calculate angle between fragment direction and base direction
                let dot = fragment.direction.dot(base_direction);
                let angle = dot.clamp(-1.0, 1.0).acos();

                assert!(
                    angle <= half_cone + 0.01, // Small tolerance for floating point
                    "Fragment direction {:?} is outside 60-degree cone (angle: {} rad, max: {} rad)",
                    fragment.direction,
                    angle,
                    half_cone
                );
            }
        }

        #[test]
        fn test_fire_ice_shards_fragments_evenly_distributed() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            let mut angles: Vec<f32> = query
                .iter(app.world())
                .map(|fragment| fragment.direction.y.atan2(fragment.direction.x))
                .collect();
            angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Check that angles are evenly spaced
            if angles.len() > 1 {
                let expected_spacing = ICE_SHARDS_CONE_ANGLE.to_radians() / (angles.len() - 1) as f32;
                for i in 1..angles.len() {
                    let spacing = angles[i] - angles[i - 1];
                    assert!(
                        (spacing - expected_spacing).abs() < 0.01,
                        "Fragments not evenly spaced: spacing {} vs expected {}",
                        spacing,
                        expected_spacing
                    );
                }
            }
        }

        #[test]
        fn test_fire_ice_shards_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            for fragment in query.iter(app.world()) {
                assert_eq!(fragment.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_ice_shards_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards_with_damage(
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

            let mut query = app.world_mut().query::<&IceShardFragment>();
            for fragment in query.iter(app.world()) {
                assert_eq!(fragment.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_ice_shards_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::GlacialSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_ice_shards(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceShardFragment>();
            for fragment in query.iter(app.world()) {
                // All fragments should have positive X direction (toward target)
                assert!(fragment.direction.x > 0.0, "Fragment should move toward target");
            }
        }

        #[test]
        fn test_ice_shard_fragment_despawns_on_collision() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<IceShardFragmentCollisionEvent>();
            app.add_message::<DamageEvent>();

            app.add_systems(
                Update,
                (ice_shards_collision_detection, ice_shards_collision_effects).chain(),
            );

            let fragment_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 20.0, 3.0, 25.0),
            )).id();

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // Fragment should be despawned after collision
            assert!(!app.world().entities().contains(fragment_entity));
        }

        #[test]
        fn test_ice_shard_fragment_despawns_at_max_range() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let fragment_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceShardFragment::new(Vec2::X, 20.0, ICE_SHARDS_LIFETIME, 25.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ICE_SHARDS_LIFETIME + 1.0));
            }

            let _ = app.world_mut().run_system_once(ice_shards_lifetime_system);

            // Fragment should be despawned after lifetime expires
            assert!(!app.world().entities().contains(fragment_entity));
        }
    }
}
