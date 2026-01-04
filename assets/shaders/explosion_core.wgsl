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
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Storage buffer (Bevy 0.17 bindless default)
// ============================================================================

struct ExplosionCoreMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
    expansion_scale: vec4<f32>,
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
}

// ============================================================================
// Noise Functions (inlined for shader compilation)
// ============================================================================

// Hash function for 3D input - improved version
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
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
    let t = globals.time;
    let prog = material.progress.x;
    let emissive = material.emissive_intensity.x;

    let pos = in.local_position;

    // Flash intensity - very bright at start, fades quickly with non-linear falloff
    // Use exponential falloff for sharp flash effect
    let intensity = pow(1.0 - prog, 2.0);

    // Noise for variation and edge distortion
    let noise = value_noise3d(pos * 5.0 + vec3<f32>(t * 15.0, t * 10.0, t * 8.0));

    // Distance from center for edge effects
    let dist = length(pos);

    // Edge glow - core is brightest, edge fades
    let edge_factor = 1.0 - smoothstep(0.0, 0.4, dist);

    // Color: white-hot at start, shifts to pale yellow/orange as it fades
    let white = vec3<f32>(1.0, 1.0, 1.0);
    let pale_yellow = vec3<f32>(1.0, 0.95, 0.7);
    let warm_orange = vec3<f32>(1.0, 0.8, 0.4);

    // Color shifts from white -> pale yellow -> warm as progress increases
    var color: vec3<f32>;
    if prog < 0.5 {
        color = mix(white, pale_yellow, prog * 2.0);
    } else {
        color = mix(pale_yellow, warm_orange, (prog - 0.5) * 2.0);
    }

    // Add noise variation
    color = color + vec3<f32>(noise * 0.1, noise * 0.05, 0.0);

    // Fast flicker for flash effect
    let flicker = 1.0 + sin(t * 50.0) * 0.1 + sin(t * 73.0) * 0.05;

    // Combine intensity, edge, and noise
    let combined_intensity = intensity * (0.3 + edge_factor * 0.7);

    // Output: very bright flash with proper fading
    let final_color = color * emissive * combined_intensity * flicker;

    // Alpha for additive blending - decreases with progress
    let alpha = intensity * (0.5 + edge_factor * 0.5);

    // Discard if too dim
    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
