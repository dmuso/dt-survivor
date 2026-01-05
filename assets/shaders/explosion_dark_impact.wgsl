// Explosion Dark Impact Shader
// Creates dark silhouette spikes radiating outward behind the initial impact.
// Features:
// - Irregular spikes similar to explosion core but dark coloring
// - Vertex displacement to create spike geometry
// - Charcoal center (0.15, 0.12, 0.1) to black edges
// - Low emissive (0.3) for silhouette effect
// - Duration: 0.4s, spawns at t=0.06s

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data
// ============================================================================

struct ExplosionDarkImpactMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
    expansion_scale: vec4<f32>,
    // spike_config: .x = seed, .y = spike_count, .z = min_length, .w = max_length
    spike_config: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: ExplosionDarkImpactMaterial;

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
        let spike_offset = hash11(seed * 100.0 + f32(i) * 7.31) * 0.15 - 0.075;
        let spike_angle = spike_base + spike_offset;

        // Distance from this spike (wrapped around circle)
        var dist = abs(normalized_angle - spike_angle);
        dist = min(dist, 1.0 - dist); // Handle wrap-around

        // Spike profile - very sharp peaks for dramatic spikes
        let spike_width = 0.008 + hash11(seed * 50.0 + f32(i) * 3.14) * 0.006;
        let normalized_dist = dist / spike_width;
        // Use 1/(1+x^6) for sharper peak
        let x2 = normalized_dist * normalized_dist;
        let spike_strength = 1.0 / (1.0 + x2 * x2 * x2);

        // Random spike length variation - larger range for more variety
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
// Growth phase (0-0.3) then hold (0.3-0.7) then shrink (0.7-1.0)
fn calculate_animation_scale(progress: f32) -> f32 {
    let growth_end = 0.3;
    let hold_end = 0.7;

    if progress < growth_end {
        // Growth phase - ease out
        let t = progress / growth_end;
        return t * t * (3.0 - 2.0 * t); // smoothstep
    } else if progress < hold_end {
        // Hold phase - stay at full size
        return 1.0;
    } else {
        // Shrink phase - ease in
        let t = (progress - hold_end) / (1.0 - hold_end);
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
    let xz_factor = 1.0 - abs(pos.y) * 0.6; // Reduce spikes at top/bottom
    let displacement = (spike - 1.0) * xz_factor * anim_scale * 1.3;

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

    // Dark intensity - darker at edges, slightly lighter in center
    // Fades with progress
    let intensity = 1.0 - prog * 0.3;

    // Dark color gradient: charcoal center to black edges
    let charcoal_center = vec3<f32>(0.15, 0.12, 0.10);
    let dark_mid = vec3<f32>(0.10, 0.08, 0.06);
    let black_edge = vec3<f32>(0.05, 0.04, 0.03);

    // Normalize distance for color lookup
    let norm_dist = clamp(dist / (spike * 0.5), 0.0, 1.0);

    // Color progression: center charcoal -> mid -> black at edges
    var color: vec3<f32>;
    if norm_dist < 0.4 {
        color = mix(charcoal_center, dark_mid, norm_dist / 0.4);
    } else {
        color = mix(dark_mid, black_edge, (norm_dist - 0.4) / 0.6);
    }

    // Subtle variation based on spike factor (spike tips slightly different)
    let spike_variation = (spike - 1.0) * 0.02;
    color = color + vec3<f32>(spike_variation, spike_variation * 0.8, spike_variation * 0.6);

    // Add subtle noise for texture
    let t = globals.time;
    let noise_sample = hash21(vec2<f32>(pos.x * 10.0 + t * 0.5, pos.z * 10.0));
    color = color + (noise_sample - 0.5) * 0.02;

    // Edge darkening - spikes are darker at tips
    let edge_darken = 1.0 - smoothstep(0.3, 0.9, norm_dist) * 0.3;

    // Final color with low emissive (silhouette effect, not glowing)
    let final_color = color * emissive * intensity * edge_darken;

    // Alpha - solid but fades at edges and with progress
    let alpha_base = 0.9 - norm_dist * 0.3;
    let alpha_fade = 1.0 - smoothstep(0.6, 1.0, prog);
    let alpha = alpha_base * alpha_fade;

    // Discard if too dim
    if alpha < 0.02 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
