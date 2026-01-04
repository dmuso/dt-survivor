// Fireball Core - BLAZING HOT sphere
// Must be unmistakably bright and glowing

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

struct FireballCoreMaterial {
    time: vec4<f32>,
    animation_speed: vec4<f32>,
    noise_scale: vec4<f32>,
    emissive_intensity: vec4<f32>,
    velocity_dir: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballCoreMaterial;

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

fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(
            mix(hash31(i), hash31(i + vec3(1.0, 0.0, 0.0)), u.x),
            mix(hash31(i + vec3(0.0, 1.0, 0.0)), hash31(i + vec3(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash31(i + vec3(0.0, 0.0, 1.0)), hash31(i + vec3(1.0, 0.0, 1.0)), u.x),
            mix(hash31(i + vec3(0.0, 1.0, 1.0)), hash31(i + vec3(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // No displacement - keep it a clean sphere
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = vertex.position;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let pos = in.local_position;

    // Distance from center
    let dist = length(pos);
    let norm_dist = clamp(dist / 0.3, 0.0, 1.0);

    // Animated subtle variation
    let noise_scroll = vec3<f32>(t * 2.0, -t * 3.0, t * 1.5);
    let noise_val = noise3d(pos * 6.0 + noise_scroll);

    // Core is WHITE HOT - almost pure white with slight yellow tint
    // Edge has orange tint
    let white_hot = vec3<f32>(1.0, 0.98, 0.95);
    let orange_edge = vec3<f32>(1.0, 0.6, 0.2);

    // Radial gradient - white center to orange edge
    let color = mix(white_hot, orange_edge, norm_dist * norm_dist);

    // Subtle animated variation in brightness
    let variation = 1.0 + (noise_val - 0.5) * 0.1;

    // Flicker
    let flicker = 1.0 + sin(t * 23.0) * 0.05 + sin(t * 37.0) * 0.03;

    // MASSIVE emissive - this needs to GLOW
    // Center is 8x brighter than edge
    let center_boost = (1.0 - norm_dist) * 7.0 + 1.0;
    let emissive = material.emissive_intensity.x * center_boost * 2.0;

    let final_color = color * emissive * variation * flicker;

    return vec4<f32>(final_color, 1.0);
}
