use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use crate::game::resources::FreeCameraState;

// Re-export MessageReader for older Bevy API compatibility
type MouseMotionReader<'w, 's> = MessageReader<'w, 's, MouseMotion>;

/// Sensitivity for camera rotation (radians per pixel of mouse movement)
const MOUSE_SENSITIVITY: f32 = 0.003;
/// Speed for camera movement in free camera mode (units per second)
const CAMERA_MOVE_SPEED: f32 = 15.0;
/// Minimum pitch angle (looking up limit) in radians
const MIN_PITCH: f32 = -std::f32::consts::FRAC_PI_2 + 0.1;
/// Maximum pitch angle (looking down limit) in radians
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.1;

/// System that toggles free camera mode based on right mouse button state.
/// When right mouse button is pressed, initializes yaw/pitch from current camera rotation.
/// When released, camera returns to following player.
pub fn free_camera_toggle(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut free_camera: ResMut<FreeCameraState>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Right) {
        free_camera.active = true;

        // Initialize yaw/pitch from current camera orientation
        if let Ok(transform) = camera_query.single() {
            let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            free_camera.yaw = yaw;
            free_camera.pitch = pitch;
        }
    } else if mouse_button_input.just_released(MouseButton::Right) {
        free_camera.active = false;
    }
}

/// System that rotates the camera based on mouse movement when in free camera mode.
/// Uses yaw (horizontal) and pitch (vertical) rotation stored in FreeCameraState.
pub fn free_camera_rotation(
    mut mouse_motion: MouseMotionReader,
    mut free_camera: ResMut<FreeCameraState>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    if !free_camera.active {
        return;
    }

    let mut delta = Vec2::ZERO;
    for motion in mouse_motion.read() {
        delta += motion.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    // Update yaw and pitch based on mouse movement
    free_camera.yaw -= delta.x * MOUSE_SENSITIVITY;
    free_camera.pitch -= delta.y * MOUSE_SENSITIVITY;

    // Clamp pitch to prevent camera flipping
    free_camera.pitch = free_camera.pitch.clamp(MIN_PITCH, MAX_PITCH);

    // Apply rotation to camera
    if let Ok(mut transform) = camera_query.single_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            free_camera.yaw,
            free_camera.pitch,
            0.0,
        );
    }
}

/// System that moves the camera with WASD keys when in free camera mode.
/// Movement is relative to camera's current facing direction.
pub fn free_camera_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    free_camera: Res<FreeCameraState>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    if !free_camera.active {
        return;
    }

    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    // Calculate movement direction based on input
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction += *transform.forward();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction += *transform.back();
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction += *transform.left();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction += *transform.right();
    }

    // Normalize and apply movement
    if direction != Vec3::ZERO {
        direction = direction.normalize();
        transform.translation += direction * CAMERA_MOVE_SPEED * time.delta_secs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    // Helper to create a test app with required resources
    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<FreeCameraState>();
        app.add_message::<MouseMotion>();
        app
    }

    // free_camera_toggle tests
    #[test]
    fn test_free_camera_toggle_activates_on_right_click() {
        let mut app = setup_test_app();

        // Create a camera
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 20.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        // Simulate right mouse button press
        {
            let mut mouse_input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse_input.press(MouseButton::Right);
        }

        let _ = app.world_mut().run_system_once(free_camera_toggle);

        let free_camera = app.world().resource::<FreeCameraState>();
        assert!(free_camera.active, "Free camera should be active after right click");
    }

    #[test]
    fn test_free_camera_toggle_deactivates_on_release() {
        let mut app = setup_test_app();

        // Set camera to active state
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create a camera
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 20.0, 15.0),
        ));

        // First press the button
        {
            let mut mouse_input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse_input.press(MouseButton::Right);
        }
        // Clear the just_pressed state
        {
            let mut mouse_input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse_input.clear();
        }
        // Now release the button
        {
            let mut mouse_input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse_input.release(MouseButton::Right);
        }

        let _ = app.world_mut().run_system_once(free_camera_toggle);

        let free_camera = app.world().resource::<FreeCameraState>();
        assert!(!free_camera.active, "Free camera should be inactive after release");
    }

    #[test]
    fn test_free_camera_toggle_initializes_yaw_pitch_from_camera() {
        let mut app = setup_test_app();

        // Create a camera with specific rotation
        let initial_rotation = Quat::from_euler(EulerRot::YXZ, 0.5, -0.3, 0.0);
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 20.0, 15.0).with_rotation(initial_rotation),
        ));

        // Simulate right mouse button press
        {
            let mut mouse_input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse_input.press(MouseButton::Right);
        }

        let _ = app.world_mut().run_system_once(free_camera_toggle);

        let free_camera = app.world().resource::<FreeCameraState>();
        assert!((free_camera.yaw - 0.5).abs() < 0.01, "Yaw should be initialized from camera");
        assert!((free_camera.pitch - (-0.3)).abs() < 0.01, "Pitch should be initialized from camera");
    }

    // free_camera_rotation tests
    #[test]
    fn test_free_camera_rotation_does_nothing_when_inactive() {
        let mut app = setup_test_app();

        // Create a camera
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 20.0, 15.0),
        )).id();

        // Free camera is inactive by default
        assert!(!app.world().resource::<FreeCameraState>().active);

        // Send mouse motion event
        app.world_mut().write_message(MouseMotion { delta: Vec2::new(100.0, 50.0) });

        let _ = app.world_mut().run_system_once(free_camera_rotation);

        // Camera rotation should not have changed
        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(transform.rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_free_camera_rotation_updates_yaw_pitch() {
        let mut app = setup_test_app();

        // Activate free camera mode
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
            free_camera.yaw = 0.0;
            free_camera.pitch = 0.0;
        }

        // Create a camera
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 20.0, 15.0),
        ));

        // Send mouse motion event
        app.world_mut().write_message(MouseMotion { delta: Vec2::new(100.0, 50.0) });

        let _ = app.world_mut().run_system_once(free_camera_rotation);

        let free_camera = app.world().resource::<FreeCameraState>();
        // Yaw should decrease (moving mouse right rotates camera left in terms of yaw)
        assert!(free_camera.yaw < 0.0, "Yaw should decrease with positive mouse X");
        // Pitch should decrease (moving mouse down rotates camera down)
        assert!(free_camera.pitch < 0.0, "Pitch should decrease with positive mouse Y");
    }

    #[test]
    fn test_free_camera_rotation_clamps_pitch() {
        let mut app = setup_test_app();

        // Activate free camera with extreme pitch
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
            free_camera.yaw = 0.0;
            free_camera.pitch = 0.0;
        }

        // Create a camera
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 20.0, 15.0),
        ));

        // Send extreme mouse motion to try to flip camera
        app.world_mut().write_message(MouseMotion { delta: Vec2::new(0.0, 10000.0) });

        let _ = app.world_mut().run_system_once(free_camera_rotation);

        let free_camera = app.world().resource::<FreeCameraState>();
        assert!(free_camera.pitch >= MIN_PITCH, "Pitch should be clamped to MIN_PITCH");
        assert!(free_camera.pitch <= MAX_PITCH, "Pitch should be clamped to MAX_PITCH");
    }

    // free_camera_movement tests
    #[test]
    fn test_free_camera_movement_does_nothing_when_inactive() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        let initial_pos = Vec3::new(0.0, 20.0, 15.0);
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_translation(initial_pos),
        )).id();

        // Free camera is inactive by default
        // Press W key
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyW);
        }

        // Advance time
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert_eq!(transform.translation, initial_pos, "Camera should not move when inactive");
    }

    #[test]
    fn test_free_camera_movement_w_moves_forward() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        // Activate free camera
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create camera looking down negative Z
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y),
        )).id();

        // Press W key
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyW);
        }

        // Advance time by 1 second
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        // Camera should have moved in negative Z direction (forward)
        assert!(transform.translation.z < 0.0, "Camera should move forward (negative Z)");
    }

    #[test]
    fn test_free_camera_movement_s_moves_backward() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        // Activate free camera
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create camera looking down negative Z
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y),
        )).id();

        // Press S key
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyS);
        }

        // Advance time
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert!(transform.translation.z > 0.0, "Camera should move backward (positive Z)");
    }

    #[test]
    fn test_free_camera_movement_a_moves_left() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        // Activate free camera
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create camera looking down negative Z
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y),
        )).id();

        // Press A key
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyA);
        }

        // Advance time
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert!(transform.translation.x < 0.0, "Camera should move left (negative X)");
    }

    #[test]
    fn test_free_camera_movement_d_moves_right() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        // Activate free camera
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create camera looking down negative Z
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y),
        )).id();

        // Press D key
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyD);
        }

        // Advance time
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        assert!(transform.translation.x > 0.0, "Camera should move right (positive X)");
    }

    #[test]
    fn test_free_camera_movement_respects_camera_orientation() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        // Activate free camera
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create camera rotated 90 degrees (looking down positive X)
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(10.0, 0.0, 0.0), Vec3::Y),
        )).id();

        // Press W key
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyW);
        }

        // Advance time
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        // Camera is looking at +X, so forward should move in positive X direction
        assert!(transform.translation.x > 0.0, "Camera should move in its forward direction (+X)");
    }

    #[test]
    fn test_free_camera_movement_diagonal() {
        let mut app = setup_test_app();
        app.add_plugins(bevy::time::TimePlugin);

        // Activate free camera
        {
            let mut free_camera = app.world_mut().resource_mut::<FreeCameraState>();
            free_camera.active = true;
        }

        // Create camera looking down negative Z
        let camera_entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y),
        )).id();

        // Press W and D keys for diagonal movement
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyW);
            keyboard.press(KeyCode::KeyD);
        }

        // Advance time
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs(1));
        }

        let _ = app.world_mut().run_system_once(free_camera_movement);

        let transform = app.world().get::<Transform>(camera_entity).unwrap();
        // Should move forward-right (negative Z, positive X)
        assert!(transform.translation.z < 0.0, "Camera should move forward");
        assert!(transform.translation.x > 0.0, "Camera should move right");

        // Diagonal movement should be normalized (same speed as single direction)
        let distance = transform.translation.length();
        assert!((distance - CAMERA_MOVE_SPEED).abs() < 0.1,
            "Diagonal movement should be normalized: {} vs {}", distance, CAMERA_MOVE_SPEED);
    }
}
