use bevy::prelude::*;
use crate::states::*;
use crate::rocket_launcher::systems::*;

pub fn plugin(app: &mut App) {
    app
        .add_systems(
            PostUpdate,
            (
                rocket_spawning_system,
                target_marking_system,
                rocket_movement_system,
                explosion_system,
                area_damage_system,
                update_rocket_visuals,
            )
                .run_if(in_state(GameState::InGame))
        );
}