use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::game::sets::GameSet;
use crate::states::GameState;
use crate::whisper::events::*;
use crate::whisper::materials::*;
use crate::whisper::resources::*;
use crate::whisper::systems::*;

pub fn plugin(app: &mut App) {
    app
        // Material plugins for additive blending
        .add_plugins((
            Material2dPlugin::<AdditiveTextureMaterial>::default(),
            Material2dPlugin::<AdditiveColorMaterial>::default(),
        ))
        // Resources
        .init_resource::<WeaponOrigin>()
        .init_resource::<WhisperState>()
        // Events
        .add_message::<WhisperCollectedEvent>()
        // Startup systems (particle effect setup)
        .add_systems(Startup, setup_whisper_particle_effect)
        // Setup systems (OnEnter)
        .add_systems(
            OnEnter(GameState::InGame),
            (reset_whisper_state, spawn_whisper_drop).chain(),
        )
        // Movement systems
        .add_systems(
            Update,
            (whisper_follow_player, update_weapon_origin)
                .chain()
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Combat/pickup detection
        .add_systems(
            Update,
            detect_whisper_pickup
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        // Effect systems
        .add_systems(
            Update,
            (
                handle_whisper_collection,
                spawn_whisper_arcs,
                spawn_lightning_bolts,
                animate_lightning_bolts,
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
