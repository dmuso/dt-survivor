pub mod components;
pub mod events;
pub mod plugin;
pub mod systems;

pub use components::{Damage, Health, Hitbox, Invincibility};
pub use events::{DamageEvent, DeathEvent, EntityType};
pub use plugin::{plugin, CombatSets};
pub use systems::tick_invincibility_system;
