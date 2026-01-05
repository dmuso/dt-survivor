---
name: shader-materials
description: Custom shader materials guide for Bevy 0.17. Use when creating shaders, materials, WGSL code, visual effects, particles, or working with AsBindGroup, uniforms, HDR bloom, Hanabi. Triggers on "shader", "material", "WGSL", "bloom", "emissive", "visual effect", "particle", "Hanabi", "alpha mode".
---

# Shader Materials Skill

This skill provides guidance on creating custom shader materials for visual effects.

## Quick Reference

- Shaders: `assets/shaders/*.wgsl`
- Materials: Rust structs with `AsBindGroup`
- Bind group: Use `#{MATERIAL_BIND_GROUP}` (resolves to `@group(3)` in Bevy 0.17)
- Alignment: Use `Vec4` for all uniforms (16-byte alignment for WebGL2)

## Creating a Material (Rust)

```rust
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;

pub const MY_EFFECT_SHADER: &str = "shaders/my_effect.wgsl";

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct MyEffectMaterial {
    #[uniform(0)]
    pub time: Vec4,      // Store scalar in .x
    #[uniform(0)]
    pub progress: Vec4,  // Store scalar in .x
}

impl Material for MyEffectMaterial {
    fn fragment_shader() -> ShaderRef {
        MY_EFFECT_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend  // Or AlphaMode::Add for glow
    }
}
```

## WGSL Shader Template

```wgsl
#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

struct MyEffectMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: MyEffectMaterial;

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
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(vertex.position, 1.0)
    );
    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let progress = material.progress.x;
    let color = vec3<f32>(1.0, 0.5, 0.0);
    return vec4<f32>(color, 1.0);
}
```

## Register Material Plugin

```rust
app.add_plugins(MaterialPlugin::<MyEffectMaterial>::default());
```

## HDR Bloom Values

| Intensity | Value Range | Use Case |
|-----------|-------------|----------|
| Subtle | 0.8-1.0 | Smoke edges, ambient |
| Medium | 3.0-5.0 | Standard fire, sparks |
| Bright | 8.0-10.0 | Main explosion fire |
| Flash | 15.0+ | Initial explosion core |

## Alpha Mode Strategy

Choose the correct alpha mode based on the visual effect:

| Alpha Mode | Use Case | Examples |
|------------|----------|----------|
| `AlphaMode::Add` | Glow, additive blending | Fire core, charge, sparks |
| `AlphaMode::Blend` | Standard transparency | Smoke, color transitions |

**Warning**: Wrong mode = broken visuals. Add makes smoke glow; Blend makes fire muddy.

## Hanabi Particle Effects

This project also uses **bevy_hanabi** for discrete particles (trails, sparks, debris).

| Feature | Shader Materials | Hanabi Particles |
|---------|-----------------|------------------|
| Use case | Volumetric effects | Discrete particles |
| Examples | Fire sphere, smoke | Sparks, trails |

```rust
use bevy_hanabi::prelude::*;

// Create particle effect
let effect = EffectAsset::new(512, Spawner::rate(100.0.into()), writer.finish())
    .with_simulation_space(SimulationSpace::Local)  // Moves with entity
    // .with_simulation_space(SimulationSpace::Global)  // Stays in world (trails)
    .init(init_pos)
    .init(init_vel)
    .render(ColorOverLifetimeModifier { gradient });
```

See `spells/fire/fireball_effects.rs` for full examples.

## Existing Shaders

| Shader | Purpose |
|--------|---------|
| `fireball_core.wgsl` | Volumetric fire sphere |
| `fireball_charge.wgsl` | Swirling energy gathering |
| `fireball_trail.wgsl` | Comet tail effect |
| `explosion_fire.wgsl` | Main blast |
| `explosion_smoke.wgsl` | Rising smoke |

## Full Documentation

- [docs/shader-materials.md](../../../docs/shader-materials.md) - Complete guide
- [docs/bevy-017-material-bindings.md](../../../docs/bevy-017-material-bindings.md) - Bevy 0.17 specifics
