use bevy::prelude::*;
#[cfg(test)]
use std::time::Duration;

use crate::bullets::components::*;
use crate::enemies::components::*;
use crate::player::components::*;
use crate::score::Score;

#[derive(Resource)]
pub struct BulletSpawnTimer(pub Timer);

impl Default for BulletSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2.0, TimerMode::Repeating))
    }
}

pub fn bullet_spawning_system(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<BulletSpawnTimer>,
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

    // Calculate direction towards closest enemy
    let direction = (target_pos - player_pos).normalize();

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
) {
    for (bullet_entity, bullet_transform) in bullet_query.iter() {
        let bullet_pos = bullet_transform.translation.truncate();

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = enemy_transform.translation.truncate();
            let distance = bullet_pos.distance(enemy_pos);

            // Simple collision detection - if bullet is close enough to enemy
            if distance < 15.0 {
                // Despawn both bullet and enemy
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
                // Increment score when enemy is defeated
                score.0 += 1;
                break; // Only hit one enemy per bullet
            }
        }
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

        // Initialize score resource
        app.init_resource::<Score>();

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

        let _ = app.world_mut().run_system_once(bullet_collision_system);

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

        // Initialize score resource
        app.init_resource::<Score>();

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

        let _ = app.world_mut().run_system_once(bullet_collision_system);

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