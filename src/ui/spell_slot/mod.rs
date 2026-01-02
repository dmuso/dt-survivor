//! Spell slot UI module.
//!
//! This module provides unified components and systems for rendering spell slots
//! in both the active spell bar and inventory bag.

pub mod components;
pub mod spawn;
pub mod systems;

pub use components::*;
// Re-exports for spawn and systems will be added when those modules have public items
