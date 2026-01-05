// Explosion Smoke Shader
// Rising smoke plume with volumetric billowing effect

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// Material uniform struct - must match Rust struct field order
struct ExplosionSmokeMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
    noise_scale: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: ExplosionSmokeMaterial;

// Vertex structures
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
// Noise Functions (from noise.wgsl pattern)
// ============================================================================

fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// Quintic interpolation for smoother gradients
fn quintic(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

// 3D Perlin-style noise
fn perlin3d(p: vec3<f32>) -> f32 {
    let Pi = floor(p);
    let Pf = fract(p);
    let w = quintic(Pf);

    // Hash corners
    let n000 = hash13(Pi + vec3<f32>(0.0, 0.0, 0.0));
    let n001 = hash13(Pi + vec3<f32>(0.0, 0.0, 1.0));
    let n010 = hash13(Pi + vec3<f32>(0.0, 1.0, 0.0));
    let n011 = hash13(Pi + vec3<f32>(0.0, 1.0, 1.0));
    let n100 = hash13(Pi + vec3<f32>(1.0, 0.0, 0.0));
    let n101 = hash13(Pi + vec3<f32>(1.0, 0.0, 1.0));
    let n110 = hash13(Pi + vec3<f32>(1.0, 1.0, 0.0));
    let n111 = hash13(Pi + vec3<f32>(1.0, 1.0, 1.0));

    // Trilinear interpolation
    let n00 = mix(n000, n100, w.x);
    let n01 = mix(n001, n101, w.x);
    let n10 = mix(n010, n110, w.x);
    let n11 = mix(n011, n111, w.x);

    let n0 = mix(n00, n10, w.y);
    let n1 = mix(n01, n11, w.y);

    return mix(n0, n1, w.z) * 2.0 - 1.0;
}

// Fractional Brownian Motion
fn fbm3(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 3; i = i + 1) {
        value = value + amplitude * perlin3d(pos * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Vertex Shader
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let t = globals.time;
    let prog = material.progress.x;

    // Get base position
    var pos = vertex.position;

    // Billowing displacement using noise
    let noise_sample = pos * 2.0 + vec3<f32>(0.0, -t * 0.5, 0.0);
    let displacement = fbm3(noise_sample) * 0.15 * (1.0 - prog * 0.3);

    // Apply displacement along normal for billowy effect
    pos = pos + vertex.normal * displacement;

    // Transform to world space
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(pos, 1.0)
    );

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

    // Use local position for consistent volumetric effect
    let pos = in.local_position;

    // Animate noise for billowing effect
    let anim = t * 1.5;
    let noise1 = perlin3d(pos * 2.5 + vec3<f32>(0.0, -anim * 0.4, 0.0));
    let noise2 = perlin3d(pos * 4.0 + vec3<f32>(anim * 0.1, -anim * 0.6, 0.0));
    let combined_noise = noise1 * 0.6 + noise2 * 0.4;

    // Smoke color - darker gray with noise variation
    let base_gray = 0.4;
    let color = vec3<f32>(
        base_gray + combined_noise * 0.1,
        base_gray + combined_noise * 0.08,
        base_gray + combined_noise * 0.06
    );

    // Fade with progress - goes to 0 at end
    let fade = 1.0 - prog;

    // Use view-facing softness based on normal
    let view_dot = abs(dot(normalize(in.world_normal), normalize(in.world_position)));
    let softness = 1.0 - view_dot * 0.3;

    // Alpha - more transparent
    let alpha = fade * softness * 0.5;

    if alpha < 0.01 { discard; }

    return vec4<f32>(color, alpha);
}
