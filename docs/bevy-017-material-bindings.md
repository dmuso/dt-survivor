# Bevy 0.17 Material Bindings Reference

This document covers shader material bindings in Bevy 0.17, including the bind group layout changes and recommended patterns.

## Bind Group Layout

Bevy 0.17 uses the following bind group layout:

| Group | Purpose | Set By |
|-------|---------|--------|
| 0 | View uniforms (camera, lights, shadows, clusters) | `SetMeshViewBindGroup<0>` |
| 1 | Mesh binding arrays (lightmaps, skins in bindless mode) | `SetMeshViewBindingArrayBindGroup<1>` |
| 2 | Mesh uniforms (transform, skin index, material slot) | `SetMeshBindGroup<2>` |
| 3 | **Material uniforms and textures** | `SetMaterialBindGroup<3>` |

**Key change from Bevy 0.15/0.16**: Materials moved from `@group(2)` to `@group(3)`.

## Using MATERIAL_BIND_GROUP

For forward compatibility, use the `#{MATERIAL_BIND_GROUP}` shader define instead of hardcoding the group number:

```wgsl
// Recommended - uses shader define
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: MyMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_sampler: sampler;

// Not recommended - hardcoded (but works in 0.17)
@group(3) @binding(0) var<uniform> material: MyMaterial;
```

The `#{MATERIAL_BIND_GROUP}` placeholder is replaced with `3` during shader compilation.

## Standard Material Pattern

### Rust Definition

```rust
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
```

### WGSL Shader

```wgsl
#import bevy_pbr::forward_io::VertexOutput

struct CustomMaterial {
    color: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: CustomMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    return material.color * textureSample(color_texture, color_sampler, mesh.uv);
}
```

## Storage Buffer Pattern

For passing arrays of data to shaders:

### Rust Definition

```rust
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[storage(0, read_only)]
    colors: Handle<ShaderStorageBuffer>,
}
```

### WGSL Shader

```wgsl
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage, read> colors: array<vec4<f32>>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let index = u32(mesh.uv.x * 10.0);
    return colors[index];
}
```

## Bindless Materials (High-Performance)

For rendering many materials efficiently with GPU batching:

### Rust Definition

```rust
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[uniform(0, BindlessMaterialUniform, binding_array(10))]
#[bindless(limit(4))]
struct BindlessMaterial {
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

#[derive(ShaderType)]
struct BindlessMaterialUniform {
    color: LinearRgba,
}

impl<'a> From<&'a BindlessMaterial> for BindlessMaterialUniform {
    fn from(material: &'a BindlessMaterial) -> Self {
        BindlessMaterialUniform { color: material.color }
    }
}
```

### Checking for Bindless Support

```wgsl
#ifdef BINDLESS
    // Use bindless resources
    let color = bindless_materials[slot].color;
#else
    // Fall back to standard bindings
    let color = material.color;
#endif
```

## WebGL2 Alignment Requirements

WebGL2 requires uniforms to be 16-byte aligned. Use `Vec4` for scalar values:

```rust
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct MyEffectMaterial {
    /// Store scalar in .x component for 16-byte alignment
    #[uniform(0)]
    pub time: Vec4,

    #[uniform(0)]
    pub progress: Vec4,
}
```

## Migration from Bevy 0.15/0.16

If you see this error:
```
wgpu error: Validation Error... Shader global ResourceBinding { group: 2, binding: X }
is not available in the pipeline layout
```

**Fix**: Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` in your WGSL shaders.

## Sources

- [Bevy 0.17 Release Notes](https://bevy.org/news/bevy-0-17/)
- [Bevy 0.16 to 0.17 Migration Guide](https://bevy.org/learn/migration-guides/0-16-to-0-17/)
- [Bevy custom_material.wgsl](https://github.com/bevyengine/bevy/blob/main/assets/shaders/custom_material.wgsl)
- [Bevy shader_material.rs](https://github.com/bevyengine/bevy/blob/main/examples/shader/shader_material.rs)
- [Bevy storage_buffer.rs](https://github.com/bevyengine/bevy/blob/main/examples/shader/storage_buffer.rs)
- [Bevy shader_material_bindless.rs](https://github.com/bevyengine/bevy/blob/main/examples/shader/shader_material_bindless.rs)
- [PR #16368: Add bindless mode to AsBindGroup](https://github.com/bevyengine/bevy/pull/16368)
