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
}

// ============================================================================
// Uniforms
// ============================================================================

struct FireballSparksMaterial {
    // Current time for animation (packed in x component)
    time: vec4<f32>,
    // Velocity direction for motion blur (xyz = direction, w = speed magnitude)
    velocity: vec4<f32>,
    // Spark lifetime progress 0.0 (new) to 1.0 (dying) (packed in x component)
    lifetime_progress: vec4<f32>,
    // Emissive intensity for HDR bloom (packed in x component)
    emissive_intensity: vec4<f32>,
}

@group(2) @binding(0)
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

// Hash function for pseudo-random values
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
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
    let time = material.time.x;
    let velocity_dir = normalize(material.velocity.xyz);
    let speed = material.velocity.w;
    let lifetime = material.lifetime_progress.x;
    let emissive = material.emissive_intensity.x;

    // Center UV for radial calculations
    let centered_uv = in.uv - vec2<f32>(0.5, 0.5);

    // Create motion blur streak based on velocity
    // Project UV onto velocity direction for elongation
    let velocity_2d = normalize(vec2<f32>(velocity_dir.x, velocity_dir.z));
    let along_velocity = dot(centered_uv, velocity_2d);
    let perp_velocity = length(centered_uv - velocity_2d * along_velocity);

    // Streak factor based on speed - faster sparks appear more elongated
    let streak_amount = clamp(speed * 0.3, 0.0, 2.0);
    let streak_length = 1.0 + streak_amount;

    // Create elongated spark shape (ellipse stretched along velocity)
    let stretched_along = along_velocity / streak_length;
    let radial_dist = sqrt(stretched_along * stretched_along + perp_velocity * perp_velocity);

    // Core spark shape with sharp bright center
    let core_radius = 0.15;
    let halo_radius = 0.4;

    // White-hot inner core
    let core = 1.0 - smoothstep(0.0, core_radius, radial_dist);
    let core_intensity = pow(core, 2.0);

    // Orange halo around core
    let halo = 1.0 - smoothstep(core_radius, halo_radius, radial_dist);
    let halo_intensity = pow(halo, 1.5) * (1.0 - core_intensity);

    // Animated flicker for lifelike behavior
    let flicker_freq = 25.0;
    let flicker_seed = hash12(in.world_position.xz * 100.0);
    let flicker = 0.8 + 0.2 * sin(time * flicker_freq + flicker_seed * TAU);

    // Random brightness variation per spark
    let brightness_var = 0.7 + 0.3 * hash13(in.world_position);

    // Intensity decreases as spark ages (lifetime 0->1)
    let age_falloff = pow(1.0 - lifetime, 0.5); // Quick at first, slows down

    // Combine core and halo colors
    let core_color = vec3<f32>(1.0, 1.0, 0.95); // White-hot
    let halo_color = cooling_gradient(lifetime); // Cools with age

    let combined_color = core_color * core_intensity + halo_color * halo_intensity;

    // Apply all intensity modifiers
    let total_intensity = (core_intensity + halo_intensity * 0.6) * flicker * brightness_var * age_falloff;

    // Final emissive color for HDR bloom
    let final_color = combined_color * emissive * total_intensity;

    // Alpha: strong in center, fading at edges and with age
    let alpha = (core_intensity + halo_intensity * 0.5) * age_falloff;

    // Discard nearly transparent fragments for performance
    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(final_color, alpha);
}
