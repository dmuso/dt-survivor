//! Spell slot UI module.
//!
//! This module provides unified components and systems for rendering spell slots
//! in both the active spell bar and inventory bag.

pub mod components;
pub mod plugin;
pub mod spawn;
pub mod systems;

pub use components::*;
pub use plugin::SpellSlotPlugin;
pub use spawn::{
    empty_slot, spawn_level_indicator, spawn_spell_icon_visual, spawn_spell_slot,
    spell_slot_background, BACKGROUND_ALPHA, BORDER_RADIUS, LEVEL_FONT_SIZE, LEVEL_PADDING_X,
    LEVEL_PADDING_Y, LEVEL_TOP_OFFSET, SLOT_SIZE,
};
pub use systems::refresh_spell_slot_visuals;
