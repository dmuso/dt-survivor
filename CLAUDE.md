# Donny Tango: Survivor

A game in the style of Vampire Survivors and Brotato, built with Rust and the Bevy ECS framework.

## Development Commands

Run make commands via nix-shell:

```bash
nix-shell --run "make <target>"
```

Available make targets:

- `make check` - Type checking
- `make lint` - Clippy linting
- `make test` - Run tests (quiet mode)
- `make build` - Debug build
- `make build-release` - Release build
- `make run` - Run the game
- `make clean` - Clean build artifacts

## Testing and Linting

- Use TDD when making changes to code. Write a failing test, and then implement the code and confirm the test passes.
- You should maintain 90% code coverage via automated tests
- Run linting and testing after every change
- Fix any errors or warnings that you get as feedback from linting and tests
- Write tests inline with code

## Visual Testing (for Claude)

When working on shaders or visual effects, you MUST verify your work visually using the screenshot tool.

### Workflow

1. **Create a test scene** for the effect you're working on in `src/visual_test/scenes.rs`
2. **Run the screenshot capture**: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. **Read the screenshot**: Use Read tool on `tmp/screenshots/<scene-name>.png`
4. **Inspect and iterate**: Fix issues, re-capture, repeat until it looks right

### Creating a Test Scene

Add to the `TestScene` enum in `src/visual_test/scenes.rs`:

```rust
pub enum TestScene {
    FireballTrailEast,
    MyNewEffect,  // Add your scene
}

impl TestScene {
    pub fn name(&self) -> &'static str {
        match self {
            // ...
            Self::MyNewEffect => "my-new-effect",
        }
    }

    fn fireball_config(&self) -> (Vec3, Vec3) {
        match self {
            // Return (spawn_position, direction) for your effect
            Self::MyNewEffect => (Vec3::new(0.0, 1.0, 0.0), Vec3::X),
            // ...
        }
    }
}

impl FromStr for TestScene {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // ...
            "my-new-effect" => Ok(TestScene::MyNewEffect),
            // ...
        }
    }
}
```

### Running Visual Tests

```bash
# List available test scenes
nix-shell --run "cargo run -- --screenshot list"

# Capture screenshot of a scene
nix-shell --run "cargo run -- --screenshot fireball-trail-east"

# Then read the screenshot file to inspect
# tmp/screenshots/fireball-trail-east.png
```

### Requirements

- Every shader change must have a corresponding test scene
- Always inspect the screenshot before considering shader work complete
- If something looks wrong, fix it - don't commit broken visuals

## File Structure and Code Organization

This project follows a domain-driven, modular architecture to support the complex features planned for the survivor game (player classes, enemy AI, inventory, weapons, levels, etc.).

### Core Architecture Principles

- **Domain-driven organization**: Group code by business domain (game logic, UI, etc.) rather than technical type
- **Plugin-based architecture**: Each major feature area exposes a plugin for easy composition and testing
- **Clear separation of concerns**: Components, systems, and resources are logically separated
- **Scalable structure**: Easy to add new features without disrupting existing code

### Current Module Structure

```
src/
├── lib.rs              # Library exports and plugin composition
├── main.rs             # Minimal app entry point using plugins
├── prelude.rs          # Common imports used across modules
├── states.rs           # Game state management (GameState enum)
│
├── game/               # Core game logic and resources
│   ├── mod.rs
│   ├── components.rs   # World components (Arena, etc.)
│   ├── events.rs       # Game events (CollisionEvent, GameOverEvent)
│   ├── sets.rs         # SystemSet definitions (GameSet enum)
│   ├── systems.rs      # Core game systems (spawning, physics)
│   ├── resources.rs    # GameMeshes, GameMaterials resources
│   └── plugin.rs       # Game plugin composition
│
├── spell/              # Spell system coordination
│   ├── mod.rs
│   ├── components.rs   # Spell base components
│   ├── resources.rs    # Spell resources
│   ├── systems.rs      # Spell casting system (iterates SpellList)
│   └── plugin.rs       # Registers all spell plugins
│
├── spells/             # Individual spell implementations by element
│   ├── mod.rs          # Re-exports all spell modules
│   ├── fire/           # Fire element spells
│   │   ├── mod.rs
│   │   ├── fireball.rs # Components, constants, fire/update systems
│   │   ├── materials.rs # FireballCoreMaterial, ExplosionFireMaterial, etc.
│   │   └── inferno.rs
│   ├── frost/          # Frost element spells
│   ├── lightning/      # Lightning element spells
│   ├── psychic/        # Psychic element spells
│   └── light/          # Light element spells
│
├── combat/             # Damage, health, and combat mechanics
│   ├── mod.rs
│   ├── components.rs   # Health, Damage, Hitbox, Invincibility, CheckDeath
│   ├── events.rs       # DamageEvent, DeathEvent
│   ├── systems.rs      # apply_damage, check_death, handle_enemy_death
│   └── plugin.rs       # Combat plugin composition
│
├── movement/           # Reusable movement components and systems
│   ├── mod.rs
│   ├── components.rs   # Speed, Velocity, Knockback, from_xz()
│   ├── systems.rs      # apply_velocity, player_movement, enemy_movement
│   └── plugin.rs       # Movement plugin composition
│
├── player/             # Player entity and controls
│   ├── mod.rs
│   ├── components.rs   # Player component
│   └── systems.rs      # Player systems
│
├── enemies/            # Enemy entities and AI
│   ├── mod.rs
│   ├── components.rs   # Enemy component, spawn patterns
│   └── systems.rs      # Enemy AI, spawning, movement toward player
│
├── enemy_death/        # Enemy death handling and effects
│   ├── mod.rs
│   ├── systems.rs      # Enemy death particles, sounds, loot drops
│   └── plugin.rs       # Enemy death plugin composition
│
├── inventory/          # Player inventory and spell management
│   ├── mod.rs
│   ├── components.rs   # Inventory components
│   ├── resources.rs    # SpellList (5 spell slots)
│   ├── systems.rs      # Inventory systems
│   └── plugin.rs       # Inventory plugin composition
│
├── loot/               # Loot spawning and pickup
│   ├── mod.rs
│   ├── components.rs   # Loot components (XP orbs, items)
│   ├── systems.rs      # Loot attraction, pickup, magnet range
│   └── plugin.rs       # Loot plugin composition
│
├── experience/         # Experience and leveling
│   ├── mod.rs
│   ├── components.rs   # Experience components
│   ├── resources.rs    # PlayerLevel, XP thresholds
│   ├── systems.rs      # Experience gain, level up
│   └── plugin.rs       # Experience plugin composition
│
├── powerup/            # Power-up selection on level up
│   ├── mod.rs
│   ├── components.rs   # PowerUp types
│   ├── systems.rs      # PowerUp UI, selection
│   └── plugin.rs       # PowerUp plugin composition
│
├── ui/                 # User interface systems
│   ├── mod.rs
│   ├── components.rs   # UI components (HUD, menus)
│   ├── materials.rs    # RadialCooldownMaterial (UiMaterial)
│   ├── systems.rs      # UI update systems
│   └── plugin.rs       # UI plugin composition
│
├── camera/             # Camera setup and control
│   ├── mod.rs
│   ├── systems.rs      # Camera spawn, follow player, HDR/Bloom config
│   └── plugin.rs       # Camera plugin composition
│
├── arena/              # Arena/level setup
│   ├── mod.rs
│   ├── components.rs   # Arena boundaries
│   ├── systems.rs      # Arena spawning
│   └── plugin.rs       # Arena plugin composition
│
├── pause/              # Pause menu
│   ├── mod.rs
│   ├── components.rs   # Pause state
│   ├── systems.rs      # Pause/resume toggle
│   └── plugin.rs       # Pause plugin composition
│
├── whisper/            # Whisper attunement system
│   ├── mod.rs
│   ├── components.rs   # Whisper components
│   ├── resources.rs    # WhisperAttunement, SpellOrigin
│   ├── systems.rs      # Whisper mechanics
│   └── plugin.rs       # Whisper plugin composition
│
├── element/            # Element types (Fire, Frost, etc.)
│   └── mod.rs          # Element enum definition
│
├── audio/              # Audio management
│   ├── mod.rs
│   ├── components.rs   # Audio components
│   ├── systems.rs      # Audio systems
│   └── plugin.rs       # Audio plugin composition
│
├── score/              # Score tracking
│   ├── mod.rs
│   ├── components.rs   # Score components
│   ├── resources.rs    # Score resource
│   └── systems.rs      # Score systems
│
└── visual_test/        # Visual testing utilities
    └── mod.rs          # Debug visualization helpers
```

### Module Organization Patterns

Each feature module should follow this pattern:

#### 1. Module Definition (`mod.rs`)
```rust
pub mod components;
pub mod systems;
pub mod resources;      // if needed
pub mod plugin;

// Re-export public API
pub use components::*;
pub use systems::*;
pub use resources::*;   // if needed
pub use plugin::*;
```

#### 2. Components (`components.rs`)
- Define ECS components for the domain
- Use descriptive names and derive necessary traits
- Group related components together

```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub health: f32,
    pub speed: f32,
}

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub damage: f32,
}
```

#### 3. Systems (`systems.rs`)
- Implement game logic systems
- Use clear, descriptive function names
- Group related systems and use system sets for ordering

```rust
use bevy::prelude::*;
use crate::game::components::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystems {
    Movement,
    Combat,
    AI,
}

pub fn player_movement_system(
    mut query: Query<(&mut Transform, &Player)>,
    time: Res<Time>,
) {
    // Movement logic
}

pub fn enemy_ai_system(
    mut query: Query<(&Transform, &mut Enemy)>,
    player_query: Query<&Transform, With<Player>>,
) {
    // AI logic
}
```

#### 4. Resources (`resources.rs`) - When Needed
- Define global game state
- Use for configuration and shared data

```rust
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    pub difficulty: Difficulty,
    pub sound_enabled: bool,
}

#[derive(Resource, Default)]
pub struct Score(pub u32);
```

#### 5. Plugin (`plugin.rs`)
- Compose systems into logical plugins
- Use run conditions and state management
- Register events and resources

```rust
use bevy::prelude::*;
use crate::states::*;
use crate::game::systems::*;

pub fn plugin(app: &mut App) {
    app
        .add_event::<PlayerDamaged>()
        .init_resource::<Score>()
        .add_systems(
            Update,
            (
                player_movement_system,
                enemy_ai_system,
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .in_set(GameSystems::Movement),
        )
        .add_systems(
            OnEnter(GameState::Playing),
            spawn_player,
        );
}
```

### Future Module Planning

As the game grows, plan for these additional modules:

- **levels/**: Level progression and world management
- **assets/**: Asset loading and management

### Spells Module Structure

The `src/spells/` directory organizes spells by element with shared materials:

```
src/spells/
├── mod.rs              # Re-exports all spell modules
├── fire/               # Fire element spells
│   ├── mod.rs
│   ├── fireball.rs     # Fireball projectile, charging, collision, explosion
│   ├── fireball_effects.rs  # Particle effect resources
│   ├── materials.rs    # All fire shader materials (core, charge, trail, explosion)
│   └── inferno.rs      # Fire nova spell
├── frost/              # Frost element spells
│   ├── mod.rs
│   ├── ice_shard.rs
│   ├── glacial_pulse.rs
│   └── materials.rs    # Frost shader materials
├── lightning/          # Lightning element spells
│   ├── mod.rs
│   ├── thunder_strike.rs
│   └── materials.rs
├── psychic/            # Psychic element spells
│   ├── mod.rs
│   ├── echo_thought.rs
│   └── mind_cage.rs
└── light/              # Light element spells
    ├── mod.rs
    └── radiant_beam.rs
```

**Spell Module Pattern:**

Each spell implementation follows this structure:
1. **Components** - Projectile markers, effect trackers, timers
2. **Constants** - Speed, damage ratios, durations, collision radii
3. **Fire function** - `fire_<spell>()` to spawn the spell entity
4. **Systems** - Movement, lifetime, collision detection, effect updates
5. **Tests** - Inline tests for all behavior

## Custom Shader Materials

This project uses WGSL shaders for visual effects. Shaders live in `assets/shaders/` and are paired with Rust material definitions.

> **Reference**: See [docs/bevy-017-material-bindings.md](docs/bevy-017-material-bindings.md) for detailed Bevy 0.17 shader binding patterns.

### Shader Asset Structure

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

### Creating a Custom 3D Material

#### 1. Define the Material Struct

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

#### 2. Register the Material Plugin

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

#### 3. Write the WGSL Shader

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

#### 4. Spawn Entities with the Material

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

### Creating UI Materials (UiMaterial)

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

### Effect Component Pattern

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

### Shared Mesh Resources

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

### HDR Bloom Camera Setup

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

### WGSL Shader Patterns

#### Noise Functions

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

#### Fire Color Gradient

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

#### Vertex Displacement

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

### Testing Shader Materials

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

## ECS Patterns

This section documents the established ECS patterns that must be followed when adding new features.

### SystemSet Ordering with GameSet

All gameplay systems must be assigned to a `GameSet` to ensure deterministic execution order. The sets are defined in `src/game/sets.rs` and chained in order:

```rust
use crate::game::sets::GameSet;

// GameSet ordering: Input -> Movement -> Combat -> Spawning -> Effects -> Cleanup

// In your plugin:
app.add_systems(
    Update,
    my_movement_system
        .in_set(GameSet::Movement)
        .run_if(in_state(GameState::InGame)),
);
```

**Set Responsibilities:**
- `GameSet::Input` - Keyboard, mouse, and controller input handling
- `GameSet::Movement` - Player, enemy, projectile, and camera movement
- `GameSet::Combat` - Damage calculation, collision detection, death checking
- `GameSet::Spawning` - Enemy spawning, projectile creation, loot drops
- `GameSet::Effects` - Visual effects, screen tints, audio triggers, regeneration
- `GameSet::Cleanup` - Entity despawning, timer expiration, garbage collection

### Event-Driven Architecture

Use events (Bevy Messages) for decoupled communication between systems. Events are centralized in module `events.rs` files.

```rust
// Define events in events.rs
#[derive(Message)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
}

// Register in plugin.rs
app.add_message::<DamageEvent>();

// Write events
fn deal_damage(mut messages: MessageWriter<DamageEvent>) {
    messages.write(DamageEvent { target: entity, amount: 25.0 });
}

// Read events
fn apply_damage(mut messages: MessageReader<DamageEvent>) {
    for event in messages.read() {
        // Handle damage
    }
}
```

**Centralized Event Registration:**
- `combat/plugin.rs`: DamageEvent, DeathEvent, EnemyDeathEvent
- `game/plugin.rs`: PlayerEnemyCollisionEvent, BulletEnemyCollisionEvent, GameOverEvent
- `loot/plugin.rs`: LootDropEvent, PickupEvent, ItemEffectEvent

**Do not** register the same event in multiple plugins.

### Composable Component Design

Use small, focused components that can be combined rather than monolithic entity-specific components.

**Health Component (combat module):**
```rust
// Use the shared Health component instead of embedding health in Player/Enemy
use crate::combat::Health;

// Spawn player with separate Health component
commands.spawn((
    Player { speed: 200.0 },
    Health::new(100.0),
    Transform::default(),
));
```

**Movement Components (movement module):**
```rust
use crate::movement::{Speed, Velocity, Knockback};

// Entities use composable movement components
commands.spawn((
    Enemy { enemy_type: EnemyType::Basic },
    Speed(150.0),
    Velocity::from_direction_and_speed(direction, 150.0),
    Transform::default(),
));

// Apply temporary knockback
commands.entity(entity).insert(Knockback::from_direction(hit_direction));
```

**Combat Components (combat module):**
```rust
use crate::combat::{Health, Damage, Hitbox, Invincibility, CheckDeath};

// Enemy with full combat components
commands.spawn((
    Enemy { enemy_type: EnemyType::Tank },
    Health::new(200.0),
    Damage(15.0),
    Hitbox(20.0),
    CheckDeath,  // Marker for death checking system
    Transform::default(),
));

// Grant temporary invincibility
commands.entity(player).insert(Invincibility::new(2.0));
```

### Type-Safe Enums Over Strings

Always use enums instead of strings for type identification to leverage compile-time checking.

```rust
// WRONG - string comparison is error-prone
pub struct WeaponIcon {
    pub weapon_type: String,  // "pistol", "laser", etc.
}

// CORRECT - use WeaponType enum
use crate::weapon::WeaponType;

pub struct WeaponIcon {
    pub weapon_type: WeaponType,
}

// Pattern matching is compile-time checked
match weapon_type {
    WeaponType::Pistol { bullet_count, spread_angle } => { /* ... */ }
    WeaponType::Laser => { /* ... */ }
    WeaponType::RocketLauncher => { /* ... */ }
}
```

### Generic Cleanup Components

Use a single generic cleanup component with an enum discriminator instead of creating separate cleanup components.

```rust
use crate::audio::components::{CleanupTimer, CleanupType};

// WRONG - creating separate timer types
#[derive(Component)]
pub struct AudioCleanupTimer(Timer);
#[derive(Component)]
pub struct LootCleanupTimer(Timer);

// CORRECT - single generic component with type enum
commands.spawn((
    AudioSource { /* ... */ },
    CleanupTimer::from_secs(2.0, CleanupType::Audio),
));

commands.spawn((
    LootPickupSound,
    CleanupTimer::from_secs(1.5, CleanupType::Loot),
));
```

### Prelude Re-exports

Commonly used types should be re-exported via `src/prelude.rs` for convenient access across the codebase.

```rust
// In prelude.rs
pub use crate::combat::{Damage, DamageEvent, DeathEvent, EntityType, Health, Hitbox, Invincibility};
pub use crate::movement::{Knockback, Speed, Velocity};
pub use crate::weapon::WeaponType;

// Usage in other modules
use crate::prelude::*;

fn my_system(query: Query<(&Health, &Speed, &Velocity)>) {
    // Types are available without explicit imports
}
```

### Marker Components for System Filtering

Use marker components to opt entities into specific system behaviors rather than checking entity types.

```rust
use crate::combat::CheckDeath;

// Entities must have CheckDeath marker to be processed by death system
fn check_death_system(
    query: Query<(Entity, &Health, &Transform, &CheckDeath)>,
    mut messages: MessageWriter<DeathEvent>,
) {
    for (entity, health, transform, _) in query.iter() {
        if health.is_dead() {
            messages.write(DeathEvent::new(entity, transform.translation, EntityType::Enemy));
        }
    }
}

// Only add CheckDeath to entities that should trigger death events
commands.spawn((Enemy::default(), Health::new(50.0), CheckDeath));
commands.spawn((Bullet::default(), Health::new(1.0))); // No CheckDeath - bullets don't fire death events
```

### Sub-Plugin Composition

Complex features should be broken into sub-plugins that the main game plugin composes.

```rust
// In game/plugin.rs
use crate::combat::plugin as combat_plugin;
use crate::movement::plugin as movement_plugin;
use crate::weapon::plugin as weapon_plugin;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        combat_plugin,
        movement_plugin,
        weapon_plugin,
        // ... other sub-plugins
    ))
    // Game-specific systems
    .add_systems(Update, game_specific_system.in_set(GameSet::Combat));
}
```

### Import Strategy

- Use `prelude.rs` for common Bevy imports and local types
- Import specific items rather than glob imports when possible
- Keep imports organized and minimal

### Testing Strategy

- Tests should be co-located with the code they test
- Test components, systems, and integration scenarios
- Maintain 90% code coverage across all modules
- Use descriptive test names that explain what they're testing

### Development Workflow

1. **Plan the feature**: Identify which module(s) it belongs in
2. **Create/update components**: Add necessary ECS components
3. **Implement systems**: Write the game logic
4. **Create/update plugins**: Wire systems together
5. **Add tests**: Ensure functionality works correctly
6. **Update documentation**: Keep AGENTS.md current
7. **Run full test suite**: Verify no regressions

## Planning and Tracking work tasks using Beads

### CLI + Hooks

Use the `bd` CLI with hooks for the best experience.

**How it works:**

1. **SessionStart hook** runs `bd prime` automatically when Claude Code starts
2. `bd prime` injects a compact workflow reference
3. You use `bd` CLI commands directly
4. Git hooks auto-sync the database with JSONL

### CLI Quick Reference

**Essential commands for AI agents:**

```bash
# Find work
bd ready --json                                    # Unblocked issues
bd stale --days 30 --json                          # Forgotten issues

# Create and manage issues
bd create "Issue title" --description="Detailed context about the issue" -t bug|feature|task -p 0-4 --json
bd create "Found bug" --description="What the bug is and how it was discovered" -p 1 --deps discovered-from:<parent-id> --json
bd update <id> --status in_progress --json
bd close <id> --reason "Done" --json

# Search and filter
bd list --status open --priority 1 --json
bd list --label-any urgent,critical --json
bd show <id> --json

# Sync (CRITICAL at end of session!)
bd sync  # Force immediate export/commit/push
```

### Workflow

1. **Check for ready work**: Run `bd ready` to see what's unblocked (or `bd stale` to find forgotten issues)
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work**: If you find bugs or TODOs, create issues:
   - Old way (two commands): `bd create "Found bug in auth" --description="Details about the bug" -t bug -p 1 --json` then `bd dep add <new-id> <current-id> --type discovered-from`
   - New way (one command): `bd create "Found bug in auth" --description="Login fails with 500 when password has special chars" -t bug -p 1 --deps discovered-from:<current-id> --json`
5. **Complete**: `bd close <id> --reason "Implemented"`
6. **Sync at end of session**: `bd sync` (see "Agent Session Workflow" below)

### IMPORTANT: Always Include Issue Descriptions

**Issues without descriptions lack context for future work.** When creating issues, always include a meaningful description with:

- **Why** the issue exists (problem statement or need)
- **What** needs to be done (scope and approach)
- **How** you discovered it (if applicable during work)

**Good examples:**

```bash
# Bug discovered during work
bd create "Fix auth bug in login handler" \
  --description="Login fails with 500 error when password contains special characters like quotes. Found while testing GH#123 feature. Stack trace shows unescaped SQL in auth/login.go:45." \
  -t bug -p 1 --deps discovered-from:bd-abc --json

# Feature request
bd create "Add password reset flow" \
  --description="Users need ability to reset forgotten passwords via email. Should follow OAuth best practices and include rate limiting to prevent abuse." \
  -t feature -p 2 --json

# Technical debt
bd create "Refactor auth package for testability" \
  --description="Current auth code has tight DB coupling making unit tests difficult. Need to extract interfaces and add dependency injection. Blocks writing tests for bd-xyz." \
  -t task -p 3 --json
```

**Bad examples (missing context):**

```bash
bd create "Fix auth bug" -t bug -p 1 --json  # What bug? Where? Why?
bd create "Add feature" -t feature --json     # What feature? Why needed?
bd create "Refactor code" -t task --json      # What code? Why refactor?
```

### Deletion Tracking

When issues are deleted (via `bd delete` or `bd cleanup`), they are recorded in `.beads/deletions.jsonl`. This manifest:

- **Propagates deletions across clones**: When you pull, deleted issues from other clones are removed from your local database
- **Provides audit trail**: See what was deleted, when, and by whom with `bd deleted`
- **Auto-prunes**: Old records are automatically cleaned up during `bd sync` (configurable retention)

**Commands:**

```bash
bd delete bd-42                # Delete issue (records to manifest)
bd cleanup -f                  # Delete closed issues (records all to manifest)
bd deleted                     # Show recent deletions (last 7 days)
bd deleted --since=30d         # Show deletions in last 30 days
bd deleted bd-xxx              # Show deletion details for specific issue
bd deleted --json              # Machine-readable output
```

**How it works:**

1. `bd delete` or `bd cleanup` appends deletion records to `deletions.jsonl`
2. The file is committed and pushed via `bd sync`
3. On other clones, `bd sync` imports the deletions and removes those issues from local DB
4. Git history fallback handles edge cases (pruned records, shallow clones)

### Issue Types

- `bug` - Something broken that needs fixing
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature composed of multiple issues (supports hierarchical children)
- `chore` - Maintenance work (dependencies, tooling)

**Hierarchical children:** Epics can have child issues with dotted IDs (e.g., `bd-a3f8e9.1`, `bd-a3f8e9.2`). Children are auto-numbered sequentially. Up to 3 levels of nesting supported. The parent hash ensures unique namespace - no coordination needed between agents working on different epics.

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (nice-to-have features, minor bugs)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Dependency Types

- `blocks` - Hard dependency (issue X blocks issue Y)
- `related` - Soft relationship (issues are connected)
- `parent-child` - Epic/subtask relationship
- `discovered-from` - Track issues discovered during work (automatically inherits parent's `source_repo`)

Only `blocks` dependencies affect the ready work queue.

**Note:** When creating an issue with a `discovered-from` dependency, the new issue automatically inherits the parent's `source_repo` field. This ensures discovered work stays in the same repository as the parent task.

### Planning Work with Dependencies

When breaking down large features into tasks, use **beads dependencies** to sequence work - NOT phases or numbered steps.

**⚠️ COGNITIVE TRAP: Temporal Language Inverts Dependencies**

Words like "Phase 1", "Step 1", "first", "before" trigger temporal reasoning that **flips dependency direction**. Your brain thinks:
- "Phase 1 comes before Phase 2" → "Phase 1 blocks Phase 2" → `bd dep add phase1 phase2`

But that's **backwards**! The correct mental model:
- "Phase 2 **depends on** Phase 1" → `bd dep add phase2 phase1`

**Solution: Use requirement language, not temporal language**

Instead of phases, name tasks by what they ARE, and think about what they NEED:

```bash
# ❌ WRONG - temporal thinking leads to inverted deps
bd create "Phase 1: Create buffer layout" ...
bd create "Phase 2: Add message rendering" ...
bd dep add phase1 phase2  # WRONG! Says phase1 depends on phase2

# ✅ RIGHT - requirement thinking
bd create "Create buffer layout" ...
bd create "Add message rendering" ...
bd dep add msg-rendering buffer-layout  # msg-rendering NEEDS buffer-layout
```

**Verification**: After adding deps, run `bd blocked` - tasks should be blocked by their prerequisites, not their dependents.

**Example breakdown** (for a multi-part feature):
```bash
# Create tasks named by what they do, not what order they're in
bd create "Implement conversation region" -t task -p 1
bd create "Add header-line status display" -t task -p 1
bd create "Render tool calls inline" -t task -p 2
bd create "Add streaming content support" -t task -p 2

# Set up dependencies: X depends on Y means "X needs Y first"
bd dep add header-line conversation-region    # header needs region
bd dep add tool-calls conversation-region     # tools need region
bd dep add streaming tool-calls               # streaming needs tools

# Verify with bd blocked - should show sensible blocking
bd blocked
```

### Duplicate Detection & Merging

AI agents should proactively detect and merge duplicate issues to keep the database clean:

**Automated duplicate detection:**

```bash
# Find all content duplicates in the database
bd duplicates

# Automatically merge all duplicates
bd duplicates --auto-merge

# Preview what would be merged
bd duplicates --dry-run

# During import
bd import -i issues.jsonl --dedupe-after
```

**Detection strategies:**

1. **Before creating new issues**: Search for similar existing issues

   ```bash
   bd list --json | grep -i "authentication"
   bd show bd-41 bd-42 --json  # Compare candidates
   ```

2. **Periodic duplicate scans**: Review issues by type or priority

   ```bash
   bd list --status open --priority 1 --json  # High-priority issues
   bd list --issue-type bug --json             # All bugs
   ```

3. **During work discovery**: Check for duplicates when filing discovered-from issues
   ```bash
   # Before: bd create "Fix auth bug" --description="Details..." --deps discovered-from:bd-100
   # First: bd list --json | grep -i "auth bug"
   # Then decide: create new or link to existing
   ```

**Merge workflow:**

```bash
# Step 1: Identify duplicates (bd-42 and bd-43 duplicate bd-41)
bd show bd-41 bd-42 bd-43 --json

# Step 2: Preview merge to verify
bd merge bd-42 bd-43 --into bd-41 --dry-run

# Step 3: Execute merge
bd merge bd-42 bd-43 --into bd-41 --json

# Step 4: Verify result
bd dep tree bd-41  # Check unified dependency tree
bd show bd-41 --json  # Verify merged content
```

**What gets merged:**

- ✅ All dependencies from source → target
- ✅ Text references updated across ALL issues (descriptions, notes, design, acceptance criteria)
- ✅ Source issues closed with "Merged into bd-X" reason
- ❌ Source issue content NOT copied (target keeps its original content)

**Important notes:**

- Merge preserves target issue completely; only dependencies/references migrate
- If source issues have valuable content, manually copy it to target BEFORE merging
- Cannot merge in daemon mode yet (bd-190); use `--no-daemon` flag
- Operation cannot be undone (but git history preserves the original)

**Best practices:**

- Merge early to prevent dependency fragmentation
- Choose the oldest or most complete issue as merge target
- Add labels like `duplicate` to source issues before merging (for tracking)
- File a discovered-from issue if you found duplicates during work:
  ```bash
  bd create "Found duplicates during bd-X" \
    --description="Issues bd-A, bd-B, and bd-C are duplicates and need merging" \
    -p 2 --deps discovered-from:bd-X --json
  ```
