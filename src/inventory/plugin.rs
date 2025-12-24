use bevy::prelude::*;
use crate::inventory::systems::*;
use crate::inventory::resources::*;
use crate::states::*;

pub fn plugin(app: &mut App) {
    app
        .init_resource::<Inventory>()
        .add_systems(Update, inventory_initialization_system.run_if(in_state(GameState::InGame)));
}