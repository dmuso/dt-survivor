pub mod scenes;

use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::render::view::Hdr;
use bevy::render::view::screenshot::Screenshot;

pub use scenes::TestScene;

use crate::game::resources::GameMeshes;
use crate::game::systems::setup_game_assets;
use crate::spells::fire::fireball::{smoke_puff_spawner_system, smoke_puff_effect_update_system};
use crate::spells::fire::fireball_effects::{FireballEffects, init_fireball_effects};
use crate::spells::fire::materials::{
    FireballCoreMaterial, FireballTrailMaterial, FireballChargeMaterial,
    ExplosionCoreMaterial, ExplosionFireMaterial, ExplosionDarkImpactMaterial,
    FireballSparksMaterial, ExplosionEmbersMaterial, ExplosionSmokeMaterial,
    update_explosion_smoke_material_time,
};

/// Resource to track screenshot state
#[derive(Resource)]
pub struct ScreenshotState {
    pub scene: TestScene,
    pub frames_remaining: u32,
    pub screenshot_taken: bool,
    pub exit_frames: u32,
    /// Current frame number for multi-frame captures
    pub current_frame: u32,
    /// Total frames to capture (1 for single, more for animations)
    pub total_frames: u32,
    /// Frames between captures for animation sequences
    pub frames_between_captures: u32,
}

/// Plugin for visual test mode
pub fn plugin(app: &mut App) {
    // Run game asset setup first, init fireball effects, then visual test scene setup
    app.add_systems(Startup, (setup_game_assets, init_fireball_effects, setup_visual_test_scene).chain());
    app.add_systems(Update, take_screenshot_and_exit);

    // Add smoke puff systems for testing multi-puff smoke effects
    app.add_systems(Update, (
        update_explosion_smoke_material_time,
        smoke_puff_spawner_system,
        smoke_puff_effect_update_system,
    ).chain());
}

/// All material assets needed for visual tests
#[allow(clippy::too_many_arguments)]
fn setup_visual_test_scene(
    mut commands: Commands,
    state: Res<ScreenshotState>,
    meshes: Res<GameMeshes>,
    fireball_effects: Option<Res<FireballEffects>>,
    mut core_materials: ResMut<Assets<FireballCoreMaterial>>,
    mut trail_materials: ResMut<Assets<FireballTrailMaterial>>,
    mut charge_materials: ResMut<Assets<FireballChargeMaterial>>,
    mut explosion_core_materials: ResMut<Assets<ExplosionCoreMaterial>>,
    mut explosion_fire_materials: ResMut<Assets<ExplosionFireMaterial>>,
    mut explosion_dark_impact_materials: ResMut<Assets<ExplosionDarkImpactMaterial>>,
    mut sparks_materials: ResMut<Assets<FireballSparksMaterial>>,
    mut embers_materials: ResMut<Assets<ExplosionEmbersMaterial>>,
    mut smoke_materials: ResMut<Assets<ExplosionSmokeMaterial>>,
) {
    // Setup camera
    let scene = &state.scene;
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::TonyMcMapface,
        Transform::from_translation(scene.camera_position())
            .looking_at(scene.camera_target(), Vec3::Y),
        Bloom {
            intensity: 0.3,
            ..default()
        },
    ));

    // Setup the scene with all available materials
    scene.setup(
        &mut commands,
        &meshes,
        fireball_effects.as_deref(),
        &mut core_materials,
        &mut trail_materials,
        &mut charge_materials,
        &mut explosion_core_materials,
        &mut explosion_fire_materials,
        &mut explosion_dark_impact_materials,
        &mut sparks_materials,
        &mut embers_materials,
        &mut smoke_materials,
    );
}

fn take_screenshot_and_exit(
    mut commands: Commands,
    mut state: ResMut<ScreenshotState>,
    mut exit: MessageWriter<AppExit>,
) {
    // Wait for initial frames to render
    if state.frames_remaining > 0 {
        state.frames_remaining -= 1;
        return;
    }

    // Take screenshot if not already done for this frame
    if !state.screenshot_taken {
        // Generate filename - include frame number for multi-frame captures
        let filename = if state.total_frames > 1 {
            format!("tmp/screenshots/{}-{:03}.png", state.scene.name(), state.current_frame)
        } else {
            format!("tmp/screenshots/{}.png", state.scene.name())
        };

        // Ensure directory exists
        std::fs::create_dir_all("tmp/screenshots").ok();

        commands.spawn(Screenshot::primary_window()).observe(
            move |trigger: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                let captured = trigger.event();
                match captured.image.clone().try_into_dynamic() {
                    Ok(dyn_img) => {
                        if let Err(e) = dyn_img.save(&filename) {
                            eprintln!("Failed to save screenshot: {}", e);
                        } else {
                            println!("Screenshot saved to: {}", filename);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to convert screenshot: {:?}", e);
                    }
                }
            },
        );

        state.screenshot_taken = true;
        state.exit_frames = 3; // Wait frames for screenshot to process
        return;
    }

    // Wait for exit frames after screenshot
    if state.exit_frames > 0 {
        state.exit_frames -= 1;
        return;
    }

    // Check if we need more frames
    state.current_frame += 1;
    if state.current_frame < state.total_frames {
        // Reset for next frame capture
        state.screenshot_taken = false;
        state.frames_remaining = state.frames_between_captures;
        return;
    }

    // All frames captured - exit
    exit.write(AppExit::Success);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_state_creation() {
        let state = ScreenshotState {
            scene: TestScene::FireballTrailEast,
            frames_remaining: 30,
            screenshot_taken: false,
            exit_frames: 0,
            current_frame: 0,
            total_frames: 1,
            frames_between_captures: 10,
        };
        assert_eq!(state.frames_remaining, 30);
        assert!(!state.screenshot_taken);
        assert_eq!(state.exit_frames, 0);
        assert_eq!(state.current_frame, 0);
        assert_eq!(state.total_frames, 1);
    }

    #[test]
    fn test_multi_frame_state() {
        let state = ScreenshotState {
            scene: TestScene::TrailNoiseAnimation,
            frames_remaining: 60,
            screenshot_taken: false,
            exit_frames: 0,
            current_frame: 0,
            total_frames: 5,
            frames_between_captures: 15,
        };
        assert_eq!(state.total_frames, 5);
        assert_eq!(state.frames_between_captures, 15);
    }
}
