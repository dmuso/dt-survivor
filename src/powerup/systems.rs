use bevy::prelude::*;
use rand::Rng;
use crate::combat::components::Health;
use crate::powerup::components::*;
use crate::player::components::*;
use crate::weapon::components::*;
use crate::game::events::EnemyDeathEvent;
use crate::game::resources::{GameMeshes, GameMaterials};

/// Height at which powerups float (slightly above ground)
const POWERUP_Y_HEIGHT: f32 = 0.25;

/// System to spawn powerups when enemies die (2% drop rate)
pub fn powerup_spawning_system(
    mut commands: Commands,
    mut enemy_death_events: MessageReader<EnemyDeathEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let (Some(game_meshes), Some(game_materials)) = (game_meshes, game_materials) else {
        return;
    };

    for event in enemy_death_events.read() {
        let enemy_pos = event.position;

        // 2% chance to drop a powerup
        if rand::thread_rng().gen_bool(0.02) {
            // Randomly select one of the 5 powerup types
            let powerup_types = [
                PowerupType::MaxHealth,
                PowerupType::HealthRegen,
                PowerupType::WeaponFireRate,
                PowerupType::PickupRadius,
                PowerupType::MovementSpeed,
            ];

            let selected_type = powerup_types[rand::thread_rng().gen_range(0..powerup_types.len())].clone();

            // Spawn the powerup with pulsing animation using 3D mesh
            // enemy_pos is already in 3D (x, y, z) where y is height and xz is ground plane
            commands.spawn((
                Mesh3d(game_meshes.powerup.clone()),
                MeshMaterial3d(game_materials.powerup.clone()),
                Transform::from_translation(Vec3::new(enemy_pos.x, POWERUP_Y_HEIGHT, enemy_pos.z)),
                PowerupItem {
                    powerup_type: selected_type,
                    velocity: Vec2::ZERO,
                },
                PowerupPulse {
                    base_scale: Vec3::new(1.0, 1.0, 1.0),
                    amplitude: 0.3,
                    frequency: 3.0,
                    time: 0.0,
                },
            ));
        }
    }
}

/// System for powerup pulsing animation
pub fn powerup_pulse_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut PowerupPulse)>,
) {
    for (mut transform, mut pulse) in query.iter_mut() {
        pulse.time += time.delta_secs();
        let scale_factor = 1.0 + pulse.amplitude * (pulse.time * pulse.frequency).sin().abs();
        transform.scale = pulse.base_scale * scale_factor;
    }
}

/// System to handle powerup pickup and collision with player
pub fn powerup_pickup_system(
    mut commands: Commands,
    player_query: Query<(&Transform, &Player), With<Player>>,
    powerup_query: Query<(Entity, &Transform, &PowerupItem)>,
    mut active_powerups: ResMut<ActivePowerups>,
) {
    if let Ok((player_transform, player)) = player_query.single() {
        // Use XZ plane for distance calculation (3D ground plane)
        let player_pos_xz = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.z,
        );

        for (powerup_entity, powerup_transform, powerup_item) in powerup_query.iter() {
            let powerup_pos_xz = Vec2::new(
                powerup_transform.translation.x,
                powerup_transform.translation.z,
            );
            let distance = player_pos_xz.distance(powerup_pos_xz);

            // Collision detection - use player's pickup radius
            if distance < player.pickup_radius {
                // Apply the powerup effect
                active_powerups.add_powerup(powerup_item.powerup_type.clone());

                // Remove the powerup entity
                commands.entity(powerup_entity).despawn();
            }
        }
    }
}

/// System to apply powerup effects to the player
pub fn apply_player_powerup_effects(
    active_powerups: Res<ActivePowerups>,
    mut player_query: Query<(&mut Player, &mut Health)>,
) {
    if let Ok((mut player, mut health)) = player_query.single_mut() {
        // Calculate base values (this assumes we know the original values)
        let base_max_health = 100.0;
        let base_regen_rate = 1.0;
        let base_pickup_radius = 50.0;
        let base_speed = 200.0;

        // Apply permanent powerup effects
        let max_health_stacks = active_powerups.get_stack_count(&PowerupType::MaxHealth);
        let regen_stacks = active_powerups.get_stack_count(&PowerupType::HealthRegen);
        let pickup_stacks = active_powerups.get_stack_count(&PowerupType::PickupRadius);
        let speed_stacks = active_powerups.get_stack_count(&PowerupType::MovementSpeed);

        // Each stack increases values by 25%
        let max_health_multiplier = 1.0 + (max_health_stacks as f32 * 0.25);
        let regen_multiplier = 1.0 + (regen_stacks as f32 * 0.25);
        let pickup_multiplier = 1.0 + (pickup_stacks as f32 * 0.25);
        let speed_multiplier = 1.0 + (speed_stacks as f32 * 0.25);

        health.max = base_max_health * max_health_multiplier;
        player.regen_rate = base_regen_rate * regen_multiplier;
        player.pickup_radius = base_pickup_radius * pickup_multiplier;
        player.speed = base_speed * speed_multiplier;

        // Ensure health doesn't exceed new max
        if health.current > health.max {
            health.current = health.max;
        }
    }
}

/// System to apply powerup effects to weapons (fire rate)
pub fn apply_weapon_powerup_effects(
    active_powerups: Res<ActivePowerups>,
    mut weapon_query: Query<&mut Weapon>,
) {
    let fire_rate_stacks = active_powerups.get_stack_count(&PowerupType::WeaponFireRate);
    let fire_rate_multiplier = if fire_rate_stacks > 0 {
        2.0 // Double fire rate when active
    } else {
        1.0
    };

    for mut weapon in weapon_query.iter_mut() {
        // Store original fire rate if not already stored
        // For simplicity, we'll assume weapons have their base fire rate
        // In a real implementation, you'd want to store the original values
        let base_fire_rate = match weapon.weapon_type {
            WeaponType::Pistol { .. } => 2.0,
            WeaponType::Laser => 3.0,
            WeaponType::RocketLauncher => 1.0,
            WeaponType::Bomb => 0.5,
            WeaponType::BouncingLaser => 2.5,
        };

        weapon.fire_rate = base_fire_rate / fire_rate_multiplier;
    }
}

/// System to update powerup timers
pub fn update_powerup_timers(
    time: Res<Time>,
    mut active_powerups: ResMut<ActivePowerups>,
) {
    active_powerups.update_timers(time.delta_secs());
}

/// System to update the powerup UI display
pub fn update_powerup_ui(
    active_powerups: Res<ActivePowerups>,
    mut commands: Commands,
    powerup_display_query: Query<Entity, With<PowerupDisplay>>,
) {
    // Remove existing powerup display
    for entity in powerup_display_query.iter() {
        commands.entity(entity).despawn();
    }

    // Create new powerup display table on the left side
    if !active_powerups.get_active_powerups().is_empty() {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(150.0), // Below health display
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            PowerupDisplay,
        ))
        .with_children(|parent| {
            // Header
            parent.spawn((
                Text::new("Powerups"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));

            // Display each active powerup
            for powerup_type in active_powerups.get_active_powerups() {
                let stack_count = active_powerups.get_stack_count(powerup_type);
                let display_name = powerup_type.display_name();

                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    PowerupRow {
                        powerup_type: (*powerup_type).clone(),
                    },
                ))
                .with_children(|row| {
                    // Powerup icon (small colored square)
                    row.spawn((
                        Node {
                            width: Val::Px(12.0),
                            height: Val::Px(12.0),
                            margin: UiRect::right(Val::Px(5.0)),
                            ..default()
                        },
                        BackgroundColor(powerup_type.color()),
                    ));

                    // Powerup name and stack count
                    let text = if stack_count > 1 {
                        format!("{} (x{})", display_name, stack_count)
                    } else {
                        display_name.to_string()
                    };

                    row.spawn((
                        Text::new(text),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Countdown timer for temporary powerups
                    if let Some(remaining) = active_powerups.get_remaining_duration(powerup_type) {
                        row.spawn((
                            Text::new(format!(" {:.1}s", remaining)),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 1.0, 0.0)), // Yellow for timer
                            Node {
                                margin: UiRect::left(Val::Px(10.0)),
                                ..default()
                            },
                        ));
                    }
                });
            }
        });
    }
}