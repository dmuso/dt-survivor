use bevy::prelude::*;
use crate::states::*;
use crate::experience::systems::*;
use crate::experience::resources::*;

pub fn plugin(app: &mut App) {
    app
        .init_resource::<ExperienceRequirements>()
        .add_systems(
            Update,
            (
                experience_orb_movement_system.after(crate::loot::systems::loot_attraction_system),
                experience_orb_collection_system.after(experience_orb_movement_system),
                update_player_level_display_system,
            )
                .run_if(in_state(GameState::InGame))
        );
}