// Explosion Fire Shader - Main Orange-Red Fireball Blast
// Creates the "meat" of the explosion with volumetric fire effect.
// Features:
// - Large expanding fireball with volumetric noise
// - Color progression: yellow-orange -> red -> dark crimson -> fade
// - Turbulent edges with animated noise
// - Rising heat effect (upward bias)
// - Duration ~0.6s
// - Billowing fire support: organic FBM noise displacement for multi-sphere explosions
//   When velocity.w > 0, enables billowing displacement for moving fire spheres

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ============================================================================
// Material Data - Storage buffer (Bevy 0.17 bindless default)
// ============================================================================

struct ExplosionFireMaterial {
    time: vec4<f32>,
    progress: vec4<f32>,
    emissive_intensity: vec4<f32>,
    noise_scale: vec4<f32>,
    // velocity: xyz = normalized direction, w = speed magnitude
    velocity: vec4<f32>,
    // growth_config: x = growth_rate (1.0-3.0)
    growth_config: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> material: ExplosionFireMaterial;

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

// 3D gradient function
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

    // Quintic interpolation for smoother results
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

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * perlin3d(p * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// Turbulence (absolute value FBM for billowing effect)
fn turbulence4(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * abs(perlin3d(p * frequency));
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Fire Color Functions
// ============================================================================

// Explosion fire gradient: yellow-orange -> red -> crimson -> fade
// This is the color progression for the main fire blast
fn explosion_fire_gradient(t: f32, progress: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    // Base fire colors
    var color: vec3<f32>;

    if x < 0.15 {
        // Core: bright yellow-white (hottest)
        color = mix(vec3<f32>(1.0, 1.0, 0.9), vec3<f32>(1.0, 0.9, 0.4), x / 0.15);
    } else if x < 0.35 {
        // Yellow to orange
        let blend = (x - 0.15) / 0.2;
        color = mix(vec3<f32>(1.0, 0.9, 0.4), vec3<f32>(1.0, 0.55, 0.1), blend);
    } else if x < 0.55 {
        // Orange to red
        let blend = (x - 0.35) / 0.2;
        color = mix(vec3<f32>(1.0, 0.55, 0.1), vec3<f32>(0.95, 0.25, 0.0), blend);
    } else if x < 0.75 {
        // Red to dark crimson
        let blend = (x - 0.55) / 0.2;
        color = mix(vec3<f32>(0.95, 0.25, 0.0), vec3<f32>(0.5, 0.08, 0.0), blend);
    } else {
        // Dark crimson to black (cooling)
        let blend = (x - 0.75) / 0.25;
        color = mix(vec3<f32>(0.5, 0.08, 0.0), vec3<f32>(0.1, 0.02, 0.0), blend);
    }

    // As progress increases, shift colors toward cooler tones
    // Early: bright yellows and oranges
    // Late: dark reds and blacks
    let cool_shift = progress * 0.4;
    let cooler_color = mix(color, color * vec3<f32>(0.7, 0.5, 0.8), cool_shift);

    return cooler_color;
}

// ============================================================================
// Vertex Shader - With Billowing Displacement
// ============================================================================

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let t = globals.time;
    let prog = material.progress.x;
    let speed = material.velocity.w;
    let growth_rate = material.growth_config.x;

    // Base position
    var pos = vertex.position;
    let normal = vertex.normal;

    // Apply billowing displacement when velocity indicates a moving sphere
    // (speed > 0 means this is a billowing fire sphere, not the static central explosion)
    if speed > 0.1 {
        // Animated noise position - scrolls with time for organic motion
        let noise_speed = 2.5;
        let noise_pos = pos * 2.5 + vec3<f32>(
            t * noise_speed * 0.3,
            -t * noise_speed,  // Upward scroll for flames rising
            t * noise_speed * 0.2
        );

        // Sample FBM noise for organic displacement
        let displacement_strength = 0.35;
        let noise = fbm4(noise_pos);

        // Displace along normal for billowing effect
        pos = pos + normal * noise * displacement_strength;

        // Additional turbulence on the edges for flame-like appearance
        let edge_noise_pos = pos * 4.0 + vec3<f32>(t * 3.0, -t * 5.0, t * 2.0);
        let edge_displacement = perlin3d(edge_noise_pos) * 0.15;
        pos = pos + normal * edge_displacement;
    }

    // Apply growth based on progress (spheres expand over time)
    let scale_factor = 1.0 + prog * (growth_rate - 1.0);
    pos = pos * scale_factor;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(pos, 1.0));

    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_position = vertex.position;  // Keep original for fragment shader

    return out;
}

// ============================================================================
// Smoke Color Functions
// ============================================================================

// Smoke color gradient based on noise-driven heat variation
fn smoke_gradient(heat: f32) -> vec3<f32> {
    // Smoke colors: dark to light gray-brown
    let dark_smoke = vec3<f32>(0.25, 0.22, 0.2);
    let mid_smoke = vec3<f32>(0.32, 0.29, 0.27);
    let light_smoke = vec3<f32>(0.4, 0.38, 0.35);

    // Map heat to smoke color (hot areas are slightly lighter)
    if heat > 0.6 {
        return mix(mid_smoke, light_smoke, (heat - 0.6) / 0.4);
    } else if heat > 0.3 {
        return mix(dark_smoke, mid_smoke, (heat - 0.3) / 0.3);
    } else {
        return dark_smoke;
    }
}

// ============================================================================
// Fragment Shader
// ============================================================================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let prog = material.progress.x;
    let base_emissive = material.emissive_intensity.x;

    let pos = in.local_position;
    let anim = t * 5.0;

    // Animated noise for fire movement - scrolls upward
    let noise1 = perlin3d(pos * 3.0 + vec3<f32>(0.0, -anim, 0.0));
    let noise2 = perlin3d(pos * 6.0 + vec3<f32>(anim * 0.2, -anim * 1.2, 0.0));
    let noise = noise1 * 0.6 + noise2 * 0.4;

    // Heat based on noise - varies across surface
    let heat = clamp(0.5 + noise * 0.5, 0.0, 1.0);

    // Fire color gradient: hot (yellow-white) to cool (red-orange)
    var fire_color: vec3<f32>;
    if heat > 0.7 {
        // Hot: bright yellow-white
        fire_color = mix(vec3<f32>(1.0, 0.8, 0.3), vec3<f32>(1.0, 1.0, 0.8), (heat - 0.7) / 0.3);
    } else if heat > 0.4 {
        // Medium: orange
        fire_color = mix(vec3<f32>(1.0, 0.4, 0.0), vec3<f32>(1.0, 0.8, 0.3), (heat - 0.4) / 0.3);
    } else {
        // Cool: red-orange
        fire_color = mix(vec3<f32>(0.6, 0.1, 0.0), vec3<f32>(1.0, 0.4, 0.0), heat / 0.4);
    }

    // Progress fades and reddens the fire (only during fire phase)
    let fire_fade = 1.0 - prog * 0.5;  // Less aggressive fade to preserve color during transition
    fire_color = mix(fire_color, fire_color * vec3<f32>(0.8, 0.3, 0.1), min(prog * 0.6, 0.3));

    // Get smoke color based on same heat value for continuity
    let smoke_color = smoke_gradient(heat);

    // Fire to smoke transition:
    // 0.0-0.4: Pure fire
    // 0.4-0.7: Transition fire -> smoke
    // 0.7-1.0: Pure smoke
    let transition_t = smoothstep(0.4, 0.7, prog);

    // Blend colors
    var color = mix(fire_color, smoke_color, transition_t);

    // Emissive intensity transition:
    // Fire: full emissive (base_emissive)
    // Smoke: reduced emissive (0.5) for ambient-lit appearance
    let smoke_emissive = 0.5;
    let emissive = mix(base_emissive, smoke_emissive, transition_t);

    // Flicker - stronger during fire phase, gentler during smoke
    let flicker_strength = mix(0.1, 0.03, transition_t);
    let flicker = 1.0 + sin(anim * 7.0) * flicker_strength;

    // Base alpha (fully opaque by default)
    var alpha = 1.0;

    // ========================================================================
    // Smoke Dissipation Mask (Stage 7)
    // During smoke phase (prog > 0.7), a rising sphere mask "eats" the smoke
    // from below, creating a dissolve effect
    // ========================================================================
    if prog > 0.7 {
        // Dissipation progress: 0.0 at prog=0.7, 1.0 at prog=1.0
        let dissipation_prog = smoothstep(0.7, 1.0, prog);

        // Mask center rises from below the sphere (-1.0) to above it (+1.5)
        let mask_center_y = -1.0 + dissipation_prog * 2.5;

        // Mask radius grows as it rises (starts small, ends large)
        let mask_radius = dissipation_prog * 1.2;

        // Calculate distance from fragment to mask center (in local space)
        let mask_center = vec3<f32>(0.0, mask_center_y, 0.0);
        let dist_to_mask = distance(pos, mask_center);

        // Soft edge mask: fragments inside the mask sphere become transparent
        // smoothstep creates a soft transition at the mask boundary
        let sphere_mask = smoothstep(mask_radius - 0.15, mask_radius + 0.05, dist_to_mask);

        // Apply mask to alpha
        alpha = sphere_mask;

        // Discard nearly-transparent fragments for performance
        if alpha < 0.02 {
            discard;
        }
    }

    // Final output with HDR emissive and alpha
    let final_color = color * emissive * fire_fade * flicker;

    return vec4<f32>(final_color, alpha);
}
