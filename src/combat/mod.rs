pub mod components;
pub mod events;
pub mod plugin;
pub mod systems;

pub use components::{Damage, DamageFlash, Health, Hitbox, Invincibility};
pub use events::{DamageEvent, DeathEvent, EntityType};
pub use plugin::{plugin, CombatSets};
pub use systems::{
    apply_damage_flash_system, apply_damage_system, check_death_system, handle_enemy_death_system,
    tick_invincibility_system, update_damage_flash_system, CheckDeath,
};
