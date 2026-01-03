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
}

// ============================================================================
// Uniforms
// ============================================================================

struct ExplosionEmbersMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Lifetime progress 0.0 (start) to 1.0 (end), packed in x
    progress: vec4<f32>,
    // Velocity direction for motion blur (xyz = direction, w = speed magnitude)
    velocity: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
}

@group(2) @binding(0)
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

// Hash function for pseudo-random values
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
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
    let time = material.time.x;
    let progress = material.progress.x;
    let velocity_dir = normalize(material.velocity.xyz);
    let speed = material.velocity.w;
    let emissive = material.emissive_intensity.x;

    // Center UV for radial calculations
    let centered_uv = in.uv - vec2<f32>(0.5, 0.5);

    // ========================================================================
    // Motion Streak Effect
    // ========================================================================
    // Project UV onto velocity direction for elongation
    // This creates the motion blur/streak effect for fast-moving embers
    let velocity_2d = normalize(vec2<f32>(velocity_dir.x, velocity_dir.z));
    let along_velocity = dot(centered_uv, velocity_2d);
    let perp_velocity = length(centered_uv - velocity_2d * along_velocity);

    // Streak factor based on speed - faster embers appear more elongated
    // Explosion embers are FAST (speed ~15), so significant streaking
    let streak_amount = clamp(speed * 0.15, 0.0, 3.0);
    let streak_length = 1.0 + streak_amount;

    // Create elongated ember shape (ellipse stretched along velocity)
    let stretched_along = along_velocity / streak_length;
    let radial_dist = sqrt(stretched_along * stretched_along + perp_velocity * perp_velocity);

    // ========================================================================
    // Ember Core Shape
    // ========================================================================
    // Hot core with halo
    let core_radius = 0.12;
    let halo_radius = 0.35;

    // Bright inner core
    let core = 1.0 - smoothstep(0.0, core_radius, radial_dist);
    let core_intensity = pow(core, 2.5);

    // Cooler halo around core
    let halo = 1.0 - smoothstep(core_radius, halo_radius, radial_dist);
    let halo_intensity = pow(halo, 1.8) * (1.0 - core_intensity);

    // ========================================================================
    // Animation and Flicker
    // ========================================================================
    // Animated flicker for lifelike ember behavior
    let flicker_freq = 30.0;
    let flicker_seed = hash12(in.world_position.xz * 100.0);
    let flicker = 0.75 + 0.25 * sin(time * flicker_freq + flicker_seed * TAU);

    // Random brightness variation per ember
    let brightness_var = 0.6 + 0.4 * hash13(in.world_position);

    // ========================================================================
    // Lifetime-Based Effects
    // ========================================================================
    // Progress phases for the 0.8s ember flight:
    // 0.0 - 0.1: Initial burst (bright start)
    // 0.1 - 0.7: Flying phase (gradual cooling)
    // 0.7 - 1.0: Final fade (dying ember)

    var intensity: f32;
    if progress < 0.1 {
        // Rapid rise to peak brightness
        intensity = smoothstep(0.0, 0.1, progress);
    } else if progress < 0.7 {
        // Gradual decay during flight
        let decay_progress = (progress - 0.1) / 0.6;
        intensity = 1.0 - decay_progress * 0.3; // Lose 30% brightness during flight
    } else {
        // Rapid fade at end
        let fade_progress = (progress - 0.7) / 0.3;
        intensity = 0.7 * (1.0 - fade_progress * fade_progress);
    }

    // ========================================================================
    // Gravity Arc Visualization
    // ========================================================================
    // Simulate gravity pulling ember downward over time
    // This affects the color (lower = cooler as it falls) and brightness
    let gravity_factor = progress * progress * 0.5; // Quadratic for realistic arc
    let cooling_from_gravity = gravity_factor * 0.2; // Extra cooling as it falls

    // ========================================================================
    // Color Calculation
    // ========================================================================
    // Core is white-hot, halo shows the cooling gradient
    let cooling_amount = progress + cooling_from_gravity;
    let core_color = vec3<f32>(1.0, 1.0, 0.9); // White-hot
    let halo_color = ember_cooling_gradient(cooling_amount);

    let combined_color = core_color * core_intensity + halo_color * halo_intensity;

    // ========================================================================
    // Final Output
    // ========================================================================
    // Apply all intensity modifiers
    let total_intensity = (core_intensity + halo_intensity * 0.5) * flicker * brightness_var * intensity;

    // Final emissive color for HDR bloom
    let final_color = combined_color * emissive * total_intensity;

    // Alpha: strong in center, fading at edges and with age
    let alpha = (core_intensity + halo_intensity * 0.4) * intensity;

    // Discard nearly transparent fragments for performance
    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
