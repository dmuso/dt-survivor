use bevy::prelude::*;
use rand::Rng;
use crate::loot::components::*;
use crate::weapon::components::{Weapon, WeaponType};
use crate::player::components::*;
use crate::inventory::resources::*;
use crate::inventory::components::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::game::resources::ScreenTintEffect;
use crate::game::events::EnemyDeathEvent;

pub fn loot_spawning_system(
    mut commands: Commands,
    time: Res<Time>,
    mut last_spawn_time: Local<f32>,
) {
    let current_time = time.elapsed_secs();
    let spawn_interval = 30.0; // Spawn random loot every 30 seconds

    // Random world spawning
    if current_time - *last_spawn_time >= spawn_interval {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(-400.0..400.0);
        let y = rng.gen_range(-300.0..300.0);

        // 50% chance for weapon, 50% for health pack
        if rng.gen_bool(0.5) {
            // 50% chance for pistol, 50% for laser
            let weapon = if rng.gen_bool(0.5) {
                Weapon {
                    weapon_type: WeaponType::Pistol {
                        bullet_count: 5,
                        spread_angle: 15.0,
                    },
                    level: 1, // Will be ignored when picked up
                    fire_rate: 2.0,
                    base_damage: 1.0,
                    last_fired: 0.0,
                }
            } else {
                Weapon {
                    weapon_type: WeaponType::Laser,
                    level: 1, // Will be ignored when picked up
                    fire_rate: 3.0,
                    base_damage: 15.0,
                    last_fired: 0.0,
                }
            };

            let color = match weapon.weapon_type {
                WeaponType::Pistol { .. } => Color::srgb(1.0, 1.0, 0.0), // Yellow for pistol
                WeaponType::Laser => Color::srgb(0.0, 0.0, 1.0), // Blue for laser
                _ => Color::srgb(0.5, 0.5, 0.5),
            };

            commands.spawn((
                Sprite::from_color(color, Vec2::new(16.0, 16.0)),
                Transform::from_translation(Vec3::new(x, y, 0.5)),
                LootItem {
                    loot_type: LootType::Weapon(weapon),
                    velocity: Vec2::ZERO,
                },
            ));
        } else {
            // Spawn health pack
            commands.spawn((
                Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(12.0, 12.0)), // Green for health pack
                Transform::from_translation(Vec3::new(x, y, 0.5)),
                LootItem {
                    loot_type: LootType::HealthPack { heal_amount: 25.0 },
                    velocity: Vec2::ZERO,
                },
            ));
        }

        *last_spawn_time = current_time;
    }
}

pub fn loot_attraction_system(
    player_query: Query<(&Transform, &Player), With<Player>>,
    mut loot_query: Query<(&Transform, &mut LootItem), Without<Player>>,
    mut orb_query: Query<(&Transform, &mut crate::experience::components::ExperienceOrb), Without<Player>>,
) {
    // Get player data
    if let Ok((player_transform, player)) = player_query.single() {
        let player_pos = player_transform.translation.truncate();

        // Process loot items - apply magnetic attraction
        for (loot_transform, mut loot_item) in loot_query.iter_mut() {
            let loot_pos = loot_transform.translation.truncate();
            let distance = player_pos.distance(loot_pos);

            // Distance-based acceleration and steering
            // Closer to player = faster acceleration and stronger homing
            let max_distance = player.pickup_radius;
            let distance_ratio = (distance / max_distance).max(0.1).min(1.0); // Clamp between 0.1 and 1.0
            let acceleration_multiplier = 1.0 / distance_ratio; // Closer = higher multiplier

            // Base acceleration scales with distance (closer = faster)
            let base_acceleration = 600.0; // Slightly slower than orbs for balance
            let acceleration = base_acceleration * acceleration_multiplier;

            // Steering strength also scales with distance (closer = stronger steering)
            let base_steering = 900.0; // Slightly slower than orbs for balance
            let steering_strength = base_steering * acceleration_multiplier;

            // Apply magnetic attraction when in range
            if distance <= player.pickup_radius && distance > 5.0 {
                let direction_to_player = (player_pos - loot_pos).normalize();
                loot_item.velocity += direction_to_player * acceleration;

                // Apply steering to correct direction
                let current_speed = loot_item.velocity.length();
                if current_speed > 0.1 {
                    let desired_velocity = direction_to_player * current_speed;
                    let steering_vector = desired_velocity - loot_item.velocity;

                    // Limit steering based on distance
                    let max_steering = steering_strength;
                    let steering_magnitude = steering_vector.length();
                    let clamped_steering = if steering_magnitude > max_steering {
                        steering_vector.normalize() * max_steering
                    } else {
                        steering_vector
                    };

                    loot_item.velocity += clamped_steering;
                }
            }
        }

        // Process experience orbs - apply magnetic attraction
        for (orb_transform, mut orb) in orb_query.iter_mut() {
            let orb_pos = orb_transform.translation.truncate();
            let distance = player_pos.distance(orb_pos);

            // Distance-based acceleration and steering
            // Closer to player = faster acceleration and stronger homing
            let max_distance = player.pickup_radius;
            let distance_ratio = (distance / max_distance).max(0.1).min(1.0); // Clamp between 0.1 and 1.0
            let acceleration_multiplier = 1.0 / distance_ratio; // Closer = higher multiplier

            // Base acceleration scales with distance (closer = faster)
            let base_acceleration = 800.0;
            let acceleration = base_acceleration * acceleration_multiplier;

            // Steering strength also scales with distance (closer = stronger steering)
            let base_steering = 1200.0;
            let steering_strength = base_steering * acceleration_multiplier;

            // Always apply acceleration towards player when in range
            if distance <= player.pickup_radius && distance > 5.0 {
                orb.velocity += (player_pos - orb_pos).normalize() * acceleration;

                // Apply steering to correct direction
                let current_speed = orb.velocity.length();
                if current_speed > 0.1 {
                    let desired_velocity = (player_pos - orb_pos).normalize() * current_speed;
                    let steering_vector = desired_velocity - orb.velocity;

                    // Limit steering based on distance
                    let max_steering = steering_strength;
                    let steering_magnitude = steering_vector.length();
                    let clamped_steering = if steering_magnitude > max_steering {
                        steering_vector.normalize() * max_steering
                    } else {
                        steering_vector
                    };

                    orb.velocity += clamped_steering;
                }
            }
        }
    }
}

pub fn loot_movement_system(
    time: Res<Time>,
    mut loot_query: Query<(&mut Transform, &LootItem)>,
) {
    // Update loot item positions based on velocity
    for (mut transform, loot_item) in loot_query.iter_mut() {
        let movement = loot_item.velocity * time.delta_secs();
        transform.translation += movement.extend(0.0);
    }
}

pub fn enemy_death_system(
    mut commands: Commands,
    mut enemy_death_events: MessageReader<EnemyDeathEvent>,
) {
    for event in enemy_death_events.read() {
        let enemy_pos = event.position;

        // Spawn experience orbs for each enemy killed
        let mut rng = rand::thread_rng();
        let orb_count = rng.gen_range(1..=3);

        for _ in 0..orb_count {
            let value = rng.gen_range(5..=15); // 5-15 experience per orb
            let offset_x = rng.gen_range(-10.0..=10.0);
            let offset_y = rng.gen_range(-10.0..=10.0);

            commands.spawn((
                Sprite::from_color(
                    Color::srgb(0.75, 0.75, 0.75), // Light grey color
                    Vec2::new(8.0, 8.0) // Same size as bullets
                ),
                Transform::from_translation(
                    Vec3::new(enemy_pos.x + offset_x, enemy_pos.y + offset_y, 0.2)
                ),
                    crate::experience::components::ExperienceOrb {
                        value,
                        velocity: Vec2::ZERO, // Start with no velocity
                    },
            ));
        }

        // Spawn loot drops with specified probabilities
        let mut loot_drops = Vec::new();

        // 1% chance to drop rocket launcher
        if rng.gen_bool(0.01) {
            loot_drops.push(LootType::Weapon(Weapon {
                weapon_type: WeaponType::RocketLauncher,
                level: 1,
                fire_rate: 1.0,
                base_damage: 50.0,
                last_fired: 0.0,
            }));
        }

        // 2% chance to drop laser
        if rng.gen_bool(0.02) {
            loot_drops.push(LootType::Weapon(Weapon {
                weapon_type: WeaponType::Laser,
                level: 1,
                fire_rate: 3.0,
                base_damage: 15.0,
                last_fired: 0.0,
            }));
        }

        // 3% chance to drop pistol
        if rng.gen_bool(0.03) {
            loot_drops.push(LootType::Weapon(Weapon {
                weapon_type: WeaponType::Pistol {
                    bullet_count: 5,
                    spread_angle: 15.0,
                },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: 0.0,
            }));
        }

        // 3% chance to drop health regen (health pack)
        if rng.gen_bool(0.03) {
            loot_drops.push(LootType::HealthPack { heal_amount: 25.0 });
        }

        // Spawn loot items spaced out around the enemy position
        let spacing = 20.0; // Distance between drops
        for (i, loot_type) in loot_drops.into_iter().enumerate() {
            let angle = (i as f32) * std::f32::consts::TAU / 4.0; // Space items in a circle
            let offset_x = angle.cos() * spacing;
            let offset_y = angle.sin() * spacing;

            let (color, size) = match &loot_type {
                LootType::Weapon(weapon) => {
                    let color = match weapon.weapon_type {
                        WeaponType::Pistol { .. } => Color::srgb(1.0, 1.0, 0.0), // Yellow for pistol
                        WeaponType::Laser => Color::srgb(0.0, 0.0, 1.0), // Blue for laser
                        WeaponType::RocketLauncher => Color::srgb(1.0, 0.5, 0.0), // Orange for rocket launcher
                        _ => Color::srgb(0.5, 0.5, 0.5),
                    };
                    (color, Vec2::new(16.0, 16.0))
                }
                LootType::HealthPack { .. } => (Color::srgb(0.0, 1.0, 0.0), Vec2::new(12.0, 12.0)), // Green for health pack
            };

            commands.spawn((
                Sprite::from_color(color, size),
                Transform::from_translation(Vec3::new(
                    enemy_pos.x + offset_x,
                    enemy_pos.y + offset_y,
                    0.5
                )),
                LootItem {
                    loot_type,
                    velocity: Vec2::ZERO,
                },
            ));
        }
    }
}

// This system is now handled in the enemy_death_system

#[allow(clippy::too_many_arguments)]
pub fn player_loot_collision_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    loot_query: Query<(Entity, &Transform, &LootItem)>,
    weapon_query: Query<(Entity, &Weapon)>,
    mut inventory: ResMut<Inventory>,
    mut player_query_mut: Query<(Entity, &mut Player)>,
    mut screen_tint: ResMut<ScreenTintEffect>,
    asset_server: Option<Res<AssetServer>>,
    mut loot_channel: Option<ResMut<AudioChannel<LootSoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_pos = player_transform.translation.truncate();

        for (loot_entity, loot_transform, loot_item) in loot_query.iter() {
            let loot_pos = loot_transform.translation.truncate();
            let distance = player_pos.distance(loot_pos);

            // Collision detection - same as player-enemy collision
            if distance < 15.0 {
                match &loot_item.loot_type {
                    LootType::Weapon(weapon) => {
                        // Try to add or level up the weapon
                        if inventory.add_or_level_weapon(weapon.clone()) {
                            // Successfully added or leveled up weapon
                            // Recreate all weapon entities to reflect changes
                            // First, despawn existing weapon entities
                            let weapon_entities: Vec<Entity> = weapon_query.iter().map(|(entity, _)| entity).collect();
                            for entity in weapon_entities {
                                commands.entity(entity).despawn();
                            }

                            // Create new weapon entities for all weapons in inventory
                            if let Ok(player_transform) = player_query.single() {
                                for (weapon_id, weapon) in inventory.iter_weapons() {
                                    commands.spawn((
                                        weapon.clone(),
                                        EquippedWeapon { weapon_type: weapon_id.clone() },
                                        Transform::from_translation(player_transform.translation),
                                    ));
                                }
                            }
                        }
                    }
                    LootType::HealthPack { heal_amount } => {
                        // Heal the player
                        if let Ok((_, mut player)) = player_query_mut.single_mut() {
                            player.health = (player.health + heal_amount).min(player.max_health);
                        }

                        // Apply green screen tint for 0.2 seconds
                        screen_tint.remaining_duration = 0.2;
                        screen_tint.color = Color::srgba(0.0, 1.0, 0.0, 0.2); // Green with 20% opacity
                    }
                }

                // Play pickup sound
                if let (Some(asset_server), Some(loot_channel), Some(sound_limiter)) =
                    (asset_server.as_ref(), loot_channel.as_mut(), sound_limiter.as_mut()) {
                    crate::audio::plugin::play_limited_sound(
                        loot_channel.as_mut(),
                        asset_server,
                        "sounds/366104__original_sound__confirmation-downward.wav",
                        sound_limiter.as_mut(),
                    );
                }

                // Remove the loot item
                commands.entity(loot_entity).try_despawn();
            }
        }
    }
}

// Enemy death events are now handled directly in the bullet collision system

#[cfg(test)]
mod tests {
    use super::*;

    use crate::weapon::components::{Weapon, WeaponType};
    use crate::player::components::Player;
    use crate::inventory::resources::Inventory;
    use crate::game::resources::ScreenTintEffect;

    #[test]
    fn test_loot_spawning_random_world() {
        let mut app = App::new();
        app.add_systems(Update, loot_spawning_system);

        // Create a time resource
        app.init_resource::<Time>();

        // Test that the system runs without errors (random spawning is hard to test deterministically)
        for _ in 0..10 {
            app.update();
        }
        // The system should run without panicking
    }

    #[test]
    fn test_player_loot_collision_weapon_pickup() {
        let mut app = App::new();
        app.add_systems(Update, player_loot_collision_system);
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Create player at (0, 0)
        let _player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create laser weapon loot at (10, 10) - within collision distance
            let weapon = Weapon {
                weapon_type: WeaponType::Laser,
                level: 1,
                fire_rate: 3.0,
                base_damage: 15.0,
                last_fired: -3.0, // Prevent immediate firing at pickup
            };

        app.world_mut().spawn((
            LootItem {
                loot_type: LootType::Weapon(weapon.clone()),
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(10.0, 10.0, 0.0)),
        ));

        // Run collision system
        app.update();

        // Check that weapon was added to inventory
        let inventory = app.world().get_resource::<Inventory>().unwrap();
        assert!(inventory.get_weapon_by_type("pistol").is_some()); // Pistol should be present
        assert!(inventory.get_weapon_by_type("laser").is_some()); // Laser should be added
    }

    #[test]
    fn test_player_loot_collision_health_pack() {
        let mut app = App::new();
        app.add_systems(Update, player_loot_collision_system);
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Create player at (0, 0) with 50 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 50.0,
                max_health: 100.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create health pack loot at (10, 10)
        app.world_mut().spawn((
            LootItem {
                loot_type: LootType::HealthPack { heal_amount: 25.0 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(10.0, 10.0, 0.0)),
        ));

        // Run collision system
        app.update();

        // Check that player health was restored
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert_eq!(player.health, 75.0); // 50 + 25 = 75

        // Check that green screen tint was applied
        let screen_tint = app.world().get_resource::<ScreenTintEffect>().unwrap();
        assert!(screen_tint.remaining_duration > 0.0);
        assert_eq!(screen_tint.color, Color::srgba(0.0, 1.0, 0.0, 0.2));
    }

    #[test]
    fn test_player_loot_collision_no_collision_when_far() {
        let mut app = App::new();
        app.add_systems(Update, player_loot_collision_system);
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Create player at (0, 0)
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

        // Create loot far away (outside collision distance)
        app.world_mut().spawn((
            LootItem {
                loot_type: LootType::HealthPack { heal_amount: 25.0 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(100.0, 100.0, 0.0)), // Far away
        ));

        // Run collision system
        app.update();

        // The system should run without errors when loot is far away
    }
}