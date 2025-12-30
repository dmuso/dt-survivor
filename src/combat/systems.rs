use bevy::prelude::*;

use super::components::{DamageFlash, Health, Invincibility};
use super::events::{DamageEvent, DeathEvent, EntityType};
use crate::enemies::components::Enemy;
use crate::game::components::Level;
use crate::game::events::EnemyDeathEvent;
use crate::game::resources::DamageFlashMaterial;
use crate::score::Score;

/// Marker component indicating an entity should have death checked
/// Entities with Health and this component will be checked for death
#[derive(Component)]
pub struct CheckDeath;

/// System to apply damage from DamageEvents to entities with Health
pub fn apply_damage_system(
    mut messages: MessageReader<DamageEvent>,
    mut query: Query<(&mut Health, Option<&Invincibility>)>,
) {
    for event in messages.read() {
        if let Ok((mut health, invincibility)) = query.get_mut(event.target) {
            // Skip if invincible
            if invincibility.is_some() {
                continue;
            }
            health.take_damage(event.amount);
        }
    }
}

/// System to check for dead entities and fire DeathEvents
pub fn check_death_system(
    query: Query<(Entity, &Health, &Transform, &CheckDeath)>,
    mut messages: MessageWriter<DeathEvent>,
) {
    for (entity, health, transform, _) in query.iter() {
        if health.is_dead() {
            messages.write(DeathEvent::new(
                entity,
                transform.translation,
                EntityType::Enemy, // Default to enemy; specific handlers can override
            ));
        }
    }
}

/// System to tick invincibility timers and remove expired ones
pub fn tick_invincibility_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Invincibility)>,
) {
    for (entity, mut invincibility) in query.iter_mut() {
        invincibility.tick(time.delta());
        if invincibility.is_expired() {
            commands.entity(entity).remove::<Invincibility>();
        }
    }
}

/// System to handle enemy death events from the combat system
/// Despawns enemies, updates score, and fires EnemyDeathEvent for loot/experience
pub fn handle_enemy_death_system(
    mut commands: Commands,
    mut death_events: MessageReader<DeathEvent>,
    mut enemy_death_events: MessageWriter<EnemyDeathEvent>,
    mut score: ResMut<Score>,
    level_query: Query<&Level>,
) {
    for event in death_events.read() {
        if event.entity_type == EntityType::Enemy {
            // Update score
            score.0 += 1;

            // Get enemy level (default to 1 if not found)
            let enemy_level = level_query
                .get(event.entity)
                .map(|l| l.value())
                .unwrap_or(1);

            // Send EnemyDeathEvent for loot/experience handling
            enemy_death_events.write(EnemyDeathEvent {
                enemy_entity: event.entity,
                position: event.position,
                enemy_level,
            });

            // Despawn the enemy
            commands.entity(event.entity).try_despawn();
        }
    }
}

/// Apply flash effect when damage is dealt to enemies
/// Listens to DamageEvent and applies a white flash material to damaged enemies
#[allow(clippy::type_complexity)]
pub fn apply_damage_flash_system(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    enemy_query: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (With<Enemy>, Without<DamageFlash>),
    >,
    flash_material: Option<Res<DamageFlashMaterial>>,
) {
    // Require flash material resource to function
    let Some(flash_material) = flash_material else {
        return;
    };

    for event in damage_events.read() {
        if let Ok((entity, current_material)) = enemy_query.get(event.target) {
            // Store original material and apply flash
            commands.entity(entity).insert((
                DamageFlash::new(current_material.0.clone(), 0.1),
                MeshMaterial3d(flash_material.0.clone()),
            ));
        }
    }
}

/// Update flash timers and restore original materials when flash ends
pub fn update_damage_flash_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageFlash, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, mut flash, mut material) in query.iter_mut() {
        flash.tick(time.delta());

        if flash.is_finished() {
            // Restore original material and remove flash component
            material.0 = flash.original_material.clone();
            commands.entity(entity).remove::<DamageFlash>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod apply_damage_tests {
        use super::*;

        #[test]
        fn test_apply_damage_reduces_health() {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, apply_damage_system);

            // Spawn entity with health
            let entity = app.world_mut().spawn(Health::new(100.0)).id();

            // Send damage event
            app.world_mut()
                .write_message(DamageEvent::new(entity, 25.0));

            // Run the system
            app.update();

            // Verify health was reduced
            let health = app.world().get::<Health>(entity).unwrap();
            assert_eq!(health.current, 75.0);
        }

        #[test]
        fn test_apply_damage_skips_invincible_entities() {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, apply_damage_system);

            // Spawn invincible entity with health
            let entity = app
                .world_mut()
                .spawn((Health::new(100.0), Invincibility::new(5.0)))
                .id();

            // Send damage event
            app.world_mut()
                .write_message(DamageEvent::new(entity, 50.0));

            // Run the system
            app.update();

            // Verify health was NOT reduced
            let health = app.world().get::<Health>(entity).unwrap();
            assert_eq!(health.current, 100.0);
        }

        #[test]
        fn test_apply_damage_handles_missing_entity() {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, apply_damage_system);

            // Create entity then despawn it
            let entity = app.world_mut().spawn(Health::new(100.0)).id();
            app.world_mut().despawn(entity);

            // Send damage event to despawned entity
            app.world_mut()
                .write_message(DamageEvent::new(entity, 25.0));

            // Should not panic
            app.update();
        }
    }

    mod check_death_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        /// Helper resource to count death events
        #[derive(Resource, Clone)]
        struct DeathEventCounter(Arc<AtomicUsize>);

        /// Helper system to count death events
        fn count_death_events(
            mut events: MessageReader<DeathEvent>,
            counter: Res<DeathEventCounter>,
        ) {
            for _ in events.read() {
                counter.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        #[test]
        fn test_check_death_fires_event_when_dead() {
            let mut app = App::new();
            let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));
            app.add_message::<DeathEvent>();
            app.insert_resource(counter.clone());
            app.add_systems(Update, (check_death_system, count_death_events).chain());

            // Spawn dead entity with transform
            let mut health = Health::new(100.0);
            health.take_damage(100.0); // Kill it
            app.world_mut().spawn((
                health,
                Transform::from_xyz(10.0, 20.0, 0.0),
                CheckDeath,
            ));

            // Run the system
            app.update();

            // Verify death event was fired
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_check_death_does_not_fire_for_living() {
            let mut app = App::new();
            let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));
            app.add_message::<DeathEvent>();
            app.insert_resource(counter.clone());
            app.add_systems(Update, (check_death_system, count_death_events).chain());

            // Spawn alive entity
            app.world_mut().spawn((
                Health::new(100.0),
                Transform::from_xyz(0.0, 0.0, 0.0),
                CheckDeath,
            ));

            // Run the system
            app.update();

            // Verify no death event
            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_check_death_requires_marker() {
            let mut app = App::new();
            let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));
            app.add_message::<DeathEvent>();
            app.insert_resource(counter.clone());
            app.add_systems(Update, (check_death_system, count_death_events).chain());

            // Spawn dead entity WITHOUT CheckDeath marker
            let mut health = Health::new(100.0);
            health.take_damage(100.0);
            app.world_mut()
                .spawn((health, Transform::from_xyz(0.0, 0.0, 0.0)));

            // Run the system
            app.update();

            // Verify no death event (marker required)
            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }
    }

    mod tick_invincibility_tests {
        use super::*;

        #[test]
        fn test_tick_invincibility_system_removes_expired() {
            let mut app = App::new();
            app.init_resource::<Time>();
            app.add_systems(Update, tick_invincibility_system);

            // Spawn entity with short invincibility
            let entity = app.world_mut().spawn(Invincibility::new(0.1)).id();

            // Verify invincibility exists
            assert!(app.world().get::<Invincibility>(entity).is_some());

            // Advance time past the invincibility duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.2));
            }

            // Run the system
            app.update();

            // Verify invincibility was removed
            assert!(app.world().get::<Invincibility>(entity).is_none());
        }

        #[test]
        fn test_tick_invincibility_system_keeps_active() {
            let mut app = App::new();
            app.init_resource::<Time>();
            app.add_systems(Update, tick_invincibility_system);

            // Spawn entity with longer invincibility
            let entity = app.world_mut().spawn(Invincibility::new(10.0)).id();

            // Advance time a little
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            // Run the system
            app.update();

            // Verify invincibility still exists
            assert!(app.world().get::<Invincibility>(entity).is_some());
        }
    }

    mod handle_enemy_death_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        /// Helper resource to count enemy death events
        #[derive(Resource, Clone)]
        struct EnemyDeathEventCounter(Arc<AtomicUsize>);

        /// Helper system to count enemy death events
        fn count_enemy_death_events(
            mut events: MessageReader<EnemyDeathEvent>,
            counter: Res<EnemyDeathEventCounter>,
        ) {
            for _ in events.read() {
                counter.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        #[test]
        fn test_handle_enemy_death_despawns_entity() {
            let mut app = App::new();
            app.add_message::<DeathEvent>();
            app.add_message::<EnemyDeathEvent>();
            app.init_resource::<Score>();
            app.add_systems(Update, handle_enemy_death_system);

            // Spawn enemy entity with transform
            let entity = app
                .world_mut()
                .spawn(Transform::from_xyz(50.0, 100.0, 0.0))
                .id();

            // Send death event for enemy
            app.world_mut().write_message(DeathEvent::new(
                entity,
                Vec3::new(50.0, 100.0, 0.0),
                EntityType::Enemy,
            ));

            // Run the system
            app.update();

            // Verify entity was despawned
            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_handle_enemy_death_updates_score() {
            let mut app = App::new();
            app.add_message::<DeathEvent>();
            app.add_message::<EnemyDeathEvent>();
            app.init_resource::<Score>();
            app.add_systems(Update, handle_enemy_death_system);

            // Spawn enemy entity
            let entity = app
                .world_mut()
                .spawn(Transform::from_xyz(0.0, 0.0, 0.0))
                .id();

            // Send death event for enemy
            app.world_mut().write_message(DeathEvent::new(
                entity,
                Vec3::ZERO,
                EntityType::Enemy,
            ));

            // Run the system
            app.update();

            // Verify score was updated
            let score = app.world().get_resource::<Score>().unwrap();
            assert_eq!(score.0, 1);
        }

        #[test]
        fn test_handle_enemy_death_fires_enemy_death_event() {
            let mut app = App::new();
            let counter = EnemyDeathEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_message::<DeathEvent>();
            app.add_message::<EnemyDeathEvent>();
            app.init_resource::<Score>();
            app.add_systems(
                Update,
                (handle_enemy_death_system, count_enemy_death_events).chain(),
            );

            // Spawn enemy entity
            let entity = app
                .world_mut()
                .spawn(Transform::from_xyz(0.0, 0.0, 0.0))
                .id();

            // Send death event for enemy
            app.world_mut().write_message(DeathEvent::new(
                entity,
                Vec3::ZERO,
                EntityType::Enemy,
            ));

            // Run the system
            app.update();

            // Verify EnemyDeathEvent was fired
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_handle_enemy_death_ignores_non_enemy_deaths() {
            let mut app = App::new();
            app.add_message::<DeathEvent>();
            app.add_message::<EnemyDeathEvent>();
            app.init_resource::<Score>();
            app.add_systems(Update, handle_enemy_death_system);

            // Spawn entity
            let entity = app
                .world_mut()
                .spawn(Transform::from_xyz(0.0, 0.0, 0.0))
                .id();

            // Send death event for non-enemy (player)
            app.world_mut().write_message(DeathEvent::new(
                entity,
                Vec3::ZERO,
                EntityType::Player,
            ));

            // Run the system
            app.update();

            // Verify entity was NOT despawned (not an enemy death)
            assert!(app.world().entities().contains(entity));

            // Verify score was NOT updated
            let score = app.world().get_resource::<Score>().unwrap();
            assert_eq!(score.0, 0);
        }
    }

    mod damage_flash_tests {
        use super::*;
        use crate::game::resources::DamageFlashMaterial;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.add_plugins(bevy::time::TimePlugin::default());
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_apply_damage_flash_applies_flash_to_enemy() {
            let mut app = setup_test_app();

            // Create flash material resource
            let flash_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                })
            };
            app.insert_resource(DamageFlashMaterial(flash_handle.clone()));

            app.add_systems(Update, apply_damage_flash_system);

            // Create enemy material and entity
            let original_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.0),
                    ..default()
                })
            };

            let entity = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    MeshMaterial3d(original_handle.clone()),
                    Transform::default(),
                ))
                .id();

            // Send damage event
            app.world_mut().write_message(DamageEvent::new(entity, 10.0));

            // Run the system
            app.update();

            // Verify DamageFlash component was added
            assert!(
                app.world().get::<DamageFlash>(entity).is_some(),
                "DamageFlash component should be added when enemy takes damage"
            );

            // Verify material was changed to flash material
            let material = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).unwrap();
            assert_eq!(
                material.0, flash_handle,
                "Entity material should be set to flash material"
            );
        }

        #[test]
        fn test_apply_damage_flash_stores_original_material() {
            let mut app = setup_test_app();

            // Create flash material resource
            let flash_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };
            app.insert_resource(DamageFlashMaterial(flash_handle));

            app.add_systems(Update, apply_damage_flash_system);

            // Create enemy material and entity
            let original_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 1.0, 0.0),
                    ..default()
                })
            };

            let entity = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    MeshMaterial3d(original_handle.clone()),
                    Transform::default(),
                ))
                .id();

            // Send damage event
            app.world_mut().write_message(DamageEvent::new(entity, 10.0));
            app.update();

            // Verify original material is stored
            let flash = app.world().get::<DamageFlash>(entity).unwrap();
            assert_eq!(
                flash.original_material, original_handle,
                "Original material should be stored in DamageFlash"
            );
        }

        #[test]
        fn test_apply_damage_flash_skips_already_flashing() {
            let mut app = setup_test_app();

            // Create flash material resource
            let flash_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };
            app.insert_resource(DamageFlashMaterial(flash_handle.clone()));

            app.add_systems(Update, apply_damage_flash_system);

            // Create enemy material
            let original_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };

            // Create enemy that already has DamageFlash
            let entity = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    MeshMaterial3d(flash_handle.clone()),
                    Transform::default(),
                    DamageFlash::new(original_handle.clone(), 0.1),
                ))
                .id();

            // Send damage event
            app.world_mut().write_message(DamageEvent::new(entity, 10.0));
            app.update();

            // Original material should still be the one stored (not overwritten)
            let flash = app.world().get::<DamageFlash>(entity).unwrap();
            assert_eq!(
                flash.original_material, original_handle,
                "Flash component should not be overwritten when already flashing"
            );
        }

        #[test]
        fn test_update_damage_flash_restores_material() {
            let mut app = setup_test_app();
            app.add_systems(Update, update_damage_flash_system);

            // Create materials
            let original_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.0),
                    ..default()
                })
            };
            let flash_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                })
            };

            // Create entity with flash that's about to expire
            let mut flash = DamageFlash::new(original_handle.clone(), 0.1);
            flash.tick(Duration::from_secs_f32(0.15)); // Expire the flash

            let entity = app
                .world_mut()
                .spawn((
                    MeshMaterial3d(flash_handle),
                    flash,
                ))
                .id();

            app.update();

            // Verify material was restored
            let material = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).unwrap();
            assert_eq!(
                material.0, original_handle,
                "Material should be restored to original after flash ends"
            );

            // Verify DamageFlash was removed
            assert!(
                app.world().get::<DamageFlash>(entity).is_none(),
                "DamageFlash component should be removed after flash ends"
            );
        }

        #[test]
        fn test_update_damage_flash_keeps_active_flash() {
            let mut app = setup_test_app();
            app.add_systems(Update, update_damage_flash_system);

            // Create materials
            let original_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };
            let flash_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };

            // Create entity with fresh flash
            let entity = app
                .world_mut()
                .spawn((
                    MeshMaterial3d(flash_handle.clone()),
                    DamageFlash::new(original_handle, 1.0), // 1 second flash
                ))
                .id();

            // Advance a small amount of time
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Verify flash material is still applied
            let material = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).unwrap();
            assert_eq!(
                material.0, flash_handle,
                "Flash material should still be applied while flash is active"
            );

            // Verify DamageFlash is still present
            assert!(
                app.world().get::<DamageFlash>(entity).is_some(),
                "DamageFlash component should remain while flash is active"
            );
        }

        #[test]
        fn test_apply_damage_flash_ignores_non_enemies() {
            let mut app = setup_test_app();

            // Create flash material resource
            let flash_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };
            app.insert_resource(DamageFlashMaterial(flash_handle));

            app.add_systems(Update, apply_damage_flash_system);

            // Create non-enemy entity with material
            let material_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };

            let entity = app
                .world_mut()
                .spawn((
                    MeshMaterial3d(material_handle),
                    Transform::default(),
                    // Note: No Enemy component!
                ))
                .id();

            // Send damage event
            app.world_mut().write_message(DamageEvent::new(entity, 10.0));
            app.update();

            // Verify no DamageFlash was added
            assert!(
                app.world().get::<DamageFlash>(entity).is_none(),
                "DamageFlash should not be added to non-enemy entities"
            );
        }

        #[test]
        fn test_apply_damage_flash_handles_missing_resource() {
            let mut app = setup_test_app();
            // Note: NOT inserting DamageFlashMaterial resource

            app.add_systems(Update, apply_damage_flash_system);

            // Create enemy
            let material_handle = {
                let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial::default())
            };

            let entity = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    MeshMaterial3d(material_handle),
                    Transform::default(),
                ))
                .id();

            // Send damage event
            app.world_mut().write_message(DamageEvent::new(entity, 10.0));

            // System should not panic
            app.update();

            // No flash should be added
            assert!(
                app.world().get::<DamageFlash>(entity).is_none(),
                "DamageFlash should not be added when resource is missing"
            );
        }
    }
}
