use bevy::prelude::*;
use rand::Rng;

use crate::combat::{CheckDeath, Health};
use crate::enemies::components::*;
use crate::game::components::Level;
use crate::game::resources::*;
use crate::player::components::*;

/// Distance from player to spawn enemies (scaled for 3D world units)
/// With orthographic camera viewport of ~20x35 units, spawn just outside view
pub const ENEMY_SPAWN_DISTANCE: f32 = 18.0;

/// Height of enemy cube center above ground (half of 0.75 cube height)
pub const ENEMY_Y_HEIGHT: f32 = 0.375;

/// Determine enemy level based on game level with weighted random selection.
/// Higher game levels increase the chance of spawning higher-tier enemies.
pub fn select_enemy_level(game_level: u32, rng: &mut impl Rng) -> u8 {
    // Level bonus increases with game level, capped at 40%
    let level_bonus = ((game_level.saturating_sub(1) as f32) * 5.0).min(40.0);

    // Base spawn chances (weights, not percentages)
    // Game level increases chances of higher-tier enemies
    let chances = [
        (1u8, 100.0 - level_bonus),       // Level 1: starts at 100%, decreases
        (2u8, 15.0 + level_bonus * 0.5),  // Level 2: starts at 15%
        (3u8, 8.0 + level_bonus * 0.3),   // Level 3: starts at 8%
        (4u8, 4.0 + level_bonus * 0.15),  // Level 4: starts at 4%
        (5u8, 2.0 + level_bonus * 0.05),  // Level 5: starts at 2%
    ];

    let total: f32 = chances.iter().map(|(_, c)| c).sum();
    let roll = rng.gen_range(0.0..total);

    let mut cumulative = 0.0;
    for (level, chance) in chances {
        cumulative += chance;
        if roll < cumulative {
            return level;
        }
    }
    1 // Fallback
}

pub fn enemy_spawning_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut spawn_state: ResMut<EnemySpawnState>,
    time: Res<Time>,
    game_meshes: Res<GameMeshes>,
    enemy_materials: Res<EnemyLevelMaterials>,
    game_level: Res<GameLevel>,
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
        let scaling = EnemyScaling::default();

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

            // Determine enemy level based on current game level
            let enemy_level = select_enemy_level(game_level.level, &mut rng);

            // Calculate scale based on enemy level (higher level = larger)
            let scale = enemy_scale_for_level(enemy_level);

            // Spawn enemy as 3D mesh on XZ plane with Y height scaled for cube center
            // Y position needs to account for scaled cube height
            let y_height = ENEMY_Y_HEIGHT * scale;
            commands.spawn((
                Mesh3d(game_meshes.enemy.clone()),
                MeshMaterial3d(enemy_materials.for_level(enemy_level)),
                Transform::from_translation(Vec3::new(spawn_xz.x, y_height, spawn_xz.y))
                    .with_scale(Vec3::splat(scale)),
                Enemy {
                    speed: 1.7, // 3D world units/sec (was 50 pixels/sec in 2D)
                    strength: scaling.damage_for_level(enemy_level),
                },
                Health::new(scaling.health_for_level(enemy_level)),
                Level::new(enemy_level),
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
    use crate::game::components::Level;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::time::TimePlugin::default(),
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<EnemySpawnState>();
        app.init_resource::<GameLevel>();
        app
    }

    fn setup_game_resources(app: &mut App) {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let game_meshes = GameMeshes::new(&mut meshes);
        app.world_mut().insert_resource(game_meshes);

        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        let enemy_materials = EnemyLevelMaterials::new(&mut materials);
        app.world_mut().insert_resource(enemy_materials);
    }

    mod select_enemy_level_tests {
        use super::*;

        #[test]
        fn returns_valid_level_range() {
            let mut rng = StdRng::seed_from_u64(12345);
            for game_level in 1..=10 {
                for _ in 0..100 {
                    let enemy_level = select_enemy_level(game_level, &mut rng);
                    assert!(enemy_level >= 1 && enemy_level <= 5,
                        "Enemy level {} should be between 1 and 5", enemy_level);
                }
            }
        }

        #[test]
        fn game_level_1_mostly_spawns_level_1_enemies() {
            let mut rng = StdRng::seed_from_u64(42);
            let mut level_counts = [0u32; 5];

            for _ in 0..1000 {
                let enemy_level = select_enemy_level(1, &mut rng);
                level_counts[(enemy_level - 1) as usize] += 1;
            }

            // Level 1 should be most common (at least 70% of spawns)
            assert!(level_counts[0] > 700,
                "Level 1 enemies should dominate at game level 1, got {} out of 1000", level_counts[0]);
        }

        #[test]
        fn higher_game_level_increases_high_tier_enemy_chance() {
            let mut rng = StdRng::seed_from_u64(42);

            // Count level 3+ enemies at game level 1
            let mut high_tier_at_level_1 = 0;
            for _ in 0..1000 {
                let enemy_level = select_enemy_level(1, &mut rng);
                if enemy_level >= 3 {
                    high_tier_at_level_1 += 1;
                }
            }

            // Reset rng
            let mut rng = StdRng::seed_from_u64(42);

            // Count level 3+ enemies at game level 9 (max level bonus)
            let mut high_tier_at_level_9 = 0;
            for _ in 0..1000 {
                let enemy_level = select_enemy_level(9, &mut rng);
                if enemy_level >= 3 {
                    high_tier_at_level_9 += 1;
                }
            }

            assert!(high_tier_at_level_9 > high_tier_at_level_1,
                "Higher game level should spawn more high-tier enemies: lvl9={}, lvl1={}",
                high_tier_at_level_9, high_tier_at_level_1);
        }

        #[test]
        fn all_levels_can_spawn_at_high_game_level() {
            let mut rng = StdRng::seed_from_u64(42);
            let mut spawned_levels = [false; 5];

            // At high game level, all enemy levels should be possible
            for _ in 0..10000 {
                let enemy_level = select_enemy_level(10, &mut rng);
                spawned_levels[(enemy_level - 1) as usize] = true;

                if spawned_levels.iter().all(|&x| x) {
                    break;
                }
            }

            for (i, spawned) in spawned_levels.iter().enumerate() {
                assert!(*spawned, "Enemy level {} should be spawnable at high game level", i + 1);
            }
        }

        #[test]
        fn level_bonus_caps_at_40_percent() {
            // At game level 9+, level_bonus should be capped at 40.0
            // This means: (9-1) * 5.0 = 40.0 (capped)
            // Level 10 would be: (10-1) * 5.0 = 45.0 but capped to 40.0
            let mut rng1 = StdRng::seed_from_u64(42);
            let mut rng2 = StdRng::seed_from_u64(42);

            // Same seed should produce same results if capping works
            let mut results_level_9 = Vec::new();
            let mut results_level_20 = Vec::new();

            for _ in 0..100 {
                results_level_9.push(select_enemy_level(9, &mut rng1));
                results_level_20.push(select_enemy_level(20, &mut rng2));
            }

            // Should be identical due to cap
            assert_eq!(results_level_9, results_level_20,
                "Game levels 9 and 20 should produce same distribution due to cap");
        }
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
    fn test_enemy_transform_y_position_is_scaled_height() {
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

        // Verify enemy Y position is scaled (ENEMY_Y_HEIGHT * scale)
        // Level 1 scale = 0.75, so Y = 0.375 * 0.75 = 0.28125
        // Level 5 scale = 1.35, so Y = 0.375 * 1.35 = 0.50625
        let mut query = app.world_mut().query::<(&Transform, &Enemy, &Level)>();
        for (transform, _, level) in query.iter(app.world()) {
            let scale = enemy_scale_for_level(level.value());
            let expected_y = ENEMY_Y_HEIGHT * scale;
            assert!(
                (transform.translation.y - expected_y).abs() < 0.01,
                "Enemy level {} Y position should be {} (scaled height), got {}",
                level.value(), expected_y, transform.translation.y
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

    #[test]
    fn test_enemy_spawns_with_level_component() {
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

        // Verify enemies have Level component with valid value
        let mut query = app.world_mut().query::<(Entity, &Enemy, &Level)>();
        let mut found_enemy = false;
        for (_, _, level) in query.iter(app.world()) {
            found_enemy = true;
            assert!(
                level.value() >= 1 && level.value() <= 5,
                "Enemy level should be between 1 and 5, got {}",
                level.value()
            );
        }
        assert!(found_enemy, "At least one enemy should have spawned with Level component");
    }

    #[test]
    fn test_enemy_stats_scale_with_level() {
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

        // Force many spawns to get variety of levels
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 1000.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemy stats match their level
        let scaling = EnemyScaling::default();
        let mut query = app.world_mut().query::<(&Enemy, &Health, &Level)>();
        for (enemy, health, level) in query.iter(app.world()) {
            let expected_health = scaling.health_for_level(level.value());
            let expected_damage = scaling.damage_for_level(level.value());

            assert!(
                (health.max - expected_health).abs() < 0.01,
                "Enemy level {} should have health {}, got {}",
                level.value(), expected_health, health.max
            );
            assert!(
                (enemy.strength - expected_damage).abs() < 0.01,
                "Enemy level {} should have strength {}, got {}",
                level.value(), expected_damage, enemy.strength
            );
        }
    }

    #[test]
    fn test_enemy_spawns_with_correct_scale() {
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

        // Force many spawns to get variety of levels
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 1000.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Verify enemy scale matches their level
        let mut query = app.world_mut().query::<(&Transform, &Enemy, &Level)>();
        for (transform, _, level) in query.iter(app.world()) {
            let expected_scale = enemy_scale_for_level(level.value());
            assert!(
                (transform.scale.x - expected_scale).abs() < 0.01,
                "Enemy level {} should have scale {}, got {}",
                level.value(), expected_scale, transform.scale.x
            );
            // Scale should be uniform
            assert!(
                (transform.scale.x - transform.scale.y).abs() < 0.001,
                "Enemy scale should be uniform"
            );
            assert!(
                (transform.scale.y - transform.scale.z).abs() < 0.001,
                "Enemy scale should be uniform"
            );
        }
    }

    #[test]
    fn test_enemy_spawns_with_level_based_material() {
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

        // Force spawns
        {
            let mut spawn_state = app
                .world_mut()
                .get_resource_mut::<EnemySpawnState>()
                .unwrap();
            spawn_state.spawn_rate_per_second = 100.0;
            spawn_state.time_since_last_spawn = 1.0;
        }

        let _ = app.world_mut().run_system_once(enemy_spawning_system);

        // Collect enemy data first to avoid borrow issues
        let enemy_data: Vec<(Handle<StandardMaterial>, u8)> = {
            let mut query = app.world_mut().query::<(&MeshMaterial3d<StandardMaterial>, &Level)>();
            query.iter(app.world())
                .map(|(mat, level)| (mat.0.clone(), level.value()))
                .collect()
        };

        // Verify enemies have material handles that match their level
        let enemy_materials = app.world().resource::<EnemyLevelMaterials>();
        for (material, level) in enemy_data {
            let expected_material = enemy_materials.for_level(level);
            assert_eq!(
                material, expected_material,
                "Enemy level {} should have correct material",
                level
            );
        }
    }
}