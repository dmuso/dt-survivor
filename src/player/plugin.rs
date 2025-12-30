use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;

use crate::player::components::{
    Player, PlayerAnimationState, PlayerAnimations, PlayerModel, PlayerSpotlight,
};
use crate::game::sets::GameSet;
use crate::states::GameState;

/// Marker component for the player's AnimationPlayer entity
#[derive(Component)]
pub struct PlayerAnimationPlayer;

/// Loads the player GLTF and creates the animation graph
/// Uses Option for asset resources to gracefully handle test environments
pub fn setup_player_animations(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    graphs: Option<ResMut<Assets<AnimationGraph>>>,
) {
    // Skip setup if asset resources aren't available (e.g., in tests)
    let (Some(asset_server), Some(mut graphs)) = (asset_server, graphs) else {
        return;
    };

    // Load the player model scene
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/player.glb"));

    // Load animation clips (indices from GLB: 0=Idle, 1=Run, 2=Walk)
    let idle_clip = asset_server.load(GltfAssetLabel::Animation(0).from_asset("models/player.glb"));
    let run_clip = asset_server.load(GltfAssetLabel::Animation(1).from_asset("models/player.glb"));

    // Build animation graph
    let (graph, idle_node) = AnimationGraph::from_clip(idle_clip);
    let mut graph = graph;
    let run_node = graph.add_clip(run_clip, 1.0, graph.root);

    let graph_handle = graphs.add(graph);

    commands.insert_resource(PlayerAnimations {
        scene,
        graph: graph_handle,
        idle_node,
        run_node,
    });
}

/// Spawns the player entity with the 3D model once animations are loaded
pub fn spawn_player_model(
    mut commands: Commands,
    player_query: Query<Entity, (With<Player>, Without<PlayerAnimationState>)>,
    animations: Res<PlayerAnimations>,
) {
    for player_entity in player_query.iter() {
        // Add the scene as a child of the player entity
        commands.entity(player_entity).with_children(|parent| {
            parent.spawn((
                SceneRoot(animations.scene.clone()),
                PlayerModel,
                // Rotate 180 degrees to face forward (Blender Y+ forward vs Bevy Z- forward)
                Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            ));
        });

        // Add animation state to the player
        commands.entity(player_entity).insert(PlayerAnimationState::default());
    }
}

/// Sets up the AnimationPlayer once the scene is loaded
/// Only sets up animation players that are descendants of a PlayerModel
pub fn setup_animation_player(
    mut commands: Commands,
    animations: Res<PlayerAnimations>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    player_model_query: Query<Entity, With<PlayerModel>>,
    parent_query: Query<&ChildOf>,
) {
    // Get all player model entities for ancestry checking
    let player_models: Vec<Entity> = player_model_query.iter().collect();
    if player_models.is_empty() {
        return;
    }

    for (anim_entity, mut player) in animation_players.iter_mut() {
        // Check if this animation player is a descendant of a player model
        let mut current = anim_entity;
        let mut is_player_anim = false;

        // Walk up the hierarchy to find if this is under a PlayerModel
        while let Ok(parent) = parent_query.get(current) {
            current = parent.get();
            if player_models.contains(&current) {
                is_player_anim = true;
                break;
            }
        }

        if is_player_anim {
            // Add marker and animation graph to the player's animation player
            commands.entity(anim_entity).insert((
                PlayerAnimationPlayer,
                AnimationGraphHandle(animations.graph.clone()),
            ));
            // Start with idle animation
            player.stop_all();
            player.play(animations.idle_node).repeat();
        }
    }
}

/// Switches player animation based on movement state
pub fn update_player_animation(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    animations: Res<PlayerAnimations>,
    mut player_query: Query<&mut PlayerAnimationState, With<Player>>,
    mut animation_players: Query<&mut AnimationPlayer, With<PlayerAnimationPlayer>>,
) {
    let is_moving = mouse_button_input.pressed(MouseButton::Left);

    for mut anim_state in player_query.iter_mut() {
        let new_state = if is_moving {
            PlayerAnimationState::Run
        } else {
            PlayerAnimationState::Idle
        };

        // Only update if state changed
        if *anim_state != new_state {
            *anim_state = new_state;

            // Update all player animation players (use marker to filter)
            for mut anim_player in animation_players.iter_mut() {
                let node = match new_state {
                    PlayerAnimationState::Idle => animations.idle_node,
                    PlayerAnimationState::Run => animations.run_node,
                };
                // Stop current animation and start new one
                anim_player.stop_all();
                anim_player.play(node).repeat();
            }
        }
    }
}

/// Rotates the player model to face the movement direction
pub fn rotate_player_model(
    player_query: Query<(&Player, &Children)>,
    mut model_query: Query<&mut Transform, With<PlayerModel>>,
) {
    for (player, children) in player_query.iter() {
        if player.last_movement_direction.length_squared() > 0.01 {
            for child in children.iter() {
                if let Ok(mut transform) = model_query.get_mut(child) {
                    // Calculate rotation to face movement direction
                    // The model faces -Z by default after the 180 degree rotation
                    let direction = player.last_movement_direction.normalize();
                    let target_rotation = Quat::from_rotation_y(
                        f32::atan2(direction.x, direction.z)
                    );
                    // Smooth rotation
                    transform.rotation = transform.rotation.slerp(target_rotation, 0.15);
                }
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app
        // Setup animations when entering the game
        .add_systems(
            OnEnter(GameState::InGame),
            setup_player_animations,
        )
        // Systems that run during gameplay
        .add_systems(
            Update,
            (
                spawn_player_model.run_if(resource_exists::<PlayerAnimations>),
                setup_animation_player.run_if(resource_exists::<PlayerAnimations>),
                update_player_animation.run_if(resource_exists::<PlayerAnimations>),
                rotate_player_model,
                spawn_player_spotlight,
                spotlight_follow_player,
            )
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Cleanup animations resource when exiting
        .add_systems(
            OnExit(GameState::InGame),
            cleanup_player_animations,
        );
}

fn cleanup_player_animations(mut commands: Commands) {
    commands.remove_resource::<PlayerAnimations>();
}

/// Player spotlight configuration
const PLAYER_LIGHT_HEIGHT: f32 = 15.0;
const PLAYER_LIGHT_INTENSITY: f32 = 2_000_000.0; // 2 million lumens (default is 1 million)
const PLAYER_LIGHT_RANGE: f32 = 50.0;
const PLAYER_LIGHT_INNER_ANGLE: f32 = 0.4; // ~23 degrees - bright center
const PLAYER_LIGHT_OUTER_ANGLE: f32 = 1.2; // ~69 degrees - very soft falloff

/// Spawns a spotlight above the player pointing down
pub fn spawn_player_spotlight(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    spotlight_query: Query<&PlayerSpotlight>,
) {
    // Only spawn if player exists but no spotlight yet
    if !spotlight_query.is_empty() {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let light_pos = Vec3::new(
        player_transform.translation.x,
        PLAYER_LIGHT_HEIGHT,
        player_transform.translation.z,
    );

    // Spawn spotlight above player pointing down
    commands.spawn((
        PlayerSpotlight,
        SpotLight {
            intensity: PLAYER_LIGHT_INTENSITY,
            color: Color::WHITE,
            shadows_enabled: false,
            range: PLAYER_LIGHT_RANGE,
            radius: 1.0,
            inner_angle: PLAYER_LIGHT_INNER_ANGLE,
            outer_angle: PLAYER_LIGHT_OUTER_ANGLE,
            ..default()
        },
        Transform::from_translation(light_pos)
            .looking_at(player_transform.translation, Vec3::X),
        Visibility::default(),
    ));
}

/// Makes the spotlight follow the player and point down at them
pub fn spotlight_follow_player(
    player_query: Query<&Transform, (With<Player>, Without<PlayerSpotlight>)>,
    mut light_query: Query<&mut Transform, With<PlayerSpotlight>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for mut light_transform in light_query.iter_mut() {
        // Position light above player
        light_transform.translation.x = player_transform.translation.x;
        light_transform.translation.y = PLAYER_LIGHT_HEIGHT;
        light_transform.translation.z = player_transform.translation.z;
        // Point down at player
        light_transform.look_at(player_transform.translation, Vec3::X);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_animation_state_transitions() {
        // Test that animation states can transition correctly
        let mut state = PlayerAnimationState::Idle;
        assert_eq!(state, PlayerAnimationState::Idle);

        state = PlayerAnimationState::Run;
        assert_eq!(state, PlayerAnimationState::Run);

        state = PlayerAnimationState::Idle;
        assert_eq!(state, PlayerAnimationState::Idle);
    }
}
