use bevy::prelude::*;

use super::components::Invincibility;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
