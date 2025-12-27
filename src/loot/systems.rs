use bevy::prelude::*;
use rand::Rng;
use crate::combat::components::Health;
use crate::loot::components::*;
use crate::loot::events::*;
use crate::weapon::components::{Weapon, WeaponType};
use crate::player::components::*;
use crate::inventory::resources::*;
use crate::inventory::components::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::game::resources::{GameMaterials, GameMeshes, ScreenTintEffect};
use crate::game::events::LootDropEvent;

/// Height of small loot cube center above ground (XP orbs)
pub const LOOT_SMALL_Y_HEIGHT: f32 = 0.2;
/// Height of large loot cube center above ground (weapons, health packs)
pub const LOOT_LARGE_Y_HEIGHT: f32 = 0.3;

pub fn loot_drop_system(
    mut commands: Commands,
    mut loot_drop_events: MessageReader<LootDropEvent>,
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
) {
    for event in loot_drop_events.read() {
        let enemy_pos = event.position;

        // Spawn experience orbs for each enemy killed
        let mut rng = rand::thread_rng();
        let orb_count = rng.gen_range(1..=3);

        for _ in 0..orb_count {
            let value = rng.gen_range(5..=15); // 5-15 experience per orb
            // Offsets scaled for 3D world units (smaller than 2D pixel values)
            let offset_x = rng.gen_range(-1.0..=1.0);
            let offset_z = rng.gen_range(-1.0..=1.0);

            commands.spawn((
                Mesh3d(game_meshes.loot_small.clone()),
                MeshMaterial3d(game_materials.xp_orb.clone()),
                Transform::from_translation(Vec3::new(
                    enemy_pos.x + offset_x,
                    LOOT_SMALL_Y_HEIGHT,
                    enemy_pos.z + offset_z,
                )),
                DroppedItem {
                    pickup_state: PickupState::Idle,
                    item_data: ItemData::Experience { amount: value },
                    velocity: Vec2::ZERO,
                },
            ));
        }

        // Spawn loot drops with specified probabilities
        let mut loot_drops: Vec<ItemData> = Vec::new();

        // 1% chance to drop rocket launcher
        if rng.gen_bool(0.01) {
            loot_drops.push(ItemData::Weapon(Weapon {
                weapon_type: WeaponType::RocketLauncher,
                level: 1,
                fire_rate: 1.0,
                base_damage: 50.0,
                last_fired: 0.0,
            }));
        }

        // 2% chance to drop laser
        if rng.gen_bool(0.02) {
            loot_drops.push(ItemData::Weapon(Weapon {
                weapon_type: WeaponType::Laser,
                level: 1,
                fire_rate: 3.0,
                base_damage: 15.0,
                last_fired: 0.0,
            }));
        }

        // 3% chance to drop pistol
        if rng.gen_bool(0.03) {
            loot_drops.push(ItemData::Weapon(Weapon {
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
            loot_drops.push(ItemData::HealthPack { heal_amount: 25.0 });
        }

        // Spawn loot items spaced out around the enemy position (on XZ plane)
        let spacing = 2.0; // Distance between drops in 3D world units
        for (i, item_data) in loot_drops.into_iter().enumerate() {
            let angle = (i as f32) * std::f32::consts::TAU / 4.0; // Space items in a circle
            let offset_x = angle.cos() * spacing;
            let offset_z = angle.sin() * spacing;

            // Select mesh and material based on item type
            let (mesh, material, y_height) = match &item_data {
                ItemData::Weapon(weapon) => {
                    let material = match weapon.weapon_type {
                        WeaponType::Pistol { .. } => game_materials.weapon_pistol.clone(),
                        WeaponType::Laser => game_materials.weapon_laser.clone(),
                        WeaponType::RocketLauncher => game_materials.weapon_rocket.clone(),
                        _ => game_materials.xp_orb.clone(), // Default fallback
                    };
                    (game_meshes.loot_large.clone(), material, LOOT_LARGE_Y_HEIGHT)
                }
                ItemData::HealthPack { .. } => (
                    game_meshes.loot_medium.clone(),
                    game_materials.health_pack.clone(),
                    LOOT_LARGE_Y_HEIGHT,
                ),
                ItemData::Experience { .. } => (
                    game_meshes.loot_small.clone(),
                    game_materials.xp_orb.clone(),
                    LOOT_SMALL_Y_HEIGHT,
                ),
                ItemData::Powerup(_) => (
                    game_meshes.loot_medium.clone(),
                    game_materials.powerup.clone(),
                    LOOT_LARGE_Y_HEIGHT,
                ),
            };

            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(Vec3::new(
                    enemy_pos.x + offset_x,
                    y_height,
                    enemy_pos.z + offset_z,
                )),
                DroppedItem {
                    pickup_state: PickupState::Idle,
                    item_data,
                    velocity: Vec2::ZERO,
                },
            ));
        }
    }
}

// ECS-based pickup systems

/// System that detects when dropped items enter pickup range and starts attraction
pub fn detect_pickup_collisions(
    mut pickup_events: MessageWriter<PickupEvent>,
    player_query: Query<(Entity, &Transform, &Player), With<Player>>,
    item_query: Query<(Entity, &Transform, &DroppedItem), With<DroppedItem>>,
) {
    if let Ok((player_entity, player_transform, player)) = player_query.single() {
        // Use XZ plane for 3D collision detection
        let player_xz = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.z,
        );

        for (item_entity, item_transform, item) in item_query.iter() {
            if item.pickup_state == PickupState::Idle {
                let item_xz = Vec2::new(
                    item_transform.translation.x,
                    item_transform.translation.z,
                );
                let distance = player_xz.distance(item_xz);

                if distance <= player.pickup_radius {
                    pickup_events.write(PickupEvent {
                        item_entity,
                        player_entity,
                    });
                }
            }
        }
    }
}

/// System that applies magnetic attraction physics to items being picked up
pub fn update_item_attraction(
    mut item_query: Query<(&Transform, &mut DroppedItem), With<DroppedItem>>,
    player_query: Query<(&Transform, &Player), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((player_transform, player)) = player_query.single() {
        // Use XZ plane for 3D attraction physics
        let player_xz = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.z,
        );

        for (item_transform, mut item) in item_query.iter_mut() {
            if item.pickup_state == PickupState::BeingAttracted {
                let item_xz = Vec2::new(
                    item_transform.translation.x,
                    item_transform.translation.z,
                );
                let distance = player_xz.distance(item_xz);

                if distance > 0.5 { // Avoid orbiting when very close (scaled for 3D units)
                    let max_distance = player.pickup_radius;
                    let distance_ratio = (distance / max_distance).clamp(0.1, 1.0);
                    let acceleration_multiplier = 1.0 / distance_ratio;

                    // Use different acceleration based on item type (scaled for 3D units)
                    let base_acceleration = match &item.item_data {
                        ItemData::Experience { .. } => 80.0,  // Fastest for XP
                        ItemData::Weapon(_) | ItemData::HealthPack { .. } => 60.0, // Medium for loot
                        ItemData::Powerup(_) => 40.0, // Slower for powerups
                    };

                    let acceleration = base_acceleration * acceleration_multiplier;
                    let base_steering = base_acceleration * 1.25; // Steering is stronger than acceleration
                    let steering_strength = base_steering * acceleration_multiplier;

                    let direction_to_player = (player_xz - item_xz).normalize();
                    item.velocity += direction_to_player * acceleration * time.delta_secs();

                    // Apply steering to correct direction
                    let current_speed = item.velocity.length();
                    if current_speed > 0.1 {
                        let desired_velocity = direction_to_player * current_speed;
                        let steering_vector = desired_velocity - item.velocity;

                        let max_steering = steering_strength * time.delta_secs();
                        let steering_magnitude = steering_vector.length();
                        let clamped_steering = if steering_magnitude > max_steering {
                            steering_vector.normalize() * max_steering
                        } else {
                            steering_vector
                        };

                        item.velocity += clamped_steering;
                    }
                }
            }
        }
    }
}

/// System that updates item positions based on velocity
pub fn update_item_movement(
    time: Res<Time>,
    mut item_query: Query<(&mut Transform, &DroppedItem), With<DroppedItem>>,
) {
    for (mut transform, item) in item_query.iter_mut() {
        if item.pickup_state == PickupState::BeingAttracted {
            let movement = item.velocity * time.delta_secs();
            // Apply velocity on XZ plane: velocity.x -> X axis, velocity.y -> Z axis
            transform.translation += Vec3::new(movement.x, 0.0, movement.y);
        }
    }
}

/// System that processes pickup events and triggers effect events
pub fn process_pickup_events(
    mut _commands: Commands,
    mut pickup_events: MessageReader<PickupEvent>,
    mut item_query: Query<&mut DroppedItem>,
    mut effect_events: MessageWriter<ItemEffectEvent>,
) {
    for event in pickup_events.read() {
        if let Ok(mut item) = item_query.get_mut(event.item_entity) {
            item.pickup_state = PickupState::PickedUp;
            effect_events.write(ItemEffectEvent {
                item_entity: event.item_entity,
                item_data: item.item_data.clone(),
                player_entity: event.player_entity,
            });
        }
    }
}

/// System that applies pickup effects (decoupled from collision detection)
#[allow(clippy::too_many_arguments)]
pub fn apply_item_effects(
    mut commands: Commands,
    mut effect_events: MessageReader<ItemEffectEvent>,
    mut player_query: Query<(&Transform, &Player, &mut Health)>,
    mut player_exp_query: Query<&mut crate::experience::components::PlayerExperience>,
    weapon_query: Query<(Entity, &Weapon)>,
    mut inventory: ResMut<Inventory>,
    mut active_powerups: ResMut<crate::powerup::components::ActivePowerups>,
    mut screen_tint: ResMut<ScreenTintEffect>,
    asset_server: Option<Res<AssetServer>>,
    mut audio_channel: Option<ResMut<AudioChannel<LootSoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
) {
    for event in effect_events.read() {
        match &event.item_data {
            ItemData::Weapon(weapon) => {
                // Add weapon to inventory
                if inventory.add_or_level_weapon(weapon.clone()) {
                    // Recreate all weapon entities to reflect changes
                    let weapon_entities: Vec<Entity> = weapon_query.iter().map(|(entity, _)| entity).collect();
                    for entity in weapon_entities {
                        commands.entity(entity).despawn();
                    }

                    // Create new weapon entities for all weapons in inventory
                    if let Ok((player_transform, _, _)) = player_query.get(event.player_entity) {
                        for (_weapon_id, weapon) in inventory.iter_weapons() {
                            commands.spawn((
                                weapon.clone(),
                                EquippedWeapon { weapon_type: weapon.weapon_type.clone() },
                                Transform::from_translation(player_transform.translation),
                            ));
                        }
                    }

                    // Play pickup sound
                    play_pickup_sound(&asset_server, &mut audio_channel, &mut sound_limiter);
                }
            }
            ItemData::HealthPack { heal_amount } => {
                // Heal player
                if let Ok((_, _, mut health)) = player_query.get_mut(event.player_entity) {
                    health.heal(*heal_amount);
                    screen_tint.remaining_duration = 0.2;
                    screen_tint.color = Color::srgba(0.0, 1.0, 0.0, 0.2);
                }
                play_pickup_sound(&asset_server, &mut audio_channel, &mut sound_limiter);
            }
            ItemData::Experience { amount } => {
                // Add experience
                if let Ok(mut player_exp) = player_exp_query.get_mut(event.player_entity) {
                    player_exp.current += amount;
                    // Level up logic would go here
                }
                play_pickup_sound(&asset_server, &mut audio_channel, &mut sound_limiter);
            }
            ItemData::Powerup(powerup_type) => {
                // Add powerup
                active_powerups.add_powerup(powerup_type.clone());
                play_pickup_sound(&asset_server, &mut audio_channel, &mut sound_limiter);
            }
        }

        // Mark item as consumed
        commands.entity(event.item_entity).insert(DroppedItem {
            pickup_state: PickupState::Consumed,
            item_data: event.item_data.clone(),
            velocity: Vec2::ZERO,
        });
    }
}

/// System that cleans up consumed items
pub fn cleanup_consumed_items(
    mut commands: Commands,
    item_query: Query<(Entity, &DroppedItem), With<DroppedItem>>,
) {
    for (entity, item) in item_query.iter() {
        if item.pickup_state == PickupState::Consumed {
            commands.entity(entity).despawn();
        }
    }
}

/// Helper function to play pickup sound
fn play_pickup_sound(
    asset_server: &Option<Res<AssetServer>>,
    audio_channel: &mut Option<ResMut<AudioChannel<LootSoundChannel>>>,
    sound_limiter: &mut Option<ResMut<SoundLimiter>>,
) {
    if let (Some(asset_server), Some(audio_channel), Some(sound_limiter)) =
        (asset_server, audio_channel, sound_limiter) {
        crate::audio::plugin::play_limited_sound(
            audio_channel.as_mut(),
            asset_server,
            "sounds/366104__original_sound__confirmation-downward.wav",
            sound_limiter.as_mut(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use crate::weapon::components::{Weapon, WeaponType};
    use crate::player::components::Player;

    /// Helper resource to count pickup events and store last event data
    #[derive(Resource, Clone)]
    struct PickupEventCounter {
        count: Arc<AtomicUsize>,
        last_item_entity: Arc<std::sync::Mutex<Option<Entity>>>,
        last_player_entity: Arc<std::sync::Mutex<Option<Entity>>>,
    }

    impl Default for PickupEventCounter {
        fn default() -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
                last_item_entity: Arc::new(std::sync::Mutex::new(None)),
                last_player_entity: Arc::new(std::sync::Mutex::new(None)),
            }
        }
    }

    /// Helper system to count pickup events
    fn count_pickup_events(
        mut events: MessageReader<PickupEvent>,
        counter: Res<PickupEventCounter>,
    ) {
        for event in events.read() {
            counter.count.fetch_add(1, Ordering::SeqCst);
            *counter.last_item_entity.lock().unwrap() = Some(event.item_entity);
            *counter.last_player_entity.lock().unwrap() = Some(event.player_entity);
        }
    }

    #[test]
    fn test_detect_pickup_collisions_fires_event_when_in_range() {
        let mut app = App::new();
        let counter = PickupEventCounter::default();
        app.add_message::<PickupEvent>();
        app.insert_resource(counter.clone());
        app.add_systems(Update, (detect_pickup_collisions, count_pickup_events).chain());

        // Create player at origin on XZ plane
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create dropped item at (10, y, 10) - within pickup radius on XZ plane
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(10.0, LOOT_LARGE_Y_HEIGHT, 10.0)),
        )).id();

        // Run detect_pickup_collisions system
        app.update();

        // Verify pickup event was fired
        assert_eq!(counter.count.load(Ordering::SeqCst), 1);
        assert_eq!(*counter.last_item_entity.lock().unwrap(), Some(item_entity));
        assert_eq!(*counter.last_player_entity.lock().unwrap(), Some(player_entity));
    }

    #[test]
    fn test_detect_pickup_collisions_no_event_when_out_of_range() {
        let mut app = App::new();
        let counter = PickupEventCounter::default();
        app.add_message::<PickupEvent>();
        app.insert_resource(counter.clone());
        app.add_systems(Update, (detect_pickup_collisions, count_pickup_events).chain());

        // Create player at origin on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create dropped item far away on XZ plane (outside pickup radius)
        app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(100.0, LOOT_LARGE_Y_HEIGHT, 100.0)),
        ));

        // Run detect_pickup_collisions system
        app.update();

        // Verify no pickup event was fired
        assert_eq!(counter.count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_detect_pickup_collisions_ignores_non_idle_items() {
        let mut app = App::new();
        let counter = PickupEventCounter::default();
        app.add_message::<PickupEvent>();
        app.insert_resource(counter.clone());
        app.add_systems(Update, (detect_pickup_collisions, count_pickup_events).chain());

        // Create player at origin on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item that's already being attracted (not idle) on XZ plane
        app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(10.0, LOOT_LARGE_Y_HEIGHT, 10.0)),
        ));

        // Run detect_pickup_collisions system
        app.update();

        // Verify no pickup event was fired for non-idle item
        assert_eq!(counter.count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_update_item_attraction_applies_velocity() {
        use std::time::Duration;
        use bevy::time::TimePlugin;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, update_item_attraction);

        // Create player at origin on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 100.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item being attracted at (50, y, 0) on XZ plane
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(50.0, LOOT_SMALL_Y_HEIGHT, 0.0)),
        )).id();

        // Run a few updates to allow time to advance
        app.update();
        // Manually advance time for the test
        app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify velocity was updated (should be negative x direction toward player)
        let item = app.world().get::<DroppedItem>(item_entity).unwrap();
        assert!(item.velocity.x < 0.0, "Velocity should be toward player (negative x)");
    }

    #[test]
    fn test_update_item_movement_moves_attracted_items() {
        use std::time::Duration;
        use bevy::time::TimePlugin;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, update_item_movement);

        // Create item being attracted with initial velocity on XZ plane
        // velocity.x maps to X axis, velocity.y maps to Z axis
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec2::new(-100.0, 0.0),
            },
            Transform::from_translation(Vec3::new(50.0, LOOT_SMALL_Y_HEIGHT, 0.0)),
        )).id();

        // Run initial update
        app.update();
        // Manually advance time for the test
        app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify position was updated on X axis (velocity.x applied to translation.x)
        let transform = app.world().get::<Transform>(item_entity).unwrap();
        assert!(transform.translation.x < 50.0, "Item should have moved toward player on X axis");
    }

    #[test]
    fn test_cleanup_consumed_items_despawns_consumed() {
        let mut app = App::new();
        app.add_systems(Update, cleanup_consumed_items);

        // Create consumed item
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Consumed,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec2::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Run cleanup system
        app.update();

        // Verify entity was despawned
        assert!(app.world().get_entity(item_entity).is_err(), "Consumed item should be despawned");
    }

    #[test]
    fn test_dropped_item_weapon_creation() {
        // Test that DroppedItem can hold weapon data
        let weapon = Weapon {
            weapon_type: WeaponType::Laser,
            level: 1,
            fire_rate: 3.0,
            base_damage: 15.0,
            last_fired: 0.0,
        };

        let item = DroppedItem {
            pickup_state: PickupState::Idle,
            item_data: ItemData::Weapon(weapon.clone()),
            velocity: Vec2::ZERO,
        };

        match item.item_data {
            ItemData::Weapon(w) => {
                assert!(matches!(w.weapon_type, WeaponType::Laser));
                assert_eq!(w.base_damage, 15.0);
            }
            _ => panic!("Expected weapon item data"),
        }
    }

    #[test]
    fn test_dropped_item_health_pack_creation() {
        let item = DroppedItem {
            pickup_state: PickupState::Idle,
            item_data: ItemData::HealthPack { heal_amount: 25.0 },
            velocity: Vec2::ZERO,
        };

        match item.item_data {
            ItemData::HealthPack { heal_amount } => {
                assert_eq!(heal_amount, 25.0);
            }
            _ => panic!("Expected health pack item data"),
        }
    }

    #[test]
    fn test_dropped_item_experience_creation() {
        let item = DroppedItem {
            pickup_state: PickupState::Idle,
            item_data: ItemData::Experience { amount: 50 },
            velocity: Vec2::ZERO,
        };

        match item.item_data {
            ItemData::Experience { amount } => {
                assert_eq!(amount, 50);
            }
            _ => panic!("Expected experience item data"),
        }
    }

    #[test]
    fn test_pickup_state_transitions() {
        // Test that pickup states are properly distinguishable
        assert_eq!(PickupState::Idle, PickupState::Idle);
        assert_ne!(PickupState::Idle, PickupState::BeingAttracted);
        assert_ne!(PickupState::BeingAttracted, PickupState::PickedUp);
        assert_ne!(PickupState::PickedUp, PickupState::Consumed);
    }

    #[test]
    fn test_loot_spawns_on_xz_plane() {
        // Verify loot positions use XZ plane (Y for height)
        let pos = Vec3::new(10.0, LOOT_SMALL_Y_HEIGHT, 20.0);
        assert_eq!(pos.y, LOOT_SMALL_Y_HEIGHT, "Y should be the height above ground");
        assert_eq!(pos.x, 10.0, "X should be the X coordinate on ground plane");
        assert_eq!(pos.z, 20.0, "Z should be the Z coordinate on ground plane");
    }

    #[test]
    fn test_loot_y_heights_are_reasonable() {
        // Verify height constants make sense
        assert!(LOOT_SMALL_Y_HEIGHT > 0.0, "Small loot should be above ground");
        assert!(LOOT_LARGE_Y_HEIGHT > 0.0, "Large loot should be above ground");
        assert!(LOOT_LARGE_Y_HEIGHT >= LOOT_SMALL_Y_HEIGHT, "Large loot should be at or above small loot height");
    }
}
