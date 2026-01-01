use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use bevy_hanabi::prelude::*;
use rand::Rng;
use crate::weapon::components::*;

use crate::enemies::components::*;
use crate::audio::plugin::*;
use crate::audio::plugin::SoundLimiter;
use crate::game::resources::{GameMeshes, GameMaterials};
use crate::movement::components::from_xz;
use crate::whisper::resources::SpellOrigin;
use crate::rocket_launcher::components::RocketExhaustEffect;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::components::EquippedWeapon;

    #[test]
    fn test_weapon_targeting_random_from_5_closest() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

        // Set up SpellOrigin at (0, 3, 0) - simulates Whisper collected at height
        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        // Create 10 enemies at different distances on XZ plane
        // Vec2(x, z) in SpellOrigin maps to Vec3(x, height, z) for enemies
        for i in 1..=10 {
            let distance = i as f32 * 20.0; // 20, 40, 60, ..., 200 units away on X axis
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(distance, 0.375, 0.0)), // Y is height
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
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
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

        // Set up SpellOrigin at (0, 3, 0) - simulates Whisper collected at height
        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        // Create enemies in a line on XZ plane (along X-axis from weapon origin)
        // Enemy positions are Vec2(x, z) mapped to Vec3(x, height, z)
        let enemy_xz_positions = vec![
            Vec2::new(100.0, 0.0),  // On laser line in XZ
            Vec2::new(200.0, 0.0),  // On laser line in XZ
            Vec2::new(100.0, 50.0), // Off laser line in XZ (Z offset)
            Vec2::new(300.0, 0.0),  // On laser line but farther
        ];

        let mut enemy_entities = Vec::new();
        for pos in enemy_xz_positions {
            let entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(pos.x, 0.375, pos.y)), // pos.y is Z
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
            EquippedWeapon { weapon_type: WeaponType::Laser },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
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

        // Set up SpellOrigin at (0, 3, 0) - simulates Whisper collected at height
        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        // Create enemies in a circle on XZ plane
        for i in 1..=6 {
            let angle = (i as f32 / 6.0) * std::f32::consts::PI * 2.0;
            let x = angle.cos() * 100.0;
            let z = angle.sin() * 100.0;
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(x, 0.375, z)), // Y is height
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
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Laser,
                level: 1,
                fire_rate: 0.1,
                base_damage: 15.0,
                last_fired: 10.0,
            },
            EquippedWeapon { weapon_type: WeaponType::Laser },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        // Add required resources
        app.init_resource::<Time>();

        // Run weapon firing system
        app.update();

        // Both weapons should fire without errors
    }

    #[test]
    fn test_weapons_disabled_without_whisper() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

        // Set up SpellOrigin with None - simulates Whisper not collected
        app.insert_resource(SpellOrigin { position: None });

        // Create enemies on XZ plane
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)), // Y is height
        ));

        // Create a weapon entity
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 0.1,
                base_damage: 1.0,
                last_fired: 10.0, // Ready to fire
            },
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        app.init_resource::<Time>();

        // Run weapon firing system
        app.update();

        // No bullets should be spawned since SpellOrigin is None
        let bullet_count = app.world_mut().query::<&crate::bullets::components::Bullet>().iter(app.world()).count();
        assert_eq!(bullet_count, 0, "No bullets should spawn when Whisper not collected");
    }

    #[test]
    fn test_laser_weapon_spawns_with_correct_damage() {
        use crate::laser::components::LaserBeam;

        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

        // Set up SpellOrigin
        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        // Create enemy for targeting
        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
        ));

        // Create level 5 laser weapon with base_damage 10.0
        // Expected damage: 10.0 * 5 * 1.25 = 62.5
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Laser,
                level: 5,
                fire_rate: 0.1,
                base_damage: 10.0,
                last_fired: -10.0, // Ready to fire
            },
            EquippedWeapon { weapon_type: WeaponType::Laser },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        app.init_resource::<Time>();
        app.update();

        // Verify laser was spawned with correct damage
        let mut laser_query = app.world_mut().query::<&LaserBeam>();
        let lasers: Vec<_> = laser_query.iter(app.world()).collect();
        assert_eq!(lasers.len(), 1, "One laser should be spawned");
        assert_eq!(
            lasers[0].damage, 62.5,
            "Laser damage should be 62.5 (10.0 * 5 * 1.25)"
        );
    }

    #[test]
    fn test_laser_damage_scales_with_weapon_level() {
        use crate::laser::components::LaserBeam;

        let test_cases = [
            (1, 10.0, 12.5),   // 10 * 1 * 1.25
            (5, 10.0, 62.5),   // 10 * 5 * 1.25
            (10, 10.0, 125.0), // 10 * 10 * 1.25
            (3, 20.0, 75.0),   // 20 * 3 * 1.25
        ];

        for (level, base_damage, expected_damage) in test_cases {
            let mut app = App::new();
            app.add_systems(Update, weapon_firing_system);

            app.insert_resource(SpellOrigin {
                position: Some(Vec3::new(0.0, 3.0, 0.0)),
            });

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.world_mut().spawn((
                Weapon {
                    weapon_type: WeaponType::Laser,
                    level,
                    fire_rate: 0.1,
                    base_damage,
                    last_fired: -10.0,
                },
                EquippedWeapon { weapon_type: WeaponType::Laser },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            ));

            app.init_resource::<Time>();
            app.update();

            let mut laser_query = app.world_mut().query::<&LaserBeam>();
            let lasers: Vec<_> = laser_query.iter(app.world()).collect();
            assert_eq!(
                lasers[0].damage, expected_damage,
                "Level {} with base {} should have damage {}",
                level, base_damage, expected_damage
            );
        }
    }

    #[test]
    fn test_laser_weapon_damage_reduces_enemy_health() {
        use crate::laser::systems::laser_beam_collision_system;
        use crate::combat::{apply_damage_system, DamageEvent, Health};

        let mut app = App::new();
        app.add_message::<DamageEvent>();
        app.add_systems(
            Update,
            (
                weapon_firing_system,
                laser_beam_collision_system,
                apply_damage_system,
            ).chain(),
        );

        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        // Create enemy with 100 HP on the laser's path
        let enemy_entity = app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            Health::new(100.0),
        )).id();

        // Create level 5 laser weapon: damage = 10 * 5 * 1.25 = 62.5
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Laser,
                level: 5,
                fire_rate: 0.1,
                base_damage: 10.0,
                last_fired: -10.0,
            },
            EquippedWeapon { weapon_type: WeaponType::Laser },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        app.init_resource::<Time>();
        app.update();

        // Verify enemy health was reduced by laser damage
        let health = app.world().get::<Health>(enemy_entity).unwrap();
        assert_eq!(
            health.current, 37.5,
            "Enemy health should be 100 - 62.5 = 37.5 after level 5 laser hit"
        );
    }

    #[test]
    fn test_rocket_weapon_spawns_with_correct_damage() {
        use crate::rocket_launcher::components::RocketProjectile;

        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
        ));

        // Create level 3 rocket launcher with base_damage 30.0
        // Expected damage: 30.0 * 3 * 1.25 = 112.5
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::RocketLauncher,
                level: 3,
                fire_rate: 2.0,
                base_damage: 30.0,
                last_fired: -10.0,
            },
            EquippedWeapon { weapon_type: WeaponType::RocketLauncher },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        app.init_resource::<Time>();
        app.update();

        // Verify rocket was spawned with correct damage
        let mut rocket_query = app.world_mut().query::<&RocketProjectile>();
        let rockets: Vec<_> = rocket_query.iter(app.world()).collect();
        assert_eq!(rockets.len(), 1, "One rocket should be spawned");
        assert_eq!(
            rockets[0].damage, 112.5,
            "Rocket damage should be 112.5 (30.0 * 3 * 1.25)"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn weapon_firing_system(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Option<Res<AssetServer>>,
    mut weapon_channel: Option<ResMut<AudioChannel<WeaponSoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
    spell_origin: Res<SpellOrigin>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
    rocket_exhaust: Option<Res<RocketExhaustEffect>>,
    enemy_query: Query<(Entity, &Transform, &Enemy)>,
    mut weapon_query: Query<&mut Weapon>,
) {
    let current_time = time.elapsed_secs();

    // Check if weapon exists
    if weapon_query.is_empty() {
        return; // No weapons to fire
    }

    // Check if Whisper has been collected (weapons enabled)
    let Some(origin_pos) = spell_origin.position else {
        return; // No Whisper = no weapons
    };

    // Extract XZ plane position for targeting calculations
    let origin_xz = from_xz(origin_pos);

    // Find 5 closest enemies to the weapon origin (Whisper)
    // Use XZ plane for distance calculation in 3D world
    let mut enemy_distances: Vec<(Entity, Vec2, f32)> = enemy_query
        .iter()
        .map(|(entity, transform, _)| {
            let pos = from_xz(transform.translation);
            let distance = origin_xz.distance(pos);
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
        if current_time - weapon.last_fired >= weapon.effective_fire_rate() {
            // Select random target from 5 closest
            let mut rng = rand::thread_rng();
            let target_index = rng.gen_range(0..closest_enemies.len());
            let target_pos = closest_enemies[target_index].1;

            // Calculate direction towards target enemy from Whisper position (on XZ plane)
            let base_direction = (target_pos - origin_xz).normalize();

            match &weapon.weapon_type {
                WeaponType::Pistol { .. } => {
                    // Delegate pistol firing to pistol module
                    crate::pistol::systems::fire_pistol(
                        &mut commands,
                        &weapon,
                        origin_pos, // Full 3D position
                        target_pos,
                        asset_server.as_ref(),
                        weapon_channel.as_mut(),
                        sound_limiter.as_mut(),
                        game_meshes.as_deref(),
                        game_materials.as_deref(),
                    );
                }
                 WeaponType::Laser => {
                     // Create a laser beam entity at Whisper's height
                     use crate::laser::components::LaserBeam;
                     commands.spawn(LaserBeam::with_height(origin_xz, base_direction, weapon.damage(), origin_pos.y));

                     // Play laser sound effect
                     if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
                         (asset_server.as_ref(), weapon_channel.as_mut(), sound_limiter.as_mut()) {
                         crate::audio::plugin::play_limited_sound_with_volume(
                             weapon_channel.as_mut(),
                             asset_server,
                             "sounds/72639__chipfork71__laser01rev.wav",
                             sound_limiter.as_mut(),
                             0.7, // Reduce laser firing sound volume by 30%
                         );
                     }
                 }
                 WeaponType::RocketLauncher => {
                     // Find closest enemy for targeting
                     let rocket_target_pos = closest_enemies.get(target_index).map(|(_, pos)| *pos);

                     // Create a rocket projectile
                     use crate::rocket_launcher::components::{RocketProjectile, RocketHissSound};
                     let (mut rocket, transform) = RocketProjectile::new(origin_pos, base_direction, weapon.damage());
                     rocket.target_position = rocket_target_pos;

                     // Play looped hiss sound and get handle for stopping on explosion
                     let hiss_handle = if let (Some(asset_server), Some(weapon_channel)) =
                         (asset_server.as_ref(), weapon_channel.as_mut()) {
                         Some(weapon_channel
                             .play(asset_server.load("sounds/45131__erh__85-hiss-b.wav"))
                             .looped()
                             .with_volume(Decibels(-6.0)) // Play at reduced volume
                             .handle())
                     } else {
                         None
                     };

                     // Spawn rocket with 3D mesh using GameMeshes/GameMaterials
                     if let (Some(ref meshes), Some(ref materials)) = (&game_meshes, &game_materials) {
                         let mut entity_commands = commands.spawn((
                             rocket,
                             transform,
                             Mesh3d(meshes.rocket.clone()),
                             MeshMaterial3d(materials.rocket_pausing.clone()), // Initial state is pausing
                         ));
                         // Attach hiss sound handle if available
                         if let Some(handle) = hiss_handle {
                             entity_commands.insert(RocketHissSound(handle));
                         }
                         // Attach exhaust particle effect as child
                         if let Some(ref exhaust) = rocket_exhaust {
                             entity_commands.with_children(|parent| {
                                 parent.spawn((
                                     ParticleEffect::new(exhaust.0.clone()),
                                     // Position exhaust at back of rocket (negative Z in local space)
                                     Transform::from_translation(Vec3::new(0.0, 0.0, -0.2)),
                                 ));
                             });
                         }
                     } else {
                         // Fallback: spawn without mesh (for tests)
                         let mut entity_commands = commands.spawn((rocket, transform));
                         if let Some(handle) = hiss_handle {
                             entity_commands.insert(RocketHissSound(handle));
                         }
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