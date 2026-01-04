// Fireball Trail Comet Tail Shader
// Creates an elongated flame trail that follows the fireball in world space.
// Features:
// - Elongated flame shape trailing behind the fireball
// - Noise-animated flame edges for organic movement
// - Color gradient: bright orange at head -> red -> dark smoke at tail
// - Fade out over distance from fireball
// - Proper velocity-based trail direction from transform matrix

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Uniform buffer (matches Rust #[uniform(0)])
// ============================================================================

struct FireballTrailMaterial {
    time: vec4<f32>,
    velocity_dir: vec4<f32>,
    trail_length: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballTrailMaterial;

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
    @location(4) trail_dir: vec3<f32>,
    @location(5) trail_progress: f32,
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

// Hash function for 3D input - improved version
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
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

// Turbulence (absolute value FBM)
fn turbulence3(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 3; i = i + 1) {
        value = value + amplitude * abs(perlin2d(pos * frequency));
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Fire Color Functions
// ============================================================================

// Trail gradient: vivid orange at head -> red -> dark smoke at tail
// Designed for HDR/bloom environments - avoid pure white to preserve color
fn trail_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.2 {
        // Bright orange at head (hottest part of trail, but cooler than core)
        return mix(vec3<f32>(1.0, 0.6, 0.1), vec3<f32>(1.0, 0.4, 0.0), x / 0.2);
    } else if x < 0.4 {
        // Orange to orange-red
        return mix(vec3<f32>(1.0, 0.4, 0.0), vec3<f32>(0.95, 0.25, 0.0), (x - 0.2) / 0.2);
    } else if x < 0.6 {
        // Orange-red to deep red
        return mix(vec3<f32>(0.95, 0.25, 0.0), vec3<f32>(0.7, 0.1, 0.0), (x - 0.4) / 0.2);
    } else if x < 0.8 {
        // Deep red to dark crimson
        return mix(vec3<f32>(0.7, 0.1, 0.0), vec3<f32>(0.35, 0.05, 0.02), (x - 0.6) / 0.2);
    } else {
        // Dark crimson to smoke (dark gray with ember tint)
        return mix(vec3<f32>(0.35, 0.05, 0.02), vec3<f32>(0.12, 0.08, 0.06), (x - 0.8) / 0.2);
    }
}

// ============================================================================
// Vertex Shader - With trail direction extraction and stretching
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    // Extract trail direction from transform matrix (Z column)
    // The fireball rotation aligns local -Z with travel direction,
    // so local +Z (matrix Z column) points in the trail direction
    let trail_dir = normalize(vec3<f32>(world_from_local[2][0], world_from_local[2][1], world_from_local[2][2]));

    let pos = vertex.position;
    let normal = vertex.normal;

    // Project vertex position onto trail direction to get "how far along trail"
    // local +Z is trail direction after rotation
    let local_trail_dir = vec3<f32>(0.0, 0.0, 1.0);
    let trail_dot = dot(pos, local_trail_dir);

    // trail_progress: 0 at head (fireball center), 1 at tail end
    // Sphere has radius 0.3, so vertices range from -0.3 to +0.3
    // We want the back half (+Z) to stretch into a trail
    let trail_progress = clamp((trail_dot + 0.15) / 0.45, 0.0, 1.0);

    // Stretch the mesh along the trail direction - use material trail_length
    let trail_length = material.trail_length.x * 2.5; // Longer trail for visibility
    let stretch = pow(trail_progress, 0.7) * trail_length; // Non-linear stretch for smoother shape

    // Move vertex along local +Z (trail direction)
    let stretched_pos = pos + local_trail_dir * stretch;

    // Taper the trail - narrower at the tail
    // Use squared falloff for more gradual taper
    let taper = 1.0 - trail_progress * trail_progress * 0.85;
    let tapered_pos = vec3<f32>(stretched_pos.x * taper, stretched_pos.y * taper, stretched_pos.z);

    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(tapered_pos, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = pos;
    out.trail_dir = trail_dir;
    out.trail_progress = trail_progress;

    return out;
}

// ============================================================================
// Fragment Shader - Uses trail_progress from vertex shader for correct direction
// ============================================================================

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let t = globals.time;
    let emissive = material.emissive_intensity.x;

    let pos = in.local_position;
    let trail_progress = in.trail_progress;

    // Multiple noise layers for animated fire
    // Scroll noise to create flickering flame effect
    let noise1 = perlin2d(pos.xz * 4.0 + vec2<f32>(t * 0.5, -t * 4.0));
    let noise2 = perlin2d(pos.xz * 7.0 + vec2<f32>(-t * 0.3, -t * 5.0));
    let noise3 = perlin2d(pos.xz * 2.5 + vec2<f32>(t * 0.2, -t * 3.0));
    let combined = noise1 * 0.5 + noise2 * 0.3 + noise3 * 0.2;

    // Color based on trail_progress
    // Add noise variation for organic look
    let gradient_t = clamp(trail_progress * 0.85 + combined * 0.2, 0.0, 1.0);

    // Use trail gradient for color based on position along trail
    let base_color = trail_gradient(gradient_t);

    // Intensity falls off along trail - brighter at head, dimmer at tail
    // Use smooth falloff for better visual
    let intensity = 1.0 - pow(trail_progress, 1.5) * 0.7;

    // Flicker effect
    let flicker = 1.0 + sin(t * 17.0) * 0.12 + sin(t * 29.0) * 0.08;

    // Edge fade based on radial distance from center axis
    // Account for taper: the actual radius decreases along the trail
    let taper = 1.0 - trail_progress * trail_progress * 0.85;
    let effective_radius = 0.35 * taper;
    let radial_dist = length(pos.xy);

    // Soft edge fade with noise for flame-like appearance
    let noise_edge = combined * 0.1;
    let edge = 1.0 - smoothstep(effective_radius * 0.3, effective_radius + noise_edge, radial_dist);

    // Combined glow strength
    let glow_strength = edge * intensity;

    // Discard very dim pixels
    if glow_strength < 0.03 {
        discard;
    }

    // Emissive output for HDR bloom
    // Higher emissive at head, lower at tail
    let trail_emissive = emissive * (1.0 - trail_progress * 0.5);
    let final_color = base_color * trail_emissive * glow_strength * flicker;

    return vec4<f32>(final_color, 1.0);
}
