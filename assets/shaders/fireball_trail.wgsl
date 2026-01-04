// Fireball Trail - Teardrop with SOLID head, noisy tail
// Trail near ball = solid, tail end = broken/animated

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

struct FireballTrailMaterial {
    time: vec4<f32>,
    velocity_dir: vec4<f32>,
    trail_length: vec4<f32>,
    emissive_intensity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: FireballTrailMaterial;

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
    @location(4) trail_progress: f32,
    @location(5) radial_dist: f32,
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

fn fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < 3; i = i + 1) {
        value = value + amplitude * noise3d(pos);
        pos = pos * 2.0;
        amplitude = amplitude * 0.5;
    }
    return value;
}

fn trail_gradient(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    // Bright yellow-orange at head, dark red at tail
    let bright = vec3<f32>(1.0, 0.7, 0.2);
    let orange = vec3<f32>(1.0, 0.4, 0.05);
    let red = vec3<f32>(0.8, 0.15, 0.0);
    let dark = vec3<f32>(0.4, 0.05, 0.0);

    if x < 0.3 {
        return mix(bright, orange, x / 0.3);
    } else if x < 0.6 {
        return mix(orange, red, (x - 0.3) / 0.3);
    } else {
        return mix(red, dark, (x - 0.6) / 0.4);
    }
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let t = globals.time;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let pos = vertex.position;

    // Trail progress: 0 at head, 1 at tail
    let local_trail_dir = vec3<f32>(0.0, 0.0, 1.0);
    let trail_dot = dot(pos, local_trail_dir);
    let trail_progress = clamp((trail_dot + 0.15) / 0.45, 0.0, 1.0);

    // Stretch into trail
    let trail_length = material.trail_length.x * 4.0;
    let stretch = pow(trail_progress, 0.5) * trail_length;
    var deformed_pos = pos + local_trail_dir * stretch;

    // TEARDROP TAPER - VERY aggressive narrowing toward tail
    // At progress 0 (head): taper = 1.0 (full width)
    // At progress 0.5: taper = 0.25 (quarter width)
    // At progress 1.0 (tail): taper = 0 (point)
    let taper = (1.0 - trail_progress) * (1.0 - trail_progress);
    deformed_pos.x = deformed_pos.x * taper;
    deformed_pos.y = deformed_pos.y * taper;

    // Edge waviness - only applies AFTER the head for organic flowing edges
    // Head (trail_progress 0-15%) is completely solid to connect seamlessly with fireball core
    // Remaining 85% has animated, flowing fire movement
    let edge_factor = smoothstep(0.15, 0.3, trail_progress);  // 0 at head, 1 after 30%
    // Fast noise scroll for dramatic flowing animation
    let edge_noise_pos = pos * 2.0 + vec3<f32>(t * 12.0, t * 9.0, -t * 20.0);
    // Use minimum taper of 0.5 so animation remains very visible even at thin tail
    let edge_taper = max(taper, 0.5);
    // Massive displacement (3.0) for very dramatic flowing fire effect
    let edge_wave = (fbm(edge_noise_pos) - 0.5) * 3.0 * edge_taper * edge_factor;
    let edge_wave2 = (fbm(edge_noise_pos * 1.3 + vec3<f32>(77.0, 0.0, 0.0)) - 0.5) * 3.0 * edge_taper * edge_factor;
    deformed_pos.x = deformed_pos.x + edge_wave;
    deformed_pos.y = deformed_pos.y + edge_wave2;

    // Extra breakup at tail end
    let breakup_zone = smoothstep(0.7, 0.95, trail_progress);
    if breakup_zone > 0.01 {
        let breakup_pos = pos * 5.0 + vec3<f32>(t * 8.0, t * 6.0, -t * 20.0);
        let breakup = breakup_zone * 0.3;
        deformed_pos.x = deformed_pos.x + (fbm(breakup_pos) - 0.5) * breakup;
        deformed_pos.y = deformed_pos.y + (fbm(breakup_pos * 1.5 + vec3<f32>(50.0, 0.0, 0.0)) - 0.5) * breakup;
    }

    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(deformed_pos, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = deformed_pos;  // Use deformed position
    out.trail_progress = trail_progress;
    out.radial_dist = length(deformed_pos.xy);  // Radial dist of DEFORMED mesh

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let emissive = material.emissive_intensity.x;
    let trail_progress = in.trail_progress;

    // With additive blending, overlap with core looks good (brighter)
    // Minimal discard just to avoid rendering fully inside the core sphere
    if trail_progress < 0.005 {
        discard;
    }

    // Animated noise
    let noise_scroll = vec3<f32>(t * 15.0, t * 10.0, -t * 35.0);
    let noise_val = fbm(in.local_position * 3.0 + noise_scroll);

    // Animation zone: starts at 15% (stable head), full animation after 30%
    let animation_zone = smoothstep(0.15, 0.3, trail_progress);

    // Breakup zone: only break up the back 30% of the trail
    let breakup_zone = smoothstep(0.7, 0.95, trail_progress);

    // In the back 30%, use noise to randomly discard pixels for broken effect
    if breakup_zone > 0.1 {
        let breakup = noise_val * breakup_zone;
        if breakup > 0.5 {
            discard;
        }
    }

    // Let the full tail render - geometry defines the end
    // No early cutoff

    // Color gradient along trail - use animation_zone so color varies in 85% of trail
    // Increased variation (0.15) for more visible fire flickering
    let color_var = (noise_val - 0.5) * 0.15 * animation_zone;
    let fire_color = trail_gradient(trail_progress + color_var);

    // Brightness falloff along trail
    let brightness = 1.0 - trail_progress * 0.4;

    // Flicker
    let flicker = 1.0 + sin(t * 35.0) * 0.1 + sin(t * 53.0) * 0.06;

    let final_color = fire_color * emissive * 2.2 * brightness * flicker;

    return vec4<f32>(final_color, 1.0);
}
