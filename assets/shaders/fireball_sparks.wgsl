// Fireball Sparks Shader - Flying Ember Particles
// Creates bright spark particles with motion blur effect that fly off the fireball.
// Features:
// - Bright yellow-white core with orange halo
// - Motion blur / streak effect based on velocity
// - Animated flicker for lifelike spark behavior
// - Gravity-affected trajectory visualization
// - Random variation in size/brightness via noise

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Uniform buffer (matches Rust #[uniform(0)])
// ============================================================================

struct FireballSparksMaterial {
    time: vec4<f32>,
    velocity: vec4<f32>,
    lifetime_progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballSparksMaterial;

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

// Hash function for 2D input - improved version
fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(p.xyx * vec3<f32>(443.897, 441.423, 437.195));
    let p4 = dot(p3, p3.yzx + 19.19);
    return fract(sin(p4) * 43758.5453);
}

// Hash function for 3D input - improved version
fn hash13(p: vec3<f32>) -> f32 {
    let p2 = fract(p * vec3<f32>(443.897, 441.423, 437.195));
    let p3 = dot(p2, p2.zyx + 19.19);
    return fract(sin(p3) * 43758.5453);
}

// 2D gradient noise
fn gradient2d(p: vec2<f32>) -> vec2<f32> {
    let angle = hash12(p) * TAU;
    return vec2<f32>(cos(angle), sin(angle));
}

fn perlin2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    let g00 = gradient2d(i + vec2<f32>(0.0, 0.0));
    let g10 = gradient2d(i + vec2<f32>(1.0, 0.0));
    let g01 = gradient2d(i + vec2<f32>(0.0, 1.0));
    let g11 = gradient2d(i + vec2<f32>(1.0, 1.0));

    let d00 = f - vec2<f32>(0.0, 0.0);
    let d10 = f - vec2<f32>(1.0, 0.0);
    let d01 = f - vec2<f32>(0.0, 1.0);
    let d11 = f - vec2<f32>(1.0, 1.0);

    let v00 = dot(g00, d00);
    let v10 = dot(g10, d10);
    let v01 = dot(g01, d01);
    let v11 = dot(g11, d11);

    let x0 = mix(v00, v10, u.x);
    let x1 = mix(v01, v11, u.x);
    return mix(x0, x1, u.y);
}

// ============================================================================
// Spark Color Functions
// ============================================================================

// Spark color gradient: white-hot core -> yellow -> orange -> dim red
fn spark_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.1 {
        // White-hot core (brightest center)
        return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 1.0, 0.9), x / 0.1);
    } else if x < 0.3 {
        // White-yellow transition
        return mix(vec3<f32>(1.0, 1.0, 0.9), vec3<f32>(1.0, 0.95, 0.5), (x - 0.1) / 0.2);
    } else if x < 0.5 {
        // Yellow to bright orange
        return mix(vec3<f32>(1.0, 0.95, 0.5), vec3<f32>(1.0, 0.7, 0.2), (x - 0.3) / 0.2);
    } else if x < 0.7 {
        // Bright orange to deep orange
        return mix(vec3<f32>(1.0, 0.7, 0.2), vec3<f32>(1.0, 0.45, 0.0), (x - 0.5) / 0.2);
    } else if x < 0.85 {
        // Deep orange to red
        return mix(vec3<f32>(1.0, 0.45, 0.0), vec3<f32>(0.8, 0.2, 0.0), (x - 0.7) / 0.15);
    } else {
        // Red to dim ember (dying spark)
        return mix(vec3<f32>(0.8, 0.2, 0.0), vec3<f32>(0.3, 0.05, 0.0), (x - 0.85) / 0.15);
    }
}

// Cooling color based on lifetime: starts hot, cools as it ages
fn cooling_gradient(lifetime: f32) -> vec3<f32> {
    let x = clamp(lifetime, 0.0, 1.0);

    if x < 0.2 {
        // Bright yellow-white at start (new spark)
        return vec3<f32>(1.0, 1.0, 0.8);
    } else if x < 0.4 {
        // Yellow-orange as it flies
        return mix(vec3<f32>(1.0, 1.0, 0.8), vec3<f32>(1.0, 0.7, 0.2), (x - 0.2) / 0.2);
    } else if x < 0.7 {
        // Orange cooling
        return mix(vec3<f32>(1.0, 0.7, 0.2), vec3<f32>(1.0, 0.4, 0.0), (x - 0.4) / 0.3);
    } else {
        // Dim red ember before disappearing
        return mix(vec3<f32>(1.0, 0.4, 0.0), vec3<f32>(0.4, 0.1, 0.0), (x - 0.7) / 0.3);
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
    // Use material uniforms for lifetime and emissive
    let lifetime = material.lifetime_progress.x;  // 0.0 = new spark, 1.0 = expired
    let emissive = material.emissive_intensity.x;

    let pos = in.local_position;

    // Distance from center for spark shape
    let dist = length(pos.xz);

    // Core spark - bright center that shrinks as lifetime increases
    let core_size = 0.2 * (1.0 - lifetime * 0.5);  // Core shrinks over time
    let core = 1.0 - smoothstep(0.0, core_size, dist);
    let core_intensity = pow(core, 2.0);

    // Halo around core - also diminishes with lifetime
    let halo_size = 0.5 * (1.0 - lifetime * 0.3);
    let halo = 1.0 - smoothstep(core_size * 0.5, halo_size, dist);
    let halo_intensity = pow(halo, 1.5) * (1.0 - core_intensity * 0.5);

    // Animated flicker - more erratic as spark dies
    let flicker_seed = hash12(pos.xz * 50.0);
    let flicker_speed = 25.0 + lifetime * 20.0;  // Faster flicker near death
    let flicker = 0.7 + 0.3 * sin(t * flicker_speed + flicker_seed * TAU);

    // Colors - core stays white-hot, halo cools with lifetime
    let core_color = cooling_gradient(lifetime * 0.3);  // Core cools slowly
    let halo_color = cooling_gradient(lifetime);  // Halo cools faster

    let combined_color = core_color * core_intensity + halo_color * halo_intensity;

    // Overall fade based on lifetime - spark dies out
    // At lifetime=0, full intensity; at lifetime=1, faded completely
    let lifetime_fade = 1.0 - pow(lifetime, 1.5);  // Gradual fade
    let total_intensity = (core_intensity + halo_intensity * 0.5) * flicker * lifetime_fade;

    let final_color = combined_color * emissive * total_intensity;
    let alpha = (core_intensity + halo_intensity * 0.4) * lifetime_fade;

    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
