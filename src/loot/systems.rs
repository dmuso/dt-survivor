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
            // Spawn laser weapon
            let weapon = Weapon {
                weapon_type: WeaponType::Laser,
                fire_rate: 3.0, // 3 seconds between shots
                damage: 15.0,
                last_fired: 0.0,
            };

            commands.spawn((
                Sprite::from_color(Color::srgb(0.0, 0.0, 1.0), Vec2::new(16.0, 16.0)), // Blue for weapon loot
                Transform::from_translation(Vec3::new(x, y, 0.5)),
                LootItem {
                    loot_type: LootType::Weapon(weapon),
                },
            ));
        } else {
            // Spawn health pack
            commands.spawn((
                Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(12.0, 12.0)), // Green for health pack
                Transform::from_translation(Vec3::new(x, y, 0.5)),
                LootItem {
                    loot_type: LootType::HealthPack { heal_amount: 25.0 },
                },
            ));
        }

        *last_spawn_time = current_time;
    }
}

// This system is now handled directly in the bullet collision system

#[allow(clippy::too_many_arguments)]
pub fn player_loot_collision_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    loot_query: Query<(Entity, &Transform, &LootItem)>,
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
                        // Find next available inventory slot
                        if let Some(slot_index) = inventory.slots.iter().position(|slot| slot.is_none()) {
                            inventory.slots[slot_index] = Some(weapon.clone());

                            // Create a separate weapon entity positioned at the player
                            if let Ok(player_transform) = player_query.single() {
                                commands.spawn((
                                    weapon.clone(),
                                    EquippedWeapon { slot_index },
                                    Transform::from_translation(player_transform.translation),
                                ));
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
                commands.entity(loot_entity).despawn();
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
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create laser weapon loot at (10, 10) - within collision distance
            let weapon = Weapon {
                weapon_type: WeaponType::Laser,
                fire_rate: 3.0,
                damage: 15.0,
                last_fired: -3.0, // Prevent immediate firing at pickup
            };

        app.world_mut().spawn((
            LootItem {
                loot_type: LootType::Weapon(weapon.clone()),
            },
            Transform::from_translation(Vec3::new(10.0, 10.0, 0.0)),
        ));

        // Run collision system
        app.update();

        // Check that weapon was added to inventory
        let inventory = app.world().get_resource::<Inventory>().unwrap();
        assert!(inventory.slots[0].is_some()); // Slot 0 should have the pistol
        assert!(inventory.slots[1].is_some()); // Slot 1 should have the laser
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
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create health pack loot at (10, 10)
        app.world_mut().spawn((
            LootItem {
                loot_type: LootType::HealthPack { heal_amount: 25.0 },
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
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create loot far away (outside collision distance)
        app.world_mut().spawn((
            LootItem {
                loot_type: LootType::HealthPack { heal_amount: 25.0 },
            },
            Transform::from_translation(Vec3::new(100.0, 100.0, 0.0)), // Far away
        ));

        // Run collision system
        app.update();

        // The system should run without errors when loot is far away
    }
}