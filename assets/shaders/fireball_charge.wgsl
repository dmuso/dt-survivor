// Fireball Charge Effect Shader
// Creates a swirling energy gathering effect for the fireball charge phase.
// Energy spirals inward from an outer ring toward the center,
// with noise distortion, color intensification, and additive blending for glow.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// ============================================================================
// Uniforms
// ============================================================================

struct FireballChargeMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Charge progress 0.0-1.0 (packed in x component)
    charge_progress: vec4<f32>,
    // Outer radius of the swirl effect (packed in x component)
    outer_radius: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
}

@group(2) @binding(0)
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

// Hash function for 2D input
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
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
    let time = material.time.x;
    let charge = material.charge_progress.x;
    let outer_r = material.outer_radius.x;
    let emissive = material.emissive_intensity.x;

    // Use local position for sphere-based effects (XZ plane)
    let pos = in.local_position;
    let pos_2d = vec2<f32>(pos.x, pos.z);

    // Distance from center (0 at center, 1 at outer radius)
    let dist = length(pos_2d);
    let normalized_dist = dist / outer_r;

    // Angle in radians for spiral effect
    let angle = atan2(pos_2d.y, pos_2d.x);

    // Spiral rotation speed increases with charge
    let spiral_speed = 3.0 + charge * 5.0;
    let spiral_angle = angle + time * spiral_speed;

    // Number of spiral arms
    let arm_count = 3.0;
    let spiral_pattern = sin(spiral_angle * arm_count + normalized_dist * 10.0) * 0.5 + 0.5;

    // Inward motion: energy gathers toward center as charge progresses
    // At charge=0, energy is at outer edge; at charge=1, energy reaches center
    let inward_progress = 1.0 - charge;
    let energy_ring_center = inward_progress * 0.8 + 0.1; // Ring moves from 0.9 to 0.1
    let ring_width = 0.3 + charge * 0.4; // Ring gets wider as it moves in

    // Ring shape with soft falloff
    let ring_dist = abs(normalized_dist - energy_ring_center);
    let ring_intensity = 1.0 - smoothstep(0.0, ring_width, ring_dist);

    // Add noise distortion to the ring
    let noise_scale = 4.0;
    let noise_offset = vec2<f32>(time * 0.5, time * 0.3);
    let noise_val = fbm3(pos_2d * noise_scale + noise_offset);
    let distorted_ring = ring_intensity * (0.7 + noise_val * 0.5);

    // Combine spiral pattern with ring
    let combined = distorted_ring * (0.5 + spiral_pattern * 0.5);

    // Intensity increases with charge (energy builds up)
    let charge_intensity = 0.5 + charge * 0.5;
    let final_intensity = combined * charge_intensity;

    // Color based on charge progress and local intensity
    // Higher charge = hotter colors
    let color_t = charge * 0.6 + final_intensity * 0.4;
    let base_color = charge_gradient(color_t);

    // Apply emissive for HDR bloom
    let final_color = base_color * emissive * (0.5 + final_intensity * 1.5);

    // Alpha: visible where there's energy, with soft edges
    // Core becomes more solid as charge completes
    let core_alpha = smoothstep(0.5, 0.0, normalized_dist) * charge * 0.3;
    let ring_alpha = final_intensity * 0.8;
    let alpha = clamp(ring_alpha + core_alpha, 0.0, 1.0);

    // Fade out at outer boundary
    let edge_fade = 1.0 - smoothstep(0.8, 1.0, normalized_dist);

    return vec4<f32>(final_color, alpha * edge_fade);
}
