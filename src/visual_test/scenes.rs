use bevy::prelude::*;
use std::str::FromStr;

use crate::game::resources::GameMeshes;
use bevy_hanabi::prelude::*;
use crate::spells::fire::fireball::{
    FireballProjectile, FireballCoreEffect, FireballTrailEffect,
    ChargingFireball, FireballChargeEffect, FireballChargeParticles,
    BillowingFireSpawner,
};
use crate::spells::fire::fireball_effects::FireballEffects;
use crate::spells::fire::materials::{
    FireballCoreMaterial, FireballTrailMaterial, FireballChargeMaterial,
    ExplosionCoreMaterial, ExplosionFireMaterial, ExplosionDarkImpactMaterial,
    FireballSparksMaterial, ExplosionEmbersMaterial,
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
    /// Explosion star-burst test - 5 star-bursts at different progress stages
    ExplosionStarburstTest,
    /// Explosion dark impact (dark silhouette spikes) at different progress stages
    ExplosionDarkImpactTest,
    /// Explosion fire (main fireball blast) at different progress stages
    ExplosionFireTest,
    /// Fireball sparks flying embers
    ExplosionSparksTest,
    /// Explosion embers flying debris
    ExplosionEmbersTest,

    // === Full Explosion Sequence (multi-frame) ===
    /// Complete explosion animation sequence - captures multiple frames
    ExplosionSequence,

    // === Dark Projectiles Test ===
    /// Dark projectiles test - 14 elongated projectiles at various speeds
    ExplosionDarkProjectilesTest,

    // === Billowing Fire Tests ===
    /// Billowing fire spheres test - 8 spheres expanding outward at different stages
    ExplosionBillowingFireTest,

    // === Ash Float Tests ===
    /// Ash float behavior test - projectiles at slow speeds showing circular shape and drift
    ExplosionAshFloatTest,

    // === Fire to Smoke Transition Tests ===
    /// Fire to smoke color transition test - 5 spheres showing progress 0.2 to 0.8
    ExplosionFireToSmokeTest,

    // === Smoke Dissipation Tests ===
    /// Smoke dissipation test - 5 spheres at progress [0.7, 0.8, 0.85, 0.9, 0.95] showing rising mask
    ExplosionSmokeDissipationTest,

    // === Full Explosion Integration Test ===
    /// Comprehensive 7-stage explosion sequence test - captures 12 frames (~2s animation)
    /// Tests all stages: star-burst, dark impact, billowing fire, dark projectiles, ash floats,
    /// fire-to-smoke transition, and smoke dissipation
    ExplosionFullSequenceNew,

    // === Fireball Charge Phase Tests ===
    /// Charging fireball with inward particle effects
    FireballChargeParticles,
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
            TestScene::ExplosionStarburstTest,
            TestScene::ExplosionDarkImpactTest,
            TestScene::ExplosionFireTest,
            TestScene::ExplosionSparksTest,
            TestScene::ExplosionEmbersTest,
            // Full explosion sequence
            TestScene::ExplosionSequence,
            // Dark projectiles test
            TestScene::ExplosionDarkProjectilesTest,
            // Billowing fire test
            TestScene::ExplosionBillowingFireTest,
            // Ash float test
            TestScene::ExplosionAshFloatTest,
            // Fire to smoke transition test
            TestScene::ExplosionFireToSmokeTest,
            // Smoke dissipation test
            TestScene::ExplosionSmokeDissipationTest,
            // Full explosion integration test
            TestScene::ExplosionFullSequenceNew,
            // Charge phase tests
            TestScene::FireballChargeParticles,
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
            TestScene::ExplosionStarburstTest => "explosion-starburst",
            TestScene::ExplosionDarkImpactTest => "explosion-dark-impact",
            TestScene::ExplosionFireTest => "explosion-fire",
            TestScene::ExplosionSparksTest => "explosion-sparks",
            TestScene::ExplosionEmbersTest => "explosion-embers",
            TestScene::ExplosionSequence => "explosion-sequence",
            TestScene::ExplosionDarkProjectilesTest => "explosion-dark-projectiles",
            TestScene::ExplosionBillowingFireTest => "explosion-billowing-fire",
            TestScene::ExplosionAshFloatTest => "explosion-ash-float",
            TestScene::ExplosionFireToSmokeTest => "explosion-fire-to-smoke",
            TestScene::ExplosionSmokeDissipationTest => "explosion-smoke-dissipation",
            TestScene::ExplosionFullSequenceNew => "explosion-full-sequence-new",
            TestScene::FireballChargeParticles => "fireball-charge-particles",
        }
    }

    /// Camera position for this scene
    pub fn camera_position(&self) -> Vec3 {
        match self {
            // Closer camera for explosion tests to see detail
            TestScene::ExplosionCoreTest
            | TestScene::ExplosionStarburstTest
            | TestScene::ExplosionDarkImpactTest
            | TestScene::ExplosionFireTest
            | TestScene::ExplosionSparksTest
            | TestScene::ExplosionEmbersTest
            | TestScene::ExplosionSequence => Vec3::new(0.0, 5.0, 8.0),
            // Wider camera for dark projectiles to see elongation and spread
            TestScene::ExplosionDarkProjectilesTest => Vec3::new(0.0, 6.0, 10.0),
            // Wider camera for billowing fire to capture all 8 spheres expanding outward
            TestScene::ExplosionBillowingFireTest => Vec3::new(0.0, 8.0, 12.0),
            // Medium camera for ash float to see drift and circular shape
            TestScene::ExplosionAshFloatTest => Vec3::new(0.0, 6.0, 10.0),
            // Medium camera for fire to smoke transition to see color gradient
            TestScene::ExplosionFireToSmokeTest => Vec3::new(0.0, 5.0, 10.0),
            // Medium camera for smoke dissipation to see mask effect clearly
            TestScene::ExplosionSmokeDissipationTest => Vec3::new(0.0, 5.0, 10.0),
            // Wide camera for full explosion sequence to capture 3-row layout
            TestScene::ExplosionFullSequenceNew => Vec3::new(0.0, 0.5, 18.0),
            // Medium camera for trail growth tests to see trail clearly
            TestScene::TrailGrowthSpawn
            | TestScene::TrailGrowthQuarter
            | TestScene::TrailGrowthHalf
            | TestScene::TrailGrowthFull
            | TestScene::TrailGrowthSequence => Vec3::new(0.0, 8.0, 12.0),
            // Closer camera for charge particles to see detail
            TestScene::FireballChargeParticles => Vec3::new(0.0, 4.0, 6.0),
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
            // Trail noise needs time for shaders to compile
            TestScene::TrailNoiseAnimation => 60,
            // Explosion sequence: static materials, wait for shader compile
            TestScene::ExplosionSequence => 45,
            // Full sequence: wait for shaders to compile (static materials, no despawn)
            TestScene::ExplosionFullSequenceNew => 45,
            // Billowing fire: only wait for shaders to compile, capture quickly (0.8s lifetime)
            TestScene::ExplosionBillowingFireTest => 10,
            // Dark projectiles: wait for shader compile
            TestScene::ExplosionDarkProjectilesTest => 30,
            // Ash float: wait for projectiles to slow down into ash float phase
            TestScene::ExplosionAshFloatTest => 30,
            // Hanabi particles need more warmup time
            TestScene::FireballChargeParticles => 120,
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
            // Full sequence: 12 frames to capture ~2s animation lifecycle
            TestScene::ExplosionFullSequenceNew => 12,
            TestScene::ExplosionDarkProjectilesTest => 5, // Capture projectile flight and deceleration
            TestScene::ExplosionBillowingFireTest => 6, // Capture expansion animation
            TestScene::ExplosionAshFloatTest => 6, // Capture ash drift animation
            TestScene::FireballChargeParticles => 8, // Capture particle flow animation
            _ => 1,
        }
    }

    /// Frames to wait between captures for multi-frame tests
    pub fn frames_between_captures(&self) -> u32 {
        match self {
            TestScene::TrailNoiseAnimation => 12,  // ~200ms at 60fps
            TestScene::TrailGrowthSequence => 10,  // ~166ms at 60fps for smooth growth
            TestScene::ExplosionSequence => 10,    // ~167ms at 60fps - show shader time-based animation
            // Full sequence: ~83ms between captures (5 frames) to show all 7 stages over ~1s
            // This faster rate captures more of the early star-burst phase
            TestScene::ExplosionFullSequenceNew => 5,
            TestScene::ExplosionDarkProjectilesTest => 6, // ~100ms at 60fps to show deceleration
            TestScene::ExplosionBillowingFireTest => 8, // ~133ms at 60fps to show sphere expansion
            TestScene::ExplosionAshFloatTest => 8, // ~133ms at 60fps to show ash drift
            TestScene::FireballChargeParticles => 10, // ~166ms at 60fps to show particle flow
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
        fireball_effects: Option<&FireballEffects>,
        core_materials: &mut Assets<FireballCoreMaterial>,
        trail_materials: &mut Assets<FireballTrailMaterial>,
        charge_materials: &mut Assets<FireballChargeMaterial>,
        explosion_core_materials: &mut Assets<ExplosionCoreMaterial>,
        explosion_fire_materials: &mut Assets<ExplosionFireMaterial>,
        explosion_dark_impact_materials: &mut Assets<ExplosionDarkImpactMaterial>,
        sparks_materials: &mut Assets<FireballSparksMaterial>,
        embers_materials: &mut Assets<ExplosionEmbersMaterial>,
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
            TestScene::ExplosionStarburstTest => {
                spawn_explosion_starburst_test(commands, meshes, explosion_core_materials);
            }
            TestScene::ExplosionDarkImpactTest => {
                spawn_explosion_dark_impact_test(commands, meshes, explosion_dark_impact_materials);
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
            TestScene::ExplosionSequence => {
                spawn_explosion_sequence(
                    commands,
                    meshes,
                    explosion_core_materials,
                    explosion_fire_materials,
                    embers_materials,
                );
            }

            TestScene::ExplosionDarkProjectilesTest => {
                spawn_dark_projectiles_test(commands, meshes, explosion_fire_materials, embers_materials);
            }

            TestScene::ExplosionBillowingFireTest => {
                spawn_billowing_fire_test(commands, meshes, explosion_fire_materials);
            }

            TestScene::ExplosionAshFloatTest => {
                spawn_ash_float_test(commands, meshes, embers_materials);
            }

            TestScene::ExplosionFireToSmokeTest => {
                spawn_fire_to_smoke_test(commands, meshes, explosion_fire_materials);
            }

            TestScene::ExplosionSmokeDissipationTest => {
                spawn_smoke_dissipation_test(commands, meshes, explosion_fire_materials);
            }

            TestScene::ExplosionFullSequenceNew => {
                spawn_explosion_full_sequence_new(
                    commands,
                    meshes,
                    explosion_core_materials,
                    explosion_dark_impact_materials,
                    explosion_fire_materials,
                    embers_materials,
                );
            }

            TestScene::FireballChargeParticles => {
                spawn_charging_fireball_test(
                    commands,
                    meshes,
                    fireball_effects,
                    core_materials,
                    charge_materials,
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

/// Spawn explosion star-burst test - 5 star-bursts at different progress stages
/// Shows the spike vertex displacement effect with different random seeds
fn spawn_explosion_starburst_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionCoreMaterial>,
) {
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionCoreMaterial::new();
        material.set_progress(progress);
        // Give each star-burst a different random seed for variety
        material.set_spike_seed(i as f32 * 0.17);
        // Vary spike count: 5, 6, 7, 6, 5
        let spike_counts = [5.0, 6.0, 7.0, 6.0, 5.0];
        material.set_spike_count(spike_counts[i]);
        let handle = materials.add(material);

        // Star-bursts are larger to show spike detail
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0, 0.0))
                .with_scale(Vec3::splat(1.5)),
        ));
    }
}

/// Spawn explosion dark impact test - 5 dark spike silhouettes at different progress stages
fn spawn_explosion_dark_impact_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionDarkImpactMaterial>,
) {
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionDarkImpactMaterial::new();
        material.set_progress(progress);
        // Give each dark impact a different random seed for variety
        material.set_spike_seed(i as f32 * 0.23);
        // Vary spike count: 5, 6, 7, 6, 5
        let spike_counts = [5.0, 6.0, 7.0, 6.0, 5.0];
        material.set_spike_count(spike_counts[i]);
        let handle = materials.add(material);

        // Dark impacts are similar size to star-bursts
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0, 0.0))
                .with_scale(Vec3::splat(1.5)),
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

/// Spawn dark projectiles test - static showcase at different speeds (no animation)
/// Shows elongation effect at various velocities without movement
fn spawn_dark_projectiles_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    fire_materials: &mut Assets<ExplosionFireMaterial>,
    embers_materials: &mut Assets<ExplosionEmbersMaterial>,
) {
    use crate::spells::fire::fireball::DARK_PROJECTILE_SCALE;

    // Spawn lighting
    spawn_lighting(commands);

    // Spawn a bright background so dark projectiles are visible
    let mut bg_material = ExplosionFireMaterial::new();
    bg_material.set_progress(0.2);
    let bg_handle = fire_materials.add(bg_material);
    commands.spawn((
        Mesh3d(meshes.explosion.clone()),
        MeshMaterial3d(bg_handle),
        Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)).with_scale(Vec3::splat(3.0)),
    ));

    // Spawn 5 static projectiles at different speeds to show elongation
    // NO ExplosionEmbersEffect - these are static showcases
    let speeds = [18.0, 12.0, 6.0, 2.0, 0.5];  // Fast to slow (elongated to circular)
    let x_positions = [-3.0, -1.5, 0.0, 1.5, 3.0];

    for (i, (&speed, &x)) in speeds.iter().zip(x_positions.iter()).enumerate() {
        let mut material = ExplosionEmbersMaterial::new();
        // Direction pointing right so elongation is visible
        material.set_velocity(Vec3::X, speed);
        // Vary progress slightly for visual variety
        material.set_progress(0.1 + i as f32 * 0.1);
        let handle = embers_materials.add(material);

        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, 1.0, 2.0))
                .with_scale(Vec3::splat(DARK_PROJECTILE_SCALE * 2.0)),  // Larger for visibility
        ));
    }
}

/// Spawn complete explosion sequence for animation capture.
/// Uses BillowingFireSpawner to create a dynamic animated explosion.
/// Animation comes from Effect components that drive material progress over time.
fn spawn_explosion_sequence(
    commands: &mut Commands,
    meshes: &GameMeshes,
    explosion_core_materials: &mut Assets<ExplosionCoreMaterial>,
    explosion_fire_materials: &mut Assets<ExplosionFireMaterial>,
    embers_materials: &mut Assets<ExplosionEmbersMaterial>,
) {
    let position = Vec3::new(0.0, 1.0, 0.0);

    // Spawn static explosion core at various progress stages for comparison
    // (Effect components cause despawn before shaders compile, so use static materials)
    let progress_values = [0.0, 0.2, 0.4, 0.6, 0.8];
    let x_offsets = [-3.0, -1.5, 0.0, 1.5, 3.0];

    // Row 1: Explosion core at different progress stages
    for (i, &progress) in progress_values.iter().enumerate() {
        let mut core_material = ExplosionCoreMaterial::new();
        core_material.set_progress(progress);
        let core_handle = explosion_core_materials.add(core_material);
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(core_handle),
            Transform::from_translation(Vec3::new(x_offsets[i], position.y + 2.0, 0.0))
                .with_scale(Vec3::splat(0.6)),
        ));
    }

    // Row 2: Explosion fire at different progress stages
    for (i, &progress) in progress_values.iter().enumerate() {
        let mut fire_material = ExplosionFireMaterial::new();
        fire_material.set_progress(progress);
        fire_material.set_velocity(Vec3::Y, 1.0); // Enable billowing
        let fire_handle = explosion_fire_materials.add(fire_material);
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(fire_handle),
            Transform::from_translation(Vec3::new(x_offsets[i], position.y, 0.0))
                .with_scale(Vec3::splat(1.0 + progress * 0.5)),
        ));
    }

    // Row 3: Embers at different progress stages with varying velocities
    let speeds = [15.0, 10.0, 5.0, 2.0, 0.5];
    for (i, (&progress, &speed)) in progress_values.iter().zip(speeds.iter()).enumerate() {
        let mut embers_material = ExplosionEmbersMaterial::new();
        embers_material.set_progress(progress);
        embers_material.set_velocity(Vec3::X, speed);
        let embers_handle = embers_materials.add(embers_material);
        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(embers_handle),
            Transform::from_translation(Vec3::new(x_offsets[i], position.y - 2.0, 0.0))
                .with_scale(Vec3::splat(0.3)),
        ));
    }
}

/// Spawn billowing fire test - spawns 8 fire spheres with billowing displacement
fn spawn_billowing_fire_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    fire_materials: &mut Assets<ExplosionFireMaterial>,
) {
    let position = Vec3::new(0.0, 1.0, 0.0);

    // Spawn lighting for the scene
    spawn_lighting(commands);

    // Spawn 8 fire spheres directly for visual testing
    // (These simulate what BillowingFireSpawner would create)
    for i in 0..8 {
        let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
        let elevation = -0.2 + (i as f32 / 8.0) * 0.5;
        let direction = Vec3::new(angle.cos(), elevation, angle.sin()).normalize();
        let speed = 3.0 + (i as f32 / 8.0) * 2.0; // 3.0 to 5.0
        let growth_rate = 1.5 + (i as f32 / 8.0) * 1.0; // 1.5 to 2.5

        // Create material with velocity and growth rate for billowing effect
        let mut material = ExplosionFireMaterial::new();
        material.set_velocity(direction, speed);
        material.set_growth_rate(growth_rate);
        material.set_progress(0.3); // Start at 30% progress to show the effect
        let handle = fire_materials.add(material);

        // Initial offset from center
        let offset = direction * 0.5;

        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(position + offset).with_scale(Vec3::splat(0.8)),
        ));
    }
}

/// Spawn fire to smoke transition test - 5 spheres at progress [0.2, 0.4, 0.5, 0.6, 0.8]
/// Shows the gradual color transition from orange fire to gray smoke
fn spawn_fire_to_smoke_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionFireMaterial>,
) {
    // Progress values spanning fire (0.0-0.4), transition (0.4-0.7), and smoke (0.7-1.0)
    let progress_values = [0.2, 0.4, 0.5, 0.6, 0.8];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        // Create fire material with billowing enabled (non-zero velocity)
        let mut material = ExplosionFireMaterial::new();
        material.set_progress(progress);
        // Set minimal velocity to enable billowing displacement for organic look
        material.set_velocity(Vec3::Y, 0.5);
        material.set_growth_rate(2.0);
        let handle = materials.add(material);

        // Scale increases with progress (spheres grow as they become smoke)
        let scale = 1.5 + progress * 0.8;

        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0, 0.0))
                .with_scale(Vec3::splat(scale)),
        ));
    }
}

/// Spawn smoke dissipation test - 5 spheres at progress [0.7, 0.8, 0.85, 0.9, 0.95]
/// Shows the rising mask that "eats" smoke from below
fn spawn_smoke_dissipation_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionFireMaterial>,
) {
    // Progress values in the smoke dissipation phase (0.7-1.0)
    let progress_values = [0.7, 0.8, 0.85, 0.9, 0.95];
    let x_positions = [-4.0, -2.0, 0.0, 2.0, 4.0];

    for (i, &progress) in progress_values.iter().enumerate() {
        // Create fire material with billowing enabled (non-zero velocity)
        let mut material = ExplosionFireMaterial::new();
        material.set_progress(progress);
        // Set minimal velocity to enable billowing displacement for organic look
        material.set_velocity(Vec3::Y, 0.5);
        material.set_growth_rate(2.0);
        let handle = materials.add(material);

        // Scale is larger for smoke phase
        let scale = 2.0;

        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x_positions[i], 1.0, 0.0))
                .with_scale(Vec3::splat(scale)),
        ));
    }
}

/// Spawn ash float test - static showcase at different speeds showing circular shape
/// Shows elongation to circular transition as speed decreases (no animation/despawn)
/// Note: Static materials don't animate - use this to verify shader behavior at different speeds
fn spawn_ash_float_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<ExplosionEmbersMaterial>,
) {
    use crate::spells::fire::fireball::DARK_PROJECTILE_SCALE;

    let position = Vec3::new(0.0, 1.0, 0.0);

    // Spawn 10 STATIC projectiles at various speeds to show elongation to circular transition
    // NO ExplosionEmbersEffect - these are static showcases (no despawn, no animation)
    // Speeds above ASH_FADE_SPEED_MAX (1.5) to keep particles visible
    let speeds = [
        15.0, // Fast - very elongated
        12.0, // Fast - elongated
        9.0,  // Medium-fast - moderately elongated
        6.0,  // Medium - somewhat elongated
        4.0,  // Slower - slightly elongated
        3.0,  // At threshold (2.0) - transitioning to circular
        2.5,  // Below threshold - mostly circular
        2.2,  // Near threshold - almost circular
        2.0,  // At threshold - circular
        1.8,  // Just below - circular (slight fade begins)
    ];

    for (i, &speed) in speeds.iter().enumerate() {
        // Spread projectiles in a circle for visibility
        let angle = (i as f32 / speeds.len() as f32) * std::f32::consts::TAU;
        let direction = Vec3::new(angle.cos(), 0.1, angle.sin()).normalize();
        let spawn_offset = direction * (2.0 + i as f32 * 0.2);

        // Create STATIC material with velocity (no Effect component = no despawn)
        let mut material = ExplosionEmbersMaterial::new();
        material.set_velocity(direction, speed);
        // Keep progress low so particles are visible (high progress = faded)
        material.set_progress(0.1);
        // Increase emissive so particles are more visible
        material.set_emissive_intensity(2.0);
        let handle = materials.add(material);

        // Spawn WITHOUT ExplosionEmbersEffect - static showcase
        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(position + spawn_offset)
                .with_scale(Vec3::splat(DARK_PROJECTILE_SCALE * 1.5)), // Larger for visibility
        ));
    }
}

/// Spawn comprehensive 7-stage explosion sequence for full integration visual testing.
/// Uses STATIC materials (no Effect components) so entities persist for shader compilation.
/// Animation comes from shader time uniforms and material progress values.
///
/// Layout shows all 7 stages side-by-side for visual comparison:
/// - Row 1: Star-burst (5 progress values), Dark impact (5 progress values)
/// - Row 2: Billowing fire (5 progress values), Dark projectiles (5 speeds)
/// - Row 3: Fire-to-smoke (5 progress values), Smoke dissipation (5 progress values)
#[allow(clippy::too_many_arguments)]
fn spawn_explosion_full_sequence_new(
    commands: &mut Commands,
    meshes: &GameMeshes,
    core_materials: &mut Assets<ExplosionCoreMaterial>,
    dark_impact_materials: &mut Assets<ExplosionDarkImpactMaterial>,
    fire_materials: &mut Assets<ExplosionFireMaterial>,
    embers_materials: &mut Assets<ExplosionEmbersMaterial>,
) {
    // Row 1: Star-burst (left) and Dark impact (right)
    let row1_y = 3.0;
    let star_burst_x = -4.0;
    let dark_impact_x = 4.0;

    // Star-burst at 5 progress stages
    let progress_values = [0.0, 0.25, 0.5, 0.75, 1.0];
    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionCoreMaterial::new();
        material.set_progress(progress);
        material.set_spike_seed(i as f32 * 0.17);
        let handle = core_materials.add(material);

        let x = star_burst_x + (i as f32 - 2.0) * 1.5;
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, row1_y, 0.0))
                .with_scale(Vec3::splat(1.2)),
        ));
    }

    // Dark impact at 5 progress stages
    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionDarkImpactMaterial::new();
        material.set_progress(progress);
        material.set_spike_seed(i as f32 * 0.23);
        let handle = dark_impact_materials.add(material);

        let x = dark_impact_x + (i as f32 - 2.0) * 1.5;
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, row1_y, 0.0))
                .with_scale(Vec3::splat(1.2)),
        ));
    }

    // Row 2: Billowing fire (left) and Dark projectiles (right)
    let row2_y = 0.5;
    let fire_x = -4.0;
    let projectile_x = 4.0;

    // Billowing fire at 5 progress stages (with velocity for displacement)
    for (i, &progress) in progress_values.iter().enumerate() {
        let mut material = ExplosionFireMaterial::new();
        material.set_progress(progress);
        material.set_velocity(Vec3::Y, 2.0); // Enable billowing displacement
        material.set_growth_rate(2.0);
        let handle = fire_materials.add(material);

        let x = fire_x + (i as f32 - 2.0) * 1.5;
        let scale = 1.0 + progress * 0.5; // Grow with progress
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, row2_y, 0.0))
                .with_scale(Vec3::splat(scale)),
        ));
    }

    // Dark projectiles at 5 different speeds (showing elongation)
    let speeds = [15.0, 10.0, 5.0, 2.0, 0.5]; // Fast to slow
    for (i, &speed) in speeds.iter().enumerate() {
        let mut material = ExplosionEmbersMaterial::new();
        material.set_velocity(Vec3::X, speed); // Horizontal for visible elongation
        material.set_progress(0.2 + i as f32 * 0.1);
        let handle = embers_materials.add(material);

        let x = projectile_x + (i as f32 - 2.0) * 1.5;
        commands.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, row2_y, 0.0))
                .with_scale(Vec3::splat(0.4)),
        ));
    }

    // Row 3: Fire-to-smoke (left) and Smoke dissipation (right)
    let row3_y = -2.5;
    let fire_smoke_x = -4.0;
    let dissipation_x = 4.0;

    // Fire-to-smoke transition (progress 0.2-0.8)
    let fire_smoke_progress = [0.2, 0.35, 0.5, 0.65, 0.8];
    for (i, &progress) in fire_smoke_progress.iter().enumerate() {
        let mut material = ExplosionFireMaterial::new();
        material.set_progress(progress);
        material.set_velocity(Vec3::Y, 0.5);
        material.set_growth_rate(2.0);
        let handle = fire_materials.add(material);

        let x = fire_smoke_x + (i as f32 - 2.0) * 1.5;
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, row3_y, 0.0))
                .with_scale(Vec3::splat(1.3)),
        ));
    }

    // Smoke dissipation (progress 0.7-0.95)
    let dissipation_progress = [0.7, 0.8, 0.85, 0.9, 0.95];
    for (i, &progress) in dissipation_progress.iter().enumerate() {
        let mut material = ExplosionFireMaterial::new();
        material.set_progress(progress);
        material.set_velocity(Vec3::Y, 0.5);
        material.set_growth_rate(2.0);
        let handle = fire_materials.add(material);

        let x = dissipation_x + (i as f32 - 2.0) * 1.5;
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(handle),
            Transform::from_translation(Vec3::new(x, row3_y, 0.0))
                .with_scale(Vec3::splat(1.5)),
        ));
    }
}

/// Spawn a charging fireball with Hanabi particles for visual testing
fn spawn_charging_fireball_test(
    commands: &mut Commands,
    meshes: &GameMeshes,
    fireball_effects: Option<&FireballEffects>,
    core_materials: &mut Assets<FireballCoreMaterial>,
    charge_materials: &mut Assets<FireballChargeMaterial>,
) {
    let position = Vec3::new(0.0, 2.0, 0.0);
    let direction = Vec3::X;

    // Create core material
    let mut core_material = FireballCoreMaterial::new();
    core_material.set_velocity_direction(direction);
    let core_handle = core_materials.add(core_material);

    // Create charge swirl material at 50% progress
    let mut charge_material = FireballChargeMaterial::new();
    charge_material.set_charge_progress(0.5);
    charge_material.set_outer_radius(1.0);
    let charge_handle = charge_materials.add(charge_material);

    // Create a mock ChargingFireball component
    let charging = ChargingFireball::new(direction, 25.0);

    // Spawn the charging fireball with effects
    commands.spawn((
        Mesh3d(meshes.fireball.clone()),
        MeshMaterial3d(core_handle.clone()),
        Transform::from_translation(position).with_scale(Vec3::splat(0.5)), // 50% charge scale
        charging,
        FireballCoreEffect { material_handle: core_handle },
    )).with_children(|parent| {
        // Add charge swirl shader effect
        parent.spawn((
            Mesh3d(meshes.fireball.clone()),
            MeshMaterial3d(charge_handle.clone()),
            Transform::from_scale(Vec3::splat(1.0)),
            FireballChargeEffect { material_handle: charge_handle },
        ));
    });

    // Spawn Hanabi charge particles at world level (same position as fireball)
    if let Some(effects) = fireball_effects {
        commands.spawn((
            ParticleEffect::new(effects.charge_effect.clone()),
            EffectMaterial {
                images: vec![effects.charge_texture.clone()],
            },
            Transform::from_translation(position),
            Visibility::Visible,
            FireballChargeParticles,
        ));
    }
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
            "explosion-starburst" => Ok(TestScene::ExplosionStarburstTest),
            "explosion-dark-impact" => Ok(TestScene::ExplosionDarkImpactTest),
            "explosion-fire" => Ok(TestScene::ExplosionFireTest),
            "explosion-sparks" => Ok(TestScene::ExplosionSparksTest),
            "explosion-embers" => Ok(TestScene::ExplosionEmbersTest),
            "explosion-sequence" => Ok(TestScene::ExplosionSequence),
            "explosion-dark-projectiles" => Ok(TestScene::ExplosionDarkProjectilesTest),
            "explosion-billowing-fire" => Ok(TestScene::ExplosionBillowingFireTest),
            "explosion-ash-float" => Ok(TestScene::ExplosionAshFloatTest),
            "explosion-fire-to-smoke" => Ok(TestScene::ExplosionFireToSmokeTest),
            "explosion-smoke-dissipation" => Ok(TestScene::ExplosionSmokeDissipationTest),
            "explosion-full-sequence-new" => Ok(TestScene::ExplosionFullSequenceNew),
            "fireball-charge-particles" => Ok(TestScene::FireballChargeParticles),
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
