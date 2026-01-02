use bevy::prelude::*;
use crate::inventory::bag::*;
use crate::inventory::resources::*;

pub fn plugin(app: &mut App) {
    app
        .init_resource::<SpellList>()
        .init_resource::<InventoryBag>();
}
