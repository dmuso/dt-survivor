// Explosion Embers Shader - Flying Debris Particles
// Creates fast-moving ember debris flying outward from an explosion.
// Features:
// - Many small bright particles radiating outward
// - Gravity arc trajectories (falling motion)
// - Cooling color: yellow -> orange -> deep red
// - Motion streaks for speed visualization
// - Duration ~0.8s

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Uniform buffer (matches Rust #[uniform(0)])
// ============================================================================

struct ExplosionEmbersMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    velocity: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: ExplosionEmbersMaterial;

// ============================================================================
// Vertex Shader Structures
// ============================================================================

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

// ============================================================================
// Constants
// ============================================================================

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

// ============================================================================
// Noise Functions
// ============================================================================

// Hash function for 2D input - improved version
fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(p.xyx * vec3<f32>(443.897, 441.423, 437.195));
    let p4 = dot(p3, p3.yzx + 19.19);
    return fract(sin(p4) * 43758.5453);
}

// Hash function for 3D input - improved version
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
}

// ============================================================================
// Color Functions
// ============================================================================

// Ember cooling gradient: bright yellow -> orange -> deep red -> dim ember
// Simulates the cooling of hot metal debris as it flies
fn ember_cooling_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.15 {
        // Bright yellow-white at start (freshly exploded, very hot)
        return mix(vec3<f32>(1.0, 1.0, 0.85), vec3<f32>(1.0, 0.95, 0.5), x / 0.15);
    } else if x < 0.35 {
        // Yellow to bright orange (cooling rapidly)
        return mix(vec3<f32>(1.0, 0.95, 0.5), vec3<f32>(1.0, 0.7, 0.2), (x - 0.15) / 0.2);
    } else if x < 0.55 {
        // Bright orange to deep orange
        return mix(vec3<f32>(1.0, 0.7, 0.2), vec3<f32>(1.0, 0.45, 0.0), (x - 0.35) / 0.2);
    } else if x < 0.75 {
        // Deep orange to red ember
        return mix(vec3<f32>(1.0, 0.45, 0.0), vec3<f32>(0.8, 0.25, 0.0), (x - 0.55) / 0.2);
    } else if x < 0.9 {
        // Red to dark crimson
        return mix(vec3<f32>(0.8, 0.25, 0.0), vec3<f32>(0.5, 0.1, 0.0), (x - 0.75) / 0.15);
    } else {
        // Dark crimson to nearly dead ember
        return mix(vec3<f32>(0.5, 0.1, 0.0), vec3<f32>(0.2, 0.03, 0.0), (x - 0.9) / 0.1);
    }
}

// ============================================================================
// Vertex Shader
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = vertex.position;

    return out;
}

// ============================================================================
// Fragment Shader
// ============================================================================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    // Use material progress (0.0 = just exploded, 1.0 = cooled and faded)
    let prog = material.progress.x;
    let emissive = material.emissive_intensity.x;

    // Ember lifecycle based on progress
    // Embers start bright and gradually cool/fade
    // At prog=0, full intensity; at prog=1, faded completely
    var intensity: f32;
    if prog < 0.5 {
        // Full intensity for first half
        intensity = 1.0 - prog * 0.2;  // Slight dimming
    } else {
        // Faster fade in second half
        intensity = 0.9 * (1.0 - pow((prog - 0.5) / 0.5, 1.5));
    }

    let pos = in.local_position;
    let dist = length(pos.xz);

    // Core shrinks as ember cools
    let core_size = 0.15 * (1.0 - prog * 0.3);
    let core = 1.0 - smoothstep(0.0, core_size, dist);
    let core_intensity = pow(core, 2.5);

    // Halo also diminishes
    let halo_size = 0.4 * (1.0 - prog * 0.2);
    let halo = 1.0 - smoothstep(core_size * 0.7, halo_size, dist);
    let halo_intensity = pow(halo, 1.8) * (1.0 - core_intensity * 0.5);

    // Flicker gets more erratic as ember cools
    let flicker_seed = hash12(pos.xz * 80.0);
    let flicker_speed = 30.0 + prog * 15.0;
    let flicker = 0.7 + 0.3 * sin(t * flicker_speed + flicker_seed * TAU);

    // Core color cools with progress
    let core_color = ember_cooling_gradient(prog * 0.5);  // Core cools slowly
    let halo_color = ember_cooling_gradient(prog);  // Halo cools faster

    let combined_color = core_color * core_intensity + halo_color * halo_intensity;
    let total_intensity = (core_intensity + halo_intensity * 0.5) * flicker * intensity;

    let alpha = (core_intensity + halo_intensity * 0.4) * intensity;
    if alpha < 0.01 { discard; }

    return vec4<f32>(combined_color * emissive * total_intensity, alpha);
}
