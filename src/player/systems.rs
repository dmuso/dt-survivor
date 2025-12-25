use bevy::prelude::*;

use crate::combat::components::Health;
use crate::player::components::*;
use crate::game::resources::*;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_camera_follow_player_basic() {
        let mut app = App::new();

        // Initialize the PlayerPosition resource
        app.init_resource::<crate::game::resources::PlayerPosition>();

        // Create player at position (50.0, 25.0, 0.0)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(50.0, 25.0, 0.0)),
        ));

        // Create camera at different position
        let camera_entity = app.world_mut().spawn((
            Camera2d,
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Run camera follow system
        let _ = app.world_mut().run_system_once(camera_follow_player);

        // Camera should now follow player position
        let camera_transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(camera_transform.translation.x, 50.0);
        assert_eq!(camera_transform.translation.y, 25.0);
        assert_eq!(camera_transform.translation.z, 0.0);
    }

    #[test]
    fn test_camera_follow_player_no_movement_when_no_player() {
        let mut app = App::new();

        // Create camera but no player
        let camera_entity = app.world_mut().spawn((
            Camera2d,
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Run camera follow system
        let _ = app.world_mut().run_system_once(camera_follow_player);

        // Camera should not have moved
        let camera_transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(camera_transform.translation, Vec3::new(0.0, 0.0, 0.0));
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

pub fn player_movement(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut player_query: Query<(&mut Transform, &Player, Option<&SlowModifier>)>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };

    if let Some(cursor_position) = window.cursor_position() {
        // Convert screen coordinates to world coordinates
        let world_position = camera
            .viewport_to_world(camera_transform, cursor_position)
            .map(|ray| ray.origin.truncate())
            .unwrap_or_default();

        for (mut transform, player, slow_modifier) in player_query.iter_mut() {
            let player_pos = transform.translation.truncate();
            let direction = (world_position - player_pos).normalize();

            // Calculate effective speed considering slow modifiers
            let effective_speed = if let Some(slow) = slow_modifier {
                player.speed * slow.speed_multiplier
            } else {
                player.speed
            };

            // Move player towards cursor
            let movement = direction * effective_speed * time.delta_secs();
            transform.translation += movement.extend(0.0);
        }
    }
}

pub fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    mut player_position: ResMut<PlayerPosition>,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_pos = player_transform.translation.truncate();
        player_position.0 = player_pos;

        for mut camera_transform in camera_query.iter_mut() {
            // Keep camera centered on player
            camera_transform.translation.x = player_transform.translation.x;
            camera_transform.translation.y = player_transform.translation.y;
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