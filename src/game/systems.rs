use bevy::prelude::*;
use bevy::ecs::world::World;
use rand::Rng;

use crate::enemies::components::*;
use crate::game::components::*;
use crate::game::resources::*;
use crate::player::components::*;
use crate::states::*;


pub fn setup_game(
    mut commands: Commands,
    camera_query: Query<Entity, With<Camera>>,
) {
    // Reuse existing camera if available, otherwise spawn new one
    if camera_query.is_empty() {
        commands.spawn(Camera2d);
    }
    // If camera exists, we reuse it (no action needed)

    // Spawn player in the center of the screen
    commands.spawn((
        Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(20.0, 20.0)), // Green player
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        Player {
            speed: 200.0,
            health: 100.0,
            max_health: 100.0,
            regen_rate: 1.0, // 1 health per second (was 1% of 100)
            pickup_radius: 50.0, // Radius within which loot is attracted to player
        },
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
    query: Query<Entity, Or<(With<Player>, With<Rock>, With<Enemy>, With<crate::loot::components::LootItem>, With<crate::weapon::components::Weapon>, With<crate::laser::components::LaserBeam>, With<crate::experience::components::ExperienceOrb>)>>,
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

pub fn player_enemy_collision_system(
    player_transform_query: Query<&Transform, With<Player>>,
    mut player_query: Query<(Entity, &mut Player)>,
    enemy_query: Query<(&Transform, &Enemy)>,
    mut damage_timer: ResMut<PlayerDamageTimer>,
    mut screen_tint: ResMut<ScreenTintEffect>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_transform_query.single() else {
        return;
    };
    let Ok((player_entity, mut player)) = player_query.single_mut() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();

    // Check for collisions with all enemies
    let mut is_touching_enemy = false;
    for (enemy_transform, _enemy) in enemy_query.iter() {
        let enemy_pos = enemy_transform.translation.truncate();
        let distance = player_pos.distance(enemy_pos);

        // Simple collision detection - if player is close enough to enemy
        if distance < 15.0 {
            is_touching_enemy = true;
            break;
        }
    }

    // Only apply damage if touching enemy
    if !is_touching_enemy {
        // Reset timer and damage flag when not touching enemies
        damage_timer.time_since_last_damage = 0.0;
        damage_timer.has_taken_damage = false;
        return;
    }

    // Apply damage immediately on first contact or every 0.5 seconds after
    let should_damage = !damage_timer.has_taken_damage || damage_timer.time_since_last_damage >= 0.5;
    if should_damage {
        // Mark that we've taken damage
        damage_timer.has_taken_damage = true;

        // Reset timer for subsequent damage
        if damage_timer.time_since_last_damage >= 0.5 {
            damage_timer.time_since_last_damage = 0.0;
        }

        // Apply damage based on enemy strength (find the closest/touching enemy)
        for (enemy_transform, enemy) in enemy_query.iter() {
            let enemy_pos = enemy_transform.translation.truncate();
            let distance = player_pos.distance(enemy_pos);

            if distance < 15.0 {
                player.health -= enemy.strength;

                // Apply slow modifier (40% speed reduction for 3 seconds)
                commands.entity(player_entity).insert(SlowModifier {
                    remaining_duration: 3.0,
                    speed_multiplier: 0.6, // 40% reduction
                });

                // Apply red screen tint for 0.1 seconds
                screen_tint.remaining_duration = 0.1;
                screen_tint.color = Color::srgba(1.0, 0.0, 0.0, 0.15); // Red with 15% opacity

                break; // Only take damage from one enemy per tick
            }
        }
    }

    // Update damage timer after damage logic
    damage_timer.time_since_last_damage += time.delta_secs();
}

pub fn player_death_system(
    player_query: Query<&Player>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok(player) = player_query.single() {
        if player.health <= 0.0 {
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
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_player_enemy_collision_immediate_damage() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.init_resource::<Time>();

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy at (10, 0) - within collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        ));

        // Run collision system
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);

        // Player should take immediate damage
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert_eq!(player.health, 90.0, "Player should take 10 damage immediately");
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
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy far away - outside collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        ));

        // Run collision system
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);

        // Player health should remain unchanged
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert_eq!(player.health, 100.0, "Player should not take damage when not touching enemy");
    }

    #[test]
    fn test_player_enemy_collision_damage_cooldown() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.init_resource::<Time>();

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy at (10, 0) - within collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        ));

        // First damage tick - immediate
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert_eq!(player.health, 90.0, "First damage should be immediate");

        // Simulate 0.3 seconds passing (less than cooldown)
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.3;
            timer.has_taken_damage = true; // Mark that damage has been taken
        }

        // Second run - should not damage yet
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert_eq!(player.health, 90.0, "Should not damage during cooldown");

        // Simulate 0.6 seconds passing (more than cooldown)
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.6;
            timer.has_taken_damage = true; // Mark that damage has been taken
        }

        // Third run - should damage again
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert_eq!(player.health, 80.0, "Should damage after cooldown period");
    }

    #[test]
    fn test_player_death_on_zero_health() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.init_resource::<Time>();

        // Create player at (0, 0) with 100 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create enemy at (10, 0) - within collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 150.0 }, // Lethal enemy (more than player health)
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        ));

        // Run collision system - should kill player
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);

        // Player should be dead (health <= 0)
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(player.health <= 0.0, "Player health should be <= 0 after lethal damage");

        // Check that game state would transition (we can't easily test NextState in isolation)
        // but the logic should trigger the transition
    }

    #[test]
    fn test_damage_timer_reset_when_not_touching() {
        let mut app = App::new();
        app.init_resource::<PlayerDamageTimer>();
        app.init_resource::<ScreenTintEffect>();
        app.init_resource::<Time>();

        // Set timer to some value
        {
            let mut timer = app.world_mut().get_resource_mut::<PlayerDamageTimer>().unwrap();
            timer.time_since_last_damage = 0.3;
            timer.has_taken_damage = true;
        }

        // Create player at (0, 0) with 100 health
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create enemy far away - outside collision distance
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        ));

        // Run collision system - should reset timer since not touching
        let _ = app.world_mut().run_system_once(player_enemy_collision_system);

        // Timer should be reset to 0
        let timer = app.world().get_resource::<PlayerDamageTimer>().unwrap();
        assert_eq!(timer.time_since_last_damage, 0.0, "Timer should reset when not touching enemies");
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
}