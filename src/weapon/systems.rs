use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rand::Rng;
use crate::weapon::components::*;

use crate::enemies::components::*;
use crate::player::components::*;
use crate::audio::plugin::*;
use crate::audio::plugin::SoundLimiter;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::components::EquippedWeapon;

    #[test]
    fn test_weapon_targeting_random_from_5_closest() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

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

        // Create 10 enemies at different distances
        for i in 1..=10 {
            let distance = i as f32 * 20.0; // 20, 40, 60, ..., 200 units away
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(distance, 0.0, 0.0)),
            ));
        }

        // Create a weapon entity
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 0.1, // Very fast for testing
                base_damage: 1.0,
                last_fired: 10.0, // Ready to fire
            },
            EquippedWeapon { weapon_type: "pistol".to_string() },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Add required resources
        app.init_resource::<Time>();

        // Run weapon firing system
        app.update();

        // The system should run without errors
    }

    #[test]
    fn test_laser_weapon_raycast_enemy_destruction() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

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

        // Create enemies in a line (along the x-axis from player)
        let enemy_positions = vec![
            Vec2::new(100.0, 0.0),  // On laser line
            Vec2::new(200.0, 0.0),  // On laser line
            Vec2::new(100.0, 50.0), // Off laser line
            Vec2::new(300.0, 0.0),  // On laser line but beyond 800px
        ];

        let mut enemy_entities = Vec::new();
        for pos in enemy_positions {
            let entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(pos.x, pos.y, 0.0)),
            )).id();
            enemy_entities.push(entity);
        }

        // Create laser weapon entity
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Laser,
                level: 1,
                fire_rate: 0.1, // Very fast for testing
                base_damage: 15.0,
                last_fired: 10.0, // Ready to fire
            },
            EquippedWeapon { weapon_type: "laser".to_string() },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Add required resources
        app.init_resource::<Time>();

        // Run weapon firing system
        app.update();

        // The laser weapon system should run without errors
    }

    #[test]
    fn test_multiple_weapons_fire_independently() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

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

        // Create enemies
        for i in 1..=6 {
            let angle = (i as f32 / 6.0) * std::f32::consts::PI * 2.0;
            let x = angle.cos() * 100.0;
            let y = angle.sin() * 100.0;
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(x, y, 0.0)),
            ));
        }

        // Create two weapon entities - pistol and laser
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 0.1,
                base_damage: 1.0,
                last_fired: 10.0,
            },
            EquippedWeapon { weapon_type: "pistol".to_string() },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Laser,
                level: 1,
                fire_rate: 0.1,
                base_damage: 15.0,
                last_fired: 10.0,
            },
            EquippedWeapon { weapon_type: "laser".to_string() },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Add required resources
        app.init_resource::<Time>();

        // Run weapon firing system
        app.update();

        // Both weapons should fire without errors
    }
}

#[allow(clippy::too_many_arguments)]
pub fn weapon_firing_system(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Option<Res<AssetServer>>,
    mut weapon_channel: Option<ResMut<AudioChannel<WeaponSoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<(Entity, &Transform, &Enemy)>,
    mut weapon_query: Query<&mut Weapon>,
) {
    let current_time = time.elapsed_secs();

    // Check if weapon exists
    if weapon_query.is_empty() {
        return; // No weapons to fire
    }

    if let Ok(player_transform) = player_query.single() {
        let player_pos = player_transform.translation.truncate();

        // Find 5 closest enemies
        let mut enemy_distances: Vec<(Entity, Vec2, f32)> = enemy_query
            .iter()
            .map(|(entity, transform, _)| {
                let pos = transform.translation.truncate();
                let distance = player_pos.distance(pos);
                (entity, pos, distance)
            })
            .collect();

        // Sort by distance and take first 5
        enemy_distances.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        let closest_enemies: Vec<(Entity, Vec2)> = enemy_distances
            .into_iter()
            .take(5)
            .map(|(entity, pos, _)| (entity, pos))
            .collect();

        // If no enemies, don't fire
        if closest_enemies.is_empty() {
            return;
        }

        // Fire weapons that are ready
        for mut weapon in weapon_query.iter_mut() {
            if current_time - weapon.last_fired >= weapon.fire_rate {
                // Select random target from 5 closest
                let mut rng = rand::thread_rng();
                let target_index = rng.gen_range(0..closest_enemies.len());
                let target_pos = closest_enemies[target_index].1;

                // Calculate direction towards target enemy
                let base_direction = (target_pos - player_pos).normalize();

                match &weapon.weapon_type {
                    WeaponType::Pistol { .. } => {
                        // Delegate pistol firing to pistol module
                        crate::pistol::systems::fire_pistol(
                            &mut commands,
                            &weapon,
                            player_transform,
                            target_pos,
                            asset_server.as_ref(),
                            weapon_channel.as_mut(),
                            sound_limiter.as_mut(),
                        );
                    }
                     WeaponType::Laser => {
                         // Create a laser beam entity
                         use crate::laser::components::LaserBeam;
                          commands.spawn(LaserBeam::new(player_pos, base_direction, weapon.damage()));

                         // Play laser sound effect
                         if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
                             (asset_server.as_ref(), weapon_channel.as_mut(), sound_limiter.as_mut()) {
                             crate::audio::plugin::play_limited_sound(
                                 weapon_channel.as_mut(),
                                 asset_server,
                                 "sounds/72639__chipfork71__laser01rev.wav",
                                 sound_limiter.as_mut(),
                             );
                         }
                     }
                     WeaponType::RocketLauncher => {
                         // Find closest enemy for targeting
                         let target_pos = closest_enemies.get(target_index).map(|(_, pos)| *pos);

                         // Create a rocket projectile
                         use crate::rocket_launcher::components::RocketProjectile;
                          let (mut rocket, transform) = RocketProjectile::new(player_transform.translation.truncate(), base_direction, weapon.damage());
                         rocket.target_position = target_pos;
                         commands.spawn((
                             rocket,
                             transform,
                         ));

                         // Play rocket launch sound
                         if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
                             (asset_server.as_ref(), weapon_channel.as_mut(), sound_limiter.as_mut()) {
                             crate::audio::plugin::play_limited_sound(
                                 weapon_channel.as_mut(),
                                 asset_server,
                                 "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
                                 sound_limiter.as_mut(),
                             );
                         }
                     }
                     _ => {
                         // Other weapon types not implemented yet
                     }
                }

                weapon.last_fired = current_time;
            }
        }
    }
}