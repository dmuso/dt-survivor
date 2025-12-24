use bevy::prelude::*;
#[cfg(test)]
use std::time::Duration;
use std::collections::HashSet;

use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::bullets::components::*;
use crate::enemies::components::*;
use crate::player::components::*;
use crate::score::Score;
use crate::game::events::EnemyDeathEvent;

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

    // Find the closest enemy
    let player_pos = player_transform.translation.truncate();
    let mut closest_enemy_pos = None;
    let mut closest_distance = f32::INFINITY;

    for enemy_transform in enemy_query.iter() {
        let enemy_pos = enemy_transform.translation.truncate();
        let distance = player_pos.distance(enemy_pos);

        if distance < closest_distance {
            closest_distance = distance;
            closest_enemy_pos = Some(enemy_pos);
        }
    }

    // If no enemies, don't spawn bullet
    let Some(target_pos) = closest_enemy_pos else {
        return;
    };

    // Calculate base direction towards closest enemy
    let base_direction = (target_pos - player_pos).normalize();

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

        // Spawn bullet
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 1.0, 0.0), Vec2::new(8.0, 8.0)), // Yellow bullet
            Transform::from_translation(player_transform.translation + Vec3::new(0.0, 0.0, 0.1)), // Slightly above player
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
        transform.translation += movement.extend(0.0);
    }
}

pub fn bullet_collision_system(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut score: ResMut<Score>,
    mut enemy_death_events: MessageWriter<EnemyDeathEvent>,
) {
    let mut bullets_to_despawn = HashSet::new();
    let mut enemies_to_despawn = HashSet::new();
    let mut enemies_killed = 0;

    // First pass: detect all collisions
    for (bullet_entity, bullet_transform) in bullet_query.iter() {
        if bullets_to_despawn.contains(&bullet_entity) {
            continue; // This bullet is already marked for despawn
        }

        let bullet_pos = bullet_transform.translation.truncate();

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            if enemies_to_despawn.contains(&enemy_entity) {
                continue; // This enemy is already marked for despawn
            }

            let enemy_pos = enemy_transform.translation.truncate();
            let distance = bullet_pos.distance(enemy_pos);

            // Simple collision detection - if bullet is close enough to enemy
            if distance < 15.0 {
                bullets_to_despawn.insert(bullet_entity);
                enemies_to_despawn.insert(enemy_entity);
                enemies_killed += 1;
                break; // Only hit one enemy per bullet
            }
        }
    }

    // Second pass: despawn entities, spawn experience orbs, and play sounds
    // Despawn bullets
    for bullet_entity in bullets_to_despawn {
        commands.entity(bullet_entity).try_despawn();
    }

    let enemies_to_despawn_vec: Vec<Entity> = enemies_to_despawn.into_iter().collect();
    for enemy_entity in &enemies_to_despawn_vec {
        // Get enemy position for loot spawning before despawning
        let enemy_pos = enemy_query.get(*enemy_entity).map(|(_, transform)| transform.translation.truncate()).unwrap_or(Vec2::ZERO);

        // Send enemy death event for centralized loot/experience handling
        enemy_death_events.write(EnemyDeathEvent {
            enemy_entity: *enemy_entity,
            position: enemy_pos,
        });

        commands.entity(*enemy_entity).try_despawn();
    }

    // Update score
    score.0 += enemies_killed;
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
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_bullet_spawn_timer_creation() {
        let timer = BulletSpawnTimer::default();
        assert_eq!(timer.0.duration(), std::time::Duration::from_secs_f32(2.0));
    }

    #[test]
    fn test_bullet_movement_basic() {
        let mut app = App::new();

        // Add Time plugin to properly handle time
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create bullet moving right
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
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

        // Bullet should have moved 100 units to the right
        let bullet_transform = app.world().get::<Transform>(bullet_entity).unwrap();
        assert_eq!(bullet_transform.translation.x, 100.0);
        assert_eq!(bullet_transform.translation.y, 0.0);
    }

    #[test]
    fn test_bullet_collision_detection() {
        let mut app = App::new();

        // Initialize resources and add plugins
        app.init_resource::<Score>();
        app.add_message::<crate::game::events::EnemyDeathEvent>();
        app.add_systems(Update, bullet_collision_system);
        app.add_systems(Update, crate::loot::systems::enemy_death_system);

        // Create bullet at (0, 0)
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Create enemy at (10, 0) - within collision distance
        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
        )).id();

        app.update();

        // Both bullet and enemy should be despawned
        assert!(!app.world().entities().contains(bullet_entity));
        assert!(!app.world().entities().contains(enemy_entity));

        // Score should be incremented
        let score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(score.0, 1);
    }

    #[test]
    fn test_bullet_collision_no_collision() {
        let mut app = App::new();

        // Initialize resources
        app.init_resource::<Score>();
        app.add_message::<crate::game::events::EnemyDeathEvent>();
        app.add_systems(Update, bullet_collision_system);
        app.add_systems(Update, crate::loot::systems::enemy_death_system);

        // Create bullet at (0, 0)
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        // Create enemy far away - outside collision distance
        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
        )).id();

        app.update();

        // Both bullet and enemy should still exist
        assert!(app.world().entities().contains(bullet_entity));
        assert!(app.world().entities().contains(enemy_entity));

        // Score should remain unchanged
        let score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(score.0, 0);
    }

    #[test]
    fn test_bullet_lifetime_expiration() {
        let mut app = App::new();

        // Add Time plugin
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create bullet with expired lifetime
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
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
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
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
}