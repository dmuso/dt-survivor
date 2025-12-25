use bevy::prelude::*;

use crate::movement::components::Velocity;

/// System that applies velocity to transform.
/// Any entity with both a Transform and Velocity component will be moved.
pub fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in query.iter_mut() {
        let movement = velocity.value() * time.delta_secs();
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
}
