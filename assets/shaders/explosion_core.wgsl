// Explosion Core Flash Shader
// Creates a white-hot burst effect at explosion center.
// Features:
// - Blinding bright white/yellow expanding sphere
// - Rapid expansion then fade (0.25s total)
// - Very high emissive for HDR bloom
// - Sharp falloff at edges

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// ============================================================================
// Uniforms
// ============================================================================

struct ExplosionCoreMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Lifetime progress 0.0 (start) to 1.0 (end), packed in x
    progress: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
    // Expansion scale multiplier (packed in x component)
    expansion_scale: vec4<f32>,
}

@group(2) @binding(0)
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

// Simple value noise for subtle variation
fn value_noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Smoothstep interpolation
    let u = f * f * (3.0 - 2.0 * f);

    // Eight corner values
    let a = hash13(i + vec3<f32>(0.0, 0.0, 0.0));
    let b = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let e = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let g = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let h = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let j = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    // Trilinear interpolation
    let x0 = mix(a, b, u.x);
    let x1 = mix(c, d, u.x);
    let x2 = mix(e, g, u.x);
    let x3 = mix(h, j, u.x);
    let y0 = mix(x0, x1, u.y);
    let y1 = mix(x2, x3, u.y);
    return mix(y0, y1, u.z);
}

// ============================================================================
// Color Functions
// ============================================================================

// White-hot to yellow-orange flash gradient
// This is the color progression for a very hot explosion flash
fn flash_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.3 {
        // Core: pure white (extremely hot)
        return vec3<f32>(1.0, 1.0, 1.0);
    } else if x < 0.5 {
        // White to pale yellow
        let blend = (x - 0.3) / 0.2;
        return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 1.0, 0.85), blend);
    } else if x < 0.7 {
        // Pale yellow to bright yellow
        let blend = (x - 0.5) / 0.2;
        return mix(vec3<f32>(1.0, 1.0, 0.85), vec3<f32>(1.0, 0.95, 0.5), blend);
    } else {
        // Bright yellow to orange (edge)
        let blend = (x - 0.7) / 0.3;
        return mix(vec3<f32>(1.0, 0.95, 0.5), vec3<f32>(1.0, 0.7, 0.2), blend);
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
    let emissive = material.emissive_intensity.x;
    let expansion = material.expansion_scale.x;

    // Progress phases for the 0.25s flash:
    // 0.0 - 0.15: Rapid expansion (flash peak)
    // 0.15 - 0.4: Peak brightness
    // 0.4 - 1.0: Fade out

    // Calculate intensity based on progress
    var intensity: f32;
    if progress < 0.15 {
        // Rapid rise to peak
        intensity = smoothstep(0.0, 0.15, progress);
    } else if progress < 0.4 {
        // Hold at peak
        intensity = 1.0;
    } else {
        // Fade out (cubic falloff for more dramatic fade)
        let fade_progress = (progress - 0.4) / 0.6;
        intensity = 1.0 - fade_progress * fade_progress * fade_progress;
    }

    // Use local position for sphere-based effects
    let pos = in.local_position;

    // Distance from center (0 at center, 1 at surface for unit sphere)
    let dist_from_center = length(pos);

    // Core brightness: much brighter in center, sharp falloff at edges
    // Use power function for sharp edge
    let radial_falloff = 1.0 - pow(dist_from_center, 3.0);
    let core_brightness = clamp(radial_falloff, 0.0, 1.0);

    // Add subtle noise for organic edge variation
    let noise_pos = pos * 8.0 + vec3<f32>(time * 2.0, time * 1.5, time);
    let edge_noise = value_noise3d(noise_pos) * 0.15;

    // Color based on distance from center
    // Center is white-hot, edges are yellow-orange
    let color_t = dist_from_center + edge_noise;
    let flash_color = flash_gradient(color_t);

    // Final intensity combines:
    // - Overall animation intensity (fades over lifetime)
    // - Core brightness (center is brightest)
    // - Edge noise variation
    let final_intensity = intensity * (core_brightness + edge_noise * 0.5);

    // Apply emissive multiplier for HDR bloom
    // Use very high values for blinding flash effect
    let emissive_color = flash_color * emissive * final_intensity;

    // Alpha based on intensity for transparency blending
    // Sharp falloff at edges for clean sphere boundary
    let edge_alpha = smoothstep(1.0, 0.85, dist_from_center);
    let alpha = final_intensity * edge_alpha;

    return vec4<f32>(emissive_color, alpha);
}
