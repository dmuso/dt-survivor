use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;
use crate::states::*;
use crate::loot::systems::*;
use crate::loot::events::*;
use crate::game::events::LootDropEvent;

/// Temporary resource to hold the GLTF handle while it loads
#[derive(Resource)]
pub struct XpOrbGltfHandle(pub Handle<Gltf>);

/// Resource holding the XP orb 3D model mesh
/// Materials are provided by XpOrbMaterials resource (defined in game/resources.rs)
#[derive(Resource)]
pub struct XpOrbModel {
    pub mesh: Handle<Mesh>,
}

/// Starts loading the XP orb GLB model
/// Uses Option for asset resources to gracefully handle test environments
pub fn setup_xp_orb_model(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    gltfs: Option<Res<Assets<Gltf>>>,
) {
    // Skip setup if asset resources aren't available (e.g., in tests)
    let (Some(asset_server), Some(_)) = (asset_server, gltfs) else {
        return;
    };

    let gltf_handle = asset_server.load("models/xp_orb.glb");
    commands.insert_resource(XpOrbGltfHandle(gltf_handle));
}

/// Extracts mesh from the loaded GLTF
/// Runs each frame until the GLTF is loaded, then creates the XpOrbModel resource
pub fn init_xp_orb_materials(
    mut commands: Commands,
    gltf_handle: Option<Res<XpOrbGltfHandle>>,
    gltfs: Option<Res<Assets<Gltf>>>,
    gltf_meshes: Option<Res<Assets<GltfMesh>>>,
    xp_orb_model: Option<Res<XpOrbModel>>,
) {
    // Skip if already initialized or resources not available
    if xp_orb_model.is_some() {
        return;
    }
    let (Some(gltf_handle), Some(gltfs), Some(gltf_meshes)) =
        (gltf_handle, gltfs, gltf_meshes)
    else {
        return;
    };

    // Wait for GLTF to load
    let Some(gltf) = gltfs.get(&gltf_handle.0) else {
        return;
    };

    // Get the first GltfMesh handle from the GLTF
    let Some(gltf_mesh_handle) = gltf.meshes.first() else {
        warn!("XP orb GLB has no meshes");
        return;
    };

    // Get the actual GltfMesh asset
    let Some(gltf_mesh) = gltf_meshes.get(gltf_mesh_handle) else {
        // GltfMesh not loaded yet, wait for next frame
        return;
    };

    // Get the first primitive's mesh handle
    let Some(primitive) = gltf_mesh.primitives.first() else {
        warn!("XP orb GltfMesh has no primitives");
        return;
    };

    commands.insert_resource(XpOrbModel {
        mesh: primitive.mesh.clone(),
    });

    // Remove the temporary GLTF handle resource
    commands.remove_resource::<XpOrbGltfHandle>();
}

/// Resource to debounce loot pickup sounds.
/// Only one sound plays within a random 100-250ms window to prevent audio spam.
#[derive(Resource)]
pub struct LootSoundCooldown {
    pub timer: Timer,
}

impl Default for LootSoundCooldown {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.1, TimerMode::Once); // Start with min duration
        // Start finished so first sound plays immediately
        timer.tick(std::time::Duration::from_secs_f32(0.1));
        Self { timer }
    }
}

impl LootSoundCooldown {
    /// Reset the cooldown with a random duration between 100-250ms
    pub fn reset_random(&mut self) {
        use rand::Rng;
        let duration = rand::thread_rng().gen_range(0.1..=0.25);
        self.timer.set_duration(std::time::Duration::from_secs_f32(duration));
        self.timer.reset();
    }
}

/// System to tick the loot sound cooldown timer
pub fn tick_loot_sound_cooldown(mut cooldown: ResMut<LootSoundCooldown>, time: Res<Time>) {
    cooldown.timer.tick(time.delta());
}

/// Cleanup XP orb model resources when exiting game
fn cleanup_xp_orb_model(mut commands: Commands) {
    commands.remove_resource::<XpOrbModel>();
    commands.remove_resource::<XpOrbGltfHandle>();
}

pub fn plugin(app: &mut App) {
    app
        .add_message::<LootDropEvent>()
        .add_message::<PickupEvent>()
        .add_message::<ItemEffectEvent>()
        .init_resource::<LootSoundCooldown>()
        // Load XP orb model when entering game
        .add_systems(OnEnter(GameState::InGame), setup_xp_orb_model)
        // Cleanup when exiting
        .add_systems(OnExit(GameState::InGame), cleanup_xp_orb_model)
        .add_systems(Update, (
            // Initialize XP orb materials once GLTF is loaded
            init_xp_orb_materials.run_if(in_state(GameState::InGame)),
            // Tick the sound cooldown timer
            tick_loot_sound_cooldown.run_if(in_state(GameState::InGame)),
            // Loot drop system spawns DroppedItem entities from enemy deaths
            loot_drop_system
                .run_if(in_state(GameState::InGame))
                .run_if(resource_exists::<XpOrbModel>),
            // Animate falling XP orbs (custom physics simulation)
            animate_falling.run_if(in_state(GameState::InGame)),

            // ECS-based pickup systems - ordered pipeline
            // 1. Detect when items enter pickup radius
            detect_pickup_collisions.run_if(in_state(GameState::InGame)),
            // 2. Start pop-up animation when pickup event received
            start_popup_animation.run_if(in_state(GameState::InGame)),
            // 3. Animate the pop-up (fly up, then fall)
            animate_popup.run_if(in_state(GameState::InGame)),
            // 4. Attract items toward player
            update_item_attraction.run_if(in_state(GameState::InGame)),
            // 5. Move items based on velocity
            update_item_movement.run_if(in_state(GameState::InGame)),
            // 6. Complete pickup when items reach player
            complete_pickup_when_close.run_if(in_state(GameState::InGame)),
            // 7. Apply pickup effects
            apply_item_effects.run_if(in_state(GameState::InGame)),
            // 8. Cleanup consumed items
            cleanup_consumed_items.run_if(in_state(GameState::InGame)),
        ));
}