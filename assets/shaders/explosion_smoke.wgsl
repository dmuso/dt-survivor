// Explosion Smoke Shader - Rising Dark Plume Effect
// Creates the aftermath smoke rising from an explosion.
// Features:
// - Dark volumetric smoke billowing upward
// - Expands as it rises (heat dissipation)
// - Semi-transparent gray with ember glow underneath
// - Duration ~1.2s (longest lasting)
// - Turbulent noise for realistic smoke motion

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// ============================================================================
// Uniforms
// ============================================================================

struct ExplosionSmokeMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Lifetime progress 0.0 (start) to 1.0 (end), packed in x
    progress: vec4<f32>,
    // Emissive intensity for subtle glow (packed in x component)
    emissive_intensity: vec4<f32>,
    // Noise scale for turbulence detail (packed in x component)
    noise_scale: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: ExplosionSmokeMaterial;

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

// Hash function for 3D input
fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

// 3D gradient function
fn gradient3d(p: vec3<f32>) -> vec3<f32> {
    let h = hash13(p) * TAU;
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

// Fractional Brownian Motion with 4 octaves for billowing smoke
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
// Smoke Color Functions
// ============================================================================

// Smoke color gradient: dark gray with ember glow underneath
// At early stages, shows warm ember glow, transitions to cool gray smoke
fn smoke_color_gradient(t: f32, progress: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    // Base smoke colors - dark grays with slight warmth early on
    var color: vec3<f32>;

    // Early stages (low progress): warmer gray with ember undertone
    // Late stages (high progress): cooler dark gray
    let warmth = 1.0 - progress;

    if x < 0.3 {
        // Hot core - slight ember glow visible through smoke
        let ember = vec3<f32>(0.35, 0.15, 0.05); // Dim ember orange
        let warm_gray = vec3<f32>(0.25, 0.2, 0.18); // Warm dark gray
        color = mix(ember, warm_gray, x / 0.3);
    } else if x < 0.6 {
        // Transition zone - warm gray to neutral gray
        let warm_gray = vec3<f32>(0.25, 0.2, 0.18);
        let neutral_gray = vec3<f32>(0.18, 0.17, 0.16);
        color = mix(warm_gray, neutral_gray, (x - 0.3) / 0.3);
    } else if x < 0.85 {
        // Main smoke body - neutral to cool gray
        let neutral_gray = vec3<f32>(0.18, 0.17, 0.16);
        let cool_gray = vec3<f32>(0.12, 0.12, 0.13);
        color = mix(neutral_gray, cool_gray, (x - 0.6) / 0.25);
    } else {
        // Outer edges - very dark, almost black
        let cool_gray = vec3<f32>(0.12, 0.12, 0.13);
        let dark = vec3<f32>(0.05, 0.05, 0.06);
        color = mix(cool_gray, dark, (x - 0.85) / 0.15);
    }

    // Apply warmth factor based on progress
    // Early: more ember glow visible, Late: cooler smoke
    let ember_boost = warmth * 0.15 * (1.0 - x);
    color = color + vec3<f32>(ember_boost, ember_boost * 0.4, 0.0);

    return color;
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

    // Progress phases for the 1.2s smoke duration:
    // 0.0 - 0.15: Fade in (smoke appears as fire fades)
    // 0.15 - 0.7: Rise and expand (main smoke phase)
    // 0.7 - 1.0: Dissipate and fade out

    // Calculate base intensity based on progress
    var intensity: f32;
    if progress < 0.15 {
        // Fade in - smoke appears gradually
        intensity = smoothstep(0.0, 0.15, progress);
    } else if progress < 0.7 {
        // Sustained with subtle variation
        let variance = sin(progress * 8.0) * 0.05;
        intensity = 0.85 + variance;
    } else {
        // Fade out - dissipate into air
        let fade_progress = (progress - 0.7) / 0.3;
        intensity = 0.85 * (1.0 - fade_progress * fade_progress);
    }

    // Use local position for sphere-based effects
    let pos = in.local_position;

    // Distance from center
    let dist_from_center = length(pos);

    // Rising smoke effect - strong upward motion
    // Smoke rises faster as it heats up, slows as it cools
    let rise_speed = mix(2.5, 1.0, progress); // Faster early, slower late
    let heat_rise = vec3<f32>(0.0, time * rise_speed, 0.0);

    // Animated noise position with rising and turbulent motion
    let noise_pos = pos * noise_scale + heat_rise + vec3<f32>(
        sin(time * 0.3) * 0.5,
        0.0,
        cos(time * 0.4) * 0.5
    );

    // Layer multiple noise for volumetric billowing smoke
    let noise1 = fbm4(noise_pos);
    let noise2 = turbulence4(noise_pos * 1.3 + vec3<f32>(0.0, time * 0.8, 0.0));
    let noise3 = fbm4(noise_pos * 0.7 - vec3<f32>(time * 0.15, 0.0, time * 0.1));

    // Combine noise layers - more turbulence than fire
    let combined_noise = noise1 * 0.35 + noise2 * 0.45 + noise3 * 0.2;

    // Smoke density based on position and noise
    // Center is denser, edges have noise-driven billowing
    let radial_falloff = 1.0 - pow(dist_from_center, 1.1);
    let noise_contribution = combined_noise * 0.5 + 0.5; // Remap to 0-1

    // Create billowing, turbulent edge effect for smoke
    let edge_turbulence = noise_contribution * (1.0 - radial_falloff * 0.3);
    let smoke_density = clamp(radial_falloff * 0.8 + edge_turbulence * 0.4, 0.0, 1.0);

    // Color based on smoke density and progress
    // Denser = darker core, Sparser = lighter edges
    let color_t = 1.0 - smoke_density + combined_noise * 0.25;
    let smoke_color = smoke_color_gradient(color_t, progress);

    // Apply overall intensity animation
    let final_intensity = intensity * smoke_density;

    // Subtle flicker for organic motion (less than fire)
    let flicker = sin(time * 6.0) * 0.02 + sin(time * 11.0) * 0.015;
    let flicker_intensity = 1.0 + flicker;

    // Apply emissive multiplier (subtle for smoke, mainly for ember glow)
    let ember_glow_factor = (1.0 - progress) * 0.5; // More glow early on
    let final_emissive = emissive * (0.5 + ember_glow_factor);
    let emissive_color = smoke_color * final_emissive * final_intensity * flicker_intensity;

    // Alpha based on density and edge softness
    // Smoke is semi-transparent with soft billowing edges
    let edge_softness = smoothstep(1.0, 0.6, dist_from_center);
    let noise_alpha = noise_contribution * 0.35 + 0.65;
    let base_alpha = final_intensity * edge_softness * noise_alpha;

    // Smoke alpha should be lower than fire - semi-transparent
    let alpha = base_alpha * 0.7;

    // Discard nearly transparent fragments for performance
    if alpha < 0.02 {
        discard;
    }

    return vec4<f32>(emissive_color, alpha);
}
