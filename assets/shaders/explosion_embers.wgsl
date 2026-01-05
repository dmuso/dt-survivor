// Explosion Dark Projectiles Shader - Velocity-Stretched Debris
// Creates elongated dark projectiles that stretch along their velocity direction.
// Features:
// - Vertex stretching along velocity axis based on speed
// - Dark charcoal color with slight noise variation
// - Low emissive for dark appearance (no glow)
// - Stretch factor: 1.0 at rest to ~4x at max speed
// - Duration ~0.6s with rapid deceleration
// - Progress-based animation (0.0 = start, 1.0 = end)

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
    velocity: vec4<f32>,      // xyz = direction (normalized), w = speed
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
    @location(4) stretch_factor: f32,
}

// ============================================================================
// Constants
// ============================================================================

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

// Dark charcoal base color (slightly warm)
const DARK_BASE: vec3<f32> = vec3<f32>(0.12, 0.10, 0.08);
// Slightly lighter edges
const DARK_EDGE: vec3<f32> = vec3<f32>(0.18, 0.15, 0.12);

// Stretch parameters
const MAX_SPEED: f32 = 18.0;  // Speed at which we hit max stretch
const MAX_STRETCH: f32 = 4.0;  // Maximum stretch multiplier (4x length at max speed)

// ============================================================================
// Noise Functions
// ============================================================================

// Hash function for 3D input
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
}

// Simple 3D noise for color variation
fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash13(i + vec3<f32>(0.0, 0.0, 0.0)), hash13(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash13(i + vec3<f32>(0.0, 1.0, 0.0)), hash13(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash13(i + vec3<f32>(0.0, 0.0, 1.0)), hash13(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash13(i + vec3<f32>(0.0, 1.0, 1.0)), hash13(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

// ============================================================================
// Vertex Shader - Velocity-based stretching
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // Get velocity direction and speed from material
    let velocity_dir = material.velocity.xyz;
    let speed = material.velocity.w;

    // Calculate stretch factor based on speed (1.0 at rest, MAX_STRETCH at max speed)
    let speed_ratio = clamp(speed / MAX_SPEED, 0.0, 1.0);
    let stretch_factor = 1.0 + speed_ratio * (MAX_STRETCH - 1.0);

    // Stretch vertex position along velocity axis
    var stretched_pos = vertex.position;

    // Only stretch if we have a valid velocity direction
    let vel_len = length(velocity_dir);
    if vel_len > 0.01 {
        let vel_normalized = velocity_dir / vel_len;

        // Project position onto velocity axis
        let along_vel = dot(vertex.position, vel_normalized);

        // Stretch only the component along velocity
        stretched_pos = vertex.position + vel_normalized * along_vel * (stretch_factor - 1.0);
    }

    // Transform to world space
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(stretched_pos, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = stretched_pos;
    out.stretch_factor = stretch_factor;

    return out;
}

// ============================================================================
// Fragment Shader - Dark charcoal coloring
// ============================================================================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let prog = material.progress.x;
    let emissive = material.emissive_intensity.x;

    let pos = in.local_position;

    // Distance from center (accounting for stretched shape)
    let dist = length(pos);

    // Core/edge gradient
    let edge_factor = smoothstep(0.0, 0.4, dist);

    // Noise for color variation (subtle)
    let noise_val = noise3d(pos * 8.0 + t * 0.5) * 0.1;

    // Base dark color with edge brightening
    let base_color = mix(DARK_BASE, DARK_EDGE, edge_factor);

    // Add subtle noise variation
    let color = base_color + vec3<f32>(noise_val);

    // Fade out as progress increases (projectile cools and fades)
    let fade_start = 0.5;
    let fade_factor = 1.0 - smoothstep(fade_start, 1.0, prog);

    // Edge softness for smooth falloff
    let edge_softness = 1.0 - smoothstep(0.35, 0.5, dist);

    // Final alpha combines edge softness and progress fade
    let alpha = edge_softness * fade_factor;

    // Discard fully transparent fragments
    if alpha < 0.01 {
        discard;
    }

    // Low emissive to keep dark appearance (no bloom glow)
    return vec4<f32>(color * emissive, alpha);
}
