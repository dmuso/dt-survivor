// Fire shader primitives and color gradients
// Common functions for all fire-based effects (fireballs, flames, explosions)

// ============================================================================
// Fire Color Palettes
// ============================================================================

// Classic fire gradient: black -> red -> orange -> yellow -> white
fn fire_gradient_classic(t: f32) -> vec3<f32> {
    // Clamped input
    let x = clamp(t, 0.0, 1.0);

    // Multi-stop gradient
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

// Simplified fire gradient using smooth polynomial
fn fire_gradient_smooth(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    return vec3<f32>(
        min(1.0, x * 3.0),                           // Red rises quickly
        max(0.0, x * 2.0 - 0.5) * x,                 // Green rises slower
        max(0.0, x - 0.7) * x * x * 3.0              // Blue only at hottest
    );
}

// Blue fire gradient (magical/arcane fire)
fn fire_gradient_blue(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.3 {
        // Dark blue to blue
        return mix(vec3<f32>(0.0, 0.0, 0.2), vec3<f32>(0.0, 0.3, 0.8), x / 0.3);
    } else if x < 0.6 {
        // Blue to cyan
        return mix(vec3<f32>(0.0, 0.3, 0.8), vec3<f32>(0.2, 0.8, 1.0), (x - 0.3) / 0.3);
    } else {
        // Cyan to white
        return mix(vec3<f32>(0.2, 0.8, 1.0), vec3<f32>(0.9, 1.0, 1.0), (x - 0.6) / 0.4);
    }
}

// Green fire gradient (toxic/poison fire)
fn fire_gradient_green(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.3 {
        // Dark to dark green
        return mix(vec3<f32>(0.0, 0.1, 0.0), vec3<f32>(0.1, 0.4, 0.0), x / 0.3);
    } else if x < 0.6 {
        // Dark green to bright green
        return mix(vec3<f32>(0.1, 0.4, 0.0), vec3<f32>(0.3, 0.9, 0.1), (x - 0.3) / 0.3);
    } else {
        // Bright green to yellow-green
        return mix(vec3<f32>(0.3, 0.9, 0.1), vec3<f32>(0.8, 1.0, 0.4), (x - 0.6) / 0.4);
    }
}

// Purple fire gradient (dark magic)
fn fire_gradient_purple(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);

    if x < 0.3 {
        // Black to dark purple
        return mix(vec3<f32>(0.1, 0.0, 0.1), vec3<f32>(0.3, 0.0, 0.4), x / 0.3);
    } else if x < 0.6 {
        // Dark purple to magenta
        return mix(vec3<f32>(0.3, 0.0, 0.4), vec3<f32>(0.8, 0.2, 0.8), (x - 0.3) / 0.3);
    } else {
        // Magenta to pink-white
        return mix(vec3<f32>(0.8, 0.2, 0.8), vec3<f32>(1.0, 0.7, 1.0), (x - 0.6) / 0.4);
    }
}

// Ember/coal gradient (for particles and trails)
fn fire_gradient_ember(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    return vec3<f32>(
        x * 1.5,                    // Red
        x * x * 0.4,                // Orange tint
        0.0                         // No blue
    );
}

// ============================================================================
// Flame Shape Functions
// ============================================================================

// Basic teardrop/flame shape (point at top, round at bottom)
// Returns 1.0 inside flame, 0.0 outside, with smooth falloff
fn flame_shape_teardrop(uv: vec2<f32>) -> f32 {
    // Center UV so (0,0) is center of flame base
    let p = uv - vec2<f32>(0.5, 0.0);

    // Flame tapers toward top (y=1)
    let width = 0.4 * (1.0 - p.y * 0.8);

    // Horizontal distance from center
    let dx = abs(p.x) / width;

    // Vertical shape (0 at base, 1 at tip)
    let dy = p.y;

    // Combine for teardrop
    let d = dx * dx + dy;
    return 1.0 - smoothstep(0.0, 1.0, d);
}

// Pointed flame shape (sharper tip)
fn flame_shape_pointed(uv: vec2<f32>, sharpness: f32) -> f32 {
    let p = uv - vec2<f32>(0.5, 0.0);

    // Width decreases with height^sharpness
    let width = 0.35 * pow(1.0 - clamp(p.y, 0.0, 1.0), sharpness);

    let dx = abs(p.x);
    if dx > width {
        return 0.0;
    }

    let shape = 1.0 - (dx / width);
    let falloff = 1.0 - clamp(p.y, 0.0, 1.0);
    return shape * falloff;
}

// Circular fireball shape
fn flame_shape_ball(uv: vec2<f32>, radius: f32) -> f32 {
    let center = vec2<f32>(0.5, 0.5);
    let d = length(uv - center);
    return 1.0 - smoothstep(radius * 0.7, radius, d);
}

// Explosion shape (expanding ring with falloff)
fn flame_shape_explosion(uv: vec2<f32>, inner_radius: f32, outer_radius: f32) -> f32 {
    let center = vec2<f32>(0.5, 0.5);
    let d = length(uv - center);

    // Ring shape
    let ring = smoothstep(inner_radius, inner_radius + 0.1, d)
             * (1.0 - smoothstep(outer_radius - 0.1, outer_radius, d));

    return ring;
}

// ============================================================================
// Fire Animation Helpers
// ============================================================================

// Animate fire by distorting UV coordinates upward
fn fire_distort_uv(uv: vec2<f32>, noise_val: f32, time: f32, speed: f32) -> vec2<f32> {
    var result = uv;
    // Push UV upward over time (fire rises)
    result.y = result.y + time * speed;
    // Add horizontal wobble based on noise
    result.x = result.x + noise_val * 0.1;
    return result;
}

// Flicker intensity for fire brightness variation
fn fire_flicker(time: f32, base_intensity: f32, flicker_amount: f32) -> f32 {
    // Combine multiple frequencies for natural flicker
    let flicker = sin(time * 15.0) * 0.5
                + sin(time * 23.0) * 0.3
                + sin(time * 37.0) * 0.2;
    return base_intensity + flicker * flicker_amount;
}

// Smooth pulsing for magical effects
fn fire_pulse(time: f32, frequency: f32, min_val: f32, max_val: f32) -> f32 {
    let t = sin(time * frequency) * 0.5 + 0.5;
    return mix(min_val, max_val, t);
}

// ============================================================================
// Composite Fire Functions
// ============================================================================

// Generate animated fire intensity from UV and time
// Returns 0-1 intensity suitable for color gradient input
fn fire_intensity(uv: vec2<f32>, time: f32, shape_mask: f32) -> f32 {
    // Sample noise at different scales
    // Note: These would use fbm from noise.wgsl in actual usage
    // For now, use simple hash-based approximation
    let p = uv * 4.0 + vec2<f32>(0.0, -time * 2.0);
    let n1 = fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let n2 = fract(sin(dot(p * 2.0, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let noise = n1 * 0.7 + n2 * 0.3;

    // Combine noise with shape
    let intensity = noise * shape_mask;

    // Fade toward edges
    let edge_fade = 1.0 - smoothstep(0.3, 0.5, abs(uv.x - 0.5));

    return clamp(intensity * edge_fade, 0.0, 1.0);
}

// HDR-ready fire color with controllable emission strength
fn fire_color_hdr(base_color: vec3<f32>, emission_strength: f32) -> vec4<f32> {
    // HDR values can exceed 1.0 for bloom effects
    let emissive = base_color * emission_strength;
    return vec4<f32>(emissive, 1.0);
}

// Apply alpha based on fire intensity for transparency
fn fire_with_alpha(color: vec3<f32>, intensity: f32, alpha_power: f32) -> vec4<f32> {
    let alpha = pow(intensity, alpha_power);
    return vec4<f32>(color, alpha);
}

// ============================================================================
// Utility
// ============================================================================

// Convert temperature (0-1) to blackbody-approximate color
fn temperature_to_color(temp: f32) -> vec3<f32> {
    let t = clamp(temp, 0.0, 1.0);
    // Approximate blackbody radiation color
    let r = clamp(1.5 - abs(t - 0.5) * 2.0, 0.0, 1.0);
    let g = clamp(t * 2.0 - 0.5, 0.0, 1.0) * 0.8;
    let b = clamp(t - 0.7, 0.0, 1.0) * 2.0;
    return vec3<f32>(r, g, b);
}

// Radial gradient from center
fn radial_gradient(uv: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
    return 1.0 - clamp(length(uv - center) / radius, 0.0, 1.0);
}
