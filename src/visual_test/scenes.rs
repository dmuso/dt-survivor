use bevy::prelude::*;
use std::str::FromStr;

use crate::game::resources::GameMeshes;
use crate::spells::fire::fireball::{FireballProjectile, FireballCoreEffect, FireballTrailEffect, SmokePuffSpawner};
use crate::spells::fire::materials::{
    FireballCoreMaterial, FireballTrailMaterial,
    ExplosionCoreMaterial, ExplosionFireMaterial,
    FireballSparksMaterial, ExplosionEmbersMaterial, ExplosionSmokeMaterial,
};

/// Available visual test scenes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestScene {
    // === Fireball Trail Direction Tests ===
    /// Fireball moving right - trail should be on the left
    FireballTrailEast,
    /// Fireball moving left - trail should be on the right
    FireballTrailWest,
    /// Fireball moving away from camera
    FireballTrailNorth,
    /// Fireball moving toward camera
    FireballTrailSouth,

    // === Trail Noise Animation Tests (multi-frame) ===
    /// Animated trail noise deformation - captures 5 frames
    TrailNoiseAnimation,

    // === Trail Growth Tests (transition continuity) ===
    /// Fireball at spawn - trail should be zero/minimal length
    TrailGrowthSpawn,
    /// Fireball at 25% travel distance - trail ~25% max length
    TrailGrowthQuarter,
    /// Fireball at 50% travel distance - trail ~50% max length
    TrailGrowthHalf,
    /// Fireball at full travel distance - trail at max length
    TrailGrowthFull,
    /// Multi-frame trail growth animation - captures 6 frames showing growth
    TrailGrowthSequence,

    // === Explosion Shader Tests ===
    /// Explosion core (white-hot flash) at different progress stages
    ExplosionCoreTest,
    /// Explosion fire (main fireball blast) at different progress stages
    ExplosionFireTest,
    /// Fireball sparks flying embers
    ExplosionSparksTest,
    /// Explosion embers flying debris
    ExplosionEmbersTest,
    /// Explosion smoke rising plume
    ExplosionSmokeTest,

    // === Full Explosion Sequence (multi-frame) ===
    /// Complete explosion animation sequence - captures multiple frames
    ExplosionSequence,
}

impl TestScene {
    /// All available scenes for listing
    pub fn all() -> &'static [TestScene] {
        &[
            // Trail direction tests
            TestScene::FireballTrailEast,
            TestScene::FireballTrailWest,
            TestScene::FireballTrailNorth,
            TestScene::FireballTrailSouth,
            // Trail animation test
            TestScene::TrailNoiseAnimation,
            // Trail growth tests (transition continuity)
            TestScene::TrailGrowthSpawn,
            TestScene::TrailGrowthQuarter,
            TestScene::TrailGrowthHalf,
            TestScene::TrailGrowthFull,
            TestScene::TrailGrowthSequence,
            // Explosion component tests
            TestScene::ExplosionCoreTest,
            TestScene::ExplosionFireTest,
            TestScene::ExplosionSparksTest,
            TestScene::ExplosionEmbersTest,
            TestScene::ExplosionSmokeTest,
            // Full explosion sequence
            TestScene::ExplosionSequence,
        ]
    }

    /// Get the CLI name for this scene
    pub fn name(&self) -> &'static str {
        match self {
            TestScene::FireballTrailEast => "fireball-trail-east",
            TestScene::FireballTrailWest => "fireball-trail-west",
            TestScene::FireballTrailNorth => "fireball-trail-north",
            TestScene::FireballTrailSouth => "fireball-trail-south",
            TestScene::TrailNoiseAnimation => "trail-noise-animation",
            TestScene::TrailGrowthSpawn => "trail-growth-spawn",
            TestScene::TrailGrowthQuarter => "trail-growth-quarter",
            TestScene::TrailGrowthHalf => "trail-growth-half",
            TestScene::TrailGrowthFull => "trail-growth-full",
            TestScene::TrailGrowthSequence => "trail-growth-sequence",
            TestScene::ExplosionCoreTest => "explosion-core",
            TestScene::ExplosionFireTest => "explosion-fire",
            TestScene::ExplosionSparksTest => "explosion-sparks",
            TestScene::ExplosionEmbersTest => "explosion-embers",
            TestScene::ExplosionSmokeTest => "explosion-smoke",
            TestScene::ExplosionSequence => "explosion-sequence",
        }
    }

    /// Camera position for this scene
    pub fn camera_position(&self) -> Vec3 {
        match self {
            // Closer camera for explosion tests to see detail
            TestScene::ExplosionCoreTest
            | TestScene::ExplosionFireTest
            | TestScene::ExplosionSparksTest
            | TestScene::ExplosionEmbersTest
            | TestScene::ExplosionSmokeTest
            | TestScene::ExplosionSequence => Vec3::new(0.0, 5.0, 8.0),
            // Medium camera for trail growth tests to see trail clearly
            TestScene::TrailGrowthSpawn
            | TestScene::TrailGrowthQuarter
            | TestScene::TrailGrowthHalf
            | TestScene::TrailGrowthFull
            | TestScene::TrailGrowthSequence => Vec3::new(0.0, 8.0, 12.0),
            // Zoomed out view to see full fireball trail
            _ => Vec3::new(0.0, 15.0, 25.0),
        }
    }

    /// Camera look-at target
    pub fn camera_target(&self) -> Vec3 {
        Vec3::ZERO
    }

    /// Number of frames to wait before first screenshot (shader compile + stabilize)
    pub fn frames_to_wait(&self) -> u32 {
        match self {
            // Animation tests need time for shaders to compile
            TestScene::TrailNoiseAnimation | TestScene::ExplosionSequence => 60,
            // Single frame tests
            _ => 45,
        }
    }

    /// Total number of frames to capture
    pub fn total_frames(&self) -> u32 {
        match self {
            TestScene::TrailNoiseAnimation => 5,
            TestScene::TrailGrowthSequence => 6,
            TestScene::ExplosionSequence => 8,
            _ => 1,
        }
    }

    /// Frames to wait between captures for multi-frame tests
    pub fn frames_between_captures(&self) -> u32 {
        match self {
            TestScene::TrailNoiseAnimation => 12,  // ~200ms at 60fps
            TestScene::TrailGrowthSequence => 10,  // ~166ms at 60fps for smooth growth
            TestScene::ExplosionSequence => 18,    // ~300ms at 60fps - longer to show smoke rising
            _ => 0,
        }
    }

    /// Get fireball spawn position and direction for trail tests
    fn fireball_config(&self) -> Option<(Vec3, Vec3)> {
        match self {
            TestScene::FireballTrailEast => Some((Vec3::new(-3.0, 1.0, 0.0), Vec3::X)),
            TestScene::FireballTrailWest => Some((Vec3::new(3.0, 1.0, 0.0), Vec3::NEG_X)),
            TestScene::FireballTrailNorth => Some((Vec3::new(0.0, 1.0, 3.0), Vec3::NEG_Z)),
            TestScene::FireballTrailSouth => Some((Vec3::new(0.0, 1.0, -3.0), Vec3::Z)),
            TestScene::TrailNoiseAnimation => Some((Vec3::new(-2.0, 1.0, 0.0), Vec3::X)),
            _ => None,
        }
    }

    /// Get trail growth test config: (position, direction, simulated_travel_distance)
    /// The spawn_position is computed to make travel_distance work correctly.
    fn trail_growth_config(&self) -> Option<(Vec3, Vec3, f32)> {
        use crate::spells::fire::fireball::FIREBALL_TRAIL_GROW_DISTANCE;
        match self {
            // Spawn: 0 distance traveled
            TestScene::TrailGrowthSpawn => Some((Vec3::new(0.0, 1.0, 0.0), Vec3::X, 0.0)),
            // Quarter: 25% of max distance
            TestScene::TrailGrowthQuarter => Some((Vec3::new(0.0, 1.0, 0.0), Vec3::X, FIREBALL_TRAIL_GROW_DISTANCE * 0.25)),
            // Half: 50% of max distance
            TestScene::TrailGrowthHalf => Some((Vec3::new(0.0, 1.0, 0.0), Vec3::X, FIREBALL_TRAIL_GROW_DISTANCE * 0.5)),
            // Full: 100% of max distance
            TestScene::TrailGrowthFull => Some((Vec3::new(0.0, 1.0, 0.0), Vec3::X, FIREBALL_TRAIL_GROW_DISTANCE)),
            // Sequence: starts at 0 and animates (fireball moves during test)
            TestScene::TrailGrowthSequence => Some((Vec3::new(-4.0, 1.0, 0.0), Vec3::X, 0.0)),
            _ => None,
        }
    }

    /// Setup the scene entities
    #[allow(clippy::too_many_arguments)]
    pub fn setup(
        &self,
        commands: &mut Commands,
        meshes: &GameMeshes,
        core_materials: &mut Assets<FireballCoreMaterial>,
        trail_materials: &mut Assets<FireballTrailMaterial>,
        explosion_core_materials: &mut Assets<ExplosionCoreMaterial>,
        explosion_fire_materials: &mut Assets<ExplosionFireMaterial>,
        sparks_materials: &mut Assets<FireballSparksMaterial>,
        embers_materials: &mut Assets<ExplosionEmbersMaterial>,
        smoke_materials: &mut Assets<ExplosionSmokeMaterial>,
    ) {
        // Spawn lighting
        spawn_lighting(commands);

        match self {
            // Fireball trail tests
            TestScene::FireballTrailEast
            | TestScene::FireballTrailWest
            | TestScene::FireballTrailNorth
            | TestScene::FireballTrailSouth
            | TestScene::TrailNoiseAnimation => {
                if let Some((position, direction)) = self.fireball_config() {
                    spawn_test_fireball(
                        commands,
                        position,
                        direction,
                        meshes,
                        core_materials,
                        trail_materials,
                    );
                }
            }

            // Trail growth tests (transition continuity)
            TestScene::TrailGrowthSpawn
            | TestScene::TrailGrowthQuarter
            | TestScene::TrailGrowthHalf
            | TestScene::TrailGrowthFull
            | TestScene::TrailGrowthSequence => {
                if let Some((position, direction, travel_distance)) = self.trail_growth_config() {
                    spawn_test_fireball_with_travel(
                        commands,
                        position,
                        direction,
                        travel_distance,
                        meshes,
                        core_materials,
                        trail_materials,
                    );
                }
            }

            // Explosion component tests
            TestScene::ExplosionCoreTest => {
                spawn_explosion_core_test(commands, meshes, explosion_core_materials);
            }
            TestScene::ExplosionFireTest => {
                spawn_explosion_fire_test(commands, meshes, explosion_fire_materials);
            }
            TestScene::ExplosionSparksTest => {
                spawn_sparks_test(commands, meshes, sparks_materials);
            }
            TestScene::ExplosionEmbersTest => {
                spawn_embers_test(commands, meshes, embers_materials);
            }
            TestScene::ExplosionSmokeTest => {
                spawn_smoke_test(commands, meshes, smoke_materials);
            }
            TestScene::ExplosionSequence => {
                spawn_explosion_sequence(
                    commands,
                    meshes,
                    explosion_core_materials,
                    explosion_fire_materials,
                    embers_materials,
                    smoke_materials,
                );
            }
        }
    }
}

/// Spawn standard lighting for visual tests
fn spawn_lighting(commands: &mut Commands) {
    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
        ..default()
    });
}

/// Spawn a fireball projectile for visual testing
fn spawn_test_fireball(
    commands: &mut Commands,
    position: Vec3,
    direction: Vec3,
    meshes: &GameMeshes,
    core_materials: &mut Assets<FireballCoreMaterial>,
    trail_materials: &mut Assets<FireballTrailMaterial>,
) {
    let direction = direction.normalize();

    // Create fireball projectile component
    let fireball = FireballProjectile::new(direction, 20.0, 10.0, 25.0);

    // Calculate rotation to face travel direction
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);

    // Create core material with velocity direction
    let mut core_material = FireballCoreMaterial::new();
    core_material.set_velocity_direction(direction);
    let core_handle = core_materials.add(core_material);

    // Create trail material with velocity direction
    let mut trail_material = FireballTrailMaterial::new();
    trail_material.set_velocity_direction(direction);
    trail_material.set_trail_length(0.75);
    let trail_handle = trail_materials.add(trail_material);

    // Spawn the fireball entity with core effect
    commands.spawn((
        Mesh3d(meshes.fireball.clone()),
        MeshMaterial3d(core_handle.clone()),
        Transform::from_translation(position)
            .with_rotation(rotation),
        fireball,
        FireballCoreEffect { material_handle: core_handle },
    )).with_children(|parent| {
        // Add trail effect as child
        parent.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(trail_handle.clone()),
            Transform::default(),
            FireballTrailEffect { material_handle: trail_handle },
        ));
    });
}

/// Spawn a fireball with simulated travel distance for trail growth testing.
/// The spawn_position is set behind the current position to simulate having traveled.
#[allow(clippy::too_many_arguments)]
fn spawn_test_fireball_with_travel(
    commands: &mut Commands,
    position: Vec3,
    direction: Vec3,
    travel_distance: f32,
    meshes: &GameMeshes,
    core_materials: &mut Assets<FireballCoreMaterial>,
    trail_materials: &mut Assets<FireballTrailMaterial>,
) {
    use crate::spells::fire::fireball::{FIREBALL_MAX_TRAIL_LENGTH, FIREBALL_TRAIL_GROW_DISTANCE};

    let direction = direction.normalize();

    // Compute spawn_position behind current position to simulate having traveled
    let spawn_position = position - direction * travel_distance;

    // Create fireball projectile with explicit spawn position
    let fireball = FireballProjectile::new_with_spawn(direction, 20.0, 10.0, 25.0, spawn_position);

    // Calculate rotation to face travel direction
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);

    // Create core material with velocity direction
    let mut core_material = FireballCoreMaterial::new();
    core_material.set_velocity_direction(direction);
    let core_handle = core_materials.add(core_material);

    // Create trail material with velocity direction and computed trail length
    let trail_progress = (travel_distance / FIREBALL_TRAIL_GROW_DISTANCE).clamp(0.0, 1.0);
    let trail_length = trail_progress * FIREBALL_MAX_TRAIL_LENGTH;

    let mut trail_material = FireballTrailMaterial::new();
    trail_material.set_velocity_direction(direction);
    trail_material.set_trail_length(trail_length);
    let trail_handle = trail_materials.add(trail_material);

    // Spawn the fireball entity with core effect
    commands.spawn((
        Mesh3d(meshes.fireball.clone()),
        MeshMaterial3d(core_handle.clone()),
        Transform::from_translation(position)
            .with_rotation(rotation),
        fireball,
        FireballCoreEffect { material_handle: core_handle },
    )).with_children(|parent| {
        // Add trail effect as child
        parent.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(trail_handle.clone()),
            Transform::default(),
            FireballTrailEffect { material_handle: trail_handle },
        ));
    });
}

/// Spawn explosion core test - multiple spheres at different progress stages
fn spawn_explosion_core_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionCoreMaterial>,
) {
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionCoreMaterial::new();
        material.set_progress(progress);
        let handle = materials.add(material);

        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0, 0.0))
                .with_scale(Vec3::splat(1.0 + progress * 0.5)),
        ));
    }
}

/// Spawn explosion fire test - multiple spheres at different progress stages
fn spawn_explosion_fire_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionFireMaterial>,
) {
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionFireMaterial::new();
        material.set_progress(progress);
        let handle = materials.add(material);

        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0, 0.0))
                .with_scale(Vec3::splat(1.5 + progress * 0.5)),
        ));
    }
}

/// Spawn sparks test - multiple spark particles at different lifetime stages
fn spawn_sparks_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<FireballSparksMaterial>,
) {
    let lifetime_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let positions = [
        Vec3::new(-3.0, 2.0, 0.0),
        Vec3::new(-1.5, 2.5, 0.5),
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(1.5, 2.5, -0.5),
        Vec3::new(3.0, 2.0, 0.0),
    ];
    let velocities = [
        (Vec3::new(-1.0, 1.0, 0.0), 5.0),
        (Vec3::new(-0.5, 1.0, 0.5), 4.0),
        (Vec3::new(0.0, 1.0, 0.0), 3.0),
        (Vec3::new(0.5, 1.0, -0.5), 4.0),
        (Vec3::new(1.0, 1.0, 0.0), 5.0),
    ];

    for (i, &lifetime) in lifetime_values.iter().enumerate() {
        let mut material = FireballSparksMaterial::new();
        material.set_lifetime_progress(lifetime);
        material.set_velocity(velocities[i].0, velocities[i].1);
        let handle = materials.add(material);

        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(positions[i])
                .with_scale(Vec3::splat(0.5)),  // Increased from 0.2 for visibility
        ));
    }
}

/// Spawn embers test - multiple ember particles at different progress stages
fn spawn_embers_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionEmbersMaterial>,
) {
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let positions = [
        Vec3::new(-3.0, 1.5, 0.0),
        Vec3::new(-1.5, 2.0, 0.5),
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(1.5, 2.0, -0.5),
        Vec3::new(3.0, 1.5, 0.0),
    ];

    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionEmbersMaterial::new();
        material.set_progress(progress);
        material.set_velocity(Vec3::new(0.5, 0.5, 0.0).normalize(), 10.0);
        let handle = materials.add(material);

        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(positions[i])
                .with_scale(Vec3::splat(0.5)),  // Increased from 0.15 for visibility
        ));
    }
}

/// Spawn smoke test - multiple smoke puffs at different progress stages
fn spawn_smoke_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionSmokeMaterial>,
) {
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionSmokeMaterial::new();
        material.set_progress(progress);
        let handle = materials.add(material);

        // Smoke rises and expands as progress increases
        let y_offset = progress * 2.0;
        let scale = 1.0 + progress * 1.5;

        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0 + y_offset, 0.0))
                .with_scale(Vec3::splat(scale)),
        ));
    }
}

/// Spawn complete explosion sequence for animation capture
fn spawn_explosion_sequence(
    commands: &mut Commands,
    meshes: &GameMeshes,
    explosion_core_materials: &mut Assets<ExplosionCoreMaterial>,
    explosion_fire_materials: &mut Assets<ExplosionFireMaterial>,
    embers_materials: &mut Assets<ExplosionEmbersMaterial>,
    _smoke_materials: &mut Assets<ExplosionSmokeMaterial>,  // Unused - SmokePuffSpawner creates materials
) {
    let position = Vec3::new(0.0, 1.0, 0.0);

    // Explosion core (brief white flash)
    let mut core_material = ExplosionCoreMaterial::new();
    core_material.set_progress(0.2);  // Start partway through
    let core_handle = explosion_core_materials.add(core_material);
    commands.spawn((
        Mesh3d(meshes.explosion.clone()),
        MeshMaterial3d(core_handle),
        Transform::from_translation(position).with_scale(Vec3::splat(0.8)),
    ));

    // Explosion fire (main fireball)
    let mut fire_material = ExplosionFireMaterial::new();
    fire_material.set_progress(0.1);
    let fire_handle = explosion_fire_materials.add(fire_material);
    commands.spawn((
        Mesh3d(meshes.explosion.clone()),
        MeshMaterial3d(fire_handle),
        Transform::from_translation(position).with_scale(Vec3::splat(1.5)),
    ));

    // Spawn several embers flying outward
    for i in 0..6 {
        let angle = (i as f32 / 6.0) * std::f32::consts::TAU;
        let ember_dir = Vec3::new(angle.cos(), 0.5, angle.sin()).normalize();
        let ember_pos = position + ember_dir * 0.5;

        let mut embers_material = ExplosionEmbersMaterial::new();
        embers_material.set_progress(0.0);
        embers_material.set_velocity(ember_dir, 12.0);
        let embers_handle = embers_materials.add(embers_material);

        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(embers_handle),
            Transform::from_translation(ember_pos).with_scale(Vec3::splat(0.1)),
        ));
    }

    // Smoke puff spawner - creates multiple puffs over time
    commands.spawn(SmokePuffSpawner::new(position));

}

impl FromStr for TestScene {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fireball-trail-east" => Ok(TestScene::FireballTrailEast),
            "fireball-trail-west" => Ok(TestScene::FireballTrailWest),
            "fireball-trail-north" => Ok(TestScene::FireballTrailNorth),
            "fireball-trail-south" => Ok(TestScene::FireballTrailSouth),
            "trail-noise-animation" => Ok(TestScene::TrailNoiseAnimation),
            "trail-growth-spawn" => Ok(TestScene::TrailGrowthSpawn),
            "trail-growth-quarter" => Ok(TestScene::TrailGrowthQuarter),
            "trail-growth-half" => Ok(TestScene::TrailGrowthHalf),
            "trail-growth-full" => Ok(TestScene::TrailGrowthFull),
            "trail-growth-sequence" => Ok(TestScene::TrailGrowthSequence),
            "explosion-core" => Ok(TestScene::ExplosionCoreTest),
            "explosion-fire" => Ok(TestScene::ExplosionFireTest),
            "explosion-sparks" => Ok(TestScene::ExplosionSparksTest),
            "explosion-embers" => Ok(TestScene::ExplosionEmbersTest),
            "explosion-smoke" => Ok(TestScene::ExplosionSmokeTest),
            "explosion-sequence" => Ok(TestScene::ExplosionSequence),
            "list" => Err("list".to_string()), // Special case handled in main
            _ => Err(format!("Unknown scene: '{}'. Use --screenshot list to see available scenes.", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_from_str() {
        assert_eq!(
            TestScene::from_str("fireball-trail-east").unwrap(),
            TestScene::FireballTrailEast
        );
        assert_eq!(
            TestScene::from_str("explosion-core").unwrap(),
            TestScene::ExplosionCoreTest
        );
    }

    #[test]
    fn test_scene_from_str_unknown() {
        let result = TestScene::from_str("unknown-scene");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown scene"));
    }

    #[test]
    fn test_all_scenes_have_names() {
        for scene in TestScene::all() {
            assert!(!scene.name().is_empty());
        }
    }

    #[test]
    fn test_scene_name_roundtrip() {
        for scene in TestScene::all() {
            let name = scene.name();
            let parsed = TestScene::from_str(name).unwrap();
            assert_eq!(*scene, parsed);
        }
    }

    #[test]
    fn test_fireball_config_directions_normalized() {
        for scene in TestScene::all() {
            if let Some((_, direction)) = scene.fireball_config() {
                let normalized = direction.normalize();
                assert!((direction.length() - normalized.length()).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_multi_frame_scenes_have_multiple_frames() {
        assert!(TestScene::TrailNoiseAnimation.total_frames() > 1);
        assert!(TestScene::ExplosionSequence.total_frames() > 1);
    }

    #[test]
    fn test_single_frame_scenes_have_one_frame() {
        assert_eq!(TestScene::FireballTrailEast.total_frames(), 1);
        assert_eq!(TestScene::ExplosionCoreTest.total_frames(), 1);
    }
}
