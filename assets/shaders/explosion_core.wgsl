// Explosion Core Star-burst Shader
// Creates an irregular star-burst effect for explosion impacts.
// Features:
// - 5-8 pointed star with irregular spike lengths
// - Vertex displacement to create spike geometry
// - Rapid growth (0-0.24) then shrink (0.24-1.0)
// - White-hot center with orange-yellow edges
// - Very high emissive for HDR bloom

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data
// ============================================================================

struct ExplosionCoreMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
    expansion_scale: vec4<f32>,
    // spike_config: .x = seed, .y = spike_count, .z = min_length, .w = max_length
    spike_config: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: ExplosionCoreMaterial;

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
    @location(4) spike_factor: f32,
}

// ============================================================================
// Noise and Hash Functions
// ============================================================================

// Simple hash for deterministic randomness based on seed
fn hash11(p: f32) -> f32 {
    let p2 = fract(p * 0.1031);
    let p3 = p2 * (p2 + 33.33);
    return fract((p3 + p2) * p3);
}

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let p4 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p4.x + p4.y) * p4.z);
}

// ============================================================================
// Star-burst Spike Functions
// ============================================================================

// Calculate spike displacement for a given angle
// Returns a spike multiplier (1.0 = base, >1.0 = spike peak)
fn calculate_spike_factor(angle: f32, seed: f32, spike_count: f32, min_len: f32, max_len: f32) -> f32 {
    let tau = 6.283185307;

    // Normalize angle to 0-1 range
    let normalized_angle = angle / tau;

    // Calculate spike positions with slight random offset
    var max_spike = 0.0;

    for (var i = 0u; i < 8u; i = i + 1u) {
        if f32(i) >= spike_count {
            break;
        }

        // Base spike angle with random offset based on seed
        let spike_base = f32(i) / spike_count;
        let spike_offset = hash11(seed * 100.0 + f32(i) * 7.31) * 0.12 - 0.06;
        let spike_angle = spike_base + spike_offset;

        // Distance from this spike (wrapped around circle)
        var dist = abs(normalized_angle - spike_angle);
        dist = min(dist, 1.0 - dist); // Handle wrap-around

        // Spike profile - VERY sharp peaks using power function
        // This creates narrower, more defined spikes than gaussian
        let spike_width = 0.012 + hash11(seed * 50.0 + f32(i) * 3.14) * 0.008;
        let normalized_dist = dist / spike_width;
        // Use 1/(1+x^4) for sharp peak, much steeper than gaussian
        let spike_strength = 1.0 / (1.0 + normalized_dist * normalized_dist * normalized_dist * normalized_dist);

        // Random spike length variation - longer spikes for dramatic effect
        let spike_len = mix(min_len, max_len, hash11(seed * 200.0 + f32(i) * 11.7));

        max_spike = max(max_spike, spike_strength * spike_len);
    }

    // Base sphere plus spike displacement
    return 1.0 + max_spike;
}

// ============================================================================
// Animation Functions
// ============================================================================

// Calculate scale based on progress
// Rapid growth (0-0.24) then shrink (0.24-1.0)
fn calculate_animation_scale(progress: f32) -> f32 {
    let growth_end = 0.24;

    if progress < growth_end {
        // Rapid growth phase - ease out
        let t = progress / growth_end;
        return t * t * (3.0 - 2.0 * t); // smoothstep
    } else {
        // Shrink phase - ease in
        let t = (progress - growth_end) / (1.0 - growth_end);
        return 1.0 - t * t;
    }
}

// ============================================================================
// Vertex Shader
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let prog = material.progress.x;
    let seed = material.spike_config.x;
    let spike_count = material.spike_config.y;
    let min_len = material.spike_config.z;
    let max_len = material.spike_config.w;

    // Get position in spherical-ish coordinates for spike calculation
    let pos = vertex.position;
    let normal = vertex.normal;

    // Calculate angle around Y axis (horizontal plane spikes)
    let angle_xz = atan2(pos.z, pos.x) + 3.141592653;

    // Calculate spike factor based on angle
    let spike = calculate_spike_factor(angle_xz, seed, spike_count, min_len, max_len);

    // Calculate animation scale
    let anim_scale = calculate_animation_scale(prog);

    // Apply spike displacement along normal
    // Spikes are more prominent in the XZ plane (horizontal burst)
    let xz_factor = 1.0 - abs(pos.y) * 0.5; // Reduce spikes at top/bottom (less aggressive)
    let displacement = (spike - 1.0) * xz_factor * anim_scale * 1.5; // Amplify displacement

    // Displaced position
    let displaced_pos = pos + normal * displacement;

    // Apply overall animation scale
    let final_pos = displaced_pos * anim_scale * material.expansion_scale.x;

    // Transform to world space
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(final_pos, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = displaced_pos;
    out.spike_factor = spike;

    return out;
}

// ============================================================================
// Fragment Shader
// ============================================================================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let prog = material.progress.x;
    let emissive = material.emissive_intensity.x;

    let pos = in.local_position;
    let spike = in.spike_factor;

    // Distance from center for color gradient
    let dist = length(pos);

    // Flash intensity - very bright at start, fades with progress
    // Use exponential falloff for sharp flash effect
    let intensity = pow(1.0 - prog, 1.5);

    // Color gradient based on spike factor and distance
    // White-hot core -> pale yellow -> orange at edges/spikes
    let white = vec3<f32>(1.0, 1.0, 1.0);
    let pale_yellow = vec3<f32>(1.0, 0.98, 0.85);
    let bright_yellow = vec3<f32>(1.0, 0.95, 0.5);
    let orange = vec3<f32>(1.0, 0.7, 0.2);

    // Normalize distance for color lookup (spike tips are further out)
    let norm_dist = clamp(dist / (spike * 0.5), 0.0, 1.0);

    // Color progression: center white -> yellow -> orange at edges
    var color: vec3<f32>;
    if norm_dist < 0.3 {
        color = mix(white, pale_yellow, norm_dist / 0.3);
    } else if norm_dist < 0.6 {
        color = mix(pale_yellow, bright_yellow, (norm_dist - 0.3) / 0.3);
    } else {
        color = mix(bright_yellow, orange, (norm_dist - 0.6) / 0.4);
    }

    // Add slight flicker for energy feel
    let t = globals.time;
    let flicker = 1.0 + sin(t * 40.0) * 0.08 + sin(t * 67.0) * 0.04;

    // Spike tips are slightly brighter (emphasized)
    let spike_emphasis = 1.0 + (spike - 1.0) * 0.3;

    // Edge glow - core is brightest
    let edge_factor = 1.0 - smoothstep(0.0, 0.5, norm_dist);
    let combined_intensity = intensity * (0.4 + edge_factor * 0.6) * spike_emphasis;

    // Final color with HDR emissive
    let final_color = color * emissive * combined_intensity * flicker;

    // Alpha for additive blending - decreases with progress
    let alpha = intensity * (0.6 + edge_factor * 0.4);

    // Discard if too dim
    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
