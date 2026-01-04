// Fireball Core Volumetric Fire Shader
// Creates an animated fire sphere with noise-based turbulence,
// color gradients from yellow core to orange edge, and HDR emissive output.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Bevy 0.17 bindless storage buffer
// ============================================================================

// Material Data - Uniform buffer (matches Rust #[uniform(0)])
struct FireballCoreMaterial {
    time: vec4<f32>,
    animation_speed: vec4<f32>,
    noise_scale: vec4<f32>,
    emissive_intensity: vec4<f32>,
    velocity_dir: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballCoreMaterial;

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
    @location(4) trail_dir: vec3<f32>,
}

// ============================================================================
// Noise Functions (inlined for shader compilation)
// ============================================================================

const PI: f32 = 3.14159265359;

// Hash function for 2D input - improved version
fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(p.xyx * vec3<f32>(443.897, 441.423, 437.195));
    let p4 = dot(p3, p3.yzx + 19.19);
    return fract(p4 * p4);
}

// Hash function for 3D input - improved version using sin
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
}

// 3D gradient noise
fn gradient3d(p: vec3<f32>) -> vec3<f32> {
    let h = hash13(p) * 6.28318530718;
    let z = hash13(p + vec3<f32>(127.1, 311.7, 74.7)) * 2.0 - 1.0;
    let r = sqrt(1.0 - z * z);
    return vec3<f32>(r * cos(h), r * sin(h), z);
}

// 3D Perlin-style noise
fn perlin3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Quintic interpolation
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

// Fractional Brownian Motion with 4 octaves
fn fbm4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * perlin3d(pos * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// Turbulence (absolute value FBM)
fn turbulence4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * abs(perlin3d(pos * frequency));
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Fire Color Functions
// ============================================================================

// Classic fire gradient: black -> red -> orange -> yellow -> white
fn fire_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.2 {
        // Black to dark red
        return mix(vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.5, 0.0, 0.0), x / 0.2);
    } else if x < 0.4 {
        // Dark red to red
        return mix(vec3<f32>(0.5, 0.0, 0.0), vec3<f32>(1.0, 0.2, 0.0), (x - 0.2) / 0.2);
    } else if x < 0.6 {
        // Red to orange
        return mix(vec3<f32>(1.0, 0.2, 0.0), vec3<f32>(1.0, 0.5, 0.0), (x - 0.4) / 0.2);
    } else if x < 0.8 {
        // Orange to yellow
        return mix(vec3<f32>(1.0, 0.5, 0.0), vec3<f32>(1.0, 0.9, 0.2), (x - 0.6) / 0.2);
    } else {
        // Yellow to white (hot core)
        return mix(vec3<f32>(1.0, 0.9, 0.2), vec3<f32>(1.0, 1.0, 0.9), (x - 0.8) / 0.2);
    }
}

// ============================================================================
// Vertex Shader - With Noise-Based Displacement for Flame Effect
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let t = globals.time;
    let pos = vertex.position;
    let normal = vertex.normal;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    // Extract trail direction from transform matrix (Z column)
    // Rust: from_rotation_arc(NEG_Z, direction) makes local -Z point toward travel
    // So local +Z (matrix Z column) points opposite to travel = trail direction
    let trail_dir = normalize(vec3<f32>(world_from_local[2][0], world_from_local[2][1], world_from_local[2][2]));

    // Sample noise at vertex position for displacement
    // Noise scrolls in the trail direction for animated flames
    let noise_pos = pos * 2.0 + trail_dir * t * 3.0;

    // Two octaves of noise for flame variation
    let noise1 = perlin3d(noise_pos);
    let noise2 = perlin3d(noise_pos * 2.0 + vec3<f32>(7.3, 0.0, 13.7)) * 0.5;
    let displacement = (noise1 + noise2) * 0.5 + 0.5; // Remap to 0-1

    // Stronger displacement on the trailing side of the sphere
    let trail_dot = dot(normal, trail_dir);
    let trail_bias = smoothstep(-0.2, 0.9, trail_dot); // Trailing side

    // Base stretch along trail direction (comet shape)
    let trail_stretch = 1.2;
    let stretch_amount = trail_bias * trail_stretch;

    // Add noise displacement along normals for flame tongues (only on trailing half)
    let flame_displacement = trail_bias * displacement * 0.4;

    // Combine: stretch backward + noise outward
    let displaced_pos = pos + trail_dir * stretch_amount + normal * flame_displacement;

    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(displaced_pos, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = pos; // Keep original for fragment shader calculations
    out.trail_dir = trail_dir; // Pass to fragment shader

    return out;
}

// ============================================================================
// Fragment Shader - With Alpha Cutoff for Flame Edges
// ============================================================================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let pos = in.local_position;

    // Get trail direction from vertex shader (passed via VertexOutput)
    let trail_dir = normalize(in.trail_dir + vec3<f32>(0.0001, 0.0001, 0.0001));

    // Distance from center of sphere (0 at center, 1 at surface)
    let dist_from_center = length(pos);

    // Multiple noise layers at different scales and speeds
    // Noise scrolls in trail direction to simulate flames being left behind
    let noise1 = perlin3d(pos * 2.5 + trail_dir * t * 3.0);  // Large, fast
    let noise2 = perlin3d(pos * 5.0 + trail_dir * t * 4.0 + vec3<f32>(t * 0.5, 0.0, t * 0.3));  // Medium detail
    let noise3 = perlin3d(pos * 8.0 + trail_dir * t * 5.0 + vec3<f32>(-t * 0.3, 0.0, 0.0));  // Fine detail

    // Combine noise with weights favoring larger features
    let combined_noise = noise1 * 0.5 + noise2 * 0.35 + noise3 * 0.15;

    // Trail gradient - flames are more intense on trailing side
    let pos_normalized = normalize(pos + vec3<f32>(0.0001, 0.0001, 0.0001));
    let trail_factor = smoothstep(-0.5, 0.8, dot(pos_normalized, trail_dir));

    // Alpha calculation for edge cutoff
    // Core is solid (alpha = 1), edges are cut based on noise
    let edge_distance = 1.0 - dist_from_center; // 1 at center, 0 at edge
    let edge_noise = combined_noise * 0.5 + 0.5; // Remap to 0-1

    // Combine edge distance with noise for irregular flame edges
    // Higher noise values push the edge inward (creating flame tongues where alpha survives)
    let alpha_base = edge_distance * 1.5 + edge_noise * 0.6 - trail_factor * 0.2;

    // Hard alpha cutoff for fire effect
    let alpha_threshold = 0.3;
    let alpha = step(alpha_threshold, alpha_base);

    // Discard fully transparent pixels
    if alpha < 0.5 {
        discard;
    }

    // Color gradient based on depth into flame
    // Core (high alpha_base) = white-hot, edge (low alpha_base) = red-orange
    let color_t = clamp(alpha_base * 0.8 + 0.2, 0.0, 1.0);
    let color = fire_gradient(color_t);

    // Flicker effect for life
    let flicker = 1.0 + sin(t * 17.0) * 0.15 + sin(t * 31.0) * 0.1 + sin(t * 47.0) * 0.05;

    // HDR emissive boost - fire should glow
    let emissive_boost = 5.0;

    return vec4<f32>(color * emissive_boost * flicker, 1.0);
}
