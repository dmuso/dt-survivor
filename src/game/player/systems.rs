use bevy::prelude::*;

use crate::game::player::components::*;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_camera_follow_player_basic() {
        let mut app = App::new();

        // Create player at position (50.0, 25.0, 0.0)
        app.world_mut().spawn((
            Player { speed: 200.0 },
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
}

pub fn player_movement(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut player_query: Query<(&mut Transform, &Player)>,
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

        for (mut transform, player) in player_query.iter_mut() {
            let player_pos = transform.translation.truncate();
            let direction = (world_position - player_pos).normalize();

            // Move player towards cursor
            let movement = direction * player.speed * time.delta_secs();
            transform.translation += movement.extend(0.0);
        }
    }
}

pub fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    if let Ok(player_transform) = player_query.single() {
        for mut camera_transform in camera_query.iter_mut() {
            // Keep camera centered on player
            camera_transform.translation.x = player_transform.translation.x;
            camera_transform.translation.y = player_transform.translation.y;
        }
    }
}