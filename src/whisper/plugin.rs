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
        // Note: WhisperAnimations resource is NOT cleaned up on exit - it holds lightweight asset
        // handles that should persist through state transitions (InGame -> AttunementSelect -> InGame).
        // The WhisperCompanion entity is spawned when Whisper is picked up, and the 3D model needs
        // to be attached by spawn_whisper_model which requires WhisperAnimations.
        // Movement systems
        .add_systems(
            Update,
            (whisper_follow_player, update_spell_origin)
                .chain()
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Model and animation spawn systems
        // spawn_dropped_whisper_model only runs in InGame (drops only exist there)
        .add_systems(
            Update,
            spawn_dropped_whisper_model
                .run_if(resource_exists::<WhisperAnimations>)
                .run_if(in_state(GameState::InGame)),
        )
        // spawn_whisper_model and animation setup need to run in multiple states
        // because WhisperCompanion is created when picking up Whisper and we
        // immediately transition to AttunementSelect before the model can be spawned
        .add_systems(
            Update,
            (spawn_whisper_model, setup_whisper_animation_player)
                .run_if(resource_exists::<WhisperAnimations>)
                .run_if(
                    in_state(GameState::InGame)
                        .or(in_state(GameState::AttunementSelect))
                        .or(in_state(GameState::InventoryOpen))
                        .or(in_state(GameState::LevelComplete))
                        .or(in_state(GameState::Paused)),
                ),
        )
        // Effect systems (lightning bolts)
        .add_systems(
            Update,
            (spawn_lightning_bolts, animate_lightning_bolts)
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        );
}
