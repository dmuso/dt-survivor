use bevy::prelude::*;

use crate::pause::components::{SpellCooldownsVisible, WallLightsEnabled};
use crate::pause::systems::*;
use crate::states::GameState;

pub fn plugin(app: &mut App) {
    app.init_resource::<WallLightsEnabled>()
        .init_resource::<SpellCooldownsVisible>()
        // ESC key to enter pause from InGame
        .add_systems(
            Update,
            enter_pause_input.run_if(in_state(GameState::InGame)),
        )
        // Setup pause menu when entering Paused state
        .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
        // Update systems while in Paused state
        .add_systems(
            Update,
            (
                pause_input,
                pause_menu_interactions,
                debug_button_interactions,
                update_toggle_button_text,
            )
                .run_if(in_state(GameState::Paused)),
        )
        // Cleanup when exiting Paused state
        .add_systems(OnExit(GameState::Paused), cleanup_pause_menu);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_can_be_created() {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));
        app.init_state::<GameState>();
        app.init_resource::<crate::ui::systems::DebugHudVisible>();
        app.init_resource::<SpellCooldownsVisible>();

        // This would panic if the plugin has configuration issues
        app.add_plugins(plugin);
    }
}
