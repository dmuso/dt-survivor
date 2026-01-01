use std::collections::HashSet;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::combat::{DamageEvent, Health};
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Shadow Bolt spell
pub const SHADOW_BOLT_SPEED: f32 = 25.0;
pub const SHADOW_BOLT_LIFETIME: f32 = 5.0;
pub const SHADOW_BOLT_LIFESTEAL_PERCENTAGE: f32 = 0.20; // 20% of damage dealt returned as healing
pub const SHADOW_BOLT_SPREAD_ANGLE: f32 = 15.0;
pub const SHADOW_BOLT_COLLISION_RADIUS: f32 = 1.0;

/// Get the dark element color for visual effects (purple)
pub fn shadow_bolt_color() -> Color {
    Element::Dark.color()
}

/// ShadowBoltProjectile component - a fast projectile that damages enemies
/// and heals the player for a percentage of damage dealt (lifesteal).
#[derive(Component, Debug, Clone)]
pub struct ShadowBoltProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Direct hit damage
    pub damage: f32,
    /// Percentage of damage dealt returned as healing (0.0 to 1.0)
    pub lifesteal_percentage: f32,
}

impl ShadowBoltProjectile {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, damage: f32, lifesteal_percentage: f32) -> Self {
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            lifesteal_percentage,
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(
            direction,
            SHADOW_BOLT_SPEED,
            SHADOW_BOLT_LIFETIME,
            spell.damage(),
            SHADOW_BOLT_LIFESTEAL_PERCENTAGE,
        )
    }
}

/// Collision event for shadow bolt hitting an enemy
#[derive(Message)]
pub struct ShadowBoltEnemyCollisionEvent {
    pub shadow_bolt_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves shadow bolt projectiles
pub fn shadow_bolt_movement_system(
    mut shadow_bolt_query: Query<(&mut Transform, &ShadowBoltProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, shadow_bolt) in shadow_bolt_query.iter_mut() {
        let movement = shadow_bolt.direction * shadow_bolt.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles shadow bolt lifetime
pub fn shadow_bolt_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut shadow_bolt_query: Query<(Entity, &mut ShadowBoltProjectile)>,
) {
    for (entity, mut shadow_bolt) in shadow_bolt_query.iter_mut() {
        shadow_bolt.lifetime.tick(time.delta());

        if shadow_bolt.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects shadow bolt-enemy collisions and fires events
pub fn shadow_bolt_collision_detection(
    shadow_bolt_query: Query<(Entity, &Transform), With<ShadowBoltProjectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<ShadowBoltEnemyCollisionEvent>,
) {
    for (shadow_bolt_entity, shadow_bolt_transform) in shadow_bolt_query.iter() {
        let shadow_bolt_xz = Vec2::new(
            shadow_bolt_transform.translation.x,
            shadow_bolt_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = shadow_bolt_xz.distance(enemy_xz);

            if distance < SHADOW_BOLT_COLLISION_RADIUS {
                collision_events.write(ShadowBoltEnemyCollisionEvent {
                    shadow_bolt_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per shadow bolt
            }
        }
    }
}

/// System that applies effects when shadow bolts collide with enemies
/// Sends DamageEvent and heals the player for lifesteal percentage
pub fn shadow_bolt_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<ShadowBoltEnemyCollisionEvent>,
    shadow_bolt_query: Query<&ShadowBoltProjectile>,
    mut player_query: Query<&mut Health, With<Player>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut bolts_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32, f32)> = Vec::new();

    for event in collision_events.read() {
        bolts_to_despawn.insert(event.shadow_bolt_entity);

        // Get shadow bolt damage and lifesteal values
        if let Ok(shadow_bolt) = shadow_bolt_query.get(event.shadow_bolt_entity) {
            effects_to_apply.push((event.enemy_entity, shadow_bolt.damage, shadow_bolt.lifesteal_percentage));
        }
    }

    // Despawn shadow bolts
    for bolt_entity in bolts_to_despawn {
        commands.entity(bolt_entity).try_despawn();
    }

    // Apply damage and lifesteal effects
    for (enemy_entity, damage, lifesteal_percentage) in effects_to_apply {
        // Send damage event with Dark element
        damage_events.write(DamageEvent::with_element(enemy_entity, damage, Element::Dark));

        // Apply lifesteal healing to player
        let heal_amount = damage * lifesteal_percentage;
        if let Ok(mut player_health) = player_query.single_mut() {
            player_health.heal(heal_amount);
        }
    }
}

/// Cast shadow bolt spell - spawns projectiles with dark element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_shadow_bolt(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_shadow_bolt_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        asset_server,
        weapon_channel,
        sound_limiter,
        game_meshes,
        game_materials,
    );
}

/// Cast shadow bolt spell with explicit damage - spawns projectiles with dark element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_shadow_bolt_with_damage(
    commands: &mut Commands,
    spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let base_direction = (target_pos - spawn_xz).normalize();

    // Get projectile count based on spell level (1 at level 1-4, 2 at 5-9, 3 at 10)
    let projectile_count = spell.projectile_count();
    let spread_angle_rad = SHADOW_BOLT_SPREAD_ANGLE.to_radians();

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

        let shadow_bolt = ShadowBoltProjectile::new(
            direction,
            SHADOW_BOLT_SPEED,
            SHADOW_BOLT_LIFETIME,
            damage,
            SHADOW_BOLT_LIFESTEAL_PERCENTAGE,
        );

        // Spawn shadow bolt at Whisper's full 3D position
        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.shadow_bolt.clone()),
                Transform::from_translation(spawn_position),
                shadow_bolt,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                shadow_bolt,
            ));
        }
    }

    // Play spell sound effect
    if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
        (asset_server, weapon_channel, sound_limiter)
    {
        play_limited_sound(
            weapon_channel,
            asset_server,
            "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
            sound_limiter,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod shadow_bolt_projectile_tests {
        use super::*;

        #[test]
        fn test_shadow_bolt_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let shadow_bolt = ShadowBoltProjectile::new(direction, 25.0, 5.0, 20.0, 0.2);

            assert_eq!(shadow_bolt.direction, direction);
            assert_eq!(shadow_bolt.speed, 25.0);
            assert_eq!(shadow_bolt.damage, 20.0);
            assert_eq!(shadow_bolt.lifesteal_percentage, 0.2);
        }

        #[test]
        fn test_shadow_bolt_from_spell() {
            let spell = Spell::new(SpellType::ShadowBolt);
            let direction = Vec2::new(0.0, 1.0);
            let shadow_bolt = ShadowBoltProjectile::from_spell(direction, &spell);

            assert_eq!(shadow_bolt.direction, direction);
            assert_eq!(shadow_bolt.speed, SHADOW_BOLT_SPEED);
            assert_eq!(shadow_bolt.damage, spell.damage());
            assert_eq!(shadow_bolt.lifesteal_percentage, SHADOW_BOLT_LIFESTEAL_PERCENTAGE);
        }

        #[test]
        fn test_shadow_bolt_lifetime_timer() {
            let shadow_bolt = ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2);
            assert_eq!(shadow_bolt.lifetime.duration(), Duration::from_secs_f32(5.0));
            assert!(!shadow_bolt.lifetime.is_finished());
        }

        #[test]
        fn test_shadow_bolt_uses_dark_element_color() {
            let color = shadow_bolt_color();
            assert_eq!(color, Element::Dark.color());
            assert_eq!(color, Color::srgb_u8(128, 0, 128)); // Purple
        }
    }

    mod fire_shadow_bolt_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_shadow_bolt_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ShadowBolt);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shadow_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 shadow bolt (level 1)
            let mut query = app.world_mut().query::<&ShadowBoltProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_shadow_bolt_spawns_multiple_at_higher_levels() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::ShadowBolt);
            spell.level = 5; // Should spawn 2 projectiles
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shadow_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ShadowBoltProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2);
        }

        #[test]
        fn test_fire_shadow_bolt_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ShadowBolt);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_shadow_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ShadowBoltProjectile>();
            for shadow_bolt in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(shadow_bolt.direction.x > 0.9, "Shadow bolt should move toward target");
            }
        }

        #[test]
        fn test_fire_shadow_bolt_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ShadowBolt);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shadow_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ShadowBoltProjectile>();
            for shadow_bolt in query.iter(app.world()) {
                assert_eq!(shadow_bolt.damage, expected_damage);
                assert_eq!(shadow_bolt.lifesteal_percentage, SHADOW_BOLT_LIFESTEAL_PERCENTAGE);
            }
        }

        #[test]
        fn test_fire_shadow_bolt_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ShadowBolt);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_shadow_bolt_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ShadowBoltProjectile>();
            for shadow_bolt in query.iter(app.world()) {
                assert_eq!(shadow_bolt.damage, explicit_damage);
            }
        }
    }

    mod shadow_bolt_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_shadow_bolt_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create shadow bolt moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::new(1.0, 0.0), 100.0, 5.0, 20.0, 0.2),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(shadow_bolt_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_shadow_bolt_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create shadow bolt moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::new(0.0, 1.0), 50.0, 5.0, 20.0, 0.2),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(shadow_bolt_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }
    }

    mod shadow_bolt_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_shadow_bolt_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(6));
            }

            let _ = app.world_mut().run_system_once(shadow_bolt_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_shadow_bolt_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(3));
            }

            let _ = app.world_mut().run_system_once(shadow_bolt_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod shadow_bolt_collision_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<ShadowBoltEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<ShadowBoltEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (shadow_bolt_collision_detection, count_collisions).chain());

            // Spawn shadow bolt at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2),
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
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<ShadowBoltEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (shadow_bolt_collision_detection, count_collisions).chain());

            // Spawn shadow bolt at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2),
            ));

            // Spawn enemy far away (beyond collision radius)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_collision_effects_despawns_shadow_bolt() {
            let mut app = setup_test_app();

            // Spawn player with health
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
            ));

            // Chain detection and effects so events are processed
            app.add_systems(
                Update,
                (shadow_bolt_collision_detection, shadow_bolt_collision_effects).chain(),
            );

            let shadow_bolt_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Shadow bolt should be despawned
            assert!(!app.world().entities().contains(shadow_bolt_entity));
            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));
        }

        #[test]
        fn test_collision_effects_heals_player_for_lifesteal() {
            let mut app = setup_test_app();

            // Chain detection and effects so events are processed
            app.add_systems(
                Update,
                (shadow_bolt_collision_detection, shadow_bolt_collision_effects).chain(),
            );

            // Spawn player with 50 health (out of 100)
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health { current: 50.0, max: 100.0 },
            )).id();

            // Shadow bolt with 20 damage and 20% lifesteal = 4 HP healing
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 20.0, 0.2),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // Player should be healed for 20 * 0.2 = 4 HP
            let player_health = app.world().get::<Health>(player_entity).unwrap();
            assert_eq!(player_health.current, 54.0, "Player should be healed for lifesteal percentage");
        }

        #[test]
        fn test_lifesteal_does_not_exceed_max_health() {
            let mut app = setup_test_app();

            // Chain detection and effects so events are processed
            app.add_systems(
                Update,
                (shadow_bolt_collision_detection, shadow_bolt_collision_effects).chain(),
            );

            // Spawn player with 95 health (out of 100)
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health { current: 95.0, max: 100.0 },
            )).id();

            // Shadow bolt with 100 damage and 50% lifesteal = 50 HP healing (but should cap at 100)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ShadowBoltProjectile::new(Vec2::X, 25.0, 5.0, 100.0, 0.5),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            // Player should be capped at max health
            let player_health = app.world().get::<Health>(player_entity).unwrap();
            assert_eq!(player_health.current, 100.0, "Healing should not exceed max health");
        }

        #[test]
        fn test_multiple_shadow_bolts_can_exist() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ShadowBolt);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            // Fire multiple shadow bolts
            for _ in 0..3 {
                let mut commands = app.world_mut().commands();
                fire_shadow_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            // Should have 3 shadow bolts
            let mut query = app.world_mut().query::<&ShadowBoltProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 3);
        }
    }
}
