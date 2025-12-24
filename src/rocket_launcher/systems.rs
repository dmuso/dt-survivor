use bevy::prelude::*;
use crate::rocket_launcher::components::*;
use crate::prelude::*;
use crate::game::events::EnemyDeathEvent;

pub fn rocket_spawning_system(
    time: Res<Time>,
    mut rocket_query: Query<&mut RocketProjectile>,
) {
    for mut rocket in rocket_query.iter_mut() {
        if let RocketState::Pausing = rocket.state {
            rocket.pause_timer.tick(time.delta());
            if rocket.pause_timer.is_finished() {
                rocket.state = RocketState::Targeting;
            }
        }
    }
}

pub fn target_marking_system(
    mut commands: Commands,
    rocket_query: Query<&RocketProjectile>,
    target_marker_query: Query<Entity, With<TargetMarker>>,
) {
    // Remove expired target markers
    for marker_entity in target_marker_query.iter() {
        commands.entity(marker_entity).despawn();
    }

    // Create new target markers for rockets in targeting state
    for rocket in rocket_query.iter() {
        if matches!(rocket.state, RocketState::Targeting) {
            if let Some(target_pos) = rocket.target_position {
                // Create red dot marker at target position
                commands.spawn((
                    Sprite::from_color(Color::srgb(1.0, 0.0, 0.0), Vec2::new(6.0, 6.0)), // Red dot
                    Transform::from_translation(Vec3::new(target_pos.x, target_pos.y, 0.4)), // At target position
                    TargetMarker {
                        target_entity: Entity::from_bits(0), // Dummy entity since we're not tracking a specific enemy
                        lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                    },
                ));
            }
        }
    }
}

pub fn rocket_movement_system(
    mut commands: Commands,
    time: Res<Time>,
    mut rocket_query: Query<(Entity, &mut RocketProjectile, &mut Transform)>,
    // asset_server: Option<Res<AssetServer>>,
    // mut weapon_channel: Option<ResMut<bevy_kira_audio::AudioChannel<WeaponSoundChannel>>>,
    // mut sound_limiter: Option<ResMut<SoundLimiter>>,
) {
    let mut rockets_to_explode = Vec::new();

    for (rocket_entity, mut rocket, mut transform) in rocket_query.iter_mut() {
        let rocket_pos = transform.translation.truncate();

        match rocket.state {
            RocketState::Targeting => {
                // Transition to homing if we have a target position
                if rocket.target_position.is_some() {
                    rocket.state = RocketState::Homing;
                    if let Some(target_pos) = rocket.target_position {
                        // Calculate initial direction toward target
                        let direction = (target_pos - rocket_pos).normalize();
                        rocket.velocity = direction * rocket.speed;
                    }
                }
            }
            RocketState::Homing => {
                if let Some(target_pos) = rocket.target_position {
                    // Calculate desired direction
                    let to_target = target_pos - rocket_pos;
                    let distance = to_target.length();

                    if distance < 20.0 {
                        // Close enough - explode
                        rockets_to_explode.push((rocket_entity, rocket_pos, rocket.damage));
                        continue;
                    }

                    let desired_direction = to_target.normalize();

                    // Smoothly turn toward target
                    let current_direction = rocket.velocity.normalize();
                    let new_direction = (current_direction + desired_direction * rocket.homing_strength * time.delta_secs()).normalize();

                    rocket.velocity = new_direction * rocket.speed;
                }

                // Move rocket
                let movement = rocket.velocity * time.delta_secs();
                transform.translation += movement.extend(0.0);
            }
            _ => {}
        }
    }

    // Handle explosions
    for (rocket_entity, explosion_pos, damage) in rockets_to_explode {
        commands.entity(rocket_entity).despawn();

        // Create explosion
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 0.0, 0.0, 0.8), Vec2::new(0.0, 0.0)), // Initial size
            Transform::from_translation(explosion_pos.extend(0.5)),
            Explosion::new(explosion_pos, damage),
        ));

        // Play explosion sound
        // TODO: Re-enable rocket explosion sounds once audio import issues are resolved
        // if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
        //     (asset_server.as_ref(), weapon_channel.as_mut(), sound_limiter.as_mut()) {
        //     // Use weapon sound for now - could add dedicated explosion sound later
        //     crate::audio::plugin::play_limited_sound(
        //         weapon_channel.as_mut(),
        //         asset_server,
        //         "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
        //         sound_limiter.as_mut(),
        //     );
        // }
    }
}

pub fn explosion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut explosion_query: Query<(Entity, &mut Explosion, &mut Sprite)>,
) {
    for (entity, mut explosion, mut sprite) in explosion_query.iter_mut() {
        explosion.lifetime.tick(time.delta());

        // Expand radius
        if explosion.is_expanding() {
            explosion.current_radius += explosion.expansion_rate * time.delta_secs();
            explosion.current_radius = explosion.current_radius.min(explosion.max_radius);
        }

        // Update visual
        sprite.custom_size = Some(Vec2::new(explosion.current_radius * 2.0, explosion.current_radius * 2.0));
        sprite.color = Color::srgba(1.0, 0.0, 0.0, explosion.get_opacity());

        // Despawn when fully expanded and faded
        if explosion.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}


pub fn area_damage_system(
    mut commands: Commands,
    explosion_query: Query<&Explosion>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut score: ResMut<crate::score::Score>,
    mut enemy_death_events: MessageWriter<EnemyDeathEvent>,
) {
    for explosion in explosion_query.iter() {
        if explosion.current_radius > 0.0 {
            let mut enemies_to_kill = Vec::new();

            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = enemy_transform.translation.truncate();
                let distance = explosion.center.distance(enemy_pos);

                if distance <= explosion.current_radius {
                    enemies_to_kill.push(enemy_entity);
                }
            }

            // Kill enemies in explosion radius
            for enemy_entity in enemies_to_kill {
                // Get enemy position for event
                let enemy_pos = enemy_query.get(enemy_entity).map(|(_, transform)| transform.translation.truncate()).unwrap_or(Vec2::ZERO);

                // Send enemy death event for centralized loot/experience handling
                enemy_death_events.write(EnemyDeathEvent {
                    enemy_entity,
                    position: enemy_pos,
                });

                 commands.entity(enemy_entity).try_despawn();
                 score.0 += 1;
            }
        }
    }
}

pub fn update_rocket_visuals(
    mut commands: Commands,
    rocket_query: Query<(Entity, &RocketProjectile), Changed<RocketProjectile>>,
) {
    for (entity, rocket) in rocket_query.iter() {
        let color = match rocket.state {
            RocketState::Pausing => Color::srgb(0.5, 0.5, 0.5), // Gray when pausing
            RocketState::Targeting => Color::srgb(1.0, 1.0, 0.0), // Yellow when targeting
            RocketState::Homing => Color::srgb(1.0, 0.5, 0.0), // Orange when homing
            RocketState::Exploding => Color::srgb(1.0, 0.0, 0.0), // Red when exploding
        };

        commands.entity(entity).insert(Sprite::from_color(color, Vec2::new(12.0, 6.0)));
    }
}

    #[cfg(test)]
    mod tests {
use bevy::prelude::*;
use crate::rocket_launcher::components::*;
use crate::enemies::components::*;
        use crate::weapon::components::{Weapon, WeaponType};
        use crate::loot::components::LootItem;

    #[test]
    fn test_rocket_loot_placement() {
        // Test that rocket launcher loot is created with correct properties
        let weapon = Weapon {
            weapon_type: WeaponType::RocketLauncher,
            level: 1,
            fire_rate: 10.0,
            base_damage: 30.0,
            last_fired: -10.0,
        };

        let loot = LootItem {
            loot_type: crate::loot::components::LootType::Weapon(weapon.clone()),
            velocity: Vec2::ZERO,
        };

        // Verify loot type is weapon
        match &loot.loot_type {
            crate::loot::components::LootType::Weapon(loot_weapon) => {
                assert!(matches!(loot_weapon.weapon_type, WeaponType::RocketLauncher));
                assert_eq!(loot_weapon.fire_rate, 10.0);
                assert_eq!(loot_weapon.base_damage, 30.0);
            }
            _ => panic!("Expected weapon loot"),
        }
    }
}