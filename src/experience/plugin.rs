use bevy::prelude::*;

use crate::experience::components::PlayerLevelUpEvent;
use crate::experience::systems::*;
use crate::states::*;

pub fn plugin(app: &mut App) {
    app.add_message::<PlayerLevelUpEvent>().add_systems(
        Update,
        (
            experience_orb_movement_system.after(crate::loot::systems::update_item_attraction),
            experience_orb_collection_system.after(experience_orb_movement_system),
            update_player_level_display_system,
        )
            .run_if(in_state(GameState::InGame)),
    );
}