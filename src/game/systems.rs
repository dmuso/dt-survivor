use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::world::World;
use bevy::image::{ImageLoaderSettings, ImageSampler, ImageAddressMode, ImageSamplerDescriptor};
use bevy::post_process::bloom::Bloom;
use bevy::render::view::Hdr;
use rand::Rng;

use crate::combat::components::Health;
use crate::enemies::components::*;
use crate::game::components::*;
use crate::game::resources::{DamageFlashMaterial, EnemyLevelMaterials, GameLevel, GameMaterials, GameMeshes, LevelStats, PlayerDamageTimer, ScreenTintEffect, SurvivalTime, XpOrbMaterials};
use crate::game::events::*;
use crate::movement::components::from_xz;
use crate::player::components::*;
use crate::states::*;
use crate::whisper::components::{WhisperCompanion, WhisperArc};


#[allow(clippy::too_many_arguments)]
pub fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    camera_query: Query<Entity, With<Camera>>,
    player_query: Query<Entity, With<Player>>,
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
    fresh_start: Res<crate::game::resources::FreshGameStart>,
) {
    // Reuse existing camera if available, otherwise spawn new one
    if camera_query.is_empty() {
        commands.spawn((
            Camera3d::default(),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical { viewport_height: 20.0 },
                ..OrthographicProjection::default_3d()
            }),
            Hdr,
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: 0.3,
                ..default()
            },
            // Position camera for isometric view: offset on both X and Z for diagonal angle
            Transform::from_xyz(15.0, 20.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        // Add directional light (sun)
        commands.spawn((
            DirectionalLight {
                illuminance: 250.0,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
        ));

        // Add ambient light resource
        commands.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 5.0,
            affects_lightmapped_meshes: false,
        });

        // Add ground plane with concrete texture (tiled)
        let ground_texture = asset_server.load_with_settings(
            "textures/concrete-01.png",
            |settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..default()
                });
            },
        );
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(100.0, 100.0)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(ground_texture),
                // Scale UVs to tile the texture across the ground plane
                uv_transform: bevy::math::Affine2::from_scale(Vec2::new(20.0, 20.0)),
                ..default()
            })),
            Transform::from_translation(Vec3::ZERO),
            GroundPlane,
        ));
    }
    // If camera exists, we reuse it (no action needed)

    // Only spawn player and rocks on a fresh game start (not when continuing from level complete)
    if fresh_start.0 && player_query.is_empty() {
        // Spawn player in the center of the screen (on XZ plane, Y=0 since model has its own height)
        // The 3D model will be attached as a child by the player plugin
        commands.spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Visibility::default(),
            Player {
                speed: 7.0, // 3D world units/sec (was 200 pixels/sec in 2D)
                regen_rate: 1.0, // 1 health per second
                pickup_radius: 2.0, // 3D world units (was 50 pixels in 2D)
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0), // Player health as separate component
            crate::experience::components::PlayerExperience::new(),
        ));

        // Spawn random rocks scattered throughout the scene (on XZ plane)
        let mut rng = rand::thread_rng();
        for _ in 0..15 {
            let x = rng.gen_range(-40.0..40.0);
            let z = rng.gen_range(-30.0..30.0);
            commands.spawn((
                Mesh3d(game_meshes.rock.clone()),
                MeshMaterial3d(game_materials.rock.clone()),
                Transform::from_translation(Vec3::new(x, 0.25, z)),
                Rock,
            ));
        }
    }
}

/// Sets up shared game asset resources (meshes and materials) for efficient entity spawning
pub fn setup_game_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GameMeshes::new(&mut meshes));
    commands.insert_resource(GameMaterials::new(&mut materials));
    commands.insert_resource(EnemyLevelMaterials::new(&mut materials));
    commands.insert_resource(XpOrbMaterials::new(&mut materials));

    // Create damage flash material - bright white emissive for visual feedback
    let flash_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: bevy::color::LinearRgba::WHITE * 3.0,
        unlit: true,
        ..default()
    });
    commands.insert_resource(DamageFlashMaterial(flash_material));
}

pub fn game_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Intro);
    }
}


#[allow(clippy::type_complexity)]
pub fn cleanup_game(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Player>, With<Rock>, With<Enemy>, With<crate::loot::components::DroppedItem>, With<crate::weapon::components::Weapon>, With<crate::laser::components::LaserBeam>, With<crate::experience::components::ExperienceOrb>, With<WhisperCompanion>, With<WhisperArc>, With<crate::player::components::PlayerModel>)>>,
) {
    // Don't despawn the camera - let the UI system reuse it
    // Collect entities first to avoid iterator invalidation issues
    let entities: Vec<Entity> = query.iter().collect();
    for entity in entities {
        // Use a direct world despawn to avoid command queuing issues
        commands.queue(move |world: &mut World| {
            // Only despawn if the entity still exists
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }
}

/// System that detects player-enemy collisions and fires events.
/// Uses XZ plane for collision detection in 3D space (Y axis is height).
pub fn player_enemy_collision_detection(
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<PlayerEnemyCollisionEvent>,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    // Use XZ plane for 3D collision detection
    let player_pos = from_xz(player_transform.translation);

    // Check for collisions with all enemies
    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = player_pos.distance(enemy_pos);

        // Simple collision detection - if player is close enough to enemy
        // (collision radius scaled for 3D world units)
        if distance < 1.5 {
            collision_events.write(PlayerEnemyCollisionEvent {
                player_entity,
                enemy_entity,
            });
            // Only detect one collision per frame to avoid spam
            break;
        }
    }
}

/// System that applies damage when player collides with enemies
pub fn player_enemy_damage_system(
    mut collision_events: MessageReader<PlayerEnemyCollisionEvent>,
    enemy_query: Query<&Enemy>,
    mut player_query: Query<&mut Health, With<Player>>,
    mut damage_timer: ResMut<PlayerDamageTimer>,
    time: Res<Time>,
) {
    let Ok(mut health) = player_query.single_mut() else {
        return;
    };

    let mut should_apply_damage = false;
    let mut damage_amount = 0.0;

    // Process collision events
    for event in collision_events.read() {
        if let Ok(enemy) = enemy_query.get(event.enemy_entity) {
            should_apply_damage = true;
            damage_amount = enemy.strength;
            break; // Only take damage from one enemy per frame
        }
    }

    // Apply damage with cooldown logic
    if should_apply_damage {
        let can_damage = !damage_timer.has_taken_damage || damage_timer.time_since_last_damage >= 0.5;

        if can_damage {
            health.take_damage(damage_amount);

            // Mark that we've taken damage
            damage_timer.has_taken_damage = true;

            // Reset timer for subsequent damage
            if damage_timer.time_since_last_damage >= 0.5 {
                damage_timer.time_since_last_damage = 0.0;
            }
        }
    } else {
        // Reset timer when not touching enemies
        damage_timer.time_since_last_damage = 0.0;
        damage_timer.has_taken_damage = false;
    }

    // Update damage timer
    damage_timer.time_since_last_damage += time.delta_secs();
}

/// System that applies visual effects when player takes damage
pub fn player_enemy_effect_system(
    collision_events: MessageReader<PlayerEnemyCollisionEvent>,
    mut screen_tint: ResMut<ScreenTintEffect>,
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
) {
    let Ok(player_entity) = player_query.single() else {
        return;
    };

    // Apply effects for any collision events
    if !collision_events.is_empty() {
        // Apply slow modifier (40% speed reduction for 3 seconds)
        commands.entity(player_entity).insert(SlowModifier {
            remaining_duration: 3.0,
            speed_multiplier: 0.6, // 40% reduction
        });

        // Apply red screen tint for 0.1 seconds
        screen_tint.remaining_duration = 0.1;
        screen_tint.color = Color::srgba(0.5, 0.0, 0.0, 0.05); // Dark red with 5% opacity
    }
}

/// System that updates the survival time tracker
pub fn update_survival_time(time: Res<Time>, mut survival_time: ResMut<SurvivalTime>) {
    survival_time.0 += time.delta_secs();
}

/// System that resets survival time when entering the game
pub fn reset_survival_time(mut survival_time: ResMut<SurvivalTime>) {
    survival_time.0 = 0.0;
}

pub fn player_death_system(
    player_query: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_over_events: MessageWriter<GameOverEvent>,
    score: Res<crate::score::Score>,
    survival_time: Res<SurvivalTime>,
) {
    if let Ok(health) = player_query.single() {
        if health.is_dead() {
            // Fire the game over event before state transition
            game_over_events.write(GameOverEvent {
                final_score: score.0,
                survival_time: survival_time.0,
            });
            next_state.set(GameState::GameOver);
        }
    }
}

pub fn update_screen_tint_timer(
    time: Res<Time>,
    mut screen_tint: ResMut<ScreenTintEffect>,
) {
    if screen_tint.remaining_duration > 0.0 {
        screen_tint.remaining_duration -= time.delta_secs();
    } else {
        // Reset tint when duration expires
        screen_tint.remaining_duration = 0.0;
        screen_tint.color = Color::NONE; // No tint
    }
}

/// Tracks enemy kills and advances game level internally.
/// Game level affects spawn rate and enemy difficulty but doesn't interrupt gameplay.
pub fn track_enemy_kills_system(
    mut death_events: MessageReader<EnemyDeathEvent>,
    mut game_level: ResMut<GameLevel>,
    mut level_up_writer: MessageWriter<GameLevelUpEvent>,
) {
    for _ in death_events.read() {
        if game_level.register_kill() {
            level_up_writer.write(GameLevelUpEvent {
                new_level: game_level.level,
            });
            // Game level advances silently - no screen transition
        }
    }
}

/// Reset game level on game start (only if it's a fresh game, not continuing from LevelComplete)
pub fn reset_game_level(
    mut game_level: ResMut<GameLevel>,
    mut fresh_start: ResMut<crate::game::resources::FreshGameStart>,
) {
    if fresh_start.0 {
        *game_level = GameLevel::new();
        fresh_start.0 = false; // Mark as no longer fresh
    }
}

/// Mark the next game start as fresh (called when entering Intro or GameOver)
pub fn mark_fresh_game_start(mut fresh_start: ResMut<crate::game::resources::FreshGameStart>) {
    fresh_start.0 = true;
}

/// Update level time tracking (increments time_elapsed each frame)
pub fn update_level_time_system(
    time: Res<Time>,
    mut level_stats: ResMut<LevelStats>,
) {
    level_stats.time_elapsed += time.delta_secs();
}

/// Track enemy kills for level stats (records kills from EnemyDeathEvent)
pub fn track_level_kills_system(
    mut death_events: MessageReader<EnemyDeathEvent>,
    mut level_stats: ResMut<LevelStats>,
) {
    for _ in death_events.read() {
        level_stats.record_kill();
    }
}

/// Track XP gained for level stats (records XP from ItemEffectEvent when experience is collected)
pub fn track_level_xp_system(
    mut effect_events: MessageReader<crate::loot::events::ItemEffectEvent>,
    mut level_stats: ResMut<LevelStats>,
) {
    for event in effect_events.read() {
        if let crate::loot::components::ItemData::Experience { amount } = &event.item_data {
            level_stats.record_xp(*amount);
        }
    }
}

/// Reset level stats when entering a new level or starting a game
pub fn reset_level_stats_system(mut level_stats: ResMut<LevelStats>) {
    level_stats.reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use crate::score::*;

    #[test]
    fn test_player_enemy_collision_immediate_damage() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<PlayerEnemyCollisionEvent>();
        app.add_systems(Update, (player_enemy_collision_detection, player_enemy_damage_system).chain());

        // Ensure damage timer is in correct initial state
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.0;
            timer.has_taken_damage = false;
        }

        // Create player at origin on XZ plane (Y=0.5 is entity height)
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Create enemy at (1.0, y, 0) - within collision distance (< 1.5) on XZ plane
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
        ));

        // Run the app update to process systems and events
        app.update();

        // Player should take immediate damage
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 90.0, "Player should take 10 damage immediately");
    }

    #[test]
    fn test_player_enemy_collision_no_damage_when_not_touching() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();

        // Create player at origin on XZ plane
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Create enemy far away on XZ plane - outside collision distance (> 1.5)
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
        ));

        // Run the app update to process systems and events
        app.update();

        // Player health should remain unchanged
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 100.0, "Player should not take damage when not touching enemy");
    }

    #[test]
    fn test_player_enemy_collision_damage_cooldown() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<PlayerEnemyCollisionEvent>();
        app.add_systems(Update, (player_enemy_collision_detection, player_enemy_damage_system).chain());

        // Ensure damage timer is in correct initial state
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.0;
            timer.has_taken_damage = false;
        }

        // Create player at origin on XZ plane
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Create enemy at (1.0, y, 0) - within collision distance (< 1.5) on XZ plane
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
        ));

        // First damage tick - immediate
        app.update();
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 90.0, "First damage should be immediate");

        // Simulate 0.3 seconds passing (less than cooldown)
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.3;
            timer.has_taken_damage = true; // Mark that damage has been taken
        }

        // Second run - should not damage yet
        app.update();
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 90.0, "Should not damage during cooldown");

        // Simulate 0.6 seconds passing (more than cooldown)
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.6;
            timer.has_taken_damage = true; // Mark that damage has been taken
        }

        // Third run - should damage again
        app.update();
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 80.0, "Should damage after cooldown period");
    }

    #[test]
    fn test_player_death_on_zero_health() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<PlayerEnemyCollisionEvent>();
        app.add_systems(Update, (player_enemy_collision_detection, player_enemy_damage_system).chain());

        // Ensure damage timer is in correct initial state
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.0;
            timer.has_taken_damage = false;
        }

        // Create player at origin on XZ plane
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Create enemy at (1.0, y, 0) - within collision distance (< 1.5) on XZ plane
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 150.0 }, // Lethal enemy (more than player health)
            Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
        ));

        // Run collision system - should kill player
        app.update();

        // Player should be dead (health <= 0)
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert!(health.is_dead(), "Player health should be <= 0 after lethal damage");

        // Check that game state would transition (we can't easily test NextState in isolation)
        // but the logic should trigger the transition
    }

    #[test]
    fn test_damage_timer_reset_when_not_touching() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<PlayerEnemyCollisionEvent>();
        app.add_systems(Update, (player_enemy_collision_detection, player_enemy_damage_system).chain());

        // Set timer to some value to simulate previous damage
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.3;
            timer.has_taken_damage = true;
        }

        // Create player at origin on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        // Create enemy far away on XZ plane - outside collision distance (> 1.5)
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
        ));

        // Run collision system - should reset timer since not touching
        app.update();

        // Timer should be reset (time_since_last_damage resets to 0 then adds delta_secs)
        // Since delta_secs is very small in tests, we just check has_taken_damage is false
        let timer = app.world().get_resource::<PlayerDamageTimer>().unwrap();
        assert!(!timer.has_taken_damage, "has_taken_damage should be false when not touching enemies");
    }

    #[test]
    fn test_score_resource_initialization() {
        let mut app = App::new();
        app.init_resource::<Score>();

        let score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(score.0, 0, "Score should initialize to 0");
    }

    #[test]
    fn test_score_resource_default() {
        let score = Score::default();
        assert_eq!(score.0, 0, "Default score should be 0");
    }

    #[test]
    fn test_score_increment() {
        let mut score = Score::default();
        assert_eq!(score.0, 0);

        score.0 += 1;
        assert_eq!(score.0, 1);

        score.0 += 5;
        assert_eq!(score.0, 6);
    }

    #[test]
    fn test_update_survival_time() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.init_resource::<SurvivalTime>();
        app.add_systems(Update, update_survival_time);

        // Initial survival time should be 0
        let time = app.world().get_resource::<SurvivalTime>().unwrap();
        assert_eq!(time.0, 0.0);

        // Run update (delta time will be small but > 0)
        app.update();
        let time = app.world().get_resource::<SurvivalTime>().unwrap();
        assert!(time.0 >= 0.0, "Survival time should increase or stay at 0");
    }

    #[test]
    fn test_reset_survival_time() {
        let mut app = App::new();
        app.init_resource::<SurvivalTime>();

        // Set survival time to some value
        {
            let mut time = app.world_mut().get_resource_mut::<SurvivalTime>().unwrap();
            time.0 = 120.5;
        }

        // Run reset system
        app.add_systems(Update, reset_survival_time);
        app.update();

        // Survival time should be reset to 0
        let time = app.world().get_resource::<SurvivalTime>().unwrap();
        assert_eq!(time.0, 0.0);
    }

    #[test]
    fn test_player_death_fires_game_over_event() {
        use std::sync::{Arc, atomic::{AtomicU32, AtomicBool, Ordering}};

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<Score>();
        app.init_resource::<SurvivalTime>();
        app.add_message::<GameOverEvent>();

        // Use atomics to capture event data
        let event_received = Arc::new(AtomicBool::new(false));
        let captured_score = Arc::new(AtomicU32::new(0));
        let event_received_clone = event_received.clone();
        let captured_score_clone = captured_score.clone();

        // Add systems with the producer first and consumer second, chained
        let event_reader = move |mut events: MessageReader<GameOverEvent>| {
            for event in events.read() {
                event_received_clone.store(true, Ordering::SeqCst);
                captured_score_clone.store(event.final_score, Ordering::SeqCst);
            }
        };

        app.add_systems(Update, (player_death_system, event_reader).chain());

        // Set score and survival time
        {
            let mut score = app.world_mut().get_resource_mut::<Score>().unwrap();
            score.0 = 1500;
        }
        {
            let mut time = app.world_mut().get_resource_mut::<SurvivalTime>().unwrap();
            time.0 = 90.5;
        }

        // Create dead player
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(0.0), // Dead player
            Transform::default(),
        ));

        // Run the system
        app.update();

        // Check that GameOverEvent was fired
        assert!(event_received.load(Ordering::SeqCst), "Should have received GameOverEvent");
        assert_eq!(captured_score.load(Ordering::SeqCst), 1500);
    }

    #[test]
    fn test_player_death_does_not_fire_when_alive() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<Score>();
        app.init_resource::<SurvivalTime>();
        app.add_message::<GameOverEvent>();

        // Use atomic to capture whether event was received
        let event_received = Arc::new(AtomicBool::new(false));
        let event_received_clone = event_received.clone();

        // Add systems with the producer first and consumer second, chained
        let event_reader = move |mut events: MessageReader<GameOverEvent>| {
            for _event in events.read() {
                event_received_clone.store(true, Ordering::SeqCst);
            }
        };

        app.add_systems(Update, (player_death_system, event_reader).chain());

        // Create alive player
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0), // Alive player
            Transform::default(),
        ));

        // Run the system
        app.update();

        // Check that no GameOverEvent was fired
        assert!(!event_received.load(Ordering::SeqCst), "Should have no GameOverEvent when player is alive");
    }

    #[test]
    fn test_player_enemy_collision_uses_xz_plane() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<PlayerEnemyCollisionEvent>();
        app.add_systems(Update, (player_enemy_collision_detection, player_enemy_damage_system).chain());

        // Ensure damage timer is in correct initial state
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.0;
            timer.has_taken_damage = false;
        }

        // Create player at origin on XZ plane
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Create enemy close on XZ plane but at different Y - should still collide
        // XZ distance = sqrt(0.5^2 + 0.5^2) ≈ 0.71 < 1.5
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(0.5, 100.0, 0.5)), // Far in Y but close in XZ
        ));

        app.update();

        // Collision should happen (XZ distance is small)
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 90.0, "Should take damage - Y axis is ignored for collision");
    }

    #[test]
    fn test_player_enemy_collision_on_z_axis() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<PlayerEnemyCollisionEvent>();
        app.add_systems(Update, (player_enemy_collision_detection, player_enemy_damage_system).chain());

        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.0;
            timer.has_taken_damage = false;
        }

        // Create player at origin
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Create enemy at (0, y, 1.0) - within collision distance on Z axis
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(0.0, 0.375, 1.0)),
        ));

        app.update();

        // Collision should happen on Z axis
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 90.0, "Should take damage from enemy on Z axis");
    }

    mod track_enemy_kills_tests {
        use super::*;
        use crate::game::resources::GameLevel;

        #[test]
        fn track_enemy_kills_increments_game_level_kills() {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.init_state::<GameState>();
            app.init_resource::<GameLevel>();
            app.add_message::<EnemyDeathEvent>();
            app.add_message::<GameLevelUpEvent>();
            app.add_systems(Update, track_enemy_kills_system);

            // Write a death event
            app.world_mut().write_message(EnemyDeathEvent {
                enemy_entity: Entity::PLACEHOLDER,
                position: Vec3::ZERO,
                enemy_level: 1,
            });

            app.update();

            let game_level = app.world().resource::<GameLevel>();
            assert_eq!(game_level.kills_this_level, 1);
            assert_eq!(game_level.total_kills, 1);
            assert_eq!(game_level.level, 1);
        }

        #[test]
        fn track_enemy_kills_advances_level_at_threshold() {
            use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};

            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.init_state::<GameState>();
            app.init_resource::<GameLevel>();
            app.add_message::<EnemyDeathEvent>();
            app.add_message::<GameLevelUpEvent>();

            let event_received = Arc::new(AtomicBool::new(false));
            let new_level = Arc::new(AtomicU32::new(0));
            let event_received_clone = event_received.clone();
            let new_level_clone = new_level.clone();

            let event_reader = move |mut events: MessageReader<GameLevelUpEvent>| {
                for event in events.read() {
                    event_received_clone.store(true, Ordering::SeqCst);
                    new_level_clone.store(event.new_level, Ordering::SeqCst);
                }
            };

            app.add_systems(Update, (track_enemy_kills_system, event_reader).chain());

            // Get the kills needed to advance (default is 10)
            let kills_to_advance = {
                let level = app.world().resource::<GameLevel>();
                level.kills_to_advance()
            };

            // Write enough death events to advance level
            for _ in 0..kills_to_advance {
                app.world_mut().write_message(EnemyDeathEvent {
                    enemy_entity: Entity::PLACEHOLDER,
                    position: Vec3::ZERO,
                    enemy_level: 1,
                });
                app.update();
            }

            assert!(event_received.load(Ordering::SeqCst), "Should have received GameLevelUpEvent");
            assert_eq!(new_level.load(Ordering::SeqCst), 2);

            let game_level = app.world().resource::<GameLevel>();
            assert_eq!(game_level.level, 2);
            assert_eq!(game_level.kills_this_level, 0);
        }

        #[test]
        fn track_enemy_kills_advances_level_without_state_change() {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.init_state::<GameState>();
            app.init_resource::<GameLevel>();
            app.add_message::<EnemyDeathEvent>();
            app.add_message::<GameLevelUpEvent>();
            app.add_systems(Update, track_enemy_kills_system);

            // Set game to InGame state
            app.world_mut()
                .resource_mut::<bevy::state::state::NextState<GameState>>()
                .set(GameState::InGame);
            app.update();

            // Get kills needed and set kills_this_level to one less
            {
                let mut game_level = app.world_mut().resource_mut::<GameLevel>();
                let threshold = game_level.kills_to_advance();
                game_level.kills_this_level = threshold - 1;
                game_level.total_kills = threshold - 1;
            }

            // Write the final death event to trigger level up
            app.world_mut().write_message(EnemyDeathEvent {
                enemy_entity: Entity::PLACEHOLDER,
                position: Vec3::ZERO,
                enemy_level: 1,
            });

            app.update();
            app.update();

            // Game level should have advanced
            let game_level = app.world().resource::<GameLevel>();
            assert_eq!(game_level.level, 2, "Game level should advance to 2");

            // State should remain InGame (no transition to LevelComplete)
            let current_state = app.world().resource::<bevy::state::state::State<GameState>>();
            assert_eq!(*current_state.get(), GameState::InGame, "Should stay in InGame state");
        }

        #[test]
        fn reset_game_level_resets_to_initial_state() {
            use crate::game::resources::FreshGameStart;

            let mut app = App::new();
            app.init_resource::<GameLevel>();
            app.insert_resource(FreshGameStart(true)); // Fresh start = should reset

            // Modify game level
            {
                let mut game_level = app.world_mut().resource_mut::<GameLevel>();
                game_level.level = 5;
                game_level.kills_this_level = 15;
                game_level.total_kills = 100;
            }

            app.add_systems(Update, reset_game_level);
            app.update();

            let game_level = app.world().resource::<GameLevel>();
            assert_eq!(game_level.level, 1);
            assert_eq!(game_level.kills_this_level, 0);
            assert_eq!(game_level.total_kills, 0);

            // FreshGameStart should be false after reset
            let fresh_start = app.world().resource::<FreshGameStart>();
            assert!(!fresh_start.0, "FreshGameStart should be false after reset");
        }

        #[test]
        fn reset_game_level_skips_reset_when_not_fresh() {
            use crate::game::resources::FreshGameStart;

            let mut app = App::new();
            app.init_resource::<GameLevel>();
            app.insert_resource(FreshGameStart(false)); // Not a fresh start = should NOT reset

            // Modify game level
            {
                let mut game_level = app.world_mut().resource_mut::<GameLevel>();
                game_level.level = 5;
                game_level.kills_this_level = 15;
                game_level.total_kills = 100;
            }

            app.add_systems(Update, reset_game_level);
            app.update();

            // Level should NOT be reset since FreshGameStart is false
            let game_level = app.world().resource::<GameLevel>();
            assert_eq!(game_level.level, 5, "Level should not reset when FreshGameStart is false");
            assert_eq!(game_level.kills_this_level, 15);
            assert_eq!(game_level.total_kills, 100);
        }

        #[test]
        fn track_enemy_kills_no_level_up_without_events() {
            let mut app = App::new();
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.init_state::<GameState>();
            app.init_resource::<GameLevel>();
            app.add_message::<EnemyDeathEvent>();
            app.add_message::<GameLevelUpEvent>();
            app.add_systems(Update, track_enemy_kills_system);

            app.update();

            let game_level = app.world().resource::<GameLevel>();
            assert_eq!(game_level.kills_this_level, 0);
            assert_eq!(game_level.level, 1);
        }
    }

    mod level_stats_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;
        use crate::game::resources::LevelStats;
        use crate::loot::events::ItemEffectEvent;
        use crate::loot::components::ItemData;

        #[test]
        fn update_level_time_increments_time_elapsed() {
            use std::time::Duration;

            let mut app = App::new();
            app.init_resource::<Time>();
            app.init_resource::<LevelStats>();

            // Initial time should be 0
            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.time_elapsed, 0.0);

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            // Run system once
            let _ = app.world_mut().run_system_once(update_level_time_system);

            let stats = app.world().resource::<LevelStats>();
            assert!((stats.time_elapsed - 0.5).abs() < 0.001, "Time should have elapsed by 0.5 seconds");
        }

        #[test]
        fn track_level_kills_increments_on_enemy_death() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();
            app.add_message::<EnemyDeathEvent>();
            app.add_systems(Update, track_level_kills_system);

            // Initial kills should be 0
            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.enemies_killed, 0);

            // Write a death event
            app.world_mut().write_message(EnemyDeathEvent {
                enemy_entity: Entity::PLACEHOLDER,
                position: Vec3::ZERO,
                enemy_level: 1,
            });
            app.update();

            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.enemies_killed, 1);
        }

        #[test]
        fn track_level_kills_handles_multiple_deaths() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();
            app.add_message::<EnemyDeathEvent>();
            app.add_systems(Update, track_level_kills_system);

            // Write multiple death events
            for _ in 0..5 {
                app.world_mut().write_message(EnemyDeathEvent {
                    enemy_entity: Entity::PLACEHOLDER,
                    position: Vec3::ZERO,
                    enemy_level: 1,
                });
            }
            app.update();

            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.enemies_killed, 5);
        }

        #[test]
        fn track_level_xp_increments_on_experience_pickup() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();
            app.add_message::<ItemEffectEvent>();
            app.add_systems(Update, track_level_xp_system);

            // Initial XP should be 0
            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.xp_gained, 0);

            // Write an experience pickup event
            app.world_mut().write_message(ItemEffectEvent {
                item_entity: Entity::PLACEHOLDER,
                item_data: ItemData::Experience { amount: 50 },
                player_entity: Entity::PLACEHOLDER,
            });
            app.update();

            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.xp_gained, 50);
        }

        #[test]
        fn track_level_xp_handles_multiple_pickups() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();
            app.add_message::<ItemEffectEvent>();
            app.add_systems(Update, track_level_xp_system);

            // Write multiple experience pickup events
            app.world_mut().write_message(ItemEffectEvent {
                item_entity: Entity::PLACEHOLDER,
                item_data: ItemData::Experience { amount: 5 },
                player_entity: Entity::PLACEHOLDER,
            });
            app.world_mut().write_message(ItemEffectEvent {
                item_entity: Entity::PLACEHOLDER,
                item_data: ItemData::Experience { amount: 15 },
                player_entity: Entity::PLACEHOLDER,
            });
            app.world_mut().write_message(ItemEffectEvent {
                item_entity: Entity::PLACEHOLDER,
                item_data: ItemData::Experience { amount: 35 },
                player_entity: Entity::PLACEHOLDER,
            });
            app.update();

            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.xp_gained, 55);
        }

        #[test]
        fn track_level_xp_ignores_non_experience_items() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();
            app.add_message::<ItemEffectEvent>();
            app.add_systems(Update, track_level_xp_system);

            // Write a health pack pickup event (not experience)
            app.world_mut().write_message(ItemEffectEvent {
                item_entity: Entity::PLACEHOLDER,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                player_entity: Entity::PLACEHOLDER,
            });
            app.update();

            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.xp_gained, 0, "Health pack should not add XP");
        }

        #[test]
        fn reset_level_stats_resets_all_values() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();

            // Set some values
            {
                let mut stats = app.world_mut().resource_mut::<LevelStats>();
                stats.time_elapsed = 120.5;
                stats.enemies_killed = 50;
                stats.xp_gained = 1000;
            }

            app.add_systems(Update, reset_level_stats_system);
            app.update();

            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.time_elapsed, 0.0);
            assert_eq!(stats.enemies_killed, 0);
            assert_eq!(stats.xp_gained, 0);
        }

        #[test]
        fn level_stats_systems_work_together() {
            use std::time::Duration;

            let mut app = App::new();
            app.init_resource::<Time>();
            app.init_resource::<LevelStats>();
            app.add_message::<EnemyDeathEvent>();
            app.add_message::<ItemEffectEvent>();

            // Simulate some gameplay: advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            // Run time update system
            let _ = app.world_mut().run_system_once(update_level_time_system);

            // Write events and run kill/xp tracking systems
            app.world_mut().write_message(EnemyDeathEvent {
                enemy_entity: Entity::PLACEHOLDER,
                position: Vec3::ZERO,
                enemy_level: 1,
            });
            let _ = app.world_mut().run_system_once(track_level_kills_system);

            app.world_mut().write_message(ItemEffectEvent {
                item_entity: Entity::PLACEHOLDER,
                item_data: ItemData::Experience { amount: 15 },
                player_entity: Entity::PLACEHOLDER,
            });
            let _ = app.world_mut().run_system_once(track_level_xp_system);

            let stats = app.world().resource::<LevelStats>();
            assert!((stats.time_elapsed - 1.0).abs() < 0.001);
            assert_eq!(stats.enemies_killed, 1);
            assert_eq!(stats.xp_gained, 15);
        }
    }
}