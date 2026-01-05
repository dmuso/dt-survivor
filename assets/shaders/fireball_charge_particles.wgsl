// Fireball Charge Particles Shader
// Creates inward-traveling energy particles that converge toward the fireball center.
// Uses surface-based rendering - particles are rendered along rays from the surface to center.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data
// ============================================================================

struct FireballChargeParticlesMaterial {
    time: vec4<f32>,
    charge_progress: vec4<f32>,
    particle_count: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballChargeParticlesMaterial;

// ============================================================================
// Vertex Structures
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

// Particle configuration - relative to unit sphere
const PARTICLE_COUNT: i32 = 20;
const PARTICLE_SIZE: f32 = 0.25;        // Angular size of each particle (larger = more visible)
const TRAVEL_SPEED: f32 = 1.2;          // How fast particles cycle inward
const SPAWN_RADIUS: f32 = 0.98;         // Start near edge of sphere
const INNER_RADIUS: f32 = 0.15;         // Fade out near center

// ============================================================================
// Hash Functions
// ============================================================================

fn hash11(p: f32) -> f32 {
    var p2 = fract(p * 443.897);
    p2 = p2 * (p2 + 19.19);
    return fract(p2 * p2);
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(443.897, 441.423, 437.195));
    p3 = p3 + dot(p3, p3.yzx + 19.19);
    return fract((p3.x + p3.y) * p3.z);
}

// ============================================================================
// Color Functions
// ============================================================================

fn particle_color(heat: f32) -> vec3<f32> {
    let t = clamp(heat, 0.0, 1.0);

    // Orange -> Yellow -> White-hot gradient
    if t < 0.5 {
        return mix(vec3<f32>(1.0, 0.4, 0.0), vec3<f32>(1.0, 0.8, 0.2), t * 2.0);
    } else {
        return mix(vec3<f32>(1.0, 0.8, 0.2), vec3<f32>(1.0, 1.0, 0.8), (t - 0.5) * 2.0);
    }
}

// ============================================================================
// Particle System
// ============================================================================

// Get particle position on unit sphere at given time
fn get_particle_direction(particle_id: f32, time: f32) -> vec3<f32> {
    // Random but stable direction for each particle
    let theta = hash11(particle_id * 127.1) * TAU;
    let phi = acos(2.0 * hash11(particle_id * 311.7) - 1.0);

    // Slight wobble animation
    let wobble = sin(time * 3.0 + particle_id * 5.0) * 0.1;

    return vec3<f32>(
        sin(phi + wobble) * cos(theta),
        cos(phi),
        sin(phi + wobble) * sin(theta)
    );
}

// Get particle's current radial position (0 = center, 1 = edge)
fn get_particle_radius(particle_id: f32, time: f32) -> f32 {
    let phase = hash11(particle_id * 173.3);
    let cycle = fract(time * TRAVEL_SPEED * (0.8 + hash11(particle_id * 234.5) * 0.4) + phase);

    // Travel from outer to inner
    return mix(SPAWN_RADIUS, INNER_RADIUS, cycle);
}

// Calculate particle contribution at a given surface point
// Renders particle with a streak/tail toward center to show inward motion
fn calculate_particle(
    surface_dir: vec3<f32>,
    particle_id: f32,
    time: f32,
    charge: f32
) -> vec3<f32> {
    let particle_dir = get_particle_direction(particle_id, time);
    let particle_radius = get_particle_radius(particle_id, time);

    // Angular distance from surface point to particle direction
    let cos_angle = dot(normalize(surface_dir), particle_dir);
    let angle_dist = acos(clamp(cos_angle, -1.0, 1.0));

    // Base particle size
    let base_size = PARTICLE_SIZE * (0.5 + 0.5 * particle_radius);

    // Core particle intensity
    let core_intensity = smoothstep(base_size, base_size * 0.2, angle_dist);

    // Add streak/tail toward center (shows motion direction)
    // Project surface direction toward center and check if it's "behind" the particle
    let toward_center = -particle_dir; // Direction from particle toward center
    let streak_alignment = dot(normalize(surface_dir - particle_dir * cos_angle), toward_center);

    // Streak extends from particle toward center
    let streak_length = base_size * 2.5; // Trail length
    let streak_width = base_size * 0.4;  // Narrow trail

    // Check if point is in the streak region (between particle and center)
    let along_streak = angle_dist; // Distance along the streak direction
    let streak_intensity = smoothstep(streak_length, 0.0, along_streak) *
                          smoothstep(streak_width, streak_width * 0.3, abs(angle_dist - along_streak * 0.5)) *
                          max(0.0, streak_alignment) * 0.6;

    // Combine core and streak
    let total_intensity = core_intensity + streak_intensity * (1.0 - core_intensity);

    // Fade based on radial position (brighter at outer edge, fading as approaches center)
    let radial_fade = smoothstep(INNER_RADIUS * 0.3, SPAWN_RADIUS * 0.4, particle_radius);

    // Color gets hotter as particle approaches center
    let heat = 1.0 - particle_radius;
    let color = particle_color(heat);

    // Charge affects overall intensity
    let charge_mult = 0.5 + charge * 0.5;

    return color * total_intensity * radial_fade * charge_mult * 1.8;
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
    let charge = material.charge_progress.x;
    let emissive = material.emissive_intensity.x;

    // Use normalized local position as direction from center
    let surface_dir = normalize(in.local_position);

    // Accumulate particle contributions
    var total_color = vec3<f32>(0.0);

    for (var i = 0; i < PARTICLE_COUNT; i = i + 1) {
        let particle_color = calculate_particle(surface_dir, f32(i), t, charge);
        total_color = total_color + particle_color;
    }

    // Add sparkle/flicker
    let flicker = 1.0 + sin(t * 20.0 + surface_dir.x * 15.0) * 0.15;
    total_color = total_color * flicker;

    // Apply emissive intensity
    let final_color = total_color * emissive;

    // Alpha based on total intensity
    let alpha = clamp(length(total_color) * 0.5, 0.0, 1.0);

    return vec4<f32>(final_color, alpha);
}
