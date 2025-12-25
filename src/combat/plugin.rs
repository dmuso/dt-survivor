use bevy::prelude::*;

use super::events::{DamageEvent, DeathEvent};
use super::systems::{
    apply_damage_system, check_death_system, handle_enemy_death_system, tick_invincibility_system,
};
use crate::game::events::EnemyDeathEvent;
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
        .add_message::<EnemyDeathEvent>()
        .configure_sets(
            Update,
            (CombatSets::Damage, CombatSets::Death, CombatSets::Cleanup)
                .chain()
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            apply_damage_system
                .in_set(CombatSets::Damage)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            (check_death_system, handle_enemy_death_system)
                .chain()
                .in_set(CombatSets::Death)
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

    #[test]
    fn test_combat_plugin_integration_with_game_state() {
        use super::super::components::Health;
        use crate::score::Score;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins((
            bevy::time::TimePlugin::default(),
            bevy::state::app::StatesPlugin,
        ));
        app.init_state::<GameState>();
        app.init_resource::<Score>();
        app.add_plugins(plugin);

        // Transition to InGame state
        app.world_mut()
            .get_resource_mut::<bevy::state::state::NextState<GameState>>()
            .unwrap()
            .set(GameState::InGame);
        app.update();

        // Spawn an entity with Health and send damage event
        let entity = app
            .world_mut()
            .spawn((Health::new(100.0), Transform::default()))
            .id();
        app.world_mut()
            .write_message(DamageEvent::new(entity, 30.0));

        // Advance time and run systems
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(16));
        }
        app.update();

        // Verify damage was applied
        let health = app.world().get::<Health>(entity).unwrap();
        assert_eq!(health.current, 70.0, "Combat plugin should apply damage in InGame state");
    }

    #[test]
    fn test_combat_plugin_inactive_in_intro_state() {
        use super::super::components::Health;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins((
            bevy::time::TimePlugin::default(),
            bevy::state::app::StatesPlugin,
        ));
        app.init_state::<GameState>(); // Starts in Intro state
        app.add_plugins(plugin);

        app.update();

        // Spawn an entity with Health and send damage event
        let entity = app
            .world_mut()
            .spawn((Health::new(100.0), Transform::default()))
            .id();
        app.world_mut()
            .write_message(DamageEvent::new(entity, 30.0));

        // Advance time and run systems
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(Duration::from_millis(16));
        }
        app.update();

        // Verify damage was NOT applied (we're in Intro state)
        let health = app.world().get::<Health>(entity).unwrap();
        assert_eq!(
            health.current, 100.0,
            "Combat plugin should not apply damage in Intro state"
        );
    }
}
