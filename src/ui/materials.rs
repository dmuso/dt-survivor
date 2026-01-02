use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Shader asset path constant.
pub const RADIAL_COOLDOWN_SHADER: &str = "shaders/radial_cooldown.wgsl";

/// Material for rendering radial cooldown overlays on spell icons.
///
/// The overlay sweeps clockwise from 12 o'clock position.
/// - progress = 0.0: Full overlay (on cooldown)
/// - progress = 1.0: No overlay (ready)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct RadialCooldownMaterial {
    /// Progress value (0.0 to 1.0) stored in x component.
    /// Vec4 used for 16-byte alignment (WebGL2 requirement).
    #[uniform(0)]
    pub progress: Vec4,

    /// Overlay color (semi-transparent black by default).
    #[uniform(0)]
    pub overlay_color: Vec4,
}

impl Default for RadialCooldownMaterial {
    fn default() -> Self {
        Self {
            progress: Vec4::ZERO,
            overlay_color: Vec4::new(0.0, 0.0, 0.0, 0.7), // 70% black overlay
        }
    }
}

impl RadialCooldownMaterial {
    /// Create a new material with the given progress (0.0 to 1.0).
    pub fn new(progress: f32) -> Self {
        Self {
            progress: Vec4::new(progress.clamp(0.0, 1.0), 0.0, 0.0, 0.0),
            ..default()
        }
    }

    /// Update the progress value.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress.x = progress.clamp(0.0, 1.0);
    }
}

impl UiMaterial for RadialCooldownMaterial {
    fn fragment_shader() -> ShaderRef {
        RADIAL_COOLDOWN_SHADER.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_cooldown_material_default_progress_is_zero() {
        let material = RadialCooldownMaterial::default();
        assert_eq!(material.progress.x, 0.0);
    }

    #[test]
    fn radial_cooldown_material_default_overlay_is_semi_transparent_black() {
        let material = RadialCooldownMaterial::default();
        assert_eq!(material.overlay_color.x, 0.0); // R
        assert_eq!(material.overlay_color.y, 0.0); // G
        assert_eq!(material.overlay_color.z, 0.0); // B
        assert_eq!(material.overlay_color.w, 0.7); // A
    }

    #[test]
    fn radial_cooldown_material_new_sets_progress() {
        let material = RadialCooldownMaterial::new(0.5);
        assert_eq!(material.progress.x, 0.5);
    }

    #[test]
    fn radial_cooldown_material_new_clamps_high_values() {
        let material = RadialCooldownMaterial::new(1.5);
        assert_eq!(material.progress.x, 1.0);
    }

    #[test]
    fn radial_cooldown_material_new_clamps_low_values() {
        let material = RadialCooldownMaterial::new(-0.5);
        assert_eq!(material.progress.x, 0.0);
    }

    #[test]
    fn set_progress_clamps_to_valid_range() {
        let mut material = RadialCooldownMaterial::default();

        material.set_progress(1.5);
        assert_eq!(material.progress.x, 1.0);

        material.set_progress(-0.5);
        assert_eq!(material.progress.x, 0.0);
    }

    #[test]
    fn set_progress_updates_only_x_component() {
        let mut material = RadialCooldownMaterial::default();
        material.set_progress(0.75);
        assert_eq!(material.progress, Vec4::new(0.75, 0.0, 0.0, 0.0));
    }

    #[test]
    fn set_progress_accepts_valid_values() {
        let mut material = RadialCooldownMaterial::default();

        material.set_progress(0.0);
        assert_eq!(material.progress.x, 0.0);

        material.set_progress(0.5);
        assert_eq!(material.progress.x, 0.5);

        material.set_progress(1.0);
        assert_eq!(material.progress.x, 1.0);
    }
}
