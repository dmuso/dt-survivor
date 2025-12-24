use bevy::prelude::*;
use rand::Rng;

use crate::enemies::components::*;
use crate::player::components::*;
use crate::game::resources::*;

pub const ENEMY_SPAWN_DISTANCE: f32 = 600.0; // Distance from player to spawn enemies
pub const ENEMY_COUNT: usize = 5; // Number of enemies to maintain

pub fn enemy_spawning_system(
    mut commands: Commands,
    enemy_query: Query<&Enemy>,
    player_query: Query<&Transform, With<Player>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let enemy_count = enemy_query.iter().count();

    // Only spawn if we have fewer enemies than desired
    if enemy_count >= ENEMY_COUNT {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let mut rng = rand::thread_rng();

    // Calculate camera viewport bounds in world space
    let viewport_size = camera.logical_viewport_size().unwrap_or(Vec2::new(800.0, 600.0));
    let half_width = viewport_size.x / 2.0;
    let half_height = viewport_size.y / 2.0;

    // Camera position in world space
    let camera_pos = camera_transform.translation().truncate();

    for _ in enemy_count..ENEMY_COUNT {
        // Generate random angle and distance for spawning outside view
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let distance = ENEMY_SPAWN_DISTANCE + rng.gen_range(0.0..200.0);

        // Calculate spawn position relative to player
        let spawn_offset = Vec2::new(angle.cos(), angle.sin()) * distance;
        let mut spawn_pos = player_transform.translation.truncate() + spawn_offset;

        // Ensure the spawn position is outside the camera viewport
        let to_spawn = spawn_pos - camera_pos;
        if to_spawn.x.abs() < half_width + 100.0 && to_spawn.y.abs() < half_height + 100.0 {
            // If too close to viewport, adjust position
            let adjusted_distance = (half_width.max(half_height) + 150.0) / to_spawn.length().max(1.0);
            let adjusted_pos = camera_pos + to_spawn * adjusted_distance;
            spawn_pos = adjusted_pos;
        }

        // Spawn enemy
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 0.0, 0.0), Vec2::new(15.0, 15.0)), // Red enemy
            Transform::from_translation(Vec3::new(spawn_pos.x, spawn_pos.y, 0.5)),
            Enemy { speed: 50.0 }, // Slower than player (player is 200.0)
        ));
    }
}

pub fn enemy_movement_system(
    mut enemy_query: Query<(&mut Transform, &Enemy)>,
    player_position: Res<PlayerPosition>,
    time: Res<Time>,
) {
    let player_pos = player_position.0;

    for (mut transform, enemy) in enemy_query.iter_mut() {
        let enemy_pos = transform.translation.truncate();
        let direction = (player_pos - enemy_pos).normalize();

        // Move enemy towards player
        let movement = direction * enemy.speed * time.delta_secs();
        transform.translation += movement.extend(0.0);
    }
}