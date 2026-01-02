//! Spell slot UI module.
//!
//! This module provides unified components and systems for rendering spell slots
//! in both the active spell bar and inventory bag.

pub mod components;
pub mod spawn;
pub mod systems;

pub use components::*;
pub use spawn::{
    empty_slot, BACKGROUND_ALPHA, BORDER_RADIUS, LEVEL_FONT_SIZE, LEVEL_PADDING_X, LEVEL_PADDING_Y,
    LEVEL_TOP_OFFSET, SLOT_SIZE,
};
