// Fireball Trail Comet Tail Shader
// Creates an elongated flame trail that follows the fireball in world space.
// Features:
// - Elongated flame shape trailing behind the fireball
// - Noise-animated flame edges for organic movement
// - Color gradient: bright orange at head -> red -> dark smoke at tail
// - Fade out over distance from fireball
// - Global space simulation (trail stays in world position)

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// ============================================================================
// Uniforms
// ============================================================================

struct FireballTrailMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Velocity direction of the fireball (xyz = normalized direction)
    velocity_dir: vec4<f32>,
    // Trail length multiplier (packed in x component)
    trail_length: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
}

@group(2) @binding(0)
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

// Hash function for 3D input
fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
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

// Trail gradient: bright orange at head -> red -> dark smoke at tail
fn trail_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.15 {
        // Bright orange-yellow at head (hottest)
        return mix(vec3<f32>(1.0, 0.9, 0.4), vec3<f32>(1.0, 0.6, 0.1), x / 0.15);
    } else if x < 0.35 {
        // Orange to orange-red
        return mix(vec3<f32>(1.0, 0.6, 0.1), vec3<f32>(1.0, 0.35, 0.0), (x - 0.15) / 0.2);
    } else if x < 0.55 {
        // Orange-red to deep red
        return mix(vec3<f32>(1.0, 0.35, 0.0), vec3<f32>(0.8, 0.15, 0.0), (x - 0.35) / 0.2);
    } else if x < 0.75 {
        // Deep red to dark crimson
        return mix(vec3<f32>(0.8, 0.15, 0.0), vec3<f32>(0.4, 0.05, 0.0), (x - 0.55) / 0.2);
    } else {
        // Dark crimson to smoke (dark gray with ember tint)
        return mix(vec3<f32>(0.4, 0.05, 0.0), vec3<f32>(0.15, 0.1, 0.08), (x - 0.75) / 0.25);
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
    let velocity_dir = normalize(material.velocity_dir.xyz);
    let trail_len = material.trail_length.x;
    let emissive = material.emissive_intensity.x;

    // Use local position for trail calculations
    let pos = in.local_position;

    // Project position onto velocity direction to get distance along trail
    // Positive = ahead of center, Negative = behind (in the trail)
    let along_trail = dot(pos, velocity_dir);

    // Distance perpendicular to the trail direction (radial distance)
    let trail_axis_point = velocity_dir * along_trail;
    let perp_dist = length(pos - trail_axis_point);

    // Normalize along-trail distance: 0 = at fireball, 1 = end of trail
    // Trail extends behind the fireball (negative along_trail values)
    let trail_pos = clamp(-along_trail / trail_len, 0.0, 1.0);

    // Trail width tapers toward the end (comet shape)
    // Wider at head (trail_pos = 0), narrower at tail (trail_pos = 1)
    let base_width = 0.4; // Width at the head
    let taper = 1.0 - trail_pos * 0.7; // Taper to 30% at tail
    let trail_width = base_width * taper;

    // Flame edge distortion using noise
    let noise_scale = 4.0;
    let noise_speed = 2.0;
    let noise_pos = vec2<f32>(trail_pos * 8.0, time * noise_speed);
    let edge_noise = fbm3(noise_pos) * 0.15;
    let turbulent_width = trail_width + edge_noise;

    // Calculate if we're inside the trail shape
    let in_trail = smoothstep(turbulent_width, turbulent_width * 0.6, perp_dist);

    // Only show trail behind the fireball (not in front)
    let behind_fireball = smoothstep(0.1, -0.05, along_trail);

    // Combine trail shape
    let trail_mask = in_trail * behind_fireball;

    // Add animated internal fire detail
    let internal_noise_pos = vec2<f32>(trail_pos * 6.0 + time * 1.5, perp_dist * 4.0 + time * 0.5);
    let internal_noise = turbulence3(internal_noise_pos);
    let fire_detail = 0.7 + internal_noise * 0.5;

    // Intensity decreases along the trail (cooler toward tail)
    let base_intensity = 1.0 - trail_pos * 0.6;
    let intensity = base_intensity * fire_detail * trail_mask;

    // Get color from gradient based on position along trail
    let color = trail_gradient(trail_pos);

    // Apply emissive for HDR bloom (stronger at head)
    let emission_falloff = 1.0 - trail_pos * 0.5;
    let final_color = color * emissive * intensity * emission_falloff;

    // Alpha: strong at head, fading toward tail
    let alpha = trail_mask * (1.0 - trail_pos * 0.7);

    // Discard nearly transparent fragments for performance
    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
