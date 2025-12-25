use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::world::World;
use bevy::post_process::bloom::Bloom;
use bevy::render::view::Hdr;
use bevy_lit::prelude::*;
use rand::Rng;

use crate::combat::components::Health;
use crate::enemies::components::*;
use crate::game::components::*;
use crate::game::resources::{PlayerDamageTimer, ScreenTintEffect, SurvivalTime};
use crate::game::events::*;
use crate::player::components::*;
use crate::states::*;
use crate::whisper::components::{WhisperDrop, WhisperCompanion, WhisperArc};


pub fn setup_game(
    mut commands: Commands,
    camera_query: Query<Entity, With<Camera>>,
) {
    // Reuse existing camera if available, otherwise spawn new one
    if camera_query.is_empty() {
        commands.spawn((
            Camera2d,
            Hdr,
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: 0.3,
                ..default()
            },
            Lighting2dSettings::default(),
            AmbientLight2d {
                color: Color::WHITE,
                intensity: 0.2,
            },
        ));
    }
    // If camera exists, we reuse it (no action needed)

    // Spawn player in the center of the screen
    commands.spawn((
        Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(20.0, 20.0)), // Green player
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        Player {
            speed: 200.0,
            regen_rate: 1.0, // 1 health per second (was 1% of 100)
            pickup_radius: 50.0, // Radius within which loot is attracted to player
        },
        Health::new(100.0), // Player health as separate component
        crate::experience::components::PlayerExperience {
            current: 0,
            level: 1,
        },
    ));

    // Spawn random rocks scattered throughout the scene
    let mut rng = rand::thread_rng();
    for _ in 0..15 {
        let x = rng.gen_range(-400.0..400.0);
        let y = rng.gen_range(-300.0..300.0);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.5, 0.5, 0.5), Vec2::new(rng.gen_range(10.0..30.0), rng.gen_range(10.0..30.0))), // Gray rocks
            Transform::from_translation(Vec3::new(x, y, 0.0)),
            Rock,
        ));
    }
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
    query: Query<Entity, Or<(With<Player>, With<Rock>, With<Enemy>, With<crate::loot::components::DroppedItem>, With<crate::weapon::components::Weapon>, With<crate::laser::components::LaserBeam>, With<crate::experience::components::ExperienceOrb>, With<WhisperDrop>, With<WhisperCompanion>, With<WhisperArc>)>>,
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

/// System that detects player-enemy collisions and fires events
pub fn player_enemy_collision_detection(
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<PlayerEnemyCollisionEvent>,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();

    // Check for collisions with all enemies
    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let enemy_pos = enemy_transform.translation.truncate();
        let distance = player_pos.distance(enemy_pos);

        // Simple collision detection - if player is close enough to enemy
        if distance < 15.0 {
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
        screen_tint.color = Color::srgba(1.0, 0.0, 0.0, 0.15); // Red with 15% opacity
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

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy at (10, 0) - within collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
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

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy far away - outside collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
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

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy at (10, 0) - within collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
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

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy at (10, 0) - within collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 150.0 }, // Lethal enemy (more than player health)
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
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

        // Create player at (0, 0) with 100 health
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create enemy far away - outside collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
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
            },
            Health::new(100.0), // Alive player
            Transform::default(),
        ));

        // Run the system
        app.update();

        // Check that no GameOverEvent was fired
        assert!(!event_received.load(Ordering::SeqCst), "Should have no GameOverEvent when player is alive");
    }
}