# Custom Shader Materials Guide

This project uses WGSL shaders for visual effects. Shaders live in `assets/shaders/` and are paired with Rust material definitions.

> **Reference**: See [bevy-017-material-bindings.md](bevy-017-material-bindings.md) for detailed Bevy 0.17 shader binding patterns.

## Shader Asset Structure

```
assets/shaders/
├── noise.wgsl              # Shared noise functions (perlin, fbm, turbulence)
├── fire_common.wgsl        # Shared fire color gradients (not yet used as import)
├── fireball_core.wgsl      # Volumetric fire sphere with displacement
├── fireball_charge.wgsl    # Swirling energy gathering effect
├── fireball_trail.wgsl     # Comet tail trailing effect
├── fireball_sparks.wgsl    # Flying ember particles
├── explosion_core.wgsl     # White-hot flash burst
├── explosion_fire.wgsl     # Main orange-red blast
├── explosion_embers.wgsl   # Flying debris particles
├── explosion_smoke.wgsl    # Rising smoke plume
└── radial_cooldown.wgsl    # UI cooldown overlay (UiMaterial)
```

## Creating a Custom 3D Material

### 1. Define the Material Struct

Materials use `AsBindGroup` to define GPU-accessible uniforms:

```rust
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Shader asset path constant (relative to assets/)
pub const MY_EFFECT_SHADER: &str = "shaders/my_effect.wgsl";

/// Material for rendering my custom effect.
///
/// IMPORTANT: All uniforms must use Vec4 for 16-byte alignment (WebGL2 requirement).
/// Store scalar values in the .x component.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct MyEffectMaterial {
    /// Current time for animation (seconds in .x component).
    #[uniform(0)]
    pub time: Vec4,

    /// Effect progress from 0.0 to 1.0 (stored in .x).
    #[uniform(0)]
    pub progress: Vec4,

    /// Emissive intensity for HDR bloom (stored in .x).
    #[uniform(0)]
    pub emissive_intensity: Vec4,
}

impl Default for MyEffectMaterial {
    fn default() -> Self {
        Self {
            time: Vec4::ZERO,
            progress: Vec4::ZERO,
            emissive_intensity: Vec4::new(3.0, 0.0, 0.0, 0.0),
        }
    }
}

impl MyEffectMaterial {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update time for animation.
    pub fn set_time(&mut self, time: f32) {
        self.time.x = time;
    }

    /// Set progress (clamped 0.0-1.0).
    pub fn set_progress(&mut self, progress: f32) {
        self.progress.x = progress.clamp(0.0, 1.0);
    }
}

impl Material for MyEffectMaterial {
    fn fragment_shader() -> ShaderRef {
        MY_EFFECT_SHADER.into()
    }

    fn vertex_shader() -> ShaderRef {
        MY_EFFECT_SHADER.into()
    }

    /// Use AlphaMode::Blend for transparency, AlphaMode::Add for glow effects.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
```

### 2. Register the Material Plugin

In your plugin.rs:

```rust
use bevy::pbr::MaterialPlugin;

pub fn plugin(app: &mut App) {
    app.add_plugins(MaterialPlugin::<MyEffectMaterial>::default());

    // Add time update system for animated materials
    app.add_systems(Update, update_my_effect_material_time);
}

/// System to update time on all instances of the material.
pub fn update_my_effect_material_time(
    time: Res<Time>,
    materials: Option<ResMut<Assets<MyEffectMaterial>>>,
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
```

### 3. Write the WGSL Shader

Create `assets/shaders/my_effect.wgsl`:

```wgsl
// My Effect Shader
// Description of what this shader does

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// Material uniform struct - must match Rust struct field order
struct MyEffectMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

// Use #{MATERIAL_BIND_GROUP} for forward compatibility (resolves to @group(3) in Bevy 0.17)
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: MyEffectMaterial;

// Vertex structures
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) local_position: vec3<f32>,
}

// Vertex shader
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = vertex.position;

    return out;
}

// Fragment shader
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;  // Use globals.time for animation
    let progress = material.progress.x;
    let emissive = material.emissive_intensity.x;

    // Your effect logic here
    let color = vec3<f32>(1.0, 0.5, 0.0);  // Orange

    return vec4<f32>(color * emissive, 1.0);
}
```

### 4. Spawn Entities with the Material

```rust
pub fn spawn_effect(
    commands: &mut Commands,
    meshes: &GameMeshes,
    materials: &mut Assets<MyEffectMaterial>,
    position: Vec3,
) {
    let material = MyEffectMaterial::new();
    let material_handle = materials.add(material);

    commands.spawn((
        Mesh3d(meshes.fireball.clone()),  // Use high-poly sphere for displacement
        MeshMaterial3d(material_handle.clone()),
        Transform::from_translation(position).with_scale(Vec3::splat(2.0)),
        MyEffectComponent { material_handle },  // Store handle for updates
    ));
}
```

## Creating UI Materials (UiMaterial)

For 2D UI effects like cooldown overlays:

```rust
use bevy::ui::UiMaterial;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct RadialCooldownMaterial {
    #[uniform(0)]
    pub progress: Vec4,  // 0.0 = on cooldown, 1.0 = ready
    #[uniform(0)]
    pub overlay_color: Vec4,
}

impl UiMaterial for RadialCooldownMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/radial_cooldown.wgsl".into()
    }
}
```

Register with `UiMaterialPlugin`:
```rust
app.add_plugins(UiMaterialPlugin::<RadialCooldownMaterial>::default());
```

## Effect Component Pattern

For effects with lifetimes, create a component that tracks the material handle and timer:

```rust
/// Component for shader-based effect with lifetime tracking.
#[derive(Component, Debug)]
pub struct MyExplosionEffect {
    /// Handle to update material progress
    pub material_handle: Handle<MyEffectMaterial>,
    /// Lifetime timer for progress and cleanup
    pub lifetime: Timer,
}

impl MyExplosionEffect {
    pub fn new(material_handle: Handle<MyEffectMaterial>) -> Self {
        Self {
            material_handle,
            lifetime: Timer::from_seconds(0.6, TimerMode::Once),
        }
    }

    pub fn progress(&self) -> f32 {
        self.lifetime.fraction()
    }

    pub fn is_finished(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// System to update effect progress and cleanup expired effects.
pub fn my_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut MyExplosionEffect)>,
    mut materials: Option<ResMut<Assets<MyEffectMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else { return };

    for (entity, mut effect) in query.iter_mut() {
        effect.lifetime.tick(time.delta());
        let progress = effect.progress();

        // Update material progress
        if let Some(material) = materials.get_mut(&effect.material_handle) {
            material.set_progress(progress);
        }

        // Cleanup when finished
        if effect.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
```

## Shared Mesh Resources

Meshes are created once and stored in `GameMeshes` resource:

```rust
/// Shared mesh handles to avoid recreating meshes per entity.
#[derive(Resource)]
pub struct GameMeshes {
    pub player: Handle<Mesh>,
    pub enemy: Handle<Mesh>,
    /// High-poly sphere for vertex displacement effects (ico subdivision 5)
    pub fireball: Handle<Mesh>,
    pub explosion: Handle<Mesh>,
    // ... other meshes
}

impl GameMeshes {
    pub fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            player: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            // Use high subdivision for smooth displacement
            fireball: meshes.add(Sphere::new(0.3).mesh().ico(5).unwrap()),
            explosion: meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap()),
            // ...
        }
    }
}
```

## HDR Bloom Camera Setup

For emissive shader effects to glow, configure the camera with HDR and bloom:

```rust
commands.spawn((
    Camera3d::default(),
    Camera {
        hdr: true,  // Required for bloom
        ..default()
    },
    Bloom::NATURAL,  // Or Bloom::default() or custom BloomSettings
    Transform::from_xyz(0.0, 20.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
));
```

**Emissive Values for Bloom:**
- Subtle glow: `LinearRgba::rgb(1.0, 0.5, 0.0)` (values ~1.0)
- Medium glow: `LinearRgba::rgb(3.0, 1.5, 0.0)` (values ~3.0)
- Bright flash: `LinearRgba::rgb(10.0, 10.0, 8.0)` (values ~10+)

## WGSL Shader Patterns

### Noise Functions

Include noise in shaders for organic effects:

```wgsl
// Hash function for pseudo-random values
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
}

// 3D Perlin-style noise
fn perlin3d(p: vec3<f32>) -> f32 {
    // Implementation with quintic interpolation
    // See assets/shaders/fireball_core.wgsl for full implementation
}

// Fractional Brownian Motion for layered detail
fn fbm4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0; i < 4; i = i + 1) {
        value += amplitude * perlin3d(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}
```

### Fire Color Gradient

```wgsl
fn fire_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    if x < 0.2 { return mix(vec3(0.0), vec3(0.5, 0.0, 0.0), x / 0.2); }
    else if x < 0.4 { return mix(vec3(0.5, 0.0, 0.0), vec3(1.0, 0.2, 0.0), (x - 0.2) / 0.2); }
    else if x < 0.6 { return mix(vec3(1.0, 0.2, 0.0), vec3(1.0, 0.5, 0.0), (x - 0.4) / 0.2); }
    else if x < 0.8 { return mix(vec3(1.0, 0.5, 0.0), vec3(1.0, 0.9, 0.2), (x - 0.6) / 0.2); }
    else { return mix(vec3(1.0, 0.9, 0.2), vec3(1.0, 1.0, 0.9), (x - 0.8) / 0.2); }
}
```

### Vertex Displacement

For flame effects, displace vertices based on noise:

```wgsl
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let t = globals.time;
    let pos = vertex.position;
    let normal = vertex.normal;

    // Sample noise for displacement
    let noise_pos = pos * 2.0 + vec3<f32>(0.0, -t * 3.0, 0.0);  // Scroll upward
    let displacement = perlin3d(noise_pos) * 0.3;

    // Displace along normal
    let displaced_pos = pos + normal * displacement;

    // Transform to world/clip space
    // ...
}
```

## Alpha Mode Strategy

Choose the correct alpha mode based on the visual effect:

| Alpha Mode | Use Case | Visual Result |
|------------|----------|---------------|
| `AlphaMode::Add` | Glow, energy, fire cores | Colors add together, brightens overlaps |
| `AlphaMode::Blend` | Smoke, transparency, color transitions | Standard alpha blending |

### When to Use AlphaMode::Add

Additive blending is best for:
- **Fire cores** - `FireballCoreMaterial` glows through other effects
- **Energy charges** - `FireballChargeMaterial` brightens as it builds
- **Sparks and flashes** - `FireballSparksMaterial`, `ExplosionCoreMaterial`
- **Anything that should "glow through"** other transparent objects

```rust
fn alpha_mode(&self) -> AlphaMode {
    AlphaMode::Add  // Bright, additive glow
}
```

### When to Use AlphaMode::Blend

Standard blending is best for:
- **Smoke and clouds** - `ExplosionSmokeMaterial` needs proper transparency
- **Color transitions** - `ExplosionFireMaterial` fades through colors
- **Dark/silhouette effects** - `ExplosionDarkImpactMaterial`
- **Anything with actual transparency** that shouldn't brighten

```rust
fn alpha_mode(&self) -> AlphaMode {
    AlphaMode::Blend  // Standard transparency
}
```

**Warning**: Using wrong alpha mode causes broken visuals - Add makes smoke look like glowing fog, Blend makes fire cores look muddy.

## Emissive Intensity Guidelines

For HDR bloom effects, use these intensity ranges:

| Intensity | Visual Effect | Use Case |
|-----------|---------------|----------|
| `0.8 - 1.0` | Subtle glow | Smoke edges, ambient |
| `3.0 - 5.0` | Moderate glow | Standard fire, sparks |
| `8.0 - 10.0` | Bright fire | Main explosion fire |
| `15.0+` | Flash/burst | Initial explosion core |

```rust
// In your material
pub emissive_intensity: Vec4,  // Store in .x component

// Typical values
material.emissive_intensity = Vec4::new(3.0, 0.0, 0.0, 0.0);  // Moderate
material.emissive_intensity = Vec4::new(15.0, 0.0, 0.0, 0.0); // Flash
```

## Hanabi Particle Effects

This project uses **bevy_hanabi** for discrete particle effects alongside shader materials. Particles are best for:
- Trailing effects (comet tails)
- Scattered debris (sparks, embers)
- Gathering/charging effects

### Hanabi vs Shader Materials

| Feature | Shader Materials | Hanabi Particles |
|---------|-----------------|------------------|
| Use case | Volumetric effects, surfaces | Discrete particles |
| Control | Per-vertex/fragment | Per-particle |
| Examples | Fire sphere, smoke cloud | Sparks, trails, charge motes |

### Creating a Particle Effect

```rust
use bevy_hanabi::prelude::*;

pub fn create_spark_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 0.8, 0.2, 1.0));  // Bright yellow
    color_gradient.add_key(1.0, Vec4::new(1.0, 0.2, 0.0, 0.0));  // Fade to transparent red

    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(0.1));
    size_gradient.add_key(1.0, Vec3::splat(0.02));

    let writer = ExprWriter::new();
    let age = writer.lit(0.).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);
    let lifetime = writer.lit(0.5).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.2).expr(),
        dimension: ShapeDimension::Surface,
    };

    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(5.0).expr(),
    };

    let effect = EffectAsset::new(512, Spawner::rate(100.0.into()), writer.finish())
        .with_simulation_space(SimulationSpace::Local)
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier { gradient: color_gradient })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        });

    effects.add(effect)
}
```

### Simulation Space

- `SimulationSpace::Local` - Particles move with entity (charge effects)
- `SimulationSpace::Global` - Particles stay in world space (trails)

```rust
// Trail that stays behind as entity moves
.with_simulation_space(SimulationSpace::Global)

// Charge that orbits around entity
.with_simulation_space(SimulationSpace::Local)
```

### Spawning Particle Effects

```rust
commands.spawn((
    Name::new("SparkEffect"),
    ParticleEffectBundle {
        effect: ParticleEffect::new(spark_effect_handle),
        transform: Transform::from_translation(position),
        ..default()
    },
));
```

Location: `spells/fire/fireball_effects.rs` contains full examples of charge, trail, and spark effects.

## Testing Shader Materials

Test material behavior without GPU:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_default_values() {
        let material = MyEffectMaterial::default();
        assert_eq!(material.time.x, 0.0);
        assert_eq!(material.progress.x, 0.0);
        assert_eq!(material.emissive_intensity.x, 3.0);
    }

    #[test]
    fn set_progress_clamps_values() {
        let mut material = MyEffectMaterial::new();
        material.set_progress(1.5);
        assert_eq!(material.progress.x, 1.0);
        material.set_progress(-0.5);
        assert_eq!(material.progress.x, 0.0);
    }

    #[test]
    fn alpha_mode_is_blend() {
        let material = MyEffectMaterial::new();
        assert_eq!(material.alpha_mode(), AlphaMode::Blend);
    }
}
```
