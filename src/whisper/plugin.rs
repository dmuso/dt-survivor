use bevy::prelude::*;

use crate::game::sets::GameSet;
use crate::states::GameState;
use crate::whisper::resources::*;
use crate::whisper::systems::*;

pub fn plugin(app: &mut App) {
    app
        // Resources
        .init_resource::<WeaponOrigin>()
        .init_resource::<WhisperState>()
        // Startup systems (particle effect setup)
        .add_systems(Startup, setup_whisper_particle_effect)
        // Setup systems (OnEnter)
        // Note: spawn_whisper_drop is called from game plugin to ensure resources are ready
        .add_systems(
            OnEnter(GameState::InGame),
            reset_whisper_state,
        )
        // Movement systems
        .add_systems(
            Update,
            (whisper_follow_player, update_weapon_origin)
                .chain()
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Effect systems (pickup is handled by loot system via DroppedItem)
        .add_systems(
            Update,
            (
                spawn_whisper_arcs,
                spawn_lightning_bolts,
                animate_lightning_bolts,
                spawn_orbital_particles,
                update_orbital_particles,
                render_particle_trails,
            )
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        )
        // Cleanup
        .add_systems(
            Update,
            update_whisper_arcs
                .in_set(GameSet::Cleanup)
                .run_if(in_state(GameState::InGame)),
        );
}
