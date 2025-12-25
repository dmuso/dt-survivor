use bevy::prelude::*;

use crate::game::sets::GameSet;
use crate::movement::systems::{apply_knockback, apply_velocity, enemy_movement_system, player_movement};
use crate::states::GameState;

/// Plugin that adds the movement module's systems to the app.
/// Systems run in the GameSet::Movement set during InGame state.
pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            player_movement,
            apply_velocity,
            apply_knockback,
            enemy_movement_system,
        )
            .in_set(GameSet::Movement)
            .run_if(in_state(GameState::InGame)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::resources::PlayerPosition;
    use crate::movement::components::Velocity;
    use std::time::Duration;

    #[test]
    fn test_plugin_can_be_added_to_app() {
        let mut app = App::new();

        // Add required plugins, states, and resources
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_plugins(bevy::input::InputPlugin::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<PlayerPosition>();

        // Configure the game set
        app.configure_sets(
            Update,
            GameSet::Movement.run_if(in_state(GameState::InGame)),
        );

        // Add the movement plugin
        plugin(&mut app);

        // Verify plugin was added by running an update
        app.update();
    }

    #[test]
    fn test_plugin_system_runs_in_game_state() {
        let mut app = App::new();

        // Add required plugins, states, and resources
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_plugins(bevy::input::InputPlugin::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<PlayerPosition>();

        // Configure the game set
        app.configure_sets(
            Update,
            GameSet::Movement.run_if(in_state(GameState::InGame)),
        );

        // Add the movement plugin
        plugin(&mut app);

        // Create entity with velocity
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Velocity::new(Vec2::new(100.0, 0.0)),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        // Run update while in Intro state (default)
        app.update();

        // Entity should NOT have moved (not in InGame state)
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(
            transform.translation.x, 0.0,
            "Entity should not move in Intro state"
        );

        // Transition to InGame state
        app.world_mut()
            .get_resource_mut::<NextState<GameState>>()
            .unwrap()
            .set(GameState::InGame);

        // Need two updates: one to apply state transition, one for systems to run
        app.update();

        // Advance time again
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        app.update();

        // Entity should have moved (now in InGame state)
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert!(
            transform.translation.x > 0.0,
            "Entity should move in InGame state"
        );
    }

    #[test]
    fn test_plugin_system_does_not_run_in_other_states() {
        let mut app = App::new();

        // Add required plugins, states, and resources
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_plugins(bevy::input::InputPlugin::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<PlayerPosition>();

        // Configure the game set
        app.configure_sets(
            Update,
            GameSet::Movement.run_if(in_state(GameState::InGame)),
        );

        // Add the movement plugin
        plugin(&mut app);

        // Create entity with velocity
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                Velocity::new(Vec2::new(100.0, 0.0)),
            ))
            .id();

        // Advance time
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_secs(1));
        }

        // Transition to GameOver state
        app.world_mut()
            .get_resource_mut::<NextState<GameState>>()
            .unwrap()
            .set(GameState::GameOver);

        app.update();
        app.update();

        // Entity should NOT have moved (in GameOver state, not InGame)
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(
            transform.translation.x, 0.0,
            "Entity should not move in GameOver state"
        );
    }
}
