use bevy::prelude::*;

use crate::combat::components::Health;
use crate::game::resources::{FreeCameraState, PlayerPosition};
use crate::movement::components::from_xz;
use crate::player::components::*;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_camera_follow_player_maintains_isometric_offset() {
        let mut app = App::new();

        // Initialize the PlayerPosition and FreeCameraState resources
        app.init_resource::<crate::game::resources::PlayerPosition>();
        app.init_resource::<crate::game::resources::FreeCameraState>();

        // Create player at position (50.0, 0.5, 25.0) on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(50.0, 0.5, 25.0)),
        ));

        // Create camera at different position
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        )).id();

        // Run camera follow system
        let _ = app.world_mut().run_system_once(camera_follow_player);

        // Camera should follow player with isometric offset (15, 20, 15) - offset on both X and Z
        let camera_transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(camera_transform.translation.x, 65.0);        // Player X + 15.0
        assert_eq!(camera_transform.translation.y, 20.5);        // Player Y + 20.0
        assert_eq!(camera_transform.translation.z, 40.0);        // Player Z + 15.0

        // Verify camera is looking at the player (rotation should point toward player)
        let forward = camera_transform.forward();
        // Camera forward should point generally toward player (negative X and Z from offset)
        assert!(forward.x < 0.0, "Camera should be looking toward negative X");
        assert!(forward.z < 0.0, "Camera should be looking toward negative Z");

        // Also check PlayerPosition was updated with XZ coordinates
        let player_pos = app.world().get_resource::<crate::game::resources::PlayerPosition>().unwrap();
        assert_eq!(player_pos.0.x, 50.0);  // X
        assert_eq!(player_pos.0.y, 25.0);  // Z (from_xz: Vec3.z -> Vec2.y)
    }

    #[test]
    fn test_camera_follow_player_no_movement_when_no_player() {
        let mut app = App::new();

        // Initialize required resources
        app.init_resource::<crate::game::resources::PlayerPosition>();
        app.init_resource::<crate::game::resources::FreeCameraState>();

        // Create camera but no player
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        )).id();

        // Run camera follow system
        let _ = app.world_mut().run_system_once(camera_follow_player);

        // Camera should not have moved (no player to follow)
        let camera_transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(camera_transform.translation, Vec3::new(0.0, 20.0, 15.0));
    }

    #[test]
    fn test_camera_follow_player_skips_when_free_camera_active() {
        let mut app = App::new();

        // Initialize resources
        app.init_resource::<crate::game::resources::PlayerPosition>();
        app.init_resource::<crate::game::resources::FreeCameraState>();

        // Set free camera to active
        {
            let mut free_camera = app.world_mut().resource_mut::<crate::game::resources::FreeCameraState>();
            free_camera.active = true;
        }

        // Create player at position (50.0, 0.5, 25.0)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(50.0, 0.5, 25.0)),
        ));

        // Create camera at initial position
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_translation(Vec3::new(0.0, 20.0, 15.0)),
        )).id();

        // Run camera follow system
        let _ = app.world_mut().run_system_once(camera_follow_player);

        // Camera should NOT have moved (free camera mode is active)
        let camera_transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(camera_transform.translation, Vec3::new(0.0, 20.0, 15.0));

        // But PlayerPosition should still be updated
        let player_pos = app.world().get_resource::<crate::game::resources::PlayerPosition>().unwrap();
        assert_eq!(player_pos.0.x, 50.0);
        assert_eq!(player_pos.0.y, 25.0);
    }

    #[test]
    fn test_player_health_regeneration() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<crate::game::resources::PlayerDamageTimer>();

        // Create player with 50 health (below max)
        let mut health = Health::new(100.0);
        health.take_damage(50.0); // Set to 50 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 2.0, // 2 health per second
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            health,
        )).id();

        // Set damage timer to indicate no recent damage (more than 3 seconds ago)
        {
            let mut timer = app.world_mut().get_resource_mut::<crate::game::resources::PlayerDamageTimer>().unwrap();
            timer.has_taken_damage = true;
            timer.time_since_last_damage = 3.5; // More than 3 seconds since last damage
        }

        // Simulate 0.5 seconds passing
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs_f32(0.5));
        }

        // Run health regeneration system
        let _ = app.world_mut().run_system_once(player_health_regeneration_system);

        // Player should have regenerated 1.0 health (2.0 * 0.5)
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 51.0, "Player should regenerate 1.0 health");

        // Run again for another 0.5 seconds
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs_f32(0.5));
        }
        let _ = app.world_mut().run_system_once(player_health_regeneration_system);

        // Player should now have 52.0 health
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 52.0, "Player should regenerate another 1.0 health");
    }

    #[test]
    fn test_player_health_regeneration_no_regen_when_recent_damage() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<crate::game::resources::PlayerDamageTimer>();

        // Create player with 50 health (below max)
        let mut health = Health::new(100.0);
        health.take_damage(50.0); // Set to 50 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 2.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            health,
        )).id();

        // Set damage timer to indicate recent damage (less than 3 seconds ago)
        {
            let mut timer = app.world_mut().get_resource_mut::<crate::game::resources::PlayerDamageTimer>().unwrap();
            timer.has_taken_damage = true;
            timer.time_since_last_damage = 2.5; // Less than 3 seconds since last damage
        }

        // Simulate 0.5 seconds passing
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs_f32(0.5));
        }

        // Run health regeneration system
        let _ = app.world_mut().run_system_once(player_health_regeneration_system);

        // Player health should remain unchanged due to recent damage
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 50.0, "Player should not regenerate when recent damage taken");
    }

    #[test]
    fn test_player_health_regeneration_capped_at_max() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<crate::game::resources::PlayerDamageTimer>();

        // Create player with 99 health (close to max)
        let mut health = Health::new(100.0);
        health.take_damage(1.0); // Set to 99 health
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 5.0, // Fast regeneration
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            health,
        )).id();

        // Set damage timer to indicate no recent damage
        {
            let mut timer = app.world_mut().get_resource_mut::<crate::game::resources::PlayerDamageTimer>().unwrap();
            timer.has_taken_damage = true;
            timer.time_since_last_damage = 3.5;
        }

        // Simulate 1 second passing
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs_f32(1.0));
        }

        // Run health regeneration system
        let _ = app.world_mut().run_system_once(player_health_regeneration_system);

        // Player should be capped at max health (100.0), not 99.0 + 5.0 = 104.0
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.current, 100.0, "Player health should be capped at max_health");
    }
}

/// Updates PlayerPosition resource and camera to follow the player.
/// PlayerPosition stores XZ coordinates (ground plane) for enemy targeting.
/// Camera maintains isometric offset while following player on XZ plane.
/// Skips camera movement when free camera mode is active (right mouse held).
pub fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    mut player_position: ResMut<PlayerPosition>,
    free_camera: Res<FreeCameraState>,
) {
    if let Ok(player_transform) = player_query.single() {
        // Update PlayerPosition with XZ coordinates (ground plane position)
        let player_pos = from_xz(player_transform.translation);
        player_position.0 = player_pos;

        // Skip camera movement when in free camera mode
        if free_camera.active {
            return;
        }

        // Isometric camera offset - positioned diagonally for proper isometric view
        // Offset on both X and Z axes creates 45-degree viewing angle on ground plane
        let camera_offset = Vec3::new(15.0, 20.0, 15.0);

        for mut camera_transform in camera_query.iter_mut() {
            // Maintain isometric offset while following player on XZ plane
            camera_transform.translation = player_transform.translation + camera_offset;
            // Keep looking at the player to maintain isometric angle
            camera_transform.look_at(player_transform.translation, Vec3::Y);
        }
    }
}

pub fn update_slow_modifiers(
    mut commands: Commands,
    time: Res<Time>,
    mut slow_query: Query<(Entity, &mut SlowModifier)>,
) {
    for (entity, mut slow_modifier) in slow_query.iter_mut() {
        slow_modifier.remaining_duration -= time.delta_secs();

        // Remove expired slow modifiers
        if slow_modifier.remaining_duration <= 0.0 {
            commands.entity(entity).remove::<SlowModifier>();
        }
    }
}

pub fn player_health_regeneration_system(
    time: Res<Time>,
    damage_timer: Res<crate::game::resources::PlayerDamageTimer>,
    mut player_query: Query<(&Player, &mut Health)>,
) {
    // Only regenerate if player hasn't taken damage for at least 3 seconds
    if damage_timer.has_taken_damage && damage_timer.time_since_last_damage < 3.0 {
        return;
    }

    for (player, mut health) in player_query.iter_mut() {
        // Only regenerate if health is below maximum
        if health.current < health.max {
            // Regenerate based on the player's regen_rate (health per second)
            let regen_amount = player.regen_rate * time.delta_secs();
            health.heal(regen_amount);
        }
    }
}