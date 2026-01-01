use bevy::prelude::*;

use crate::game::sets::GameSet;
use crate::states::GameState;
use crate::whisper::resources::*;
use crate::whisper::systems::*;

pub fn plugin(app: &mut App) {
    app
        // Resources
        .init_resource::<SpellOrigin>()
        .init_resource::<WhisperState>()
        .init_resource::<WhisperAttunement>()
        // Startup systems (animation setup)
        .add_systems(Startup, setup_whisper_animations)
        // Setup systems (OnEnter)
        // Note: spawn_whisper_drop is called from game plugin to ensure resources are ready
        .add_systems(OnEnter(GameState::InGame), reset_whisper_state)
        // Cleanup systems (OnExit)
        .add_systems(OnExit(GameState::InGame), cleanup_whisper_animations)
        // Movement systems
        .add_systems(
            Update,
            (whisper_follow_player, update_spell_origin)
                .chain()
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Model and animation spawn systems
        .add_systems(
            Update,
            (
                spawn_whisper_model,
                spawn_dropped_whisper_model,
                setup_whisper_animation_player,
            )
                .run_if(resource_exists::<WhisperAnimations>)
                .run_if(in_state(GameState::InGame)),
        )
        // Effect systems (lightning bolts)
        .add_systems(
            Update,
            (spawn_lightning_bolts, animate_lightning_bolts)
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        );
}
