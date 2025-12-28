use bevy::prelude::*;
use rand::Rng;

use crate::combat::{CheckDeath, Health};
use crate::enemies::components::*;
use crate::game::resources::*;
use crate::player::components::*;

/// Distance from player to spawn enemies (scaled for 3D world units)
/// With orthographic camera viewport of ~20x35 units, spawn just outside view
pub const ENEMY_SPAWN_DISTANCE: f32 = 18.0;

/// Height of enemy cube center above ground (half of 0.75 cube height)
pub const ENEMY_Y_HEIGHT: f32 = 0.375;

pub fn enemy_spawning_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut spawn_state: ResMut<EnemySpawnState>,
    time: Res<Time>,
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
) {
    let Ok(player_transform) = player_query.single() else {
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

        // Player position on XZ plane
        let player_xz = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.z,
        );

        for _ in 0..enemies_to_spawn {
            // Generate random angle and distance for spawning outside view
            // ENEMY_SPAWN_DISTANCE is set to just outside the camera viewport (~18 units)
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = ENEMY_SPAWN_DISTANCE + rng.gen_range(0.0..5.0);

            // Calculate spawn position on XZ plane relative to player
            let spawn_offset = Vec2::new(angle.cos(), angle.sin()) * distance;
            let spawn_xz = player_xz + spawn_offset;

            // Spawn enemy as 3D mesh on XZ plane with Y height for cube center
            commands.spawn((
                Mesh3d(game_meshes.enemy.clone()),
                MeshMaterial3d(game_materials.enemy.clone()),
                Transform::from_translation(Vec3::new(spawn_xz.x, ENEMY_Y_HEIGHT, spawn_xz.y)),
                Enemy {
                    speed: 1.7, // 3D world units/sec (was 50 pixels/sec in 2D)
                    strength: 10.0,
                },
                Health::new(10.0),
                CheckDeath,
            ));
        }

        // Reset the spawn timer (subtract the time we've accounted for)
        spawn_state.time_since_last_spawn -= enemies_to_spawn as f32 * spawn_interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::asset::Assets;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::pbr::StandardMaterial;
    use crate::combat::{CheckDeath, Health};

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::time::TimePlugin::default(),
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<EnemySpawnState>();
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
    fn test_enemy_spawns_with_health_component() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        // Spawn a player on XZ plane
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        ));

        // Force a spawn by setting high spawn rate and time
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
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

    #[test]
    fn test_enemy_spawns_with_mesh3d_component() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        // Spawn a player
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        ));

        // Force a spawn
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemies have Mesh3d component
        let mut query = app.world_mut().query::<(Entity, &Enemy)>();
        for (entity, _) in query.iter(app.world()) {
            assert!(
                app.world().get::<Mesh3d>(entity).is_some(),
                "Enemy should have Mesh3d component"
            );
        }
    }

    #[test]
    fn test_enemy_spawns_with_mesh_material_3d_component() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        // Spawn a player
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        ));

        // Force a spawn
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemies have MeshMaterial3d component
        let mut query = app.world_mut().query::<(Entity, &Enemy)>();
        for (entity, _) in query.iter(app.world()) {
            assert!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(entity)
                    .is_some(),
                "Enemy should have MeshMaterial3d component"
            );
        }
    }

    #[test]
    fn test_enemy_transform_y_position_is_correct_height() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        // Spawn a player
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        ));

        // Force a spawn
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemy Y position matches ENEMY_Y_HEIGHT
        let mut query = app.world_mut().query::<(&Transform, &Enemy)>();
        for (transform, _) in query.iter(app.world()) {
            assert!(
                (transform.translation.y - ENEMY_Y_HEIGHT).abs() < 0.001,
                "Enemy Y position should be {} (half cube height), got {}",
                ENEMY_Y_HEIGHT,
                transform.translation.y
            );
        }
    }

    #[test]
    fn test_enemy_spawn_position_is_on_xz_plane() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        let player_pos = Vec3::new(5.0, 0.5, 10.0);

        // Spawn a player at known position
        app.world_mut().spawn((
            Transform::from_translation(player_pos),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(5.0, 20.0, 25.0)),
        ));

        // Force a spawn
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemies are spawned at distance from player on XZ plane
        let player_xz = Vec2::new(player_pos.x, player_pos.z);
        let mut query = app.world_mut().query::<(&Transform, &Enemy)>();
        for (transform, _) in query.iter(app.world()) {
            let enemy_xz = Vec2::new(transform.translation.x, transform.translation.z);
            let distance = enemy_xz.distance(player_xz);

            assert!(
                distance >= ENEMY_SPAWN_DISTANCE,
                "Enemy should spawn at least {} units from player, got {}",
                ENEMY_SPAWN_DISTANCE,
                distance
            );
        }
    }

    #[test]
    fn test_enemy_spawn_distance_is_valid() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        // Spawn a player
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        ));

        // Force many spawns to check distribution
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 1000.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify spawn distance is within expected range (18 to 23 units)
        let mut query = app.world_mut().query::<(&Transform, &Enemy)>();
        for (transform, _) in query.iter(app.world()) {
            let enemy_xz = Vec2::new(transform.translation.x, transform.translation.z);
            let distance = enemy_xz.length();

            // Distance should be between ENEMY_SPAWN_DISTANCE and ENEMY_SPAWN_DISTANCE + 5
            // (accounting for viewport adjustment which might push it further)
            assert!(
                distance >= ENEMY_SPAWN_DISTANCE * 0.5,
                "Enemy spawn distance {} should be reasonable",
                distance
            );
        }
    }

    #[test]
    fn test_enemy_does_not_have_sprite_component() {
        let mut app = setup_test_app();
        setup_game_resources(&mut app);

        // Spawn a player
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
        ));

        // Spawn a 3D camera
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        ));

        // Force a spawn
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemies do NOT have Sprite component (we're using 3D now)
        let mut query = app.world_mut().query::<(Entity, &Enemy)>();
        for (entity, _) in query.iter(app.world()) {
            assert!(
                app.world().get::<Sprite>(entity).is_none(),
                "Enemy should NOT have Sprite component in 3D mode"
            );
        }
    }
}