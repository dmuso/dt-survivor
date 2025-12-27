use bevy::prelude::*;
use crate::rocket_launcher::components::*;
use crate::prelude::*;
use crate::game::events::EnemyDeathEvent;
use crate::game::resources::{GameMeshes, GameMaterials};
use crate::movement::components::{from_xz, to_xz};

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
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
) {
    // Remove expired target markers
    for marker_entity in target_marker_query.iter() {
        commands.entity(marker_entity).despawn();
    }

    // Create new target markers for rockets in targeting state
    for rocket in rocket_query.iter() {
        if matches!(rocket.state, RocketState::Targeting) {
            if let Some(target_pos) = rocket.target_position {
                // Create red marker at target position on XZ plane
                // target_pos is Vec2 where x=X, y=Z (XZ coordinates)
                let marker_pos = to_xz(target_pos) + Vec3::new(0.0, 0.1, 0.0); // Slightly above ground
                commands.spawn((
                    Mesh3d(game_meshes.target_marker.clone()),
                    MeshMaterial3d(game_materials.target_marker.clone()),
                    Transform::from_translation(marker_pos),
                    TargetMarker::position_only(),
                ));
            }
        }
    }
}

/// Rocket movement system that uses XZ plane for targeting and movement.
/// Y axis is height, rockets move on the ground plane.
pub fn rocket_movement_system(
    mut commands: Commands,
    time: Res<Time>,
    mut rocket_query: Query<(Entity, &mut RocketProjectile, &mut Transform)>,
    game_meshes: Res<GameMeshes>,
    game_materials: Res<GameMaterials>,
) {
    let mut rockets_to_explode = Vec::new();

    for (rocket_entity, mut rocket, mut transform) in rocket_query.iter_mut() {
        // Extract XZ position from 3D transform
        let rocket_pos = from_xz(transform.translation);

        match rocket.state {
            RocketState::Targeting => {
                // Transition to homing if we have a target position
                if rocket.target_position.is_some() {
                    rocket.state = RocketState::Homing;
                    if let Some(target_pos) = rocket.target_position {
                        // Calculate initial direction toward target on XZ plane
                        let direction = (target_pos - rocket_pos).normalize();
                        rocket.velocity = direction * rocket.speed;
                    }
                }
            }
            RocketState::Homing => {
                if let Some(target_pos) = rocket.target_position {
                    // Calculate desired direction on XZ plane
                    let to_target = target_pos - rocket_pos;
                    let distance = to_target.length();

                    // Explosion distance scaled for 3D world units
                    if distance < 2.0 {
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

                // Move rocket on XZ plane
                let movement = rocket.velocity * time.delta_secs();
                transform.translation += to_xz(movement);
            }
            _ => {}
        }
    }

    // Handle explosions
    for (rocket_entity, explosion_pos, damage) in rockets_to_explode {
        commands.entity(rocket_entity).despawn();

        // Create explosion at XZ position (Y at ground level)
        let explosion_translation = to_xz(explosion_pos) + Vec3::new(0.0, 0.2, 0.0);
        commands.spawn((
            Mesh3d(game_meshes.explosion.clone()),
            MeshMaterial3d(game_materials.explosion.clone()),
            Transform::from_translation(explosion_translation).with_scale(Vec3::ZERO), // Start at zero size
            Explosion::new(explosion_pos, damage),
        ));
    }
}

pub fn explosion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut explosion_query: Query<(Entity, &mut Explosion, &mut Transform)>,
) {
    for (entity, mut explosion, mut transform) in explosion_query.iter_mut() {
        explosion.lifetime.tick(time.delta());

        // Expand radius
        if explosion.is_expanding() {
            explosion.current_radius += explosion.expansion_rate * time.delta_secs();
            explosion.current_radius = explosion.current_radius.min(explosion.max_radius);
        }

        // Update visual - scale the sphere mesh based on current radius
        // The base explosion mesh is a unit sphere (radius 1.0), scale to current radius
        transform.scale = Vec3::splat(explosion.current_radius);

        // Despawn when fully expanded and faded
        if explosion.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}


/// Area damage system that checks explosion radius on XZ plane.
/// Y axis (height) is ignored for damage radius checks.
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
                // Extract XZ position for distance calculation
                let enemy_pos = from_xz(enemy_transform.translation);
                let distance = explosion.center.distance(enemy_pos);

                if distance <= explosion.current_radius {
                    enemies_to_kill.push(enemy_entity);
                }
            }

            // Kill enemies in explosion radius
            for enemy_entity in enemies_to_kill {
                // Get enemy position for event
                let enemy_pos = enemy_query.get(enemy_entity).map(|(_, transform)| transform.translation).unwrap_or(Vec3::ZERO);

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
    game_materials: Res<GameMaterials>,
) {
    for (entity, rocket) in rocket_query.iter() {
        let material = match rocket.state {
            RocketState::Pausing => game_materials.rocket_pausing.clone(),
            RocketState::Targeting => game_materials.rocket_targeting.clone(),
            RocketState::Homing => game_materials.rocket_homing.clone(),
            RocketState::Exploding => game_materials.rocket_exploding.clone(),
        };

        commands.entity(entity).insert(MeshMaterial3d(material));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon::components::{Weapon, WeaponType};
    use crate::loot::components::{DroppedItem, PickupState, ItemData};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

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

        let loot = DroppedItem {
            pickup_state: PickupState::Idle,
            item_data: ItemData::Weapon(weapon.clone()),
            velocity: Vec2::ZERO,
        };

        // Verify item data is weapon
        match &loot.item_data {
            ItemData::Weapon(loot_weapon) => {
                assert!(matches!(loot_weapon.weapon_type, WeaponType::RocketLauncher));
                assert_eq!(loot_weapon.fire_rate, 10.0);
                assert_eq!(loot_weapon.base_damage, 30.0);
            }
            _ => panic!("Expected weapon item data"),
        }
    }

    #[test]
    fn test_area_damage_uses_xz_plane() {
        #[derive(Resource, Clone)]
        struct DeathEventCounter(Arc<AtomicUsize>);

        fn count_death_events(
            mut events: MessageReader<EnemyDeathEvent>,
            counter: Res<DeathEventCounter>,
        ) {
            for _ in events.read() {
                counter.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut app = App::new();
        let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));
        app.insert_resource(counter.clone());
        app.init_resource::<crate::score::Score>();
        app.add_message::<EnemyDeathEvent>();
        app.add_systems(Update, (area_damage_system, count_death_events).chain());

        // Create explosion at origin with radius 5.0
        app.world_mut().spawn(Explosion {
            center: Vec2::ZERO,
            damage: 50.0,
            current_radius: 5.0,
            max_radius: 10.0,
            expansion_rate: 50.0,
            lifetime: Timer::from_seconds(0.5, TimerMode::Once),
            max_lifetime: 0.5,
        });

        // Create enemy close on XZ plane but at different Y height - should be killed
        // XZ distance = sqrt(3^2 + 0^2) = 3 < 5 (within radius)
        let enemy_entity = app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)), // Far in Y, close in XZ
        )).id();

        app.update();

        // Enemy should be killed (XZ distance is within radius, Y is ignored)
        assert!(!app.world().entities().contains(enemy_entity), "Enemy should be despawned");
        assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Death event should be sent");
    }

    #[test]
    fn test_area_damage_on_z_axis() {
        #[derive(Resource, Clone)]
        struct DeathEventCounter(Arc<AtomicUsize>);

        fn count_death_events(
            mut events: MessageReader<EnemyDeathEvent>,
            counter: Res<DeathEventCounter>,
        ) {
            for _ in events.read() {
                counter.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut app = App::new();
        let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));
        app.insert_resource(counter.clone());
        app.init_resource::<crate::score::Score>();
        app.add_message::<EnemyDeathEvent>();
        app.add_systems(Update, (area_damage_system, count_death_events).chain());

        // Create explosion at (0, 0) in XZ coordinates with radius 5.0
        app.world_mut().spawn(Explosion {
            center: Vec2::ZERO,
            damage: 50.0,
            current_radius: 5.0,
            max_radius: 10.0,
            expansion_rate: 50.0,
            lifetime: Timer::from_seconds(0.5, TimerMode::Once),
            max_lifetime: 0.5,
        });

        // Create enemy at (0, y, 4) - within radius on Z axis
        // XZ distance = sqrt(0^2 + 4^2) = 4 < 5 (within radius)
        let enemy_entity = app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(0.0, 0.375, 4.0)), // Close in Z
        )).id();

        app.update();

        // Enemy should be killed
        assert!(!app.world().entities().contains(enemy_entity), "Enemy should be killed on Z axis");
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_area_damage_outside_radius() {
        let mut app = App::new();
        app.init_resource::<crate::score::Score>();
        app.add_message::<EnemyDeathEvent>();
        app.add_systems(Update, area_damage_system);

        // Create explosion at origin with radius 5.0
        app.world_mut().spawn(Explosion {
            center: Vec2::ZERO,
            damage: 50.0,
            current_radius: 5.0,
            max_radius: 10.0,
            expansion_rate: 50.0,
            lifetime: Timer::from_seconds(0.5, TimerMode::Once),
            max_lifetime: 0.5,
        });

        // Create enemy far away on XZ plane - outside radius
        // XZ distance = sqrt(10^2 + 0^2) = 10 > 5 (outside radius)
        let enemy_entity = app.world_mut().spawn((
            Enemy { speed: 50.0, strength: 10.0 },
            Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
        )).id();

        app.update();

        // Enemy should survive
        assert!(app.world().entities().contains(enemy_entity), "Enemy outside radius should survive");
    }
}