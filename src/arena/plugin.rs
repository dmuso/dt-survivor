use bevy::prelude::*;

use crate::arena::resources::ArenaBounds;
use crate::arena::systems::{animate_torch_lights, cleanup_arena_walls, load_wall_model, spawn_arena_walls};
use crate::game::sets::GameSet;
use crate::states::GameState;

pub fn plugin(app: &mut App) {
    app.init_resource::<ArenaBounds>()
        .add_systems(Startup, load_wall_model)
        .add_systems(
            OnEnter(GameState::InGame),
            spawn_arena_walls.in_set(GameSet::Spawning),
        )
        .add_systems(
            Update,
            animate_torch_lights
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(OnExit(GameState::InGame), cleanup_arena_walls);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;

    #[test]
    fn plugin_registers_arena_bounds_resource() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.add_plugins(plugin);

        assert!(
            app.world().get_resource::<ArenaBounds>().is_some(),
            "ArenaBounds resource should be registered"
        );
    }
}
