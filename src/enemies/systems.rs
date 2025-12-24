use bevy::prelude::*;
use rand::Rng;

use crate::enemies::components::*;
use crate::player::components::*;
use crate::game::resources::*;

pub const ENEMY_SPAWN_DISTANCE: f32 = 600.0; // Distance from player to spawn enemies

pub fn enemy_spawning_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut spawn_state: ResMut<EnemySpawnState>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Update timers
    spawn_state.time_since_last_spawn += time.delta_secs();
    spawn_state.time_since_last_rate_increase += time.delta_secs();

    // Check if we should increase the spawn rate (every 20 seconds)
    if spawn_state.time_since_last_rate_increase >= 20.0 {
        spawn_state.spawn_rate_per_second *= 2.0;
        spawn_state.rate_level += 1;
        spawn_state.time_since_last_rate_increase = 0.0;
    }

    // Calculate how many enemies should spawn this frame
    let spawn_interval = 1.0 / spawn_state.spawn_rate_per_second;
    let enemies_to_spawn = (spawn_state.time_since_last_spawn / spawn_interval) as usize;

    if enemies_to_spawn > 0 {
        let mut rng = rand::thread_rng();

        // Calculate camera viewport bounds in world space
        let viewport_size = camera.logical_viewport_size().unwrap_or(Vec2::new(800.0, 600.0));
        let half_width = viewport_size.x / 2.0;
        let half_height = viewport_size.y / 2.0;

        // Camera position in world space
        let camera_pos = camera_transform.translation().truncate();

        for _ in 0..enemies_to_spawn {
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
                Enemy { speed: 50.0, strength: 10.0 }, // Slower than player (player is 200.0), moderate strength
            ));
        }

        // Reset the spawn timer (subtract the time we've accounted for)
        spawn_state.time_since_last_spawn -= enemies_to_spawn as f32 * spawn_interval;
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