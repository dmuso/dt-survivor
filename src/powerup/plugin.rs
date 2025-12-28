use bevy::prelude::*;
use crate::states::*;
use crate::powerup::systems::*;
use crate::powerup::components::ActivePowerups;

pub fn plugin(app: &mut App) {
    app
        .init_resource::<ActivePowerups>()
        .add_systems(
            Update,
            (
                powerup_spawning_system,
                powerup_pulse_system,
                // Powerup pickup is now handled by the loot system (DroppedItem)
                apply_player_powerup_effects,
                apply_weapon_powerup_effects,
                update_powerup_timers,
                update_powerup_ui,
            )
                .run_if(in_state(GameState::InGame))
        );
}