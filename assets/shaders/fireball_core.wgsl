// Fireball Core Volumetric Fire Shader
// Creates an animated fire sphere with noise-based turbulence,
// color gradients from yellow core to orange edge, and HDR emissive output.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// ============================================================================
// Uniforms
// ============================================================================

struct FireballCoreMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Animation speed multiplier (packed in x component)
    animation_speed: vec4<f32>,
    // Noise scale for turbulence detail (packed in x component)
    noise_scale: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: FireballCoreMaterial;

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
// Noise Functions (inlined for shader compilation)
// ============================================================================

const PI: f32 = 3.14159265359;

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

// 3D gradient noise
fn gradient3d(p: vec3<f32>) -> vec3<f32> {
    let h = hash13(p) * 6.28318530718;
    let z = hash13(p + vec3<f32>(127.1, 311.7, 74.7)) * 2.0 - 1.0;
    let r = sqrt(1.0 - z * z);
    return vec3<f32>(r * cos(h), r * sin(h), z);
}

// 3D Perlin-style noise
fn perlin3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Quintic interpolation
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    // Eight corner gradients
    let g000 = gradient3d(i + vec3<f32>(0.0, 0.0, 0.0));
    let g100 = gradient3d(i + vec3<f32>(1.0, 0.0, 0.0));
    let g010 = gradient3d(i + vec3<f32>(0.0, 1.0, 0.0));
    let g110 = gradient3d(i + vec3<f32>(1.0, 1.0, 0.0));
    let g001 = gradient3d(i + vec3<f32>(0.0, 0.0, 1.0));
    let g101 = gradient3d(i + vec3<f32>(1.0, 0.0, 1.0));
    let g011 = gradient3d(i + vec3<f32>(0.0, 1.0, 1.0));
    let g111 = gradient3d(i + vec3<f32>(1.0, 1.0, 1.0));

    // Distance vectors
    let d000 = f - vec3<f32>(0.0, 0.0, 0.0);
    let d100 = f - vec3<f32>(1.0, 0.0, 0.0);
    let d010 = f - vec3<f32>(0.0, 1.0, 0.0);
    let d110 = f - vec3<f32>(1.0, 1.0, 0.0);
    let d001 = f - vec3<f32>(0.0, 0.0, 1.0);
    let d101 = f - vec3<f32>(1.0, 0.0, 1.0);
    let d011 = f - vec3<f32>(0.0, 1.0, 1.0);
    let d111 = f - vec3<f32>(1.0, 1.0, 1.0);

    // Dot products
    let v000 = dot(g000, d000);
    let v100 = dot(g100, d100);
    let v010 = dot(g010, d010);
    let v110 = dot(g110, d110);
    let v001 = dot(g001, d001);
    let v101 = dot(g101, d101);
    let v011 = dot(g011, d011);
    let v111 = dot(g111, d111);

    // Trilinear interpolation
    let x00 = mix(v000, v100, u.x);
    let x10 = mix(v010, v110, u.x);
    let x01 = mix(v001, v101, u.x);
    let x11 = mix(v011, v111, u.x);
    let y0 = mix(x00, x10, u.y);
    let y1 = mix(x01, x11, u.y);
    return mix(y0, y1, u.z);
}

// Fractional Brownian Motion with 4 octaves
fn fbm4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * perlin3d(pos * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// Turbulence (absolute value FBM)
fn turbulence4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * abs(perlin3d(pos * frequency));
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Fire Color Functions
// ============================================================================

// Classic fire gradient: black -> red -> orange -> yellow -> white
fn fire_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.2 {
        // Black to dark red
        return mix(vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.5, 0.0, 0.0), x / 0.2);
    } else if x < 0.4 {
        // Dark red to red
        return mix(vec3<f32>(0.5, 0.0, 0.0), vec3<f32>(1.0, 0.2, 0.0), (x - 0.2) / 0.2);
    } else if x < 0.6 {
        // Red to orange
        return mix(vec3<f32>(1.0, 0.2, 0.0), vec3<f32>(1.0, 0.5, 0.0), (x - 0.4) / 0.2);
    } else if x < 0.8 {
        // Orange to yellow
        return mix(vec3<f32>(1.0, 0.5, 0.0), vec3<f32>(1.0, 0.9, 0.2), (x - 0.6) / 0.2);
    } else {
        // Yellow to white (hot core)
        return mix(vec3<f32>(1.0, 0.9, 0.2), vec3<f32>(1.0, 1.0, 0.9), (x - 0.8) / 0.2);
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
    let anim_speed = material.animation_speed.x;
    let noise_scale = material.noise_scale.x;
    let emissive = material.emissive_intensity.x;

    // Use local position for sphere-based effects
    let pos = in.local_position;

    // Distance from center (0 at center, 1 at surface for unit sphere)
    let dist_from_center = length(pos);

    // Animated noise position (scrolls upward for rising flame effect)
    let anim_offset = vec3<f32>(0.0, -time * anim_speed * 0.5, 0.0);
    let noise_pos = pos * noise_scale + anim_offset;

    // Layer multiple noise frequencies for detail
    let noise1 = fbm4(noise_pos);
    let noise2 = turbulence4(noise_pos * 2.0 + vec3<f32>(time * anim_speed * 0.3, 0.0, 0.0));
    let combined_noise = noise1 * 0.6 + noise2 * 0.4;

    // Map noise to fire intensity (0-1)
    // Center is hottest (1.0), edges cooler with noise variation
    let base_intensity = 1.0 - pow(dist_from_center, 1.5);
    let noise_contribution = combined_noise * 0.5 + 0.5; // Remap to 0-1
    let fire_intensity = clamp(base_intensity + (noise_contribution - 0.5) * 0.4, 0.0, 1.0);

    // Apply fire color gradient
    let fire_color = fire_gradient(fire_intensity);

    // Edge falloff for soft sphere boundary
    let edge_falloff = smoothstep(1.0, 0.7, dist_from_center);

    // Flicker effect
    let flicker = sin(time * 15.0) * 0.03 + sin(time * 23.0) * 0.02 + sin(time * 37.0) * 0.01;
    let flicker_intensity = 1.0 + flicker;

    // Final color with emissive for HDR bloom
    let final_color = fire_color * emissive * flicker_intensity;

    // Alpha based on intensity and edge falloff
    let alpha = fire_intensity * edge_falloff;

    return vec4<f32>(final_color, alpha);
}
