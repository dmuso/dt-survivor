//! Ice Lance spell - High-speed piercing frost projectile.
//!
//! A Frost element spell (FrozenRay SpellType) that fires a fast projectile
//! which pierces through all enemies in its path, dealing damage to each.
//! Each enemy can only be damaged once per lance.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::frost::ice_shard::SlowedDebuff;

/// Default configuration for Ice Lance spell
pub const ICE_LANCE_SPEED: f32 = 50.0; // Much faster than Ice Shard (25.0)
pub const ICE_LANCE_LIFETIME: f32 = 3.0; // Shorter lifetime since it travels faster
pub const ICE_LANCE_COLLISION_RADIUS: f32 = 1.5; // Slightly wider hitbox
pub const ICE_LANCE_SLOW_DURATION: f32 = 1.5;
pub const ICE_LANCE_SLOW_MULTIPLIER: f32 = 0.6; // 40% speed reduction

/// Get the frost element color for visual effects
pub fn ice_lance_color() -> Color {
    Element::Frost.color()
}

/// Marker component for ice lance projectiles.
/// Ice lances pierce through enemies and track which ones they've hit.
#[derive(Component, Debug, Clone)]
pub struct IceLanceProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second (higher than ice shard)
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Damage dealt on hit
    pub damage: f32,
    /// Entities already damaged by this lance (prevents double-hit)
    pub hit_entities: HashSet<Entity>,
    /// Duration of slow effect to apply
    pub slow_duration: f32,
    /// Speed multiplier for slow effect
    pub slow_multiplier: f32,
}

impl IceLanceProjectile {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, damage: f32) -> Self {
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            hit_entities: HashSet::new(),
            slow_duration: ICE_LANCE_SLOW_DURATION,
            slow_multiplier: ICE_LANCE_SLOW_MULTIPLIER,
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(direction, ICE_LANCE_SPEED, ICE_LANCE_LIFETIME, spell.damage())
    }

    /// Check if this enemy has already been hit by this lance
    pub fn has_hit(&self, entity: Entity) -> bool {
        self.hit_entities.contains(&entity)
    }

    /// Mark an enemy as hit
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_entities.insert(entity);
    }
}

/// System that moves ice lance projectiles at high speed
pub fn ice_lance_movement_system(
    mut ice_lance_query: Query<(&mut Transform, &IceLanceProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, ice_lance) in ice_lance_query.iter_mut() {
        let movement = ice_lance.direction * ice_lance.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles ice lance lifetime
pub fn ice_lance_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut ice_lance_query: Query<(Entity, &mut IceLanceProjectile)>,
) {
    for (entity, mut ice_lance) in ice_lance_query.iter_mut() {
        ice_lance.lifetime.tick(time.delta());

        if ice_lance.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects ice lance-enemy collisions and applies damage.
/// Unlike ice shard, the lance continues through enemies (piercing behavior).
pub fn ice_lance_collision_system(
    mut commands: Commands,
    mut ice_lance_query: Query<(&Transform, &mut IceLanceProjectile)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (lance_transform, mut ice_lance) in ice_lance_query.iter_mut() {
        let lance_xz = Vec2::new(
            lance_transform.translation.x,
            lance_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            // Skip if we've already hit this enemy
            if ice_lance.has_hit(enemy_entity) {
                continue;
            }

            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = lance_xz.distance(enemy_xz);

            if distance < ICE_LANCE_COLLISION_RADIUS {
                // Apply damage
                damage_events.write(DamageEvent::new(enemy_entity, ice_lance.damage));

                // Apply slow effect
                commands.entity(enemy_entity).try_insert(
                    SlowedDebuff::new(ice_lance.slow_duration, ice_lance.slow_multiplier)
                );

                // Mark this enemy as hit (lance continues through)
                ice_lance.mark_hit(enemy_entity);
            }
        }
    }
}

/// Cast ice lance spell - spawns a high-speed piercing projectile.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_ice_lance(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_ice_lance_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast ice lance spell with explicit damage - spawns a high-speed piercing projectile.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_ice_lance_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let direction = (target_pos - spawn_xz).normalize();

    let ice_lance = IceLanceProjectile::new(
        direction,
        ICE_LANCE_SPEED,
        ICE_LANCE_LIFETIME,
        damage,
    );

    // Spawn ice lance at Whisper's full 3D position
    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.ice_shard.clone()), // Reuse ice_shard material
            Transform::from_translation(spawn_position)
                .with_scale(Vec3::new(2.0, 1.0, 1.0)), // Elongated lance shape
            ice_lance,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            ice_lance,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod ice_lance_projectile_tests {
        use super::*;

        #[test]
        fn test_ice_lance_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let ice_lance = IceLanceProjectile::new(direction, 50.0, 3.0, 25.0);

            assert_eq!(ice_lance.direction, direction);
            assert_eq!(ice_lance.speed, 50.0);
            assert_eq!(ice_lance.damage, 25.0);
            assert!(ice_lance.hit_entities.is_empty());
            assert_eq!(ice_lance.slow_duration, ICE_LANCE_SLOW_DURATION);
            assert_eq!(ice_lance.slow_multiplier, ICE_LANCE_SLOW_MULTIPLIER);
        }

        #[test]
        fn test_ice_lance_from_spell() {
            let spell = Spell::new(SpellType::FrozenRay);
            let direction = Vec2::new(0.0, 1.0);
            let ice_lance = IceLanceProjectile::from_spell(direction, &spell);

            assert_eq!(ice_lance.direction, direction);
            assert_eq!(ice_lance.speed, ICE_LANCE_SPEED);
            assert_eq!(ice_lance.damage, spell.damage());
        }

        #[test]
        fn test_ice_lance_speed_faster_than_ice_shard() {
            // Ice Lance should be much faster than Ice Shard
            assert!(ICE_LANCE_SPEED > crate::spells::frost::ice_shard::ICE_SHARD_SPEED);
            assert_eq!(ICE_LANCE_SPEED, 50.0); // 2x faster than ice shard (25.0)
        }

        #[test]
        fn test_ice_lance_lifetime_timer() {
            let ice_lance = IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0);
            assert_eq!(ice_lance.lifetime.duration(), Duration::from_secs_f32(3.0));
            assert!(!ice_lance.lifetime.is_finished());
        }

        #[test]
        fn test_ice_lance_uses_frost_element_color() {
            let color = ice_lance_color();
            assert_eq!(color, Element::Frost.color());
            assert_eq!(color, Color::srgb_u8(135, 206, 235));
        }

        #[test]
        fn test_ice_lance_has_hit() {
            let mut ice_lance = IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0);
            let entity = Entity::from_bits(1);

            assert!(!ice_lance.has_hit(entity));

            ice_lance.mark_hit(entity);
            assert!(ice_lance.has_hit(entity));
        }

        #[test]
        fn test_ice_lance_mark_hit_multiple_entities() {
            let mut ice_lance = IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0);
            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);
            let entity3 = Entity::from_bits(3);

            ice_lance.mark_hit(entity1);
            ice_lance.mark_hit(entity2);

            assert!(ice_lance.has_hit(entity1));
            assert!(ice_lance.has_hit(entity2));
            assert!(!ice_lance.has_hit(entity3));
        }
    }

    mod ice_lance_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ice_lance_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create ice lance moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::new(1.0, 0.0), 50.0, 3.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(ice_lance_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 50.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_ice_lance_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create ice lance moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::new(0.0, 1.0), 50.0, 3.0, 25.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(ice_lance_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }

        #[test]
        fn test_ice_lance_faster_than_ice_shard() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create ice lance at 50.0 speed
            let lance_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::new(1.0, 0.0), ICE_LANCE_SPEED, 3.0, 25.0),
            )).id();

            // Advance time 0.5 seconds
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(ice_lance_movement_system);

            let lance_transform = app.world().get::<Transform>(lance_entity).unwrap();
            // Ice lance at 50.0 speed for 0.5s = 25.0 units
            assert_eq!(lance_transform.translation.x, 25.0);
            // Ice shard at 25.0 speed for 0.5s would only go 12.5 units
        }
    }

    mod ice_lance_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ice_lance_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(4));
            }

            let _ = app.world_mut().run_system_once(ice_lance_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_ice_lance_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(2));
            }

            let _ = app.world_mut().run_system_once(ice_lance_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod ice_lance_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_ice_lance_damages_enemy() {
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

            app.add_systems(Update, (ice_lance_collision_system, count_damage_events).chain());

            // Spawn ice lance at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
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
        fn test_ice_lance_pierces_enemy() {
            // Ice lance should not despawn after hitting an enemy
            let mut app = setup_test_app();

            app.add_systems(Update, ice_lance_collision_system);

            // Spawn ice lance
            let lance_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
            )).id();

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // Ice lance should still exist (piercing behavior)
            assert!(app.world().entities().contains(lance_entity));
        }

        #[test]
        fn test_ice_lance_does_not_hit_same_enemy_twice() {
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

            app.add_systems(Update, (ice_lance_collision_system, count_damage_events).chain());

            // Spawn ice lance
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
            ));

            // Spawn single enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Should only have one damage event (not repeated hits)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_ice_lance_hits_multiple_enemies_in_line() {
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

            app.add_systems(Update, ice_lance_collision_system);
            app.add_systems(Update, count_damage_events.after(ice_lance_collision_system));

            // Spawn ice lance at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
            ));

            // Spawn 3 enemies in a line (all within collision radius of origin)
            // Collision radius is 1.5, so place enemies at 0.3, 0.6, 0.9
            for i in 0..3 {
                app.world_mut().spawn((
                    Transform::from_translation(Vec3::new(0.3 * (i + 1) as f32, 0.375, 0.0)),
                    Enemy { speed: 50.0, strength: 10.0 },
                ));
            }

            app.update();

            // All 3 enemies should be hit
            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn test_ice_lance_applies_slow_effect() {
            let mut app = setup_test_app();

            app.add_systems(Update, ice_lance_collision_system);

            // Spawn ice lance
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
            ));

            // Spawn enemy within collision radius
            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Enemy should have SlowedDebuff component
            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "Enemy should have SlowedDebuff after ice lance hit");
            assert_eq!(slowed.unwrap().speed_multiplier, ICE_LANCE_SLOW_MULTIPLIER);
        }

        #[test]
        fn test_ice_lance_no_damage_outside_radius() {
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

            app.add_systems(Update, (ice_lance_collision_system, count_damage_events).chain());

            // Spawn ice lance at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                IceLanceProjectile::new(Vec2::X, 50.0, 3.0, 25.0),
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

    mod fire_ice_lance_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_ice_lance_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrozenRay);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_lance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 ice lance
            let mut query = app.world_mut().query::<&IceLanceProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_ice_lance_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrozenRay);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_ice_lance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceLanceProjectile>();
            for ice_lance in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(ice_lance.direction.x > 0.9, "Ice lance should move toward target");
            }
        }

        #[test]
        fn test_fire_ice_lance_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrozenRay);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_lance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceLanceProjectile>();
            for ice_lance in query.iter(app.world()) {
                assert_eq!(ice_lance.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_ice_lance_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrozenRay);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_lance_with_damage(
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

            let mut query = app.world_mut().query::<&IceLanceProjectile>();
            for ice_lance in query.iter(app.world()) {
                assert_eq!(ice_lance.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_ice_lance_starts_with_empty_hit_list() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::FrozenRay);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ice_lance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IceLanceProjectile>();
            for ice_lance in query.iter(app.world()) {
                assert!(ice_lance.hit_entities.is_empty());
            }
        }
    }
}
