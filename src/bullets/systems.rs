use bevy::prelude::*;
#[cfg(test)]
use std::time::Duration;
use std::collections::HashSet;

use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::bullets::components::*;
use crate::combat::DamageEvent;
use crate::enemies::components::*;
use crate::game::events::BulletEnemyCollisionEvent;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::player::components::*;

/// Height of bullet cube center above ground (half of 0.3 cube height)
pub const BULLET_Y_HEIGHT: f32 = 0.15;

#[derive(Resource)]
pub struct BulletSpawnTimer(pub Timer);

impl Default for BulletSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2.0, TimerMode::Repeating))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn bullet_spawning_system(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<BulletSpawnTimer>,
    asset_server: Option<Res<AssetServer>>,
    weapon_channel: Option<ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<ResMut<SoundLimiter>>,
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<&Transform, With<Enemy>>,
) {
    // Update the spawn timer
    spawn_timer.0.tick(time.delta());

    // Only spawn if timer finished and there's a player
    if !spawn_timer.0.just_finished() {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    // Reset the timer
    spawn_timer.0.reset();

    // Find the closest enemy (using XZ plane for 3D)
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    let mut closest_enemy_pos = None;
    let mut closest_distance = f32::INFINITY;

    for enemy_transform in enemy_query.iter() {
        let enemy_xz = Vec2::new(
            enemy_transform.translation.x,
            enemy_transform.translation.z,
        );
        let distance = player_xz.distance(enemy_xz);

        if distance < closest_distance {
            closest_distance = distance;
            closest_enemy_pos = Some(enemy_xz);
        }
    }

    // If no enemies, don't spawn bullet
    let Some(target_pos) = closest_enemy_pos else {
        return;
    };

    // Calculate base direction towards closest enemy (on XZ plane)
    let base_direction = (target_pos - player_xz).normalize();

    // Spawn 5 bullets in a burst with slight directional spread
    let spread_angle = std::f32::consts::PI / 12.0; // 15 degrees spread between bullets
    for i in -2..=2 {
        let angle_offset = i as f32 * spread_angle;
        // Rotate the base direction by the spread angle
        let cos_offset = angle_offset.cos();
        let sin_offset = angle_offset.sin();
        let direction = Vec2::new(
            base_direction.x * cos_offset - base_direction.y * sin_offset,
            base_direction.x * sin_offset + base_direction.y * cos_offset,
        );

        // Spawn bullet as 3D mesh on XZ plane
        commands.spawn((
            Mesh3d(game_meshes.bullet.clone()),
            MeshMaterial3d(game_materials.bullet.clone()),
            Transform::from_translation(Vec3::new(
                player_transform.translation.x,
                BULLET_Y_HEIGHT,
                player_transform.translation.z,
            )),
            Bullet {
                direction,
                speed: 200.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        ));
    }

    // Play weapon sound effect once for the burst (only if AssetServer and AudioChannel are available)
    if let (Some(asset_server), Some(mut weapon_channel), Some(mut sound_limiter)) =
        (asset_server, weapon_channel, sound_limiter) {
        crate::audio::plugin::play_limited_sound(
            weapon_channel.as_mut(),
            &asset_server,
            "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
            sound_limiter.as_mut(),
        );
    }
}

pub fn bullet_movement_system(
    mut bullet_query: Query<(&mut Transform, &Bullet)>,
    time: Res<Time>,
) {
    for (mut transform, bullet) in bullet_query.iter_mut() {
        let movement = bullet.direction * bullet.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// Collision radius for bullet-enemy detection (scaled for 3D world units)
pub const BULLET_COLLISION_RADIUS: f32 = 1.0;

/// System that detects bullet-enemy collisions and fires events
pub fn bullet_collision_detection(
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<BulletEnemyCollisionEvent>,
) {
    // Detect all collisions and fire events
    for (bullet_entity, bullet_transform) in bullet_query.iter() {
        // Use XZ plane for 3D collision detection
        let bullet_xz = Vec2::new(
            bullet_transform.translation.x,
            bullet_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = bullet_xz.distance(enemy_xz);

            // Simple collision detection - if bullet is close enough to enemy
            if distance < BULLET_COLLISION_RADIUS {
                collision_events.write(BulletEnemyCollisionEvent {
                    bullet_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per bullet
            }
        }
    }
}

/// System that applies effects when bullets collide with enemies
/// Sends DamageEvent to damage enemies through the combat system
pub fn bullet_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<BulletEnemyCollisionEvent>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut bullets_to_despawn = HashSet::new();
    let mut enemies_damaged = HashSet::new();

    // Process collision events
    for event in collision_events.read() {
        bullets_to_despawn.insert(event.bullet_entity);
        enemies_damaged.insert(event.enemy_entity);
    }

    // Despawn bullets
    for bullet_entity in bullets_to_despawn {
        commands.entity(bullet_entity).try_despawn();
    }

    // Send damage events to enemies (combat system handles death)
    for enemy_entity in enemies_damaged {
        damage_events.write(DamageEvent::with_source(
            enemy_entity,
            BULLET_DAMAGE,
            enemy_entity, // source could be the bullet, but we already despawned it
        ));
    }
}

pub fn bullet_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut bullet_query: Query<(Entity, &mut Bullet)>,
) {
    for (entity, mut bullet) in bullet_query.iter_mut() {
        bullet.lifetime.tick(time.delta());

        // Despawn bullet if lifetime expired
        if bullet.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::asset::Assets;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::pbr::StandardMaterial;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::time::TimePlugin::default(),
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app
    }

    fn setup_game_resources(app: &mut App) {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let game_meshes = GameMeshes::new(&mut meshes);
        app.world_mut().insert_resource(game_meshes);

        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        let game_materials = GameMaterials::new(&mut materials);
        app.world_mut().insert_resource(game_materials);
    }

    #[test]
    fn test_bullet_spawn_timer_creation() {
        let timer = BulletSpawnTimer::default();
        assert_eq!(timer.0.duration(), std::time::Duration::from_secs_f32(2.0));
    }

    #[test]
    fn test_bullet_movement_on_xz_plane() {
        let mut app = App::new();

        // Add Time plugin to properly handle time
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create bullet at BULLET_Y_HEIGHT moving in X direction (direction.x = 1, direction.y = 0)
        // In 3D, direction.y maps to Z axis
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, BULLET_Y_HEIGHT, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0), // X direction only
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Manually set time to simulate 1 second passed
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(bullet_movement_system);

        // Bullet should have moved 100 units in X, Y unchanged, Z unchanged
        let bullet_transform = app.world().get::<Transform>(bullet_entity).unwrap();
        assert_eq!(bullet_transform.translation.x, 100.0);
        assert_eq!(bullet_transform.translation.y, BULLET_Y_HEIGHT); // Y stays constant
        assert_eq!(bullet_transform.translation.z, 0.0);
    }

    #[test]
    fn test_bullet_movement_z_direction() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create bullet moving in Z direction (direction.y maps to Z axis in 3D)
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, BULLET_Y_HEIGHT, 0.0)),
            Bullet {
                direction: Vec2::new(0.0, 1.0), // This maps to +Z direction
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(bullet_movement_system);

        let bullet_transform = app.world().get::<Transform>(bullet_entity).unwrap();
        assert_eq!(bullet_transform.translation.x, 0.0);
        assert_eq!(bullet_transform.translation.y, BULLET_Y_HEIGHT);
        assert_eq!(bullet_transform.translation.z, 100.0); // Moved in Z
    }

    #[test]
    fn test_bullet_collision_on_xz_plane() {
        use crate::combat::{CheckDeath, DamageEvent, Health};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mut app = App::new();

        // Counter for damage events
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

        // Initialize resources and add plugins
        app.add_message::<DamageEvent>();
        app.add_message::<crate::game::events::BulletEnemyCollisionEvent>();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Chain the systems so collision detection writes events before effects reads them
        app.add_systems(
            Update,
            (bullet_collision_detection, bullet_collision_effects, count_damage_events).chain(),
        );

        // Create bullet at origin on XZ plane (Y at bullet height)
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, BULLET_Y_HEIGHT, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Create enemy at (0.5, enemy_height, 0) - within BULLET_COLLISION_RADIUS on XZ plane
        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
            CheckDeath,
        )).id();

        app.update();

        // Bullet should be despawned
        assert!(!app.world().entities().contains(bullet_entity));

        // Enemy should still exist (damage doesn't kill instantly now)
        assert!(app.world().entities().contains(enemy_entity));

        // DamageEvent should have been sent
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_bullet_collision_ignores_y_distance() {
        use crate::combat::{CheckDeath, DamageEvent, Health};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

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
        app.add_message::<crate::game::events::BulletEnemyCollisionEvent>();
        app.add_plugins(bevy::time::TimePlugin::default());

        app.add_systems(
            Update,
            (bullet_collision_detection, bullet_collision_effects, count_damage_events).chain(),
        );

        // Create bullet at Y=0
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Create enemy at same XZ but different Y - should still collide (Y is ignored)
        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 100.0, 0.0)), // Far in Y but close in XZ
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
            CheckDeath,
        )).id();

        app.update();

        // Collision should happen (XZ distance is small)
        assert!(!app.world().entities().contains(bullet_entity));
        assert!(app.world().entities().contains(enemy_entity));
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_bullet_collision_no_collision_outside_radius() {
        use crate::combat::DamageEvent;

        let mut app = App::new();

        // Initialize resources
        app.add_message::<DamageEvent>();
        app.add_message::<crate::game::events::BulletEnemyCollisionEvent>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_systems(Update, (bullet_collision_detection, bullet_collision_effects).chain());

        // Create bullet at origin
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, BULLET_Y_HEIGHT, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Create enemy far away on XZ plane - outside BULLET_COLLISION_RADIUS
        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
        )).id();

        app.update();

        // Both bullet and enemy should still exist
        assert!(app.world().entities().contains(bullet_entity));
        assert!(app.world().entities().contains(enemy_entity));
    }

    #[test]
    fn test_bullet_lifetime_expiration() {
        let mut app = App::new();

        // Add Time plugin
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create bullet with expired lifetime
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, BULLET_Y_HEIGHT, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Advance time past the lifetime
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs(16));
        }

        let _ = app.world_mut().run_system_once(bullet_lifetime_system);

        // Bullet should be despawned
        assert!(!app.world().entities().contains(bullet_entity));
    }

    #[test]
    fn test_bullet_lifetime_not_expired() {
        let mut app = App::new();

        // Add Time plugin
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create bullet with lifetime not expired
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, BULLET_Y_HEIGHT, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Advance time but not past lifetime
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs(10));
        }

        let _ = app.world_mut().run_system_once(bullet_lifetime_system);

        // Bullet should still exist
        assert!(app.world().entities().contains(bullet_entity));
    }

    #[test]
    fn test_bullet_spawns_with_mesh3d_component() {
        use crate::combat::Health;

        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        app.init_resource::<BulletSpawnTimer>();

        // Spawn player on XZ plane
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        // Spawn enemy nearby on XZ plane
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(5.0, 0.375, 5.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
        ));

        // Advance time to allow timer to complete when system runs
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(3));
        }

        let _ = app.world_mut().run_system_once(bullet_spawning_system);

        // Verify bullets have Mesh3d component
        let mut query = app.world_mut().query::<(Entity, &Bullet)>();
        let bullet_count = query.iter(app.world()).count();
        assert!(bullet_count > 0, "At least one bullet should have spawned");

        for (entity, _) in query.iter(app.world()) {
            assert!(
                app.world().get::<Mesh3d>(entity).is_some(),
                "Bullet should have Mesh3d component"
            );
        }
    }

    #[test]
    fn test_bullet_spawns_with_mesh_material_3d() {
        use crate::combat::Health;

        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        app.init_resource::<BulletSpawnTimer>();

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(5.0, 0.375, 5.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
        ));

        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(3));
        }

        let _ = app.world_mut().run_system_once(bullet_spawning_system);

        let mut query = app.world_mut().query::<(Entity, &Bullet)>();
        for (entity, _) in query.iter(app.world()) {
            assert!(
                app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).is_some(),
                "Bullet should have MeshMaterial3d component"
            );
        }
    }

    #[test]
    fn test_bullet_spawns_at_correct_y_height() {
        use crate::combat::Health;

        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        app.init_resource::<BulletSpawnTimer>();

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(5.0, 0.375, 5.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
        ));

        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(3));
        }

        let _ = app.world_mut().run_system_once(bullet_spawning_system);

        let mut query = app.world_mut().query::<(&Transform, &Bullet)>();
        for (transform, _) in query.iter(app.world()) {
            assert!(
                (transform.translation.y - BULLET_Y_HEIGHT).abs() < 0.001,
                "Bullet Y position should be {}, got {}",
                BULLET_Y_HEIGHT,
                transform.translation.y
            );
        }
    }

    #[test]
    fn test_bullet_spawns_at_player_xz_position() {
        use crate::combat::Health;

        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        app.init_resource::<BulletSpawnTimer>();

        let player_x = 10.0;
        let player_z = 15.0;

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(player_x, 0.5, player_z)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(20.0, 0.375, 20.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
        ));

        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(3));
        }

        let _ = app.world_mut().run_system_once(bullet_spawning_system);

        let mut query = app.world_mut().query::<(&Transform, &Bullet)>();
        for (transform, _) in query.iter(app.world()) {
            assert!(
                (transform.translation.x - player_x).abs() < 0.001,
                "Bullet X should match player X: expected {}, got {}",
                player_x,
                transform.translation.x
            );
            assert!(
                (transform.translation.z - player_z).abs() < 0.001,
                "Bullet Z should match player Z: expected {}, got {}",
                player_z,
                transform.translation.z
            );
        }
    }

    #[test]
    fn test_bullet_does_not_have_sprite_component() {
        use crate::combat::Health;

        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        app.init_resource::<BulletSpawnTimer>();

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(5.0, 0.375, 5.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
        ));

        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(3));
        }

        let _ = app.world_mut().run_system_once(bullet_spawning_system);

        let mut query = app.world_mut().query::<(Entity, &Bullet)>();
        for (entity, _) in query.iter(app.world()) {
            assert!(
                app.world().get::<Sprite>(entity).is_none(),
                "Bullet should NOT have Sprite component in 3D mode"
            );
        }
    }

    #[test]
    fn test_bullet_direction_targets_enemy_on_xz_plane() {
        use crate::combat::Health;

        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        app.init_resource::<BulletSpawnTimer>();

        // Player at origin
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        // Enemy directly in +X direction on XZ plane
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
        ));

        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(3));
        }

        let _ = app.world_mut().run_system_once(bullet_spawning_system);

        // The center bullet (no spread) should have direction pointing to +X
        let mut query = app.world_mut().query::<(&Transform, &Bullet)>();
        let bullets: Vec<_> = query.iter(app.world()).collect();

        // With 5 bullets in spread, the middle one should be closest to pure +X direction
        let center_bullet = bullets.iter()
            .max_by(|(_, b1), (_, b2)| {
                b1.direction.x.partial_cmp(&b2.direction.x).unwrap()
            })
            .expect("Should have bullets");

        // The most +X direction bullet should have direction.x close to 1.0
        assert!(
            center_bullet.1.direction.x > 0.9,
            "Center bullet direction.x should be close to 1.0, got {}",
            center_bullet.1.direction.x
        );
    }
}