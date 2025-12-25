use bevy::prelude::*;

use super::events::{DamageEvent, DeathEvent};
use super::systems::tick_invincibility_system;
use crate::states::GameState;

/// System sets for combat systems ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CombatSets {
    /// Damage application and health updates
    Damage,
    /// Death detection and handling
    Death,
    /// Cleanup expired effects (invincibility, etc.)
    Cleanup,
}

/// Combat plugin providing unified damage and death handling
pub fn plugin(app: &mut App) {
    app.add_message::<DamageEvent>()
        .add_message::<DeathEvent>()
        .configure_sets(
            Update,
            (CombatSets::Damage, CombatSets::Death, CombatSets::Cleanup)
                .chain()
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            tick_invincibility_system
                .in_set(CombatSets::Cleanup)
                .run_if(in_state(GameState::InGame)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registers_messages() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.add_plugins(plugin);

        // Verify messages are registered by writing them
        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(DamageEvent::new(entity, 10.0));
        app.world_mut().write_message(DeathEvent::new(
            entity,
            Vec3::ZERO,
            super::super::events::EntityType::Enemy,
        ));

        // If we get here without panicking, messages are registered
    }

    #[test]
    fn test_plugin_configures_system_sets() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.add_plugins(plugin);

        // The plugin should configure without panicking
        // System set configuration is validated at runtime
        app.update();
    }
}
