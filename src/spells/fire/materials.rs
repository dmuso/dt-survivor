use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Shader asset path for the fireball core volumetric fire effect.
pub const FIREBALL_CORE_SHADER: &str = "shaders/fireball_core.wgsl";

/// Shader asset path for the fireball charge swirling energy effect.
pub const FIREBALL_CHARGE_SHADER: &str = "shaders/fireball_charge.wgsl";

/// Shader asset path for the fireball trail comet tail effect.
pub const FIREBALL_TRAIL_SHADER: &str = "shaders/fireball_trail.wgsl";

/// Shader asset path for the explosion core white-hot flash effect.
pub const EXPLOSION_CORE_SHADER: &str = "shaders/explosion_core.wgsl";

/// Shader asset path for the explosion fire main blast effect.
pub const EXPLOSION_FIRE_SHADER: &str = "shaders/explosion_fire.wgsl";

/// Shader asset path for the fireball sparks flying ember effect.
pub const FIREBALL_SPARKS_SHADER: &str = "shaders/fireball_sparks.wgsl";

/// Shader asset path for the explosion embers flying debris effect.
pub const EXPLOSION_EMBERS_SHADER: &str = "shaders/explosion_embers.wgsl";

/// Shader asset path for the explosion smoke rising plume effect.
pub const EXPLOSION_SMOKE_SHADER: &str = "shaders/explosion_smoke.wgsl";

/// Material for rendering the fireball core with volumetric fire effect.
///
/// This shader creates an animated fire sphere with:
/// - Noise-based flame animation
/// - Color gradient from bright yellow core to orange edge
/// - Animated turbulence that scrolls with time
/// - Emissive output for bloom compatibility
/// - Sphere UV mapping that works with existing mesh
/// - Velocity-based flame trailing (flames trail opposite to travel direction)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FireballCoreMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Animation speed multiplier.
    #[uniform(0)]
    pub animation_speed: Vec4,

    /// Noise scale for turbulence detail.
    #[uniform(0)]
    pub noise_scale: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,

    /// Velocity direction of the fireball (normalized xyz, w unused).
    /// Flames trail in the opposite direction.
    #[uniform(0)]
    pub velocity_dir: Vec4,
}

impl Default for FireballCoreMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            animation_speed: Vec4::new(1.0, 0.0, 0.0, 0.0),
            noise_scale: Vec4::new(4.0, 0.0, 0.0, 0.0),
            // VERY bright core - shader multiplies this further
            emissive_intensity: Vec4::new(10.0, 0.0, 0.0, 0.0),
            // Default: flames trail upward (as if moving down)
            velocity_dir: Vec4::new(0.0, -1.0, 0.0, 0.0),
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

    /// Set the velocity direction (will be normalized).
    /// Flames trail in the opposite direction of travel.
    pub fn set_velocity_direction(&mut self, direction: Vec3) {
        let normalized = direction.normalize_or_zero();
        self.velocity_dir = Vec4::new(normalized.x, normalized.y, normalized.z, 0.0);
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
        // Use additive blending so core glows through the trail
        AlphaMode::Add
    }
}

/// Resource to store the fireball core material handle for reuse.
#[derive(Resource)]
pub struct FireballCoreMaterialHandle(pub Handle<FireballCoreMaterial>);

/// System to update the time uniform on all fireball core materials.
pub fn update_fireball_core_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<FireballCoreMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let current_time = time.elapsed_secs();
    // Collect IDs and current materials, then re-insert to force GPU upload
    let updates: Vec<_> = materials.ids().map(|id| {
        let mat = materials.get(id).cloned();
        (id, mat)
    }).collect();
    for (id, mat_opt) in updates {
        if let Some(mut mat) = mat_opt {
            mat.set_time(current_time);
            let _ = materials.insert(id, mat);
        }
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
    #[uniform(0)]
    pub time: Vec4,

    /// Charge progress from 0.0 (start) to 1.0 (complete).
    #[uniform(0)]
    pub charge_progress: Vec4,

    /// Outer radius of the swirl effect.
    #[uniform(0)]
    pub outer_radius: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
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
    materials: Option<ResMut<Assets<FireballChargeMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
    }
}

// ============================================================================
// Fireball Trail Material - Comet tail effect
// ============================================================================

/// Material for rendering the fireball trail as a comet tail effect.
///
/// This shader creates an elongated flame trail that:
/// - Shows an elongated flame shape trailing behind the fireball
/// - Uses noise-animated flame edges for organic movement
/// - Color gradient: bright orange at head -> red -> dark smoke at tail
/// - Fades out over distance from fireball
/// - Works in global space (trail stays in world position)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FireballTrailMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Velocity direction of the fireball (normalized).
    #[uniform(0)]
    pub velocity_dir: Vec4,

    /// Trail length multiplier (.x) and wave phase offset (.y).
    #[uniform(0)]
    pub trail_length: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,
}

impl Default for FireballTrailMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            // Default direction: pointing in -Z (common forward direction)
            velocity_dir: Vec4::new(0.0, 0.0, -1.0, 0.0),
            // .x = trail length, .y = wave phase offset
            trail_length: Vec4::new(0.75, 0.0, 0.0, 0.0),
            emissive_intensity: Vec4::new(2.5, 0.0, 0.0, 0.0),
        }
    }
}

impl FireballTrailMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the velocity direction (will be normalized).
    /// The trail extends opposite to this direction.
    pub fn set_velocity_direction(&mut self, direction: Vec3) {
        let normalized = direction.normalize_or_zero();
        self.velocity_dir = Vec4::new(normalized.x, normalized.y, normalized.z, 0.0);
    }

    /// Set the trail length multiplier.
    pub fn set_trail_length(&mut self, length: f32) {
        self.trail_length.x = length.max(0.1);
    }

    /// Set the wave phase offset for unique trail movement.
    pub fn set_wave_phase_offset(&mut self, offset: f32) {
        self.trail_length.y = offset;
    }

    /// Set the emissive intensity for HDR bloom.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }
}

impl Material for FireballTrailMaterial {
    fn fragment_shader() -> ShaderRef {
        FIREBALL_TRAIL_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        FIREBALL_TRAIL_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Use Add to avoid alpha blending artifacts
        AlphaMode::Add
    }
}

/// Resource to store the fireball trail material handle for reuse.
#[derive(Resource)]
pub struct FireballTrailMaterialHandle(pub Handle<FireballTrailMaterial>);

/// System to update time on all fireball trail materials.
pub fn update_fireball_trail_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<FireballTrailMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
    }
}

// ============================================================================
// Explosion Core Material - White-hot flash effect
// ============================================================================

/// Material for rendering the explosion core as a white-hot flash.
///
/// This shader creates a blinding flash effect at explosion center:
/// - Blinding bright white/yellow expanding sphere
/// - Rapid expansion then fade (0.25s total duration)
/// - Very high emissive for HDR bloom
/// - Sharp falloff at edges for clean sphere boundary
/// - Progress-based animation (0.0 = start, 1.0 = end)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ExplosionCoreMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Lifetime progress from 0.0 (start) to 1.0 (end).
    #[uniform(0)]
    pub progress: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,

    /// Expansion scale multiplier.
    #[uniform(0)]
    pub expansion_scale: Vec4,
}

impl Default for ExplosionCoreMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            progress: Vec4::ZERO,
            // Very high emissive for blinding flash
            emissive_intensity: Vec4::new(15.0, 0.0, 0.0, 0.0),
            expansion_scale: Vec4::new(1.0, 0.0, 0.0, 0.0),
        }
    }
}

impl ExplosionCoreMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for noise animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the lifetime progress (0.0 to 1.0).
    /// Controls the expansion and fade animation.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress.x = progress.clamp(0.0, 1.0);
    }

    /// Set the emissive intensity for HDR bloom.
    /// High values (10+) recommended for blinding flash effect.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }

    /// Set the expansion scale multiplier.
    pub fn set_expansion_scale(&mut self, scale: f32) {
        self.expansion_scale.x = scale.max(0.1);
    }
}

impl Material for ExplosionCoreMaterial {
    fn fragment_shader() -> ShaderRef {
        EXPLOSION_CORE_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        EXPLOSION_CORE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Resource to store the explosion core material handle for reuse.
#[derive(Resource)]
pub struct ExplosionCoreMaterialHandle(pub Handle<ExplosionCoreMaterial>);

/// System to update time on all explosion core materials.
pub fn update_explosion_core_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<ExplosionCoreMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
    }
}

// ============================================================================
// Explosion Fire Material - Main orange-red fireball blast
// ============================================================================

/// Material for rendering the explosion fire as the main blast effect.
///
/// This shader creates the "meat" of the explosion:
/// - Large expanding fireball with volumetric noise
/// - Color progression: yellow-orange -> red -> dark crimson -> fade
/// - Turbulent edges with animated noise
/// - Rising heat effect (upward bias)
/// - Duration ~0.6s
/// - Progress-based animation (0.0 = start, 1.0 = end)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ExplosionFireMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Lifetime progress from 0.0 (start) to 1.0 (end).
    #[uniform(0)]
    pub progress: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,

    /// Noise scale for turbulence detail.
    #[uniform(0)]
    pub noise_scale: Vec4,
}

impl Default for ExplosionFireMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            progress: Vec4::ZERO,
            // High emissive for bright fire effect
            emissive_intensity: Vec4::new(8.0, 0.0, 0.0, 0.0),
            // Medium noise scale for balanced detail
            noise_scale: Vec4::new(3.0, 0.0, 0.0, 0.0),
        }
    }
}

impl ExplosionFireMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for noise animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the lifetime progress (0.0 to 1.0).
    /// Controls the expansion and fade animation.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress.x = progress.clamp(0.0, 1.0);
    }

    /// Set the emissive intensity for HDR bloom.
    /// Values 5-10 recommended for bright fire effect.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }

    /// Set the noise scale for turbulence detail.
    pub fn set_noise_scale(&mut self, scale: f32) {
        self.noise_scale.x = scale.max(0.1);
    }
}

impl Material for ExplosionFireMaterial {
    fn fragment_shader() -> ShaderRef {
        EXPLOSION_FIRE_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        EXPLOSION_FIRE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Resource to store the explosion fire material handle for reuse.
#[derive(Resource)]
pub struct ExplosionFireMaterialHandle(pub Handle<ExplosionFireMaterial>);

/// System to update time on all explosion fire materials.
pub fn update_explosion_fire_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<ExplosionFireMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
    }
}

// ============================================================================
// Fireball Sparks Material - Flying ember particles effect
// ============================================================================

/// Material for rendering fireball sparks as flying ember particles.
///
/// This shader creates bright flying spark particles that:
/// - Have a bright yellow-white core with orange halo
/// - Show motion blur / streak effect based on velocity
/// - Animate with flicker for lifelike behavior
/// - Cool from white-hot to orange to red as they age
/// - Support HDR bloom for bright sparks
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FireballSparksMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Velocity of the spark (xyz = direction, w = speed magnitude).
    #[uniform(0)]
    pub velocity: Vec4,

    /// Lifetime progress from 0.0 (new spark) to 1.0 (dying).
    #[uniform(0)]
    pub lifetime_progress: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,
}

impl Default for FireballSparksMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            // Default velocity: moving outward from center
            velocity: Vec4::new(1.0, 0.5, 0.0, 3.0),
            lifetime_progress: Vec4::ZERO,
            // High emissive for bright sparks
            emissive_intensity: Vec4::new(5.0, 0.0, 0.0, 0.0),
        }
    }
}

impl FireballSparksMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the velocity for motion blur (direction and speed).
    pub fn set_velocity(&mut self, direction: Vec3, speed: f32) {
        let normalized = direction.normalize_or_zero();
        self.velocity = Vec4::new(normalized.x, normalized.y, normalized.z, speed.max(0.0));
    }

    /// Set the lifetime progress (0.0 to 1.0).
    /// Controls the cooling color transition.
    pub fn set_lifetime_progress(&mut self, progress: f32) {
        self.lifetime_progress.x = progress.clamp(0.0, 1.0);
    }

    /// Set the emissive intensity for HDR bloom.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }
}

impl Material for FireballSparksMaterial {
    fn fragment_shader() -> ShaderRef {
        FIREBALL_SPARKS_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        FIREBALL_SPARKS_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Resource to store the fireball sparks material handle for reuse.
#[derive(Resource)]
pub struct FireballSparksMaterialHandle(pub Handle<FireballSparksMaterial>);

/// System to update time on all fireball sparks materials.
pub fn update_fireball_sparks_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<FireballSparksMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
    }
}

// ============================================================================
// Explosion Embers Material - Flying debris particles effect
// ============================================================================

/// Material for rendering explosion embers as fast-moving flying debris.
///
/// This shader creates bright ember particles that fly outward with:
/// - Many small bright particles radiating outward at high speed
/// - Gravity arc trajectories (falling motion)
/// - Cooling color progression: yellow -> orange -> deep red
/// - Motion streaks for speed visualization
/// - Duration ~0.8s
/// - Progress-based animation (0.0 = start, 1.0 = end)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ExplosionEmbersMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Lifetime progress from 0.0 (start) to 1.0 (end).
    #[uniform(0)]
    pub progress: Vec4,

    /// Velocity of the ember (xyz = direction, w = speed magnitude).
    #[uniform(0)]
    pub velocity: Vec4,

    /// Emissive intensity for HDR bloom effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,
}

impl Default for ExplosionEmbersMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            progress: Vec4::ZERO,
            // Default velocity: fast outward with slight upward arc
            velocity: Vec4::new(1.0, 0.5, 0.0, 15.0),
            // High emissive for bright embers
            emissive_intensity: Vec4::new(6.0, 0.0, 0.0, 0.0),
        }
    }
}

impl ExplosionEmbersMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the lifetime progress (0.0 to 1.0).
    /// Controls the cooling color transition and fade out.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress.x = progress.clamp(0.0, 1.0);
    }

    /// Set the velocity for motion blur (direction and speed).
    pub fn set_velocity(&mut self, direction: Vec3, speed: f32) {
        let normalized = direction.normalize_or_zero();
        self.velocity = Vec4::new(normalized.x, normalized.y, normalized.z, speed.max(0.0));
    }

    /// Set the emissive intensity for HDR bloom.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }
}

impl Material for ExplosionEmbersMaterial {
    fn fragment_shader() -> ShaderRef {
        EXPLOSION_EMBERS_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        EXPLOSION_EMBERS_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Resource to store the explosion embers material handle for reuse.
#[derive(Resource)]
pub struct ExplosionEmbersMaterialHandle(pub Handle<ExplosionEmbersMaterial>);

/// System to update time on all explosion embers materials.
pub fn update_explosion_embers_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<ExplosionEmbersMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
    }
}

// ============================================================================
// Explosion Smoke Material - Rising smoke plume effect
// ============================================================================

/// Material for rendering explosion smoke as a rising dark plume.
///
/// This shader creates the aftermath smoke that rises from an explosion:
/// - Dark volumetric smoke billowing upward
/// - Expands as it rises (heat dissipation effect)
/// - Semi-transparent gray with ember glow underneath
/// - Duration ~1.2s (longest lasting effect)
/// - Turbulent noise for realistic smoke motion
/// - Progress-based animation (0.0 = start, 1.0 = end)
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ExplosionSmokeMaterial {
    /// Current time for animation (in seconds).
    #[uniform(0)]
    pub time: Vec4,

    /// Lifetime progress from 0.0 (start) to 1.0 (end).
    #[uniform(0)]
    pub progress: Vec4,

    /// Emissive intensity for subtle glow effect.
    #[uniform(0)]
    pub emissive_intensity: Vec4,

    /// Noise scale for turbulence detail.
    #[uniform(0)]
    pub noise_scale: Vec4,
}

impl Default for ExplosionSmokeMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            progress: Vec4::ZERO,
            // Lower emissive for smoke (it's not glowing like fire)
            emissive_intensity: Vec4::new(0.8, 0.0, 0.0, 0.0),
            // Medium noise scale for smoke turbulence
            noise_scale: Vec4::new(2.5, 0.0, 0.0, 0.0),
        }
    }
}

impl ExplosionSmokeMaterial {
    /// Create a new material with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the time uniform for noise animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set the lifetime progress (0.0 to 1.0).
    /// Controls the rise, expansion, and fade animation.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress.x = progress.clamp(0.0, 1.0);
    }

    /// Set the emissive intensity.
    /// Lower values (0.5-1.5) recommended for smoke.
    pub fn set_emissive_intensity(&mut self, intensity: f32) {
        self.emissive_intensity.x = intensity.max(0.0);
    }

    /// Set the noise scale for turbulence detail.
    pub fn set_noise_scale(&mut self, scale: f32) {
        self.noise_scale.x = scale.max(0.1);
    }
}

impl Material for ExplosionSmokeMaterial {
    fn fragment_shader() -> ShaderRef {
        EXPLOSION_SMOKE_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        EXPLOSION_SMOKE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Resource to store the explosion smoke material handle for reuse.
#[derive(Resource)]
pub struct ExplosionSmokeMaterialHandle(pub Handle<ExplosionSmokeMaterial>);

/// System to update time on all explosion smoke materials.
pub fn update_explosion_smoke_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<ExplosionSmokeMaterial>>>,
) {
    let Some(mut materials) = materials else { return };
    let current_time = time.elapsed_secs();
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(material) = materials.get_mut(id) {
            material.set_time(current_time);
        }
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
            assert_eq!(material.emissive_intensity.x, 10.0);
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, -1.0);
            assert_eq!(material.velocity_dir.z, 0.0);
        }

        #[test]
        fn new_matches_default() {
            let material = FireballCoreMaterial::new();
            let default = FireballCoreMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.animation_speed, default.animation_speed);
            assert_eq!(material.noise_scale, default.noise_scale);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
            assert_eq!(material.velocity_dir, default.velocity_dir);
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
        fn alpha_mode_is_add() {
            let material = FireballCoreMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Add);
        }

        #[test]
        fn set_velocity_direction_normalizes_input() {
            let mut material = FireballCoreMaterial::new();
            material.set_velocity_direction(Vec3::new(3.0, 0.0, 4.0));
            // Should be normalized to (0.6, 0.0, 0.8)
            assert!((material.velocity_dir.x - 0.6).abs() < 0.001);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert!((material.velocity_dir.z - 0.8).abs() < 0.001);
        }

        #[test]
        fn set_velocity_direction_handles_zero_vector() {
            let mut material = FireballCoreMaterial::new();
            material.set_velocity_direction(Vec3::ZERO);
            // normalize_or_zero returns zero for zero vector
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, 0.0);
        }

        #[test]
        fn set_velocity_direction_unit_vectors() {
            let mut material = FireballCoreMaterial::new();

            material.set_velocity_direction(Vec3::X);
            assert_eq!(material.velocity_dir.x, 1.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, 0.0);

            material.set_velocity_direction(Vec3::Y);
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 1.0);
            assert_eq!(material.velocity_dir.z, 0.0);

            material.set_velocity_direction(Vec3::Z);
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, 1.0);
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

    mod fireball_trail_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = FireballTrailMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, -1.0);
            assert_eq!(material.trail_length.x, 0.75);
            assert_eq!(material.emissive_intensity.x, 2.5);
        }

        #[test]
        fn new_matches_default() {
            let material = FireballTrailMaterial::new();
            let default = FireballTrailMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.velocity_dir, default.velocity_dir);
            assert_eq!(material.trail_length, default.trail_length);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = FireballTrailMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_velocity_direction_normalizes_input() {
            let mut material = FireballTrailMaterial::new();
            material.set_velocity_direction(Vec3::new(3.0, 0.0, 4.0));
            // Should be normalized to (0.6, 0.0, 0.8)
            assert!((material.velocity_dir.x - 0.6).abs() < 0.001);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert!((material.velocity_dir.z - 0.8).abs() < 0.001);
        }

        #[test]
        fn set_velocity_direction_handles_zero_vector() {
            let mut material = FireballTrailMaterial::new();
            material.set_velocity_direction(Vec3::ZERO);
            // normalize_or_zero returns zero for zero vector
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, 0.0);
        }

        #[test]
        fn set_velocity_direction_unit_vectors() {
            let mut material = FireballTrailMaterial::new();

            material.set_velocity_direction(Vec3::X);
            assert_eq!(material.velocity_dir.x, 1.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, 0.0);

            material.set_velocity_direction(Vec3::Y);
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 1.0);
            assert_eq!(material.velocity_dir.z, 0.0);

            material.set_velocity_direction(Vec3::Z);
            assert_eq!(material.velocity_dir.x, 0.0);
            assert_eq!(material.velocity_dir.y, 0.0);
            assert_eq!(material.velocity_dir.z, 1.0);
        }

        #[test]
        fn set_trail_length_clamps_to_minimum() {
            let mut material = FireballTrailMaterial::new();
            material.set_trail_length(0.05);
            assert_eq!(material.trail_length.x, 0.1);
        }

        #[test]
        fn set_trail_length_accepts_valid_values() {
            let mut material = FireballTrailMaterial::new();
            material.set_trail_length(3.0);
            assert_eq!(material.trail_length.x, 3.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = FireballTrailMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = FireballTrailMaterial::new();
            material.set_emissive_intensity(10.0);
            assert_eq!(material.emissive_intensity.x, 10.0);
        }

        #[test]
        fn alpha_mode_is_add() {
            let material = FireballTrailMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Add);
        }
    }

    mod explosion_core_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = ExplosionCoreMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.progress.x, 0.0);
            assert_eq!(material.emissive_intensity.x, 15.0);
            assert_eq!(material.expansion_scale.x, 1.0);
        }

        #[test]
        fn new_matches_default() {
            let material = ExplosionCoreMaterial::new();
            let default = ExplosionCoreMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.progress, default.progress);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
            assert_eq!(material.expansion_scale, default.expansion_scale);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_zero() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_progress(-0.5);
            assert_eq!(material.progress.x, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_one() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_progress(1.5);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_progress_accepts_valid_values() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_progress(0.5);
            assert_eq!(material.progress.x, 0.5);
        }

        #[test]
        fn set_progress_at_boundaries() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_progress(0.0);
            assert_eq!(material.progress.x, 0.0);
            material.set_progress(1.0);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_emissive_intensity(20.0);
            assert_eq!(material.emissive_intensity.x, 20.0);
        }

        #[test]
        fn set_expansion_scale_clamps_to_minimum() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_expansion_scale(0.05);
            assert_eq!(material.expansion_scale.x, 0.1);
        }

        #[test]
        fn set_expansion_scale_accepts_valid_values() {
            let mut material = ExplosionCoreMaterial::new();
            material.set_expansion_scale(2.5);
            assert_eq!(material.expansion_scale.x, 2.5);
        }

        #[test]
        fn alpha_mode_is_add() {
            let material = ExplosionCoreMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Add);
        }
    }

    mod explosion_fire_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = ExplosionFireMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.progress.x, 0.0);
            assert_eq!(material.emissive_intensity.x, 8.0);
            assert_eq!(material.noise_scale.x, 3.0);
        }

        #[test]
        fn new_matches_default() {
            let material = ExplosionFireMaterial::new();
            let default = ExplosionFireMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.progress, default.progress);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
            assert_eq!(material.noise_scale, default.noise_scale);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = ExplosionFireMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_zero() {
            let mut material = ExplosionFireMaterial::new();
            material.set_progress(-0.5);
            assert_eq!(material.progress.x, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_one() {
            let mut material = ExplosionFireMaterial::new();
            material.set_progress(1.5);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_progress_accepts_valid_values() {
            let mut material = ExplosionFireMaterial::new();
            material.set_progress(0.5);
            assert_eq!(material.progress.x, 0.5);
        }

        #[test]
        fn set_progress_at_boundaries() {
            let mut material = ExplosionFireMaterial::new();
            material.set_progress(0.0);
            assert_eq!(material.progress.x, 0.0);
            material.set_progress(1.0);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = ExplosionFireMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = ExplosionFireMaterial::new();
            material.set_emissive_intensity(15.0);
            assert_eq!(material.emissive_intensity.x, 15.0);
        }

        #[test]
        fn set_noise_scale_clamps_to_minimum() {
            let mut material = ExplosionFireMaterial::new();
            material.set_noise_scale(0.05);
            assert_eq!(material.noise_scale.x, 0.1);
        }

        #[test]
        fn set_noise_scale_accepts_valid_values() {
            let mut material = ExplosionFireMaterial::new();
            material.set_noise_scale(5.0);
            assert_eq!(material.noise_scale.x, 5.0);
        }

        #[test]
        fn alpha_mode_is_blend() {
            let material = ExplosionFireMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Blend);
        }
    }

    mod fireball_sparks_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = FireballSparksMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.velocity.x, 1.0);
            assert_eq!(material.velocity.y, 0.5);
            assert_eq!(material.velocity.z, 0.0);
            assert_eq!(material.velocity.w, 3.0);
            assert_eq!(material.lifetime_progress.x, 0.0);
            assert_eq!(material.emissive_intensity.x, 5.0);
        }

        #[test]
        fn new_matches_default() {
            let material = FireballSparksMaterial::new();
            let default = FireballSparksMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.velocity, default.velocity);
            assert_eq!(material.lifetime_progress, default.lifetime_progress);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = FireballSparksMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_velocity_normalizes_direction() {
            let mut material = FireballSparksMaterial::new();
            material.set_velocity(Vec3::new(3.0, 0.0, 4.0), 5.0);
            // Should be normalized to (0.6, 0.0, 0.8)
            assert!((material.velocity.x - 0.6).abs() < 0.001);
            assert_eq!(material.velocity.y, 0.0);
            assert!((material.velocity.z - 0.8).abs() < 0.001);
            assert_eq!(material.velocity.w, 5.0);
        }

        #[test]
        fn set_velocity_handles_zero_vector() {
            let mut material = FireballSparksMaterial::new();
            material.set_velocity(Vec3::ZERO, 2.0);
            // normalize_or_zero returns zero for zero vector
            assert_eq!(material.velocity.x, 0.0);
            assert_eq!(material.velocity.y, 0.0);
            assert_eq!(material.velocity.z, 0.0);
            assert_eq!(material.velocity.w, 2.0);
        }

        #[test]
        fn set_velocity_clamps_negative_speed() {
            let mut material = FireballSparksMaterial::new();
            material.set_velocity(Vec3::X, -5.0);
            assert_eq!(material.velocity.w, 0.0);
        }

        #[test]
        fn set_velocity_unit_vectors() {
            let mut material = FireballSparksMaterial::new();

            material.set_velocity(Vec3::X, 1.0);
            assert_eq!(material.velocity.x, 1.0);
            assert_eq!(material.velocity.y, 0.0);
            assert_eq!(material.velocity.z, 0.0);

            material.set_velocity(Vec3::Y, 2.0);
            assert_eq!(material.velocity.x, 0.0);
            assert_eq!(material.velocity.y, 1.0);
            assert_eq!(material.velocity.z, 0.0);

            material.set_velocity(Vec3::Z, 3.0);
            assert_eq!(material.velocity.x, 0.0);
            assert_eq!(material.velocity.y, 0.0);
            assert_eq!(material.velocity.z, 1.0);
        }

        #[test]
        fn set_lifetime_progress_clamps_to_zero() {
            let mut material = FireballSparksMaterial::new();
            material.set_lifetime_progress(-0.5);
            assert_eq!(material.lifetime_progress.x, 0.0);
        }

        #[test]
        fn set_lifetime_progress_clamps_to_one() {
            let mut material = FireballSparksMaterial::new();
            material.set_lifetime_progress(1.5);
            assert_eq!(material.lifetime_progress.x, 1.0);
        }

        #[test]
        fn set_lifetime_progress_accepts_valid_values() {
            let mut material = FireballSparksMaterial::new();
            material.set_lifetime_progress(0.5);
            assert_eq!(material.lifetime_progress.x, 0.5);
        }

        #[test]
        fn set_lifetime_progress_at_boundaries() {
            let mut material = FireballSparksMaterial::new();
            material.set_lifetime_progress(0.0);
            assert_eq!(material.lifetime_progress.x, 0.0);
            material.set_lifetime_progress(1.0);
            assert_eq!(material.lifetime_progress.x, 1.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = FireballSparksMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = FireballSparksMaterial::new();
            material.set_emissive_intensity(15.0);
            assert_eq!(material.emissive_intensity.x, 15.0);
        }

        #[test]
        fn alpha_mode_is_add() {
            let material = FireballSparksMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Add);
        }
    }

    mod explosion_embers_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = ExplosionEmbersMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.progress.x, 0.0);
            assert_eq!(material.velocity.x, 1.0);
            assert_eq!(material.velocity.y, 0.5);
            assert_eq!(material.velocity.z, 0.0);
            assert_eq!(material.velocity.w, 15.0);
            assert_eq!(material.emissive_intensity.x, 6.0);
        }

        #[test]
        fn new_matches_default() {
            let material = ExplosionEmbersMaterial::new();
            let default = ExplosionEmbersMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.progress, default.progress);
            assert_eq!(material.velocity, default.velocity);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_zero() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_progress(-0.5);
            assert_eq!(material.progress.x, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_one() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_progress(1.5);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_progress_accepts_valid_values() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_progress(0.5);
            assert_eq!(material.progress.x, 0.5);
        }

        #[test]
        fn set_progress_at_boundaries() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_progress(0.0);
            assert_eq!(material.progress.x, 0.0);
            material.set_progress(1.0);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_velocity_normalizes_direction() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_velocity(Vec3::new(3.0, 0.0, 4.0), 10.0);
            // Should be normalized to (0.6, 0.0, 0.8)
            assert!((material.velocity.x - 0.6).abs() < 0.001);
            assert_eq!(material.velocity.y, 0.0);
            assert!((material.velocity.z - 0.8).abs() < 0.001);
            assert_eq!(material.velocity.w, 10.0);
        }

        #[test]
        fn set_velocity_handles_zero_vector() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_velocity(Vec3::ZERO, 5.0);
            // normalize_or_zero returns zero for zero vector
            assert_eq!(material.velocity.x, 0.0);
            assert_eq!(material.velocity.y, 0.0);
            assert_eq!(material.velocity.z, 0.0);
            assert_eq!(material.velocity.w, 5.0);
        }

        #[test]
        fn set_velocity_clamps_negative_speed() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_velocity(Vec3::X, -5.0);
            assert_eq!(material.velocity.w, 0.0);
        }

        #[test]
        fn set_velocity_unit_vectors() {
            let mut material = ExplosionEmbersMaterial::new();

            material.set_velocity(Vec3::X, 1.0);
            assert_eq!(material.velocity.x, 1.0);
            assert_eq!(material.velocity.y, 0.0);
            assert_eq!(material.velocity.z, 0.0);

            material.set_velocity(Vec3::Y, 2.0);
            assert_eq!(material.velocity.x, 0.0);
            assert_eq!(material.velocity.y, 1.0);
            assert_eq!(material.velocity.z, 0.0);

            material.set_velocity(Vec3::Z, 3.0);
            assert_eq!(material.velocity.x, 0.0);
            assert_eq!(material.velocity.y, 0.0);
            assert_eq!(material.velocity.z, 1.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_high_values() {
            let mut material = ExplosionEmbersMaterial::new();
            material.set_emissive_intensity(15.0);
            assert_eq!(material.emissive_intensity.x, 15.0);
        }

        #[test]
        fn alpha_mode_is_add() {
            let material = ExplosionEmbersMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Add);
        }
    }

    mod explosion_smoke_material_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let material = ExplosionSmokeMaterial::default();
            assert_eq!(material.time.x, 0.0);
            assert_eq!(material.progress.x, 0.0);
            assert_eq!(material.emissive_intensity.x, 0.8);
            assert_eq!(material.noise_scale.x, 2.5);
        }

        #[test]
        fn new_matches_default() {
            let material = ExplosionSmokeMaterial::new();
            let default = ExplosionSmokeMaterial::default();
            assert_eq!(material.time, default.time);
            assert_eq!(material.progress, default.progress);
            assert_eq!(material.emissive_intensity, default.emissive_intensity);
            assert_eq!(material.noise_scale, default.noise_scale);
        }

        #[test]
        fn set_time_updates_x_component() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_time(3.5);
            assert_eq!(material.time.x, 3.5);
            assert_eq!(material.time.y, 0.0);
            assert_eq!(material.time.z, 0.0);
            assert_eq!(material.time.w, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_zero() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_progress(-0.5);
            assert_eq!(material.progress.x, 0.0);
        }

        #[test]
        fn set_progress_clamps_to_one() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_progress(1.5);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_progress_accepts_valid_values() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_progress(0.5);
            assert_eq!(material.progress.x, 0.5);
        }

        #[test]
        fn set_progress_at_boundaries() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_progress(0.0);
            assert_eq!(material.progress.x, 0.0);
            material.set_progress(1.0);
            assert_eq!(material.progress.x, 1.0);
        }

        #[test]
        fn set_emissive_intensity_clamps_negative() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_emissive_intensity(-5.0);
            assert_eq!(material.emissive_intensity.x, 0.0);
        }

        #[test]
        fn set_emissive_intensity_accepts_positive_values() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_emissive_intensity(2.0);
            assert_eq!(material.emissive_intensity.x, 2.0);
        }

        #[test]
        fn set_noise_scale_clamps_to_minimum() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_noise_scale(0.05);
            assert_eq!(material.noise_scale.x, 0.1);
        }

        #[test]
        fn set_noise_scale_accepts_valid_values() {
            let mut material = ExplosionSmokeMaterial::new();
            material.set_noise_scale(5.0);
            assert_eq!(material.noise_scale.x, 5.0);
        }

        #[test]
        fn alpha_mode_is_blend() {
            let material = ExplosionSmokeMaterial::new();
            assert_eq!(material.alpha_mode(), AlphaMode::Blend);
        }
    }
}
