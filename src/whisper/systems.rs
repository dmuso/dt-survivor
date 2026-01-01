use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use rand::Rng;

use crate::game::resources::{GameMaterials, GameMeshes};
use crate::loot::components::{DroppedItem, ItemData, PickupState};
use crate::player::components::Player;
use crate::whisper::components::{
    LightningBolt, LightningSegment, LightningSpawnTimer, WhisperAnimationPlayer, WhisperCompanion,
    WhisperModel, WhisperOuterGlow,
};
use crate::whisper::resources::*;

/// Color constants for Whisper visual effects (white mode)
pub const WHISPER_LIGHT_COLOR: Color = Color::srgb(1.0, 1.0, 1.0); // White
/// 3D PointLight intensity (lumens) - very bright to stand out in dark world
pub const WHISPER_LIGHT_INTENSITY: f32 = 10000.0;
/// 3D PointLight radius
pub const WHISPER_LIGHT_RADIUS: f32 = 50.0;

/// Whisper core radius in 3D world units (also used as max bolt length)
const WHISPER_CORE_RADIUS: f32 = 1.2;

/// Lightning bolt visual constants
const LIGHTNING_BOLTS_PER_SPAWN: u32 = 3;
/// Minimum bolt size as fraction of max (0.2 = 20%)
const BOLT_MIN_SIZE_FRACTION: f32 = 0.2;

/// Loads the whisper GLTF and creates the animation graph.
/// Uses Option for asset resources to gracefully handle test environments.
/// Loads multiple animation clips if present (one per animated mesh).
pub fn setup_whisper_animations(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    graphs: Option<ResMut<Assets<AnimationGraph>>>,
) {
    // Skip setup if asset resources aren't available (e.g., in tests)
    let (Some(asset_server), Some(mut graphs)) = (asset_server, graphs) else {
        return;
    };

    // Load the whisper model scene
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/whisper.glb"));

    // Load animation clips (one per animated mesh)
    let clip_0 = asset_server.load(GltfAssetLabel::Animation(0).from_asset("models/whisper.glb"));
    let clip_1 = asset_server.load(GltfAssetLabel::Animation(1).from_asset("models/whisper.glb"));

    // Build animation graph with both clips
    let (graph, node_0) = AnimationGraph::from_clip(clip_0);
    let mut graph = graph;
    let node_1 = graph.add_clip(clip_1, 1.0, graph.root);

    let graph_handle = graphs.add(graph);

    commands.insert_resource(WhisperAnimations {
        scene,
        graph: graph_handle,
        animation_nodes: vec![node_0, node_1],
    });
}

/// Spawns the whisper 3D model as a child of WhisperCompanion entities that don't have a model yet.
pub fn spawn_whisper_model(
    mut commands: Commands,
    whisper_query: Query<(Entity, Option<&Children>), With<WhisperCompanion>>,
    model_query: Query<&WhisperModel>,
    animations: Res<WhisperAnimations>,
) {
    for (whisper_entity, children) in whisper_query.iter() {
        // Check if this entity already has a WhisperModel child
        let has_model = children.is_some_and(|children| {
            children.iter().any(|child| model_query.get(child).is_ok())
        });

        if !has_model {
            // Add the scene as a child of the whisper entity
            commands.entity(whisper_entity).with_children(|parent| {
                parent.spawn((
                    SceneRoot(animations.scene.clone()),
                    WhisperModel,
                    // Scale the model appropriately (adjust as needed based on model size)
                    Transform::from_scale(Vec3::splat(1.0)),
                ));
            });
        }
    }
}

/// Spawns the whisper 3D model for dropped whisper items that don't have a model yet.
pub fn spawn_dropped_whisper_model(
    mut commands: Commands,
    dropped_query: Query<(Entity, &DroppedItem, Option<&Children>)>,
    model_query: Query<&WhisperModel>,
    animations: Res<WhisperAnimations>,
) {
    for (entity, dropped_item, children) in dropped_query.iter() {
        // Only process whisper drops
        if !matches!(dropped_item.item_data, ItemData::Whisper) {
            continue;
        }

        // Check if this entity already has a WhisperModel child
        let has_model = children.is_some_and(|children| {
            children.iter().any(|child| model_query.get(child).is_ok())
        });

        if !has_model {
            // Add the scene as a child of the dropped item
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    SceneRoot(animations.scene.clone()),
                    WhisperModel,
                    // Scale the model appropriately for the dropped version
                    Transform::from_scale(Vec3::splat(1.0)),
                ));
            });
        }
    }
}

/// Sets up the AnimationPlayer once the scene is loaded.
/// Only sets up animation players that are descendants of a WhisperModel.
pub fn setup_whisper_animation_player(
    mut commands: Commands,
    animations: Res<WhisperAnimations>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    whisper_model_query: Query<Entity, With<WhisperModel>>,
    parent_query: Query<&ChildOf>,
) {
    // Get all whisper model entities for ancestry checking
    let whisper_models: Vec<Entity> = whisper_model_query.iter().collect();
    if whisper_models.is_empty() {
        return;
    }

    for (anim_entity, mut player) in animation_players.iter_mut() {
        // Check if this animation player is a descendant of a whisper model
        let mut current = anim_entity;
        let mut is_whisper_anim = false;

        // Walk up the hierarchy to find if this is under a WhisperModel
        while let Ok(parent) = parent_query.get(current) {
            current = parent.get();
            if whisper_models.contains(&current) {
                is_whisper_anim = true;
                break;
            }
        }

        if is_whisper_anim {
            // Add marker and animation graph to the whisper's animation player
            commands.entity(anim_entity).insert((
                WhisperAnimationPlayer,
                AnimationGraphHandle(animations.graph.clone()),
            ));
            // Play all animation nodes (each targets different meshes)
            player.stop_all();
            for &node in &animations.animation_nodes {
                player.play(node).repeat();
            }
        }
    }
}

/// Cleans up whisper animations resource when exiting the game state.
pub fn cleanup_whisper_animations(mut commands: Commands) {
    commands.remove_resource::<WhisperAnimations>();
}

/// Spawns Whisper drop close to player (2.5-3.5 units) but outside pickup radius.
/// Uses polar coordinates to ensure uniform distribution in a ring around the player.
/// Runs on OnEnter(GameState::InGame)
/// Only spawns on a fresh game start (not when continuing from LevelComplete)
pub fn spawn_whisper_drop(
    mut commands: Commands,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
    player_query: Query<&Transform, With<Player>>,
    fresh_start: Res<crate::game::resources::FreshGameStart>,
) {
    // Only spawn Whisper on a fresh game start
    if !fresh_start.0 {
        return;
    }

    let Some(game_meshes) = game_meshes else {
        return;
    };
    let Some(game_materials) = game_materials else {
        return;
    };

    // Get player position, default to origin if no player exists yet
    let player_pos = player_query
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let mut rng = rand::thread_rng();

    // Spawn close to the player but outside pickup_radius (2.0) so it's visible but not auto-collected
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let distance = rng.gen_range(2.5..3.5);
    let x = player_pos.x + angle.cos() * distance;
    let z = player_pos.z + angle.sin() * distance;
    // Y is height - place whisper slightly above ground
    let position = Vec3::new(x, 1.0, z);

    // Spawn the Whisper drop with 3D visual elements and DroppedItem for loot system
    commands
        .spawn((
            DroppedItem {
                pickup_state: PickupState::Idle,
                item_data: ItemData::Whisper,
                velocity: Vec3::ZERO,
                rotation_speed: 0.0,
                rotation_direction: 1.0,
            },
            LightningSpawnTimer::default(),
            Transform::from_translation(position),
            Visibility::default(),
            // Add 3D PointLight for glow effect
            PointLight {
                color: WHISPER_LIGHT_COLOR,
                intensity: WHISPER_LIGHT_INTENSITY,
                radius: WHISPER_LIGHT_RADIUS,
                shadows_enabled: false,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Core glow sphere using 3D mesh
            parent.spawn((
                WhisperOuterGlow,
                Mesh3d(game_meshes.whisper_core.clone()),
                MeshMaterial3d(game_materials.whisper_drop.clone()),
                Transform::default(),
            ));
        });
}

/// Resets whisper state when entering game.
/// Runs on OnEnter(GameState::InGame)
/// Only resets on a fresh game start (not when continuing from LevelComplete)
pub fn reset_whisper_state(
    mut whisper_state: ResMut<WhisperState>,
    mut weapon_origin: ResMut<WeaponOrigin>,
    fresh_start: Res<crate::game::resources::FreshGameStart>,
) {
    // Only reset on a fresh game start
    if fresh_start.0 {
        whisper_state.collected = false;
        weapon_origin.position = None;
    }
}

/// Makes Whisper companion follow the player with bobbing motion.
/// Runs in GameSet::Movement
pub fn whisper_follow_player(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut whisper_query: Query<(&mut Transform, &mut WhisperCompanion), Without<Player>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (mut whisper_transform, mut companion) in whisper_query.iter_mut() {
        // Update bobbing phase
        companion.bob_phase += time.delta_secs() * 3.0; // 3 Hz bobbing frequency

        // Calculate bobbing offset
        let bob_offset = companion.bob_amplitude * companion.bob_phase.sin();

        // Position Whisper at player + follow_offset + bob offset
        whisper_transform.translation.x =
            player_transform.translation.x + companion.follow_offset.x;
        whisper_transform.translation.y =
            player_transform.translation.y + companion.follow_offset.y + bob_offset;
        whisper_transform.translation.z =
            player_transform.translation.z + companion.follow_offset.z;
    }
}

/// Updates WeaponOrigin resource with Whisper's current 3D position.
/// Weapons fire from Whisper's full 3D position.
/// Runs in GameSet::Movement (after whisper_follow_player)
pub fn update_weapon_origin(
    whisper_query: Query<&Transform, With<WhisperCompanion>>,
    mut weapon_origin: ResMut<WeaponOrigin>,
) {
    if let Ok(whisper_transform) = whisper_query.single() {
        // Store full 3D position so weapons fire from Whisper's height
        weapon_origin.position = Some(whisper_transform.translation);
    } else {
        weapon_origin.position = None;
    }
}

/// Spawns lightning bolts from center of Whisper that animate outward.
/// Works on any entity with LightningSpawnTimer (DroppedItem whisper or WhisperCompanion).
/// Bolts are spawned as children of the whisper so they move with it.
/// Timer resets to a random duration after each spawn for varied timing.
/// Uses 3D meshes and XZ plane for bolt orientation.
/// Runs in GameSet::Effects
pub fn spawn_lightning_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut LightningSpawnTimer)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(game_meshes) = game_meshes else {
        return;
    };
    let Some(game_materials) = game_materials else {
        return;
    };
    let mut rng = rand::thread_rng();

    for (whisper_entity, mut timer) in query.iter_mut() {
        timer.timer.tick(time.delta());

        if !timer.timer.just_finished() {
            continue;
        }

        spawn_bolts_as_children(
            &mut commands,
            whisper_entity,
            &mut rng,
            &game_meshes,
            &game_materials,
        );

        // Reset timer with a new random duration
        timer.reset_with_random_duration(&mut rng);
    }
}

/// Helper function to spawn lightning bolts as children of the whisper entity.
/// Uses local coordinates so bolts move with the whisper.
/// Bolts radiate outward on the XZ plane in 3D space.
fn spawn_bolts_as_children(
    commands: &mut Commands,
    whisper_entity: Entity,
    rng: &mut impl Rng,
    game_meshes: &GameMeshes,
    game_materials: &GameMaterials,
) {
    // Max bolt length in 3D world units
    let max_bolt_distance = WHISPER_CORE_RADIUS;

    // Local center (relative to whisper parent)
    let local_center = Vec3::new(0.0, 0.0, 0.0);

    // Spawn multiple bolts at different angles
    for _ in 0..LIGHTNING_BOLTS_PER_SPAWN {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let seed = rng.gen::<u32>();

        // Random size from 20% to 100%, weighted so longer bolts are rarer
        let raw = rng.gen::<f32>();
        let size_multiplier =
            BOLT_MIN_SIZE_FRACTION + (1.0 - raw * raw) * (1.0 - BOLT_MIN_SIZE_FRACTION);

        // Use local center (Vec3::ZERO relative to parent)
        let bolt = LightningBolt::new(angle, seed, local_center, max_bolt_distance, size_multiplier);
        let segment_count = bolt.segment_count;

        // Spawn bolt as child of whisper, with segments as children of bolt
        commands.entity(whisper_entity).with_children(|parent| {
            parent
                .spawn((
                    bolt,
                    Transform::from_translation(local_center),
                    Visibility::default(),
                ))
                .with_children(|bolt_parent| {
                    // Spawn segment meshes as children of the bolt (start with zero scale to avoid flash)
                    for i in 0..segment_count {
                        bolt_parent.spawn((
                            LightningSegment {
                                index: i,
                                bolt_entity: Entity::PLACEHOLDER,
                            },
                            Mesh3d(game_meshes.lightning_segment.clone()),
                            MeshMaterial3d(game_materials.lightning.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                0.01 + i as f32 * 0.001,
                                0.0,
                            ))
                            .with_scale(Vec3::ZERO),
                        ));
                    }
                });
        });
    }
}

/// Animates lightning bolts moving outward from center and fading.
/// Uses local coordinates since bolts are children of whisper entities.
/// Uses 3D transforms on XZ plane - bolts radiate from center.
/// Runs in GameSet::Effects
pub fn animate_lightning_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut bolt_query: Query<(Entity, &mut LightningBolt), Without<LightningSegment>>,
    mut segment_query: Query<(&LightningSegment, &ChildOf, &mut Transform)>,
) {
    // First, update all bolt distances
    for (entity, mut bolt) in bolt_query.iter_mut() {
        bolt.distance += bolt.speed * time.delta_secs();

        // Check if bolt should be despawned (despawn cleans up children automatically)
        if bolt.is_expired() {
            commands.entity(entity).despawn();
        }
    }

    // Then update all segments based on their parent bolt
    for (segment, child_of, mut transform) in segment_query.iter_mut() {
        // Get the parent bolt data using the ChildOf component
        let Ok((_, bolt)) = bolt_query.get(child_of.parent()) else {
            // Parent bolt was despawned, segment will be cleaned up automatically
            continue;
        };

        // If bolt is expired, hide segment
        if bolt.is_expired() {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let segment_idx = segment.index as usize;

        // Calculate how much of this segment should be visible based on bolt distance
        let segment_start_dist = bolt.segment_start_distance(segment_idx);
        let segment_end_dist = bolt.segment_end_distance(segment_idx);
        let segment_len = bolt.segment_length(segment_idx);

        // Only show segment if the bolt has reached it
        if bolt.distance < segment_start_dist {
            transform.scale = Vec3::ZERO;
            continue;
        }

        // Calculate visibility progress for this segment
        let segment_progress = if bolt.distance >= segment_end_dist {
            1.0
        } else if segment_len > 0.0 {
            (bolt.distance - segment_start_dist) / segment_len
        } else {
            1.0
        };

        // Calculate opacity based on overall bolt progress and segment position
        let base_opacity = bolt.current_opacity();
        let segment_fade = 1.0 - (segment_idx as f32 / bolt.segment_count as f32) * 0.3;
        let opacity = (base_opacity * segment_fade * segment_progress).clamp(0.0, 1.0);

        // Hide segments that are nearly invisible
        if opacity < 0.01 {
            transform.scale = Vec3::ZERO;
            continue;
        }

        // Get joint positions for this segment (in local coords relative to bolt center)
        // These are in XY space; we need to map to XZ for 3D
        let start_pos = bolt.joint_position(segment_idx);
        let end_pos = bolt.joint_position(segment_idx + 1);

        // Interpolate end position based on progress
        let current_end = start_pos + (end_pos - start_pos) * segment_progress;

        // Calculate segment midpoint and length
        let midpoint = (start_pos + current_end) / 2.0;
        let segment_vec = current_end - start_pos;
        let length = segment_vec.length().max(0.01);
        let rotation = segment_vec.y.atan2(segment_vec.x);

        // Update local transform - map XY to XZ plane
        transform.translation.x = midpoint.x;
        transform.translation.y = 0.01 + segment_idx as f32 * 0.001; // Slight Y offset
        transform.translation.z = midpoint.y; // Y -> Z for 3D

        // Rotate around Y axis (vertical) instead of Z
        transform.rotation = Quat::from_rotation_y(-rotation);

        // Calculate thickness (tapers from center to tip)
        let thickness = bolt.thickness_at_segment(segment.index) * 3.0; // Scale for 3D

        // Update size via transform scale
        // Scale factor for opacity effect
        let scale_factor = opacity;
        transform.scale = Vec3::new(
            length * scale_factor,
            thickness * scale_factor,
            thickness * scale_factor,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::pbr::StandardMaterial;
    use std::time::Duration;

    fn setup_test_app_with_game_resources() -> App {
        use crate::game::resources::FreshGameStart;

        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_resource(FreshGameStart(true)); // Fresh start = should spawn

        // Set up game meshes and materials
        {
            let world = app.world_mut();
            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                    let game_meshes = GameMeshes::new(&mut meshes);
                    let game_materials = GameMaterials::new(&mut materials);
                    world.insert_resource(game_meshes);
                    world.insert_resource(game_materials);
                });
            });
        }
        app
    }

    #[test]
    fn test_spawn_whisper_drop_creates_dropped_item() {
        use crate::loot::components::{DroppedItem, ItemData, PickupState};

        let mut app = setup_test_app_with_game_resources();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify DroppedItem entity was spawned with ItemData::Whisper
        let mut query = app.world_mut().query::<&DroppedItem>();
        let items: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(items.len(), 1, "Should spawn exactly one DroppedItem");

        let item = items[0];
        assert!(
            matches!(item.item_data, ItemData::Whisper),
            "ItemData should be Whisper variant"
        );
        assert_eq!(
            item.pickup_state,
            PickupState::Idle,
            "Pickup state should be Idle"
        );
    }

    #[test]
    fn test_spawn_whisper_drop_position_within_range_of_player() {
        // Run multiple times to verify random positions are within expected range of player
        for _ in 0..20 {
            let mut app = setup_test_app_with_game_resources();

            // Spawn a player at origin for the whisper to spawn relative to
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            ));

            app.add_systems(Startup, spawn_whisper_drop);

            app.update();

            let mut query = app.world_mut().query::<(&DroppedItem, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                let pos = transform.translation;
                // Use XZ plane for 3D distance from player (at origin)
                let distance = (pos.x * pos.x + pos.z * pos.z).sqrt();

                // Whisper should spawn within 3.5 world units of player
                assert!(
                    distance <= 3.5,
                    "Whisper spawned at distance {} which exceeds 3.5 units",
                    distance
                );

                // Whisper should spawn at least 2.5 units away (outside pickup_radius of 2.0)
                assert!(
                    distance >= 2.5,
                    "Whisper spawned too close to player at distance {}",
                    distance
                );
            }
        }
    }

    #[test]
    fn test_reset_whisper_state() {
        use crate::game::resources::FreshGameStart;

        let mut app = App::new();
        app.init_resource::<WhisperState>();
        app.init_resource::<WeaponOrigin>();
        app.insert_resource(FreshGameStart(true)); // Fresh start = should reset

        // Set initial state
        app.world_mut().resource_mut::<WhisperState>().collected = true;
        app.world_mut().resource_mut::<WeaponOrigin>().position =
            Some(Vec3::new(10.0, 3.0, 20.0));

        app.add_systems(Update, reset_whisper_state);
        app.update();

        // Verify state was reset
        assert!(!app.world().resource::<WhisperState>().collected);
        assert!(app.world().resource::<WeaponOrigin>().position.is_none());
    }

    #[test]
    fn test_reset_whisper_state_skips_when_not_fresh() {
        use crate::game::resources::FreshGameStart;

        let mut app = App::new();
        app.init_resource::<WhisperState>();
        app.init_resource::<WeaponOrigin>();
        app.insert_resource(FreshGameStart(false)); // Not fresh = should NOT reset

        // Set initial state
        app.world_mut().resource_mut::<WhisperState>().collected = true;
        app.world_mut().resource_mut::<WeaponOrigin>().position =
            Some(Vec3::new(10.0, 3.0, 20.0));

        app.add_systems(Update, reset_whisper_state);
        app.update();

        // Verify state was NOT reset
        assert!(app.world().resource::<WhisperState>().collected);
        assert!(app.world().resource::<WeaponOrigin>().position.is_some());
    }

    #[test]
    fn test_whisper_follow_player() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, whisper_follow_player);

        // Create player at X=100, Z=100 on ground plane (Y=0.5 is height)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Transform::from_translation(Vec3::new(100.0, 0.5, 100.0)),
        ));

        // Create WhisperCompanion at origin
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();

        app.update();
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify Whisper followed player on XZ plane
        let whisper_transform = app.world().get::<Transform>(whisper_entity).unwrap();
        assert!(
            (whisper_transform.translation.x - 100.0).abs() < 1.0,
            "Whisper X should be near player X"
        );
        assert!(
            (whisper_transform.translation.z - 100.0).abs() < 1.0,
            "Whisper Z should be near player Z"
        );
        // Y (height) will have follow_offset plus some bobbing, should be above player
        assert!(
            whisper_transform.translation.y > 0.5,
            "Whisper Y should be above player height"
        );
    }

    #[test]
    fn test_update_weapon_origin_with_companion() {
        let mut app = App::new();
        app.init_resource::<WeaponOrigin>();
        app.add_systems(Update, update_weapon_origin);

        // Create WhisperCompanion at (50, 3.0, 60) (Y is height)
        app.world_mut().spawn((
            WhisperCompanion::default(),
            Transform::from_translation(Vec3::new(50.0, 3.0, 60.0)),
        ));

        app.update();

        // Verify WeaponOrigin was updated with full 3D position
        let weapon_origin = app.world().resource::<WeaponOrigin>();
        assert!(weapon_origin.position.is_some());
        assert_eq!(
            weapon_origin.position.unwrap(),
            Vec3::new(50.0, 3.0, 60.0)
        );
    }

    #[test]
    fn test_update_weapon_origin_without_companion() {
        let mut app = App::new();
        app.init_resource::<WeaponOrigin>();
        app.add_systems(Update, update_weapon_origin);

        // Set initial position
        app.world_mut().resource_mut::<WeaponOrigin>().position =
            Some(Vec3::new(10.0, 3.0, 20.0));

        app.update();

        // Verify WeaponOrigin was cleared
        let weapon_origin = app.world().resource::<WeaponOrigin>();
        assert!(weapon_origin.position.is_none());
    }

    #[test]
    fn test_spawn_whisper_drop_has_point_light() {
        let mut app = setup_test_app_with_game_resources();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify DroppedItem Whisper entity was spawned with 3D PointLight
        let mut query = app.world_mut().query::<(&DroppedItem, &PointLight)>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "Whisper DroppedItem should have PointLight component"
        );
    }

    #[test]
    fn test_spawn_whisper_drop_light_properties() {
        let mut app = setup_test_app_with_game_resources();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify the light properties
        let mut query = app.world_mut().query::<&PointLight>();
        let light = query.single(app.world()).unwrap();

        // Drop now uses full intensity (same as companion)
        assert_eq!(light.intensity, WHISPER_LIGHT_INTENSITY);
        assert_eq!(light.radius, WHISPER_LIGHT_RADIUS);
    }

    #[test]
    fn test_lightning_bolt_animation_updates_distance() {
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, animate_lightning_bolts);

        // Create a lightning bolt at origin facing right (angle = 0)
        let center = Vec3::new(0.0, 0.0, 0.0);
        let bolt = LightningBolt::new(0.0, 42, center, 0.5, 1.0);
        let bolt_entity = app
            .world_mut()
            .spawn((
                bolt,
                Transform::from_translation(center),
                Visibility::default(),
            ))
            .id();

        // Create a segment for this bolt using 3D mesh
        let mesh_handle = {
            let game_meshes = app.world().resource::<GameMeshes>();
            game_meshes.lightning_segment.clone()
        };
        let material_handle = {
            let game_materials = app.world().resource::<GameMaterials>();
            game_materials.lightning.clone()
        };

        app.world_mut().spawn((
            LightningSegment {
                index: 0,
                bolt_entity,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(center),
        ));

        // First update to initialize time
        app.update();

        // Advance time and update again
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify bolt distance increased
        let bolt = app.world().get::<LightningBolt>(bolt_entity).unwrap();
        assert!(
            bolt.distance > 0.0,
            "Bolt distance should have increased, got {}",
            bolt.distance
        );
    }

    #[test]
    fn test_lightning_bolt_despawns_when_expired() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, animate_lightning_bolts);

        // Create a lightning bolt that's already expired (distance >= max_distance)
        let center = Vec3::ZERO;
        let mut bolt = LightningBolt::new(0.0, 42, center, 0.5, 1.0);
        bolt.distance = bolt.max_distance; // At max, should be despawned

        let bolt_entity = app
            .world_mut()
            .spawn((
                bolt,
                Transform::from_translation(center),
                Visibility::default(),
            ))
            .id();

        // First update to initialize time
        app.update();

        // Advance time and update
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify bolt was despawned
        assert!(
            app.world().get_entity(bolt_entity).is_err(),
            "Expired lightning bolt should be despawned"
        );
    }

    #[test]
    fn test_lightning_segments_despawn_with_parent_bolt() {
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, animate_lightning_bolts);

        // Get mesh and material for segment
        let mesh_handle = {
            let game_meshes = app.world().resource::<GameMeshes>();
            game_meshes.lightning_segment.clone()
        };
        let material_handle = {
            let game_materials = app.world().resource::<GameMaterials>();
            game_materials.lightning.clone()
        };

        // Create an expired lightning bolt with a child segment
        let center = Vec3::ZERO;
        let mut bolt = LightningBolt::new(0.0, 42, center, 0.5, 1.0);
        bolt.distance = bolt.max_distance; // Mark as expired

        let bolt_entity = app
            .world_mut()
            .spawn((
                bolt,
                Transform::from_translation(center),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    LightningSegment {
                        index: 0,
                        bolt_entity: Entity::PLACEHOLDER,
                    },
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform::default(),
                ));
            })
            .id();

        // Get the segment entity
        let segment_entity = app
            .world_mut()
            .query::<(Entity, &LightningSegment)>()
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("Segment should exist before update");

        app.update();

        // Verify both bolt and segment were despawned
        assert!(
            app.world().get_entity(bolt_entity).is_err(),
            "Expired bolt should be despawned"
        );
        assert!(
            app.world().get_entity(segment_entity).is_err(),
            "Child segment should be despawned with parent bolt"
        );
    }

    #[test]
    fn test_spawn_whisper_drop_has_lightning_spawn_timer() {
        let mut app = setup_test_app_with_game_resources();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify Whisper DroppedItem entity has LightningSpawnTimer
        let mut query = app
            .world_mut()
            .query::<(&DroppedItem, &LightningSpawnTimer)>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "Whisper DroppedItem should have LightningSpawnTimer component"
        );
    }

    #[test]
    fn test_spawn_bolts_as_children_creates_bolts_and_segments() {
        let mut app = setup_test_app_with_game_resources();

        // Create a whisper entity to be the parent
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::new(50.0, 1.5, 50.0)),
            ))
            .id();

        // Spawn bolts as children of the whisper entity
        {
            let world = app.world_mut();
            world.resource_scope(|world, game_meshes: Mut<GameMeshes>| {
                world.resource_scope(|world, game_materials: Mut<GameMaterials>| {
                    let mut commands = world.commands();
                    spawn_bolts_as_children(
                        &mut commands,
                        whisper_entity,
                        &mut rand::thread_rng(),
                        &game_meshes,
                        &game_materials,
                    );
                });
            });
        }
        app.update();

        // Verify lightning bolts were spawned
        let bolt_count = app
            .world_mut()
            .query::<&LightningBolt>()
            .iter(app.world())
            .count();
        assert_eq!(
            bolt_count,
            LIGHTNING_BOLTS_PER_SPAWN as usize,
            "Should spawn {} lightning bolts",
            LIGHTNING_BOLTS_PER_SPAWN
        );

        // Verify segments were spawned (5 segments per bolt)
        let segment_count = app
            .world_mut()
            .query::<&LightningSegment>()
            .iter(app.world())
            .count();
        assert_eq!(
            segment_count,
            (LIGHTNING_BOLTS_PER_SPAWN * 5) as usize,
            "Should spawn 5 segments per bolt"
        );
    }

    #[test]
    fn test_lightning_bolts_use_local_coordinates() {
        let mut app = setup_test_app_with_game_resources();

        // Create a whisper entity at some world position
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::new(50.0, 1.5, 75.0)),
            ))
            .id();

        // Spawn bolts as children
        {
            let world = app.world_mut();
            world.resource_scope(|world, game_meshes: Mut<GameMeshes>| {
                world.resource_scope(|world, game_materials: Mut<GameMaterials>| {
                    let mut commands = world.commands();
                    spawn_bolts_as_children(
                        &mut commands,
                        whisper_entity,
                        &mut rand::thread_rng(),
                        &game_meshes,
                        &game_materials,
                    );
                });
            });
        }
        app.update();

        // Verify bolts store local center (0, 0, 0) not world position
        let mut query = app.world_mut().query::<&LightningBolt>();
        for bolt in query.iter(app.world()) {
            assert!(
                bolt.center.x.abs() < 0.001,
                "Bolt center X should be 0 (local coords), got {}",
                bolt.center.x
            );
            assert!(
                bolt.center.y.abs() < 0.001,
                "Bolt center Y should be 0 (local coords), got {}",
                bolt.center.y
            );
        }
    }
}
