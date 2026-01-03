use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Shader asset path for the fireball core volumetric fire effect.
pub const FIREBALL_CORE_SHADER: &str = "shaders/fireball_core.wgsl";

/// Shader asset path for the fireball charge swirling energy effect.
pub const FIREBALL_CHARGE_SHADER: &str = "shaders/fireball_charge.wgsl";

/// Material for rendering the fireball core with volumetric fire effect.
///
/// This shader creates an animated fire sphere with:
/// - Noise-based flame animation
/// - Color gradient from bright yellow core to orange edge
/// - Animated turbulence that scrolls with time
/// - Emissive output for bloom compatibility
/// - Sphere UV mapping that works with existing mesh
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FireballCoreMaterial {
    /// Current time for animation (in seconds).
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(0)]
    pub time: Vec4,

    /// Animation speed multiplier.
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(1)]
    pub animation_speed: Vec4,

    /// Noise scale for turbulence detail.
    /// Higher values = more detailed/smaller flames.
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(2)]
    pub noise_scale: Vec4,

    /// Emissive intensity for HDR bloom effect.
    /// Values > 1.0 will bloom with HDR rendering.
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(3)]
    pub emissive_intensity: Vec4,
}

impl Default for FireballCoreMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            animation_speed: Vec4::new(1.0, 0.0, 0.0, 0.0),
            noise_scale: Vec4::new(4.0, 0.0, 0.0, 0.0),
            emissive_intensity: Vec4::new(3.0, 0.0, 0.0, 0.0),
        }
    }
}

impl FireballCoreMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the animation speed multiplier.
    pub fn set_animation_speed(&mut self, speed: f32) {
        self.animation_speed.x = speed.max(0.0);
    }

    /// Set the noise scale for turbulence detail.
    pub fn set_noise_scale(&mut self, scale: f32) {
        self.noise_scale.x = scale.max(0.1);
    }

    /// Set the emissive intensity for HDR bloom.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }
}

impl Material for FireballCoreMaterial {
    fn fragment_shader() -> ShaderRef {
        FIREBALL_CORE_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        FIREBALL_CORE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Resource to store the fireball core material handle for reuse.
#[derive(Resource)]
pub struct FireballCoreMaterialHandle(pub Handle<FireballCoreMaterial>);

/// System to update the time uniform on all fireball core materials.
pub fn update_fireball_core_material_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<FireballCoreMaterial>>,
) {
    let current_time = time.elapsed_secs();
    for (_, material) in materials.iter_mut() {
        material.set_time(current_time);
    }
}

// ============================================================================
// Fireball Charge Material - Swirling energy gathering effect
// ============================================================================

/// Material for rendering the fireball charge effect with swirling energy.
///
/// This shader creates an animated swirling energy effect that:
/// - Shows energy gathering inward from an outer ring
/// - Uses spiral motion with noise distortion
/// - Intensifies color as charge completes
/// - Supports additive blending for glow effect
/// - Scale controlled by charge_progress (0.0-1.0)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FireballChargeMaterial {
    /// Current time for animation (in seconds).
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(0)]
    pub time: Vec4,

    /// Charge progress from 0.0 (start) to 1.0 (complete).
    /// Controls inward energy gathering and color intensification.
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(1)]
    pub charge_progress: Vec4,

    /// Outer radius of the swirl effect.
    /// Energy gathers from this radius toward the center.
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(2)]
    pub outer_radius: Vec4,

    /// Emissive intensity for HDR bloom effect.
    /// Values > 1.0 will bloom with HDR rendering.
    /// Packed as x component of Vec4 for 16-byte alignment.
    #[uniform(3)]
    pub emissive_intensity: Vec4,
}

impl Default for FireballChargeMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            charge_progress: Vec4::ZERO,
            outer_radius: Vec4::new(1.0, 0.0, 0.0, 0.0),
            emissive_intensity: Vec4::new(3.0, 0.0, 0.0, 0.0),
        }
    }
}

impl FireballChargeMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the charge progress (0.0 to 1.0).
    /// Controls how far the energy has gathered toward center.
    pub fn set_charge_progress(&mut self, progress: f32) {
        self.charge_progress.x = progress.clamp(0.0, 1.0);
    }

    /// Set the outer radius of the swirl effect.
    pub fn set_outer_radius(&mut self, radius: f32) {
        self.outer_radius.x = radius.max(0.1);
    }

    /// Set the emissive intensity for HDR bloom.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }
}

impl Material for FireballChargeMaterial {
    fn fragment_shader() -> ShaderRef {
        FIREBALL_CHARGE_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        FIREBALL_CHARGE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Resource to store the fireball charge material handle for reuse.
#[derive(Resource)]
pub struct FireballChargeMaterialHandle(pub Handle<FireballChargeMaterial>);

/// System to update time and charge progress on all fireball charge materials.
/// Note: Charge progress must be updated per-entity by the fireball charge system.
pub fn update_fireball_charge_material_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<FireballChargeMaterial>>,
) {
    let current_time = time.elapsed_secs();
    for (_, material) in materials.iter_mut() {
        material.set_time(current_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fireball_core_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = FireballCoreMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.animation_speed.x, 1.0);
            assert_eq!(material.noise_scale.x, 4.0);
            assert_eq!(material.emissive_intensity.x, 3.0);
        }

        #[test]
        fn new_matches_default() {
            let material = FireballCoreMaterial::new();
            let default = FireballCoreMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.animation_speed, default.animation_speed);
            assert_eq!(material.noise_scale, default.noise_scale);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = FireballCoreMaterial::new();
            material.set_time(5.5);
            assert_eq!(material.time.x, 5.5);
            // Other components should remain zero
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_animation_speed_clamps_negative() {
            let mut material = FireballCoreMaterial::new();
            material.set_animation_speed(-1.0);
            assert_eq!(material.animation_speed.x, 0.0);
        }

        #[test]
        fn set_animation_speed_accepts_positive() {
            let mut material = FireballCoreMaterial::new();
            material.set_animation_speed(2.5);
            assert_eq!(material.animation_speed.x, 2.5);
        }

        #[test]
        fn set_noise_scale_clamps_to_minimum() {
            let mut material = FireballCoreMaterial::new();
            material.set_noise_scale(0.05);
            assert_eq!(material.noise_scale.x, 0.1);
        }

        #[test]
        fn set_noise_scale_accepts_valid_values() {
            let mut material = FireballCoreMaterial::new();
            material.set_noise_scale(8.0);
            assert_eq!(material.noise_scale.x, 8.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = FireballCoreMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = FireballCoreMaterial::new();
            material.set_emissive_intensity(10.0);
            assert_eq!(material.emissive_intensity.x, 10.0);
        }

        #[test]
        fn alpha_mode_is_blend() {
            let material = FireballCoreMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Blend);
        }
    }

    mod fireball_charge_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = FireballChargeMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.charge_progress.x, 0.0);
            assert_eq!(material.outer_radius.x, 1.0);
            assert_eq!(material.emissive_intensity.x, 3.0);
        }

        #[test]
        fn new_matches_default() {
            let material = FireballChargeMaterial::new();
            let default = FireballChargeMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.charge_progress, default.charge_progress);
            assert_eq!(material.outer_radius, default.outer_radius);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = FireballChargeMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            // Other components should remain zero
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_charge_progress_clamps_to_zero() {
            let mut material = FireballChargeMaterial::new();
            material.set_charge_progress(-0.5);
            assert_eq!(material.charge_progress.x, 0.0);
        }

        #[test]
        fn set_charge_progress_clamps_to_one() {
            let mut material = FireballChargeMaterial::new();
            material.set_charge_progress(1.5);
            assert_eq!(material.charge_progress.x, 1.0);
        }

        #[test]
        fn set_charge_progress_accepts_valid_values() {
            let mut material = FireballChargeMaterial::new();
            material.set_charge_progress(0.5);
            assert_eq!(material.charge_progress.x, 0.5);
        }

        #[test]
        fn set_charge_progress_at_boundaries() {
            let mut material = FireballChargeMaterial::new();
            material.set_charge_progress(0.0);
            assert_eq!(material.charge_progress.x, 0.0);
            material.set_charge_progress(1.0);
            assert_eq!(material.charge_progress.x, 1.0);
        }

        #[test]
        fn set_outer_radius_clamps_to_minimum() {
            let mut material = FireballChargeMaterial::new();
            material.set_outer_radius(0.05);
            assert_eq!(material.outer_radius.x, 0.1);
        }

        #[test]
        fn set_outer_radius_accepts_valid_values() {
            let mut material = FireballChargeMaterial::new();
            material.set_outer_radius(2.0);
            assert_eq!(material.outer_radius.x, 2.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = FireballChargeMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = FireballChargeMaterial::new();
            material.set_emissive_intensity(10.0);
            assert_eq!(material.emissive_intensity.x, 10.0);
        }

        #[test]
        fn alpha_mode_is_add() {
            let material = FireballChargeMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Add);
        }
    }
}
