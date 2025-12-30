use bevy::prelude::*;
use bevy_hanabi::prelude::ParticleEffect;
use rand::Rng;
use crate::combat::components::Health;
use crate::loot::components::{DroppedItem, ItemData, PickupState, PopUpAnimation};
use crate::loot::events::*;
use crate::weapon::components::{Weapon, WeaponType};
use crate::player::components::*;
use crate::inventory::resources::*;
use crate::inventory::components::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::game::components::Level;
use crate::game::resources::{GameMaterials, GameMeshes, ScreenTintEffect, XpOrbMaterials};
use crate::game::events::LootDropEvent;
use crate::whisper::components::{
    ArcBurstTimer, LightningSpawnTimer, OrbitalParticleSpawnTimer,
    WhisperCompanion, WhisperOuterGlow,
};
use crate::whisper::resources::{WhisperSparkEffect, WhisperState};
use crate::whisper::systems::{WHISPER_LIGHT_COLOR, WHISPER_LIGHT_INTENSITY, WHISPER_LIGHT_RADIUS};
// Note: WHISPER_LIGHT_* constants are still used for the WhisperCompanion entity (not drops)

/// Height of small loot cube center above ground (XP orbs)
pub const LOOT_SMALL_Y_HEIGHT: f32 = 0.2;
/// Height of large loot cube center above ground (weapons, health packs)
pub const LOOT_LARGE_Y_HEIGHT: f32 = 0.3;

/// XP value scaling by level
/// Higher level orbs give exponentially more XP
pub fn xp_value_for_level(level: u8) -> u32 {
    match level {
        1 => 5,    // Common: 5 XP
        2 => 15,   // Uncommon: 15 XP
        3 => 35,   // Rare: 35 XP
        4 => 75,   // Epic: 75 XP
        _ => 150,  // Legendary: 150 XP
    }
}

/// Determine XP orb level based on enemy level
/// Higher enemy levels have better chance of dropping higher level orbs
/// Enemy level acts as minimum possible orb level with 20% chance per level above base
pub fn select_xp_level(enemy_level: u8, rng: &mut impl Rng) -> u8 {
    let base_level = enemy_level.clamp(1, 5);

    // Roll for potential upgrade (20% chance per level above base, up to level 5)
    let mut orb_level = base_level;
    while orb_level < 5 && rng.gen_bool(0.2) {
        orb_level += 1;
    }
    orb_level
}

pub fn loot_drop_system(
    mut commands: Commands,
    mut loot_drop_events: MessageReader<LootDropEvent>,
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
    xp_materials: Res<XpOrbMaterials>,
) {
    for event in loot_drop_events.read() {
        let enemy_pos = event.position;
        let enemy_level = event.enemy_level;

        // Spawn experience orbs for each enemy killed
        let mut rng = rand::thread_rng();
        let orb_count = rng.gen_range(1..=3);

        for _ in 0..orb_count {
            // Determine orb level based on enemy level (with upgrade chance)
            let orb_level = select_xp_level(enemy_level, &mut rng);
            let xp_value = xp_value_for_level(orb_level);

            // Offsets scaled for 3D world units (smaller than 2D pixel values)
            let offset_x = rng.gen_range(-1.0..=1.0);
            let offset_z = rng.gen_range(-1.0..=1.0);

            // Get material based on orb level (using rarity colors with emissive glow)
            let orb_material = xp_materials.for_level(orb_level);

            commands.spawn((
                Mesh3d(game_meshes.loot_small.clone()),
                MeshMaterial3d(orb_material),
                Transform::from_translation(Vec3::new(
                    enemy_pos.x + offset_x,
                    LOOT_SMALL_Y_HEIGHT,
                    enemy_pos.z + offset_z,
                )),
                DroppedItem {
                    pickup_state: PickupState::Idle,
                    item_data: ItemData::Experience { amount: xp_value },
                    velocity: Vec3::ZERO,
                    rotation_speed: 0.0,
                    rotation_direction: 1.0,
                },
                Level::new(orb_level),
            ));
        }

        // Spawn loot drops with specified probabilities
        let mut loot_drops: Vec<ItemData> = Vec::new();

        // 1% chance to drop rocket launcher
        if rng.gen_bool(0.01) {
            loot_drops.push(ItemData::Weapon(Weapon {
                weapon_type: WeaponType::RocketLauncher,
                level: 1,
                fire_rate: 5.0,
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

            // Select mesh and material based on item type (emissive materials handle glow via bloom)
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
                ItemData::Whisper => (
                    game_meshes.whisper_core.clone(),
                    game_materials.whisper_drop.clone(),
                    1.0, // Whisper floats higher
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
                    velocity: Vec3::ZERO,
                    rotation_speed: 0.0,
                    rotation_direction: 1.0,
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
/// Items are attracted toward the player's full 3D position (including Y)
pub fn update_item_attraction(
    mut item_query: Query<(&Transform, &mut DroppedItem), With<DroppedItem>>,
    player_query: Query<(&Transform, &Player), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((player_transform, player)) = player_query.single() {
        let player_pos = player_transform.translation;

        for (item_transform, mut item) in item_query.iter_mut() {
            if item.pickup_state == PickupState::BeingAttracted {
                let item_pos = item_transform.translation;
                let distance = player_pos.distance(item_pos);

                if distance > 0.5 { // Avoid orbiting when very close
                    let max_distance = player.pickup_radius;
                    let distance_ratio = (distance / max_distance).clamp(0.1, 1.0);
                    let acceleration_multiplier = 1.0 / distance_ratio;

                    // Use different acceleration based on item type
                    let base_acceleration = match &item.item_data {
                        ItemData::Experience { .. } => 80.0,  // Fastest for XP
                        ItemData::Weapon(_) | ItemData::HealthPack { .. } => 60.0, // Medium for loot
                        ItemData::Powerup(_) | ItemData::Whisper => 40.0, // Slower for powerups and whisper
                    };

                    let acceleration = base_acceleration * acceleration_multiplier;
                    let base_steering = base_acceleration * 1.25; // Steering is stronger than acceleration
                    let steering_strength = base_steering * acceleration_multiplier;

                    // 3D direction to player (including Y for vertical movement)
                    let direction_to_player = (player_pos - item_pos).normalize();
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
/// Applies full 3D velocity including vertical movement toward player
/// Also applies rotation during attraction phase
pub fn update_item_movement(
    time: Res<Time>,
    mut item_query: Query<(&mut Transform, &DroppedItem), With<DroppedItem>>,
) {
    for (mut transform, item) in item_query.iter_mut() {
        if item.pickup_state == PickupState::BeingAttracted {
            let delta = time.delta_secs();

            // Apply full 3D velocity
            transform.translation += item.velocity * delta;

            // Apply rotation around Y axis
            let rotation_angle = item.rotation_speed * item.rotation_direction * delta;
            transform.rotate_y(rotation_angle);
        }
    }
}

/// System that starts the pop-up animation when a pickup event is received.
/// Transitions items from Idle to PopUp state and adds the PopUpAnimation component.
/// Sets rotation based on player's movement direction when pickup was triggered.
pub fn start_popup_animation(
    mut commands: Commands,
    mut pickup_events: MessageReader<PickupEvent>,
    mut item_query: Query<(&Transform, &mut DroppedItem)>,
    player_query: Query<&Player>,
) {
    use crate::loot::components::BASE_ROTATION_SPEED;

    for event in pickup_events.read() {
        if let Ok((transform, mut item)) = item_query.get_mut(event.item_entity) {
            if item.pickup_state == PickupState::Idle {
                item.pickup_state = PickupState::PopUp;

                // Set rotation based on player's last movement direction
                if let Ok(player) = player_query.get(event.player_entity) {
                    item.rotation_speed = BASE_ROTATION_SPEED;
                    // Rotation direction based on player's X movement component
                    // Moving right -> clockwise (negative Y rotation)
                    // Moving left -> counter-clockwise (positive Y rotation)
                    item.rotation_direction = if player.last_movement_direction.x >= 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                }

                commands.entity(event.item_entity).insert(
                    PopUpAnimation::new(transform.translation.y)
                );
            }
        }
    }
}

/// Gravity constant for pop-up animation (units per second squared)
/// High value for fast, snappy animation (2x speed)
const POPUP_GRAVITY: f32 = 120.0;

/// System that animates items in the PopUp state.
/// Items fly upward quickly, hang briefly at peak, then fly to player.
/// Transitions to BeingAttracted immediately after hanging ends.
/// Rotates items around Y axis, ramping up speed during hang phase.
pub fn animate_popup(
    mut commands: Commands,
    time: Res<Time>,
    mut item_query: Query<(Entity, &mut Transform, &mut DroppedItem, &mut PopUpAnimation)>,
) {
    use crate::loot::components::{BASE_ROTATION_SPEED, MAX_ROTATION_MULTIPLIER};

    for (entity, mut transform, mut item, mut anim) in item_query.iter_mut() {
        if item.pickup_state != PickupState::PopUp {
            continue;
        }

        let delta = time.delta_secs();

        // Apply rotation around Y axis
        let rotation_angle = item.rotation_speed * item.rotation_direction * delta;
        transform.rotate_y(rotation_angle);

        // Handle hanging at peak
        if anim.hanging {
            anim.hang_time_remaining -= delta;

            // Gradually increase rotation speed toward 10x during hang
            let max_speed = BASE_ROTATION_SPEED * MAX_ROTATION_MULTIPLIER;
            let speed_increase_rate = (max_speed - BASE_ROTATION_SPEED) / 0.15; // Ramp over hang duration
            item.rotation_speed = (item.rotation_speed + speed_increase_rate * delta).min(max_speed);

            if anim.hang_time_remaining <= 0.0 {
                // Done hanging - immediately transition to attraction (fly to player)
                item.pickup_state = PickupState::BeingAttracted;
                commands.entity(entity).remove::<PopUpAnimation>();
            }
            continue;
        }

        // Apply gravity to vertical velocity
        anim.vertical_velocity -= POPUP_GRAVITY * delta;

        // Check if we've reached the peak (velocity goes negative) and should start hanging
        if anim.vertical_velocity <= 0.0 {
            anim.hanging = true;
            anim.vertical_velocity = 0.0; // Stop at peak
            continue;
        }

        // Update Y position (only while ascending)
        transform.translation.y += anim.vertical_velocity * delta;
    }
}

/// Distance threshold for completing pickup (in world units)
const PICKUP_COMPLETE_DISTANCE: f32 = 0.5;

/// System that completes the pickup when attracted items reach the player.
/// Transitions from BeingAttracted to PickedUp and fires ItemEffectEvent.
/// Uses full 3D distance since items fly toward player's position.
pub fn complete_pickup_when_close(
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut item_query: Query<(Entity, &Transform, &mut DroppedItem)>,
    mut effect_events: MessageWriter<ItemEffectEvent>,
) {
    if let Ok((player_entity, player_transform)) = player_query.single() {
        let player_pos = player_transform.translation;

        for (item_entity, item_transform, mut item) in item_query.iter_mut() {
            if item.pickup_state == PickupState::BeingAttracted {
                // Use 3D distance since items fly toward player in 3D
                let distance = player_pos.distance(item_transform.translation);

                if distance <= PICKUP_COMPLETE_DISTANCE {
                    item.pickup_state = PickupState::PickedUp;
                    effect_events.write(ItemEffectEvent {
                        item_entity,
                        item_data: item.item_data.clone(),
                        player_entity,
                    });
                }
            }
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
    mut whisper_state: ResMut<WhisperState>,
    spark_effect: Option<Res<WhisperSparkEffect>>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
    asset_server: Option<Res<AssetServer>>,
    mut audio_channel: Option<ResMut<AudioChannel<LootSoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
    mut loot_cooldown: Option<ResMut<crate::loot::plugin::LootSoundCooldown>>,
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

                    // Play powerup sound for weapon pickups
                    play_powerup_sound(&asset_server, &mut audio_channel, &mut sound_limiter, &mut loot_cooldown);
                }
            }
            ItemData::HealthPack { heal_amount } => {
                // Heal player
                if let Ok((_, _, mut health)) = player_query.get_mut(event.player_entity) {
                    health.heal(*heal_amount);
                    screen_tint.remaining_duration = 0.2;
                    screen_tint.color = Color::srgba(0.0, 0.5, 0.0, 0.05); // Dark green with 5% opacity
                }
                play_pickup_sound(&asset_server, &mut audio_channel, &mut sound_limiter, &mut loot_cooldown);
            }
            ItemData::Experience { amount } => {
                // Add experience and handle level-ups
                if let Ok(mut player_exp) = player_exp_query.get_mut(event.player_entity) {
                    let _levels_gained = player_exp.add_xp(*amount);
                    // TODO: Fire PlayerLevelUpEvent if levels_gained > 0
                }
                play_pickup_sound(&asset_server, &mut audio_channel, &mut sound_limiter, &mut loot_cooldown);
            }
            ItemData::Powerup(powerup_type) => {
                // Add powerup
                active_powerups.add_powerup(powerup_type.clone());
                play_powerup_sound(&asset_server, &mut audio_channel, &mut sound_limiter, &mut loot_cooldown);
            }
            ItemData::Whisper => {
                // Skip if already collected (prevents double-processing)
                if whisper_state.collected {
                    continue;
                }

                // Get resources needed for spawning companion
                let Some(game_meshes) = game_meshes.as_ref() else { continue };
                let Some(game_materials) = game_materials.as_ref() else { continue };

                // Get player position for spawning companion
                let player_pos = player_query
                    .get(event.player_entity)
                    .map(|(t, _, _)| t.translation)
                    .unwrap_or(Vec3::ZERO);

                // Spawn WhisperCompanion at player position with offset
                let companion = WhisperCompanion::default();
                let companion_pos = player_pos + companion.follow_offset;

                let mut companion_entity = commands.spawn((
                    companion,
                    ArcBurstTimer::default(),
                    LightningSpawnTimer::default(),
                    OrbitalParticleSpawnTimer::default(),
                    Transform::from_translation(companion_pos),
                    Visibility::default(),
                    PointLight {
                        color: WHISPER_LIGHT_COLOR,
                        intensity: WHISPER_LIGHT_INTENSITY,
                        radius: WHISPER_LIGHT_RADIUS,
                        shadows_enabled: false,
                        ..default()
                    },
                ));

                // Add particle effect if available
                if let Some(effect) = spark_effect.as_ref() {
                    companion_entity.insert(ParticleEffect::new(effect.0.clone()));
                }

                companion_entity.with_children(|parent| {
                    parent.spawn((
                        WhisperOuterGlow,
                        Mesh3d(game_meshes.whisper_core.clone()),
                        MeshMaterial3d(game_materials.whisper_core.clone()),
                        Transform::default(),
                    ));
                });

                // Mark as collected
                whisper_state.collected = true;

                // Add default pistol to inventory
                let pistol = Weapon {
                    weapon_type: WeaponType::Pistol {
                        bullet_count: 5,
                        spread_angle: 15.0,
                    },
                    level: 1,
                    fire_rate: 2.0,
                    base_damage: 1.0,
                    last_fired: -2.0,
                };
                inventory.add_or_level_weapon(pistol.clone());

                // Recreate weapon entities
                let weapon_entities: Vec<Entity> = weapon_query.iter().map(|(entity, _)| entity).collect();
                for entity in weapon_entities {
                    commands.entity(entity).despawn();
                }

                // Create new weapon entities for all weapons in inventory
                for (_weapon_id, weapon) in inventory.iter_weapons() {
                    commands.spawn((
                        weapon.clone(),
                        EquippedWeapon {
                            weapon_type: weapon.weapon_type.clone(),
                        },
                        Transform::from_translation(player_pos),
                    ));
                }

                play_powerup_sound(&asset_server, &mut audio_channel, &mut sound_limiter, &mut loot_cooldown);
            }
        }

        // Mark item as consumed
        commands.entity(event.item_entity).insert(DroppedItem {
            pickup_state: PickupState::Consumed,
            item_data: event.item_data.clone(),
            velocity: Vec3::ZERO,
            rotation_speed: 0.0,
            rotation_direction: 1.0,
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

/// Sound path for powerup, weapon, and whisper pickups
pub const POWERUP_SOUND_PATH: &str = "sounds/422090__profmudkip__8-bit-powerup-2.wav";

/// Helper function to play powerup/weapon/whisper pickup sound with random 100-250ms debounce
fn play_powerup_sound(
    asset_server: &Option<Res<AssetServer>>,
    audio_channel: &mut Option<ResMut<AudioChannel<LootSoundChannel>>>,
    sound_limiter: &mut Option<ResMut<SoundLimiter>>,
    loot_cooldown: &mut Option<ResMut<crate::loot::plugin::LootSoundCooldown>>,
) {
    // Check cooldown - skip if still cooling down
    if let Some(cooldown) = loot_cooldown {
        if !cooldown.timer.is_finished() {
            return; // Still in cooldown, skip this sound
        }
        // Reset cooldown timer with random 100-250ms duration
        cooldown.reset_random();
    }

    if let (Some(asset_server), Some(audio_channel), Some(sound_limiter)) =
        (asset_server, audio_channel, sound_limiter) {
        crate::audio::plugin::play_limited_sound(
            audio_channel.as_mut(),
            asset_server,
            POWERUP_SOUND_PATH,
            sound_limiter.as_mut(),
        );
    }
}

/// Helper function to play pickup sound with random 100-250ms debounce
fn play_pickup_sound(
    asset_server: &Option<Res<AssetServer>>,
    audio_channel: &mut Option<ResMut<AudioChannel<LootSoundChannel>>>,
    sound_limiter: &mut Option<ResMut<SoundLimiter>>,
    loot_cooldown: &mut Option<ResMut<crate::loot::plugin::LootSoundCooldown>>,
) {
    // Check cooldown - skip if still cooling down
    if let Some(cooldown) = loot_cooldown {
        if !cooldown.timer.is_finished() {
            return; // Still in cooldown, skip this sound
        }
        // Reset cooldown timer with random 100-250ms duration
        cooldown.reset_random();
    }

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

    mod xp_value_tests {
        use super::*;

        #[test]
        fn xp_value_for_level_returns_correct_values() {
            assert_eq!(xp_value_for_level(1), 5);
            assert_eq!(xp_value_for_level(2), 15);
            assert_eq!(xp_value_for_level(3), 35);
            assert_eq!(xp_value_for_level(4), 75);
            assert_eq!(xp_value_for_level(5), 150);
        }

        #[test]
        fn xp_value_increases_with_level() {
            let mut prev_value = 0;
            for level in 1..=5 {
                let value = xp_value_for_level(level);
                assert!(value > prev_value, "XP value should increase with level");
                prev_value = value;
            }
        }

        #[test]
        fn xp_value_for_level_above_5_returns_legendary() {
            // Any level above 5 should return legendary value (150)
            assert_eq!(xp_value_for_level(6), 150);
            assert_eq!(xp_value_for_level(10), 150);
            assert_eq!(xp_value_for_level(255), 150);
        }

        #[test]
        fn xp_value_for_level_0_returns_legendary() {
            // Level 0 is invalid, falls through match to default (150)
            assert_eq!(xp_value_for_level(0), 150);
        }
    }

    mod select_xp_level_tests {
        use super::*;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        #[test]
        fn select_xp_level_respects_enemy_level_minimum() {
            let mut rng = StdRng::seed_from_u64(12345);
            for enemy_level in 1..=5 {
                for _ in 0..50 {
                    let orb_level = select_xp_level(enemy_level, &mut rng);
                    assert!(
                        orb_level >= enemy_level,
                        "Orb level {} should be >= enemy level {}",
                        orb_level,
                        enemy_level
                    );
                }
            }
        }

        #[test]
        fn select_xp_level_never_exceeds_5() {
            let mut rng = StdRng::seed_from_u64(54321);
            for enemy_level in 1..=5 {
                for _ in 0..100 {
                    let orb_level = select_xp_level(enemy_level, &mut rng);
                    assert!(orb_level <= 5, "Orb level {} should be <= 5", orb_level);
                }
            }
        }

        #[test]
        fn select_xp_level_clamps_high_enemy_level() {
            let mut rng = StdRng::seed_from_u64(98765);
            // Enemy level 10 (invalid) should be clamped to 5
            for _ in 0..50 {
                let orb_level = select_xp_level(10, &mut rng);
                assert_eq!(orb_level, 5, "Clamped enemy level should produce level 5 orbs");
            }
        }

        #[test]
        fn select_xp_level_can_upgrade() {
            // With many trials, we should see at least some upgrades
            let mut rng = StdRng::seed_from_u64(11111);
            let mut saw_upgrade = false;
            for _ in 0..100 {
                let orb_level = select_xp_level(1, &mut rng);
                if orb_level > 1 {
                    saw_upgrade = true;
                    break;
                }
            }
            assert!(saw_upgrade, "Should see at least one upgrade with 20% chance");
        }

        #[test]
        fn select_xp_level_level_5_enemy_always_drops_level_5() {
            // Level 5 enemies always drop level 5 (can't upgrade further)
            let mut rng = StdRng::seed_from_u64(22222);
            for _ in 0..50 {
                let orb_level = select_xp_level(5, &mut rng);
                assert_eq!(orb_level, 5, "Level 5 enemy should always drop level 5 orbs");
            }
        }
    }

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
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Create dropped item at (10, y, 10) - within pickup radius on XZ plane
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
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
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create dropped item far away on XZ plane (outside pickup radius)
        app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
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
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item that's already being attracted (not idle) on XZ plane
        app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::HealthPack { heal_amount: 25.0 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
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
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item being attracted at (50, y, 0) on XZ plane
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
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
                velocity: Vec3::new(-100.0, 0.0, 0.0),
                rotation_speed: 0.0,
                rotation_direction: 1.0,
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
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
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
            velocity: Vec3::ZERO,
            rotation_speed: 0.0,
            rotation_direction: 1.0,
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
            velocity: Vec3::ZERO,
            rotation_speed: 0.0,
            rotation_direction: 1.0,
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
            velocity: Vec3::ZERO,
            rotation_speed: 0.0,
            rotation_direction: 1.0,
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

    #[test]
    fn test_popup_animation_component_creation() {
        use crate::loot::components::PopUpAnimation;

        let anim = PopUpAnimation::new(0.3);
        assert_eq!(anim.start_y, 0.3);
        assert_eq!(anim.peak_height, 1.0);
        assert!(anim.vertical_velocity > 0.0, "Should start with upward velocity");
        assert!(!anim.hanging, "Should not start hanging");
        assert!(anim.hang_time_remaining > 0.0, "Should have hang time");
    }

    #[test]
    fn test_popup_animation_with_custom_height() {
        use crate::loot::components::PopUpAnimation;

        let anim = PopUpAnimation::with_peak_height(0.5, 3.0);
        assert_eq!(anim.start_y, 0.5);
        assert_eq!(anim.peak_height, 3.0);
    }

    #[test]
    fn test_start_popup_animation_transitions_to_popup_state() {
        use crate::loot::components::PopUpAnimation;

        let mut app = App::new();
        app.add_message::<PickupEvent>();
        // Chain detect_pickup_collisions with start_popup_animation
        app.add_systems(Update, (detect_pickup_collisions, start_popup_animation).chain());

        // Create player at origin with pickup radius
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item in Idle state within pickup range
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(10.0, LOOT_SMALL_Y_HEIGHT, 10.0)),
        )).id();

        // Run update - detect_pickup_collisions will fire event, start_popup_animation will process it
        app.update();

        // Verify item transitioned to PopUp state
        let item = app.world().get::<DroppedItem>(item_entity).unwrap();
        assert_eq!(item.pickup_state, PickupState::PopUp);

        // Verify PopUpAnimation component was added
        let anim = app.world().get::<PopUpAnimation>(item_entity);
        assert!(anim.is_some(), "PopUpAnimation component should be added");
    }

    #[test]
    fn test_animate_popup_moves_item_upward() {
        use std::time::Duration;
        use bevy::time::TimePlugin;
        use crate::loot::components::PopUpAnimation;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, animate_popup);

        // Create item in PopUp state with animation
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::PopUp,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(0.0, LOOT_SMALL_Y_HEIGHT, 0.0)),
            PopUpAnimation::new(LOOT_SMALL_Y_HEIGHT),
        )).id();

        let initial_y = app.world().get::<Transform>(item_entity).unwrap().translation.y;

        // Run update
        app.update();
        app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify item moved upward
        let new_y = app.world().get::<Transform>(item_entity).unwrap().translation.y;
        assert!(new_y > initial_y, "Item should move upward during popup animation");
    }

    #[test]
    fn test_animate_popup_transitions_to_attracted_after_hang() {
        use bevy::time::TimePlugin;
        use crate::loot::components::PopUpAnimation;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, animate_popup);

        // Create item that has finished hanging (hang timer expired)
        let mut anim = PopUpAnimation::new(LOOT_SMALL_Y_HEIGHT);
        anim.hanging = true;
        anim.hang_time_remaining = -0.01; // Timer already expired

        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::PopUp,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(0.0, LOOT_SMALL_Y_HEIGHT + 2.0, 0.0)), // At peak
            anim,
        )).id();

        app.update();

        // Verify transitioned directly to BeingAttracted (no falling back to ground)
        let item = app.world().get::<DroppedItem>(item_entity).unwrap();
        assert_eq!(item.pickup_state, PickupState::BeingAttracted);

        // Verify PopUpAnimation component was removed
        let anim = app.world().get::<PopUpAnimation>(item_entity);
        assert!(anim.is_none(), "PopUpAnimation should be removed after transition");

        // Verify item is still at peak height (didn't fall)
        let transform = app.world().get::<Transform>(item_entity).unwrap();
        assert!(transform.translation.y > LOOT_SMALL_Y_HEIGHT + 1.0, "Item should still be near peak");
    }

    #[test]
    fn test_popup_animation_hang_state_creation() {
        use crate::loot::components::PopUpAnimation;

        // Verify hanging state fields are properly initialized
        let anim = PopUpAnimation::new(LOOT_SMALL_Y_HEIGHT);
        assert!(!anim.hanging, "Should not start hanging");
        assert!(anim.hang_time_remaining > 0.0, "Should have hang time");
        assert!(anim.vertical_velocity > 0.0, "Should have upward velocity");
    }

    #[test]
    fn test_popup_animation_state_transitions() {
        use crate::loot::components::PopUpAnimation;

        // Test the state machine logic directly
        let mut anim = PopUpAnimation::new(LOOT_SMALL_Y_HEIGHT);

        // Initial state: ascending with positive velocity
        assert!(!anim.hanging);
        assert!(anim.vertical_velocity > 0.0);

        // Simulate reaching peak (velocity goes to zero)
        anim.vertical_velocity = 0.0;
        anim.hanging = true;

        // Now in hanging state
        assert!(anim.hanging);
        assert!(anim.hang_time_remaining > 0.0);

        // After hang timer expires, item transitions to BeingAttracted
        // (handled by the system, which removes PopUpAnimation component)
        anim.hang_time_remaining = 0.0;
        // At this point the system would transition to BeingAttracted
    }

    #[test]
    fn test_item_never_falls_at_original_position() {
        use std::time::Duration;
        use bevy::time::TimePlugin;

        // This test ensures we never have a state where Y is decreasing
        // while XZ remains at the original spawn position. This would indicate
        // the item is falling back to the ground instead of flying to the player.

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_message::<PickupEvent>();
        app.add_message::<ItemEffectEvent>();
        app.add_systems(Update, (
            detect_pickup_collisions,
            start_popup_animation,
            animate_popup,
            update_item_attraction,
            update_item_movement,
            complete_pickup_when_close,
        ).chain());

        // Create player at origin
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item at specific XZ position within pickup radius
        let initial_x = 10.0;
        let initial_z = 10.0;
        let initial_y = LOOT_SMALL_Y_HEIGHT;

        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(initial_x, initial_y, initial_z)),
        )).id();

        let mut prev_y = initial_y;

        // Run the full pickup flow for many frames
        for _ in 0..100 {
            app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.016));
            app.update();

            if let Some(transform) = app.world().get::<Transform>(item_entity) {
                let current_y = transform.translation.y;
                let current_x = transform.translation.x;
                let current_z = transform.translation.z;

                // If Y is decreasing (falling), XZ must be different from original
                // (meaning item is flying to player, not falling back to ground)
                if current_y < prev_y {
                    let xz_unchanged = (current_x - initial_x).abs() < 0.01
                                    && (current_z - initial_z).abs() < 0.01;
                    assert!(!xz_unchanged,
                        "Item should never fall (Y decreasing) while at original XZ position. \
                         This would mean falling to ground instead of flying to player. \
                         Y: {} -> {}, XZ: ({}, {}) vs original ({}, {})",
                        prev_y, current_y, current_x, current_z, initial_x, initial_z);
                }

                prev_y = current_y;
            } else {
                // Item was despawned (consumed), test complete
                break;
            }
        }
    }

    #[test]
    fn test_complete_pickup_transitions_when_close_to_player() {
        let mut app = App::new();
        app.add_message::<ItemEffectEvent>();
        app.add_systems(Update, complete_pickup_when_close);

        // Create player at origin
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item very close to player (within pickup threshold)
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::new(-10.0, 0.0, 0.0),
                rotation_speed: 0.0,
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(0.3, LOOT_SMALL_Y_HEIGHT, 0.0)), // Very close
        )).id();

        app.update();

        // Verify transitioned to PickedUp
        let item = app.world().get::<DroppedItem>(item_entity).unwrap();
        assert_eq!(item.pickup_state, PickupState::PickedUp);
    }

    #[test]
    fn test_pickup_state_popup_is_distinct() {
        assert_ne!(PickupState::Idle, PickupState::PopUp);
        assert_ne!(PickupState::PopUp, PickupState::BeingAttracted);
    }

    #[test]
    fn test_item_rotates_during_popup_animation() {
        use std::time::Duration;
        use bevy::time::TimePlugin;
        use crate::loot::components::PopUpAnimation;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, animate_popup);

        // Create item in PopUp state with rotation
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::PopUp,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 2.0,         // 2 rad/s base rotation
                rotation_direction: 1.0,      // Clockwise
            },
            Transform::from_translation(Vec3::new(0.0, LOOT_SMALL_Y_HEIGHT, 0.0)),
            PopUpAnimation::new(LOOT_SMALL_Y_HEIGHT),
        )).id();

        let initial_rotation = app.world().get::<Transform>(item_entity).unwrap().rotation;

        // Run a few frames
        app.update();
        app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify rotation has changed
        let new_rotation = app.world().get::<Transform>(item_entity).unwrap().rotation;
        assert_ne!(initial_rotation, new_rotation, "Item should rotate during popup");
    }

    #[test]
    fn test_rotation_speed_increases_during_hang() {
        use std::time::Duration;
        use bevy::time::TimePlugin;
        use crate::loot::components::PopUpAnimation;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, animate_popup);

        // Create item already hanging at peak
        let mut anim = PopUpAnimation::new(LOOT_SMALL_Y_HEIGHT);
        anim.hanging = true;
        anim.hang_time_remaining = 0.15; // Full hang time

        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::PopUp,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 2.0,         // Base speed
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)), // At peak
            anim,
        )).id();

        // Run through most of hang time
        for _ in 0..10 {
            app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.01));
            app.update();
        }

        // Verify rotation speed has increased toward 10x
        let item = app.world().get::<DroppedItem>(item_entity).unwrap();
        assert!(item.rotation_speed > 2.0, "Rotation speed should increase during hang");
    }

    #[test]
    fn test_rotation_continues_during_attraction() {
        use std::time::Duration;
        use bevy::time::TimePlugin;

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.add_systems(Update, (update_item_attraction, update_item_movement).chain());

        // Create player at origin
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 100.0,
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create item being attracted with max rotation speed
        let item_entity = app.world_mut().spawn((
            DroppedItem {
                pickup_state: PickupState::BeingAttracted,
                item_data: ItemData::Experience { amount: 10 },
                velocity: Vec3::ZERO,
                rotation_speed: 20.0,        // 10x the base 2.0
                rotation_direction: 1.0,
            },
            Transform::from_translation(Vec3::new(50.0, LOOT_SMALL_Y_HEIGHT, 0.0)),
        )).id();

        let initial_rotation = app.world().get::<Transform>(item_entity).unwrap().rotation;

        // Run a few frames
        app.update();
        app.world_mut().resource_mut::<Time<()>>().advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify rotation continues
        let new_rotation = app.world().get::<Transform>(item_entity).unwrap().rotation;
        assert_ne!(initial_rotation, new_rotation, "Item should continue rotating during attraction");
    }

    #[test]
    fn test_powerup_sound_path_is_defined() {
        // Verify the powerup sound path constant exists and is distinct from loot sound
        assert_eq!(
            super::POWERUP_SOUND_PATH,
            "sounds/422090__profmudkip__8-bit-powerup-2.wav"
        );
        assert_ne!(
            super::POWERUP_SOUND_PATH,
            "sounds/366104__original_sound__confirmation-downward.wav",
            "Powerup sound should be different from loot pickup sound"
        );
    }

    // Tests for LootSoundCooldown
    mod loot_sound_cooldown_tests {
        use crate::loot::plugin::LootSoundCooldown;
        use std::time::Duration;

        #[test]
        fn test_loot_sound_cooldown_starts_ready_to_play() {
            let cooldown = LootSoundCooldown::default();
            assert!(cooldown.timer.is_finished(), "Cooldown should start finished so first sound plays");
        }

        #[test]
        fn test_loot_sound_cooldown_blocks_during_cooldown() {
            let mut cooldown = LootSoundCooldown::default();

            // Simulate playing a sound with random reset
            cooldown.reset_random();

            // Timer should not be finished immediately after reset
            assert!(!cooldown.timer.is_finished(), "Timer should block during cooldown");
        }

        #[test]
        fn test_loot_sound_cooldown_random_duration_in_range() {
            let mut cooldown = LootSoundCooldown::default();

            // Test multiple resets to verify random range
            for _ in 0..20 {
                cooldown.reset_random();
                let duration_ms = cooldown.timer.duration().as_millis();
                assert!(
                    (100..=250).contains(&duration_ms),
                    "Random duration {} should be between 100-250ms",
                    duration_ms
                );
            }
        }

        #[test]
        fn test_loot_sound_cooldown_allows_after_max_elapsed() {
            let mut cooldown = LootSoundCooldown::default();

            // Simulate playing a sound with random reset
            cooldown.reset_random();

            // Tick past the maximum 250ms cooldown
            cooldown.timer.tick(Duration::from_millis(251));

            assert!(cooldown.timer.is_finished(), "Timer should allow sound after max 250ms");
        }

        #[test]
        fn test_tick_loot_sound_cooldown_advances_timer() {
            let mut cooldown = LootSoundCooldown::default();

            // Reset timer with random duration
            cooldown.reset_random();
            assert!(!cooldown.timer.is_finished(), "Timer should start not finished after reset");

            // Tick past maximum possible cooldown (250ms)
            cooldown.timer.tick(Duration::from_millis(300));
            assert!(cooldown.timer.is_finished(), "Timer should be finished after 300ms");
        }
    }

}
