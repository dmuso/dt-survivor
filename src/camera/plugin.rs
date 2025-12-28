use bevy::prelude::*;

use crate::game::resources::FreeCameraState;
use crate::game::sets::GameSet;
use crate::states::GameState;
use crate::camera::systems::{free_camera_movement, free_camera_rotation, free_camera_toggle};

pub fn plugin(app: &mut App) {
    app.init_resource::<FreeCameraState>()
        .add_systems(
            Update,
            (
                free_camera_toggle,
                free_camera_rotation,
                free_camera_movement,
            )
                .chain()
                .in_set(GameSet::Input)
                .run_if(in_state(GameState::InGame)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_plugin_initializes_resource() {
        let mut app = App::new();
        app.add_plugins((
            bevy::input::InputPlugin,
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin,
        ));
        app.init_state::<GameState>();
        app.add_plugins(plugin);

        assert!(app.world().get_resource::<FreeCameraState>().is_some());
    }
}
