// Fireball Charge Effect Shader
// Creates a swirling energy gathering effect for the fireball charge phase.
// Energy spirals inward from an outer ring toward the center,
// with noise distortion, color intensification, and additive blending for glow.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Bevy 0.17 bindless storage buffer
// ============================================================================

// Material Data - Uniform buffer (matches Rust #[uniform(0)])
struct FireballChargeMaterial {
    time: vec4<f32>,
    charge_progress: vec4<f32>,
    outer_radius: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballChargeMaterial;

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
// Noise Functions (inlined for shader compilation)
// ============================================================================

// Hash function for 2D input - improved version
fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(p.xyx * vec3<f32>(443.897, 441.423, 437.195));
    let p4 = dot(p3, p3.yzx + 19.19);
    return fract(sin(p4) * 43758.5453);
}

// 2D gradient for noise
fn gradient2d(p: vec2<f32>) -> vec2<f32> {
    let angle = hash12(p) * TAU;
    return vec2<f32>(cos(angle), sin(angle));
}

// 2D Perlin-style gradient noise
fn perlin2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Quintic interpolation
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    // Four corner gradients
    let g00 = gradient2d(i + vec2<f32>(0.0, 0.0));
    let g10 = gradient2d(i + vec2<f32>(1.0, 0.0));
    let g01 = gradient2d(i + vec2<f32>(0.0, 1.0));
    let g11 = gradient2d(i + vec2<f32>(1.0, 1.0));

    // Distance vectors
    let d00 = f - vec2<f32>(0.0, 0.0);
    let d10 = f - vec2<f32>(1.0, 0.0);
    let d01 = f - vec2<f32>(0.0, 1.0);
    let d11 = f - vec2<f32>(1.0, 1.0);

    // Dot products
    let v00 = dot(g00, d00);
    let v10 = dot(g10, d10);
    let v01 = dot(g01, d01);
    let v11 = dot(g11, d11);

    // Bilinear interpolation
    let x0 = mix(v00, v10, u.x);
    let x1 = mix(v01, v11, u.x);
    return mix(x0, x1, u.y);
}

// Fractional Brownian Motion with 3 octaves
fn fbm3(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 3; i = i + 1) {
        value = value + amplitude * perlin2d(pos * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Fire Color Functions
// ============================================================================

// Energy charge gradient: deep orange -> bright yellow -> white-hot
fn charge_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.3 {
        // Dark orange to orange
        return mix(vec3<f32>(0.8, 0.2, 0.0), vec3<f32>(1.0, 0.5, 0.0), x / 0.3);
    } else if x < 0.6 {
        // Orange to yellow-orange
        return mix(vec3<f32>(1.0, 0.5, 0.0), vec3<f32>(1.0, 0.8, 0.2), (x - 0.3) / 0.3);
    } else if x < 0.85 {
        // Yellow-orange to bright yellow
        return mix(vec3<f32>(1.0, 0.8, 0.2), vec3<f32>(1.0, 1.0, 0.5), (x - 0.6) / 0.25);
    } else {
        // Bright yellow to white-hot
        return mix(vec3<f32>(1.0, 1.0, 0.5), vec3<f32>(1.0, 1.0, 0.95), (x - 0.85) / 0.15);
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
    // Hardcoded values since material reading is broken in Bevy 0.17
    let charge = 0.5;  // Default charge level for testing
    let outer_r = 1.0;
    let emissive = 4.0;

    // Use local position for sphere-based effects
    let pos = in.local_position;

    // Multiple noise layers for dramatic swirling effect
    let noise1 = perlin2d(pos.xz * 3.0 + vec2<f32>(t * 2.0, -t * 1.5));
    let noise2 = perlin2d(pos.xz * 5.0 + vec2<f32>(-t * 1.0, t * 2.5));
    let noise3 = perlin2d(pos.xz * 2.0 + vec2<f32>(t * 0.5, t * 3.0));
    let combined = noise1 * 0.5 + noise2 * 0.3 + noise3 * 0.2;

    // Angle for spiral effect
    let angle = atan2(pos.z, pos.x);
    let dist = length(pos.xz);

    // Rotating spiral arms
    let spiral_speed = 5.0;
    let spiral = sin(angle * 4.0 + t * spiral_speed + dist * 8.0) * 0.5 + 0.5;

    // Combine noise and spiral
    let pattern = combined * 0.6 + spiral * 0.4 + 0.3;
    let gradient_t = clamp(pattern, 0.0, 1.0);

    let color = charge_gradient(gradient_t);

    // Flicker
    let flicker = 1.0 + sin(t * 20.0) * 0.15 + sin(t * 31.0) * 0.1;

    // Edge fade
    let edge = 1.0 - smoothstep(0.7, 1.0, dist);

    return vec4<f32>(color * emissive * flicker, edge);
}
