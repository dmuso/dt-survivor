use bevy::prelude::*;
use rand::Rng;

use crate::combat::{CheckDeath, Health};
use crate::enemies::components::*;
use crate::game::resources::*;
use crate::player::components::*;

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

            // Spawn enemy with Health component for damage system
            commands.spawn((
                Sprite::from_color(Color::srgb(1.0, 0.0, 0.0), Vec2::new(15.0, 15.0)), // Red enemy
                Transform::from_translation(Vec3::new(spawn_pos.x, spawn_pos.y, 0.5)),
                Enemy { speed: 50.0, strength: 10.0 }, // Slower than player (player is 200.0), moderate strength
                Health::new(10.0), // Enemies have 10 HP
                CheckDeath, // Enable death checking via combat system
            ));
        }

        // Reset the spawn timer (subtract the time we've accounted for)
        spawn_state.time_since_last_spawn -= enemies_to_spawn as f32 * spawn_interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CheckDeath, Health};

    #[test]
    fn test_enemy_spawns_with_health_component() {
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.init_resource::<EnemySpawnState>();

        // Spawn a player
        app.world_mut().spawn((
            Transform::from_translation(Vec3::ZERO),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
        ));

        // Spawn a camera
        app.world_mut().spawn((
            Camera2d::default(),
            GlobalTransform::default(),
        ));

        // Force a spawn by setting high spawn rate and time
        {
            let mut spawn_state = app.world_mut().get_resource_mut::<EnemySpawnState>().unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Find spawned enemies and verify they have Health and CheckDeath components
        let mut enemy_count = 0;
        let mut query = app.world_mut().query::<(Entity, &Enemy)>();
        for (entity, _enemy) in query.iter(app.world()) {
            enemy_count += 1;
            assert!(
                app.world().get::<Health>(entity).is_some(),
                "Enemy should have Health component"
            );
            assert!(
                app.world().get::<CheckDeath>(entity).is_some(),
                "Enemy should have CheckDeath component"
            );
        }

        assert!(enemy_count > 0, "At least one enemy should have spawned");
    }
}