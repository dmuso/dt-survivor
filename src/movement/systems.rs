use bevy::prelude::*;

use crate::enemies::components::Enemy;
use crate::game::resources::PlayerPosition;
use crate::movement::components::{Knockback, Velocity};
use crate::player::components::{Player, SlowModifier};

/// System that applies velocity to transform.
/// Any entity with both a Transform and Velocity component will be moved.
pub fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in query.iter_mut() {
        let movement = velocity.value() * time.delta_secs();
        transform.translation += movement.extend(0.0);
    }
}

/// System that applies and decays knockback forces.
/// Ticks the knockback timer, applies velocity, and removes finished knockbacks.
pub fn apply_knockback(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Knockback)>,
) {
    for (entity, mut transform, mut knockback) in query.iter_mut() {
        // Apply knockback velocity
        let movement = knockback.velocity() * time.delta_secs();
        transform.translation += movement.extend(0.0);

        // Tick the knockback timer
        knockback.tick(time.delta());

        // Remove knockback when finished
        if knockback.is_finished() {
            commands.entity(entity).remove::<Knockback>();
        }
    }
}

/// System that moves the player towards the mouse cursor when left mouse button is pressed.
/// Takes into account slow modifiers when calculating effective speed.
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

/// System that moves enemies towards the player position.
pub fn enemy_movement_system(
    mut enemy_query: Query<(&mut Transform, &Enemy)>,
    player_position: Res<PlayerPosition>,
    time: Res<Time>,
) {
    let player_pos = player_position.0;

    for (mut transform, enemy) in enemy_query.iter_mut() {
        let enemy_pos = transform.translation.truncate();
        let direction = (player_pos - enemy_pos).normalize();

        // Move enemy towards player
        let movement = direction * enemy.speed * time.delta_secs();
        transform.translation += movement.extend(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::components::Speed;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;
    use std::time::Duration;

    #[test]
    fn test_apply_velocity_moves_entity() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with transform and velocity
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                Velocity::new(Vec2::new(100.0, 50.0)),
            ))
            .id();

        // Advance time by 1 second
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Entity should have moved by velocity * time
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.x, 100.0);
        assert_eq!(transform.translation.y, 50.0);
        assert_eq!(transform.translation.z, 0.0);
    }

    #[test]
    fn test_apply_velocity_with_zero_velocity() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with zero velocity
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(10.0, 20.0, 0.0)),
                Velocity::new(Vec2::ZERO),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Entity should not have moved
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.x, 10.0);
        assert_eq!(transform.translation.y, 20.0);
    }

    #[test]
    fn test_apply_velocity_preserves_z() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with z offset
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)),
                Velocity::new(Vec2::new(100.0, 100.0)),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Z should be preserved
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.z, 5.0);
    }

    #[test]
    fn test_apply_velocity_multiple_entities() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create multiple entities with different velocities
        let entity1 = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Velocity::new(Vec2::new(100.0, 0.0)),
            ))
            .id();

        let entity2 = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Velocity::new(Vec2::new(0.0, 200.0)),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Both entities should have moved according to their velocities
        let transform1 = app.world().get::<Transform>(entity1).unwrap();
        assert_eq!(transform1.translation.x, 100.0);
        assert_eq!(transform1.translation.y, 0.0);

        let transform2 = app.world().get::<Transform>(entity2).unwrap();
        assert_eq!(transform2.translation.x, 0.0);
        assert_eq!(transform2.translation.y, 200.0);
    }

    #[test]
    fn test_apply_velocity_fractional_time() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Velocity::new(Vec2::new(100.0, 100.0)),
            ))
            .id();

        // Advance time by 0.5 seconds
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(500));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Entity should have moved half the velocity
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.x, 50.0);
        assert_eq!(transform.translation.y, 50.0);
    }

    #[test]
    fn test_entity_without_velocity_not_affected() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with only transform and speed (no velocity)
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(10.0, 20.0, 0.0)),
                Speed::new(100.0),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Entity should not have moved (no velocity component)
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.x, 10.0);
        assert_eq!(transform.translation.y, 20.0);
    }

    #[test]
    fn test_apply_velocity_negative_direction() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(100.0, 100.0, 0.0)),
                Velocity::new(Vec2::new(-50.0, -25.0)),
            ))
            .id();

        // Advance time by 1 second
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(apply_velocity);

        // Entity should have moved in negative direction
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.x, 50.0);
        assert_eq!(transform.translation.y, 75.0);
    }

    // apply_knockback tests
    #[test]
    fn test_apply_knockback_moves_entity() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with knockback
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Knockback::new(Vec2::new(1.0, 0.0), 200.0, 0.5),
            ))
            .id();

        // Advance time by 0.1 seconds
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(100));
        }

        let _ = app.world_mut().run_system_once(apply_knockback);

        // Entity should have moved by knockback velocity * time
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert!((transform.translation.x - 20.0).abs() < 0.01);
        assert_eq!(transform.translation.y, 0.0);
    }

    #[test]
    fn test_apply_knockback_removes_when_finished() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with short knockback
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Knockback::new(Vec2::new(1.0, 0.0), 200.0, 0.1),
            ))
            .id();

        // Advance time past knockback duration
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(150));
        }

        let _ = app.world_mut().run_system_once(apply_knockback);

        // Knockback component should be removed
        assert!(
            app.world().get::<Knockback>(entity).is_none(),
            "Knockback should be removed when finished"
        );
    }

    #[test]
    fn test_apply_knockback_preserves_z() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity at z = 5.0
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)),
                Knockback::new(Vec2::new(1.0, 1.0), 100.0, 0.5),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(100));
        }

        let _ = app.world_mut().run_system_once(apply_knockback);

        // Z should be preserved
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.z, 5.0);
    }

    #[test]
    fn test_apply_knockback_not_finished_keeps_component() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());

        // Create entity with longer knockback
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Knockback::new(Vec2::new(1.0, 0.0), 200.0, 0.5),
            ))
            .id();

        // Advance time but not past duration
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(100));
        }

        let _ = app.world_mut().run_system_once(apply_knockback);

        // Knockback component should still exist
        assert!(
            app.world().get::<Knockback>(entity).is_some(),
            "Knockback should remain when not finished"
        );
    }

    // enemy_movement_system tests
    #[test]
    fn test_enemy_movement_towards_player() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.init_resource::<PlayerPosition>();

        // Set player position
        {
            let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
            player_pos.0 = Vec2::new(100.0, 100.0);
        }

        // Create enemy at origin
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Enemy {
                    speed: 100.0,
                    strength: 10.0,
                },
            ))
            .id();

        // Advance time by 1 second
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(enemy_movement_system);

        // Enemy should have moved towards player
        let transform = app.world().get::<Transform>(entity).unwrap();
        // Direction is normalized (1, 1) -> (0.707, 0.707), speed is 100
        let expected_distance = 100.0 * std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (transform.translation.x - expected_distance).abs() < 0.1,
            "Expected x={}, got {}",
            expected_distance,
            transform.translation.x
        );
        assert!(
            (transform.translation.y - expected_distance).abs() < 0.1,
            "Expected y={}, got {}",
            expected_distance,
            transform.translation.y
        );
    }

    #[test]
    fn test_enemy_movement_respects_speed() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.init_resource::<PlayerPosition>();

        // Set player position directly to the right
        {
            let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
            player_pos.0 = Vec2::new(100.0, 0.0);
        }

        // Create enemy with specific speed
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
            ))
            .id();

        // Advance time by 1 second
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(enemy_movement_system);

        // Enemy should have moved 50 units (speed * time)
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert!((transform.translation.x - 50.0).abs() < 0.01);
        assert_eq!(transform.translation.y, 0.0);
    }

    #[test]
    fn test_enemy_movement_multiple_enemies() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.init_resource::<PlayerPosition>();

        // Set player position
        {
            let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
            player_pos.0 = Vec2::new(0.0, 0.0);
        }

        // Create enemies at different positions
        let entity1 = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
            ))
            .id();

        let entity2 = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 100.0, 0.0)),
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
            ))
            .id();

        // Advance time by 1 second
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(enemy_movement_system);

        // Both enemies should have moved towards player
        let transform1 = app.world().get::<Transform>(entity1).unwrap();
        assert!(
            transform1.translation.x < 100.0,
            "Enemy 1 should move left towards player"
        );
        assert_eq!(transform1.translation.x, 50.0); // Moved 50 units left

        let transform2 = app.world().get::<Transform>(entity2).unwrap();
        assert!(
            transform2.translation.y < 100.0,
            "Enemy 2 should move down towards player"
        );
        assert_eq!(transform2.translation.y, 50.0); // Moved 50 units down
    }
}
