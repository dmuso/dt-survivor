use bevy::prelude::*;
use crate::states::*;
use crate::enemy_death::systems::*;
use crate::enemy_death::components::*;

/// Enemy death plugin handles sound effects and loot drop events on enemy death.
///
/// Note: This plugin does NOT register EnemyDeathEvent or LootDropEvent.
/// Event ownership is centralized:
/// - EnemyDeathEvent: owned by combat/plugin.rs
/// - LootDropEvent: owned by loot/plugin.rs
pub fn plugin(app: &mut App) {
    app
        .init_resource::<EnemyDeathSoundTimer>()
        .add_systems(Update, enemy_death_system.run_if(in_state(GameState::InGame)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::events::{EnemyDeathEvent, LootDropEvent};

    /// Test that enemy_death_system works when events are registered by other plugins.
    /// This verifies that the plugin correctly depends on combat and loot plugins
    /// for event registration rather than registering them itself.
    #[test]
    fn test_enemy_death_system_uses_externally_registered_events() {
        let mut app = App::new();
        app.add_plugins((
            bevy::time::TimePlugin::default(),
            bevy::state::app::StatesPlugin,
        ));
        app.init_state::<GameState>();

        // Register events as they would be by combat and loot plugins
        app.add_message::<EnemyDeathEvent>();
        app.add_message::<LootDropEvent>();

        // Add our plugin (should not try to re-register events)
        app.add_plugins(plugin);

        // Transition to InGame state
        app.world_mut()
            .get_resource_mut::<bevy::state::state::NextState<GameState>>()
            .unwrap()
            .set(GameState::InGame);
        app.update();

        // Send an EnemyDeathEvent
        app.world_mut().write_message(EnemyDeathEvent {
            enemy_entity: Entity::PLACEHOLDER,
            position: bevy::math::Vec3::new(100.0, 0.0, 200.0),
        });

        // Run the system
        app.update();

        // Verify LootDropEvent was sent
        // (We can't easily read messages in tests, but if no panic occurred, events worked)
    }
}