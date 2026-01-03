// Explosion Fire Shader - Main Orange-Red Fireball Blast
// Creates the "meat" of the explosion with volumetric fire effect.
// Features:
// - Large expanding fireball with volumetric noise
// - Color progression: yellow-orange -> red -> dark crimson -> fade
// - Turbulent edges with animated noise
// - Rising heat effect (upward bias)
// - Duration ~0.6s

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// ============================================================================
// Uniforms
// ============================================================================

struct ExplosionFireMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Lifetime progress 0.0 (start) to 1.0 (end), packed in x
    progress: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
    // Noise scale for turbulence detail (packed in x component)
    noise_scale: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: ExplosionFireMaterial;

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

// Hash function for 3D input
fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

// 3D gradient function
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

    // Quintic interpolation for smoother results
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

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * perlin3d(p * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// Turbulence (absolute value FBM for billowing effect)
fn turbulence4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * abs(perlin3d(p * frequency));
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Fire Color Functions
// ============================================================================

// Explosion fire gradient: yellow-orange -> red -> crimson -> fade
// This is the color progression for the main fire blast
fn explosion_fire_gradient(t: f32, progress: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    // Base fire colors
    var color: vec3<f32>;

    if x < 0.15 {
        // Core: bright yellow-white (hottest)
        color = mix(vec3<f32>(1.0, 1.0, 0.9), vec3<f32>(1.0, 0.9, 0.4), x / 0.15);
    } else if x < 0.35 {
        // Yellow to orange
        let blend = (x - 0.15) / 0.2;
        color = mix(vec3<f32>(1.0, 0.9, 0.4), vec3<f32>(1.0, 0.55, 0.1), blend);
    } else if x < 0.55 {
        // Orange to red
        let blend = (x - 0.35) / 0.2;
        color = mix(vec3<f32>(1.0, 0.55, 0.1), vec3<f32>(0.95, 0.25, 0.0), blend);
    } else if x < 0.75 {
        // Red to dark crimson
        let blend = (x - 0.55) / 0.2;
        color = mix(vec3<f32>(0.95, 0.25, 0.0), vec3<f32>(0.5, 0.08, 0.0), blend);
    } else {
        // Dark crimson to black (cooling)
        let blend = (x - 0.75) / 0.25;
        color = mix(vec3<f32>(0.5, 0.08, 0.0), vec3<f32>(0.1, 0.02, 0.0), blend);
    }

    // As progress increases, shift colors toward cooler tones
    // Early: bright yellows and oranges
    // Late: dark reds and blacks
    let cool_shift = progress * 0.4;
    let cooler_color = mix(color, color * vec3<f32>(0.7, 0.5, 0.8), cool_shift);

    return cooler_color;
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
    let progress = material.progress.x;
    let emissive = material.emissive_intensity.x;
    let noise_scale = material.noise_scale.x;

    // Progress phases for the 0.6s fire blast:
    // 0.0 - 0.2: Rapid expansion (peak fire)
    // 0.2 - 0.5: Sustained burn with turbulence
    // 0.5 - 1.0: Fade out and dissipate

    // Calculate intensity based on progress
    var intensity: f32;
    if progress < 0.2 {
        // Rapid rise to peak
        intensity = smoothstep(0.0, 0.2, progress);
    } else if progress < 0.5 {
        // Sustained at high intensity with slight variation
        let variance = sin(progress * 20.0) * 0.05;
        intensity = 0.95 + variance;
    } else {
        // Fade out (quadratic for natural dissipation)
        let fade_progress = (progress - 0.5) / 0.5;
        intensity = 1.0 - fade_progress * fade_progress;
    }

    // Use local position for sphere-based effects
    let pos = in.local_position;

    // Distance from center (0 at center, 1 at surface for unit sphere)
    let dist_from_center = length(pos);

    // Rising heat effect - bias noise upward
    let heat_rise = vec3<f32>(0.0, time * 1.5, 0.0);

    // Animated noise position with rising effect
    let noise_pos = pos * noise_scale + heat_rise + vec3<f32>(time * 0.3, 0.0, time * 0.2);

    // Layer multiple noise for volumetric fire look
    let noise1 = fbm4(noise_pos);
    let noise2 = turbulence4(noise_pos * 1.5 + vec3<f32>(0.0, time * 0.5, 0.0));
    let noise3 = fbm4(noise_pos * 0.5 - vec3<f32>(time * 0.2, 0.0, 0.0));

    // Combine noise layers for complex fire pattern
    let combined_noise = noise1 * 0.4 + noise2 * 0.4 + noise3 * 0.2;

    // Fire intensity based on position and noise
    // Center is hottest, edges have noise-driven turbulence
    let radial_falloff = 1.0 - pow(dist_from_center, 1.3);
    let noise_contribution = combined_noise * 0.5 + 0.5; // Remap to 0-1

    // Create billowing, turbulent edge effect
    let edge_turbulence = noise_contribution * (1.0 - radial_falloff * 0.5);
    let fire_intensity = clamp(radial_falloff + edge_turbulence * 0.3, 0.0, 1.0);

    // Color based on fire intensity and progress
    // Higher intensity = hotter colors (yellow-white)
    // Lower intensity = cooler colors (red-crimson)
    let color_t = 1.0 - fire_intensity + combined_noise * 0.2;
    let fire_color = explosion_fire_gradient(color_t, progress);

    // Apply overall intensity animation
    let final_intensity = intensity * fire_intensity;

    // Flicker effect for fire realism
    let flicker = sin(time * 18.0) * 0.04 + sin(time * 31.0) * 0.03 + sin(time * 47.0) * 0.02;
    let flicker_intensity = 1.0 + flicker;

    // Apply emissive multiplier for HDR bloom
    let emissive_color = fire_color * emissive * final_intensity * flicker_intensity;

    // Alpha based on intensity and edge falloff
    // Creates soft, billowing edges
    let edge_softness = smoothstep(1.0, 0.7, dist_from_center);
    let noise_alpha = noise_contribution * 0.3 + 0.7;
    let alpha = final_intensity * edge_softness * noise_alpha;

    return vec4<f32>(emissive_color, alpha);
}
