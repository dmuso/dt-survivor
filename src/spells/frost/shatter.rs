//! Shatter spell - Frost projectile that deals bonus damage to slowed/frozen enemies.
//!
//! A Frost element projectile spell that synergizes with other frost spells.
//! Deals 2x damage to enemies with SlowedDebuff and 3x damage to enemies with
//! FrozenStatus. Frozen multiplier takes priority over slowed.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::frost::ice_shard::SlowedDebuff;
use crate::spells::frost::permafrost::FrozenStatus;

/// Default configuration for Shatter spell
pub const SHATTER_SPEED: f32 = 30.0;
pub const SHATTER_LIFETIME: f32 = 4.0;
pub const SHATTER_COLLISION_RADIUS: f32 = 1.2;
pub const SHATTER_SLOW_MULTIPLIER: f32 = 2.0;
pub const SHATTER_FROZEN_MULTIPLIER: f32 = 3.0;

/// Get the frost element color for visual effects
pub fn shatter_color() -> Color {
    Element::Frost.color()
}

/// Marker component for shatter projectiles
#[derive(Component, Debug, Clone)]
pub struct ShatterProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Base damage dealt on hit
    pub base_damage: f32,
    /// Damage multiplier for slowed enemies
    pub slow_multiplier: f32,
    /// Damage multiplier for frozen enemies
    pub frozen_multiplier: f32,
}

impl ShatterProjectile {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, base_damage: f32) -> Self {
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            base_damage,
            slow_multiplier: SHATTER_SLOW_MULTIPLIER,
            frozen_multiplier: SHATTER_FROZEN_MULTIPLIER,
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(direction, SHATTER_SPEED, SHATTER_LIFETIME, spell.damage())
    }

    /// Calculate final damage based on enemy status
    pub fn calculate_damage(&self, has_slowed: bool, has_frozen: bool) -> f32 {
        if has_frozen {
            self.base_damage * self.frozen_multiplier
        } else if has_slowed {
            self.base_damage * self.slow_multiplier
        } else {
            self.base_damage
        }
    }
}

/// Event fired when a shatter projectile collides with an enemy
#[derive(Message)]
pub struct ShatterEnemyCollisionEvent {
    pub shatter_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves shatter projectiles
pub fn shatter_movement_system(
    mut shatter_query: Query<(&mut Transform, &ShatterProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, shatter) in shatter_query.iter_mut() {
        let movement = shatter.direction * shatter.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles shatter lifetime
pub fn shatter_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut shatter_query: Query<(Entity, &mut ShatterProjectile)>,
) {
    for (entity, mut shatter) in shatter_query.iter_mut() {
        shatter.lifetime.tick(time.delta());

        if shatter.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects shatter-enemy collisions and fires events
pub fn shatter_collision_detection(
    shatter_query: Query<(Entity, &Transform), With<ShatterProjectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<ShatterEnemyCollisionEvent>,
) {
    for (shatter_entity, shatter_transform) in shatter_query.iter() {
        let shatter_xz = Vec2::new(
            shatter_transform.translation.x,
            shatter_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = shatter_xz.distance(enemy_xz);

            if distance < SHATTER_COLLISION_RADIUS {
                collision_events.write(ShatterEnemyCollisionEvent {
                    shatter_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per shatter projectile
            }
        }
    }
}

/// System that applies effects when shatter projectiles collide with enemies
/// Sends DamageEvent with bonus damage based on enemy status (slowed/frozen)
pub fn shatter_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<ShatterEnemyCollisionEvent>,
    shatter_query: Query<&ShatterProjectile>,
    slowed_query: Query<&SlowedDebuff>,
    frozen_query: Query<&FrozenStatus>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut shatters_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32)> = Vec::new();

    for event in collision_events.read() {
        shatters_to_despawn.insert(event.shatter_entity);

        // Get shatter damage
        if let Ok(shatter) = shatter_query.get(event.shatter_entity) {
            // Check enemy status for damage multiplier
            let has_slowed = slowed_query.get(event.enemy_entity).is_ok();
            let has_frozen = frozen_query.get(event.enemy_entity).is_ok();
            let damage = shatter.calculate_damage(has_slowed, has_frozen);
            effects_to_apply.push((event.enemy_entity, damage));
        }
    }

    // Despawn shatter projectiles
    for shatter_entity in shatters_to_despawn {
        commands.entity(shatter_entity).try_despawn();
    }

    // Apply damage
    for (enemy_entity, damage) in effects_to_apply {
        damage_events.write(DamageEvent::with_element(enemy_entity, damage, Element::Frost));
    }
}

/// Cast shatter spell - spawns projectile with frost element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_shatter(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_shatter_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast shatter spell with explicit damage - spawns projectile with frost element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_shatter_with_damage(
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

    let shatter = ShatterProjectile::new(direction, SHATTER_SPEED, SHATTER_LIFETIME, damage);

    // Spawn shatter at Whisper's full 3D position
    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.ice_shard.clone()),
            Transform::from_translation(spawn_position),
            shatter,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            shatter,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    mod shatter_projectile_tests {
        use super::*;

        #[test]
        fn test_shatter_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let shatter = ShatterProjectile::new(direction, 30.0, 4.0, 40.0);

            assert_eq!(shatter.direction, direction);
            assert_eq!(shatter.speed, 30.0);
            assert_eq!(shatter.base_damage, 40.0);
            assert_eq!(shatter.slow_multiplier, SHATTER_SLOW_MULTIPLIER);
            assert_eq!(shatter.frozen_multiplier, SHATTER_FROZEN_MULTIPLIER);
        }

        #[test]
        fn test_shatter_from_spell() {
            let spell = Spell::new(SpellType::Shatter);
            let direction = Vec2::new(0.0, 1.0);
            let shatter = ShatterProjectile::from_spell(direction, &spell);

            assert_eq!(shatter.direction, direction);
            assert_eq!(shatter.speed, SHATTER_SPEED);
            assert_eq!(shatter.base_damage, spell.damage());
        }

        #[test]
        fn test_shatter_lifetime_timer() {
            let shatter = ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0);
            assert_eq!(shatter.lifetime.duration(), Duration::from_secs_f32(4.0));
            assert!(!shatter.lifetime.is_finished());
        }

        #[test]
        fn test_shatter_uses_frost_element_color() {
            let color = shatter_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod calculate_damage_tests {
        use super::*;

        #[test]
        fn test_base_damage_without_debuffs() {
            let shatter = ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0);
            let damage = shatter.calculate_damage(false, false);
            assert_eq!(damage, 40.0);
        }

        #[test]
        fn test_slowed_multiplier_damage() {
            let shatter = ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0);
            let damage = shatter.calculate_damage(true, false);
            assert_eq!(damage, 80.0); // 40 * 2
        }

        #[test]
        fn test_frozen_multiplier_damage() {
            let shatter = ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0);
            let damage = shatter.calculate_damage(false, true);
            assert_eq!(damage, 120.0); // 40 * 3
        }

        #[test]
        fn test_frozen_takes_priority_over_slowed() {
            let shatter = ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0);
            let damage = shatter.calculate_damage(true, true);
            // Frozen multiplier (3x) takes priority over slowed (2x)
            assert_eq!(damage, 120.0);
        }

        #[test]
        fn test_custom_multipliers() {
            let mut shatter = ShatterProjectile::new(Vec2::X, 30.0, 4.0, 50.0);
            shatter.slow_multiplier = 1.5;
            shatter.frozen_multiplier = 4.0;

            assert_eq!(shatter.calculate_damage(true, false), 75.0); // 50 * 1.5
            assert_eq!(shatter.calculate_damage(false, true), 200.0); // 50 * 4.0
        }
    }

    mod shatter_movement_system_tests {
        use super::*;

        #[test]
        fn test_shatter_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create shatter moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::new(1.0, 0.0), 100.0, 4.0, 40.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(shatter_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_shatter_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create shatter moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::new(0.0, 1.0), 50.0, 4.0, 40.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(shatter_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }

        #[test]
        fn test_shatter_diagonal_movement() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create shatter moving diagonally
            let direction = Vec2::new(1.0, 1.0).normalize();
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ShatterProjectile::new(direction, 100.0, 4.0, 40.0),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(shatter_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Should have moved ~70.71 in both X and Z
            let expected = 100.0 / std::f32::consts::SQRT_2;
            assert!((transform.translation.x - expected).abs() < 0.1);
            assert!((transform.translation.z - expected).abs() < 0.1);
        }
    }

    mod shatter_lifetime_system_tests {
        use super::*;

        #[test]
        fn test_shatter_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(5));
            }

            let _ = app.world_mut().run_system_once(shatter_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_shatter_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(2));
            }

            let _ = app.world_mut().run_system_once(shatter_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod shatter_collision_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<ShatterEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<ShatterEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (shatter_collision_detection, count_collisions).chain());

            // Spawn shatter at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
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
                mut events: MessageReader<ShatterEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let mut app = setup_test_app();

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (shatter_collision_detection, count_collisions).chain());

            // Spawn shatter at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
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
        fn test_collision_effects_despawns_shatter() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (shatter_collision_detection, shatter_collision_effects).chain(),
            );

            let shatter_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Shatter should be despawned
            assert!(!app.world().entities().contains(shatter_entity));
            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));
        }

        #[test]
        fn test_collision_effects_base_damage_without_debuffs() {
            #[derive(Resource)]
            struct DamageReceived(f32);

            fn capture_damage(
                mut events: MessageReader<DamageEvent>,
                mut damage: ResMut<DamageReceived>,
            ) {
                for event in events.read() {
                    damage.0 = event.amount;
                }
            }

            let mut app = setup_test_app();
            app.insert_resource(DamageReceived(0.0));

            app.add_systems(
                Update,
                (shatter_collision_detection, shatter_collision_effects, capture_damage).chain(),
            );

            // Spawn shatter with 40 base damage
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            ));

            // Spawn enemy without any debuffs
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            let damage = app.world().get_resource::<DamageReceived>().unwrap();
            assert_eq!(damage.0, 40.0); // Base damage, no multiplier
        }

        #[test]
        fn test_collision_effects_slowed_damage_multiplier() {
            #[derive(Resource)]
            struct DamageReceived(f32);

            fn capture_damage(
                mut events: MessageReader<DamageEvent>,
                mut damage: ResMut<DamageReceived>,
            ) {
                for event in events.read() {
                    damage.0 = event.amount;
                }
            }

            let mut app = setup_test_app();
            app.insert_resource(DamageReceived(0.0));

            app.add_systems(
                Update,
                (shatter_collision_detection, shatter_collision_effects, capture_damage).chain(),
            );

            // Spawn shatter with 40 base damage
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            ));

            // Spawn enemy with SlowedDebuff
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
                SlowedDebuff::default(),
            ));

            app.update();

            let damage = app.world().get_resource::<DamageReceived>().unwrap();
            assert_eq!(damage.0, 80.0); // 40 * 2 (slowed multiplier)
        }

        #[test]
        fn test_collision_effects_frozen_damage_multiplier() {
            #[derive(Resource)]
            struct DamageReceived(f32);

            fn capture_damage(
                mut events: MessageReader<DamageEvent>,
                mut damage: ResMut<DamageReceived>,
            ) {
                for event in events.read() {
                    damage.0 = event.amount;
                }
            }

            let mut app = setup_test_app();
            app.insert_resource(DamageReceived(0.0));

            app.add_systems(
                Update,
                (shatter_collision_detection, shatter_collision_effects, capture_damage).chain(),
            );

            // Spawn shatter with 40 base damage
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            ));

            // Spawn enemy with FrozenStatus
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
                FrozenStatus::default(),
            ));

            app.update();

            let damage = app.world().get_resource::<DamageReceived>().unwrap();
            assert_eq!(damage.0, 120.0); // 40 * 3 (frozen multiplier)
        }

        #[test]
        fn test_collision_effects_frozen_takes_priority_over_slowed() {
            #[derive(Resource)]
            struct DamageReceived(f32);

            fn capture_damage(
                mut events: MessageReader<DamageEvent>,
                mut damage: ResMut<DamageReceived>,
            ) {
                for event in events.read() {
                    damage.0 = event.amount;
                }
            }

            let mut app = setup_test_app();
            app.insert_resource(DamageReceived(0.0));

            app.add_systems(
                Update,
                (shatter_collision_detection, shatter_collision_effects, capture_damage).chain(),
            );

            // Spawn shatter with 40 base damage
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            ));

            // Spawn enemy with BOTH SlowedDebuff AND FrozenStatus
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
                SlowedDebuff::default(),
                FrozenStatus::default(),
            ));

            app.update();

            let damage = app.world().get_resource::<DamageReceived>().unwrap();
            // Frozen multiplier (3x) takes priority over slowed (2x)
            assert_eq!(damage.0, 120.0);
        }

        #[test]
        fn test_damage_event_has_frost_element() {
            #[derive(Resource)]
            struct DamageElement(Option<Element>);

            fn capture_element(
                mut events: MessageReader<DamageEvent>,
                mut captured: ResMut<DamageElement>,
            ) {
                for event in events.read() {
                    captured.0 = event.element;
                }
            }

            let mut app = setup_test_app();
            app.insert_resource(DamageElement(None));

            app.add_systems(
                Update,
                (shatter_collision_detection, shatter_collision_effects, capture_element).chain(),
            );

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShatterProjectile::new(Vec2::X, 30.0, 4.0, 40.0),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            let element = app.world().get_resource::<DamageElement>().unwrap();
            assert_eq!(element.0, Some(Element::Frost));
        }
    }

    mod fire_shatter_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_shatter_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Shatter);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shatter(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 shatter
            let mut query = app.world_mut().query::<&ShatterProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_shatter_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Shatter);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_shatter(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ShatterProjectile>();
            for shatter in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(shatter.direction.x > 0.9, "Shatter should move toward target");
            }
        }

        #[test]
        fn test_fire_shatter_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Shatter);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shatter(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ShatterProjectile>();
            for shatter in query.iter(app.world()) {
                assert_eq!(shatter.base_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_shatter_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Shatter);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shatter_with_damage(
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

            let mut query = app.world_mut().query::<&ShatterProjectile>();
            for shatter in query.iter(app.world()) {
                assert_eq!(shatter.base_damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_shatter_spawns_at_origin_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Shatter);
            let spawn_pos = Vec3::new(5.0, 3.0, 10.0);
            let target_pos = Vec2::new(15.0, 10.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shatter(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&Transform, &ShatterProjectile)>();
            let transforms: Vec<_> = query.iter(app.world()).collect();
            assert_eq!(transforms.len(), 1);
            assert_eq!(transforms[0].0.translation, spawn_pos);
        }
    }
}
