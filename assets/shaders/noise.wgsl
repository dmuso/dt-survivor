// Noise function library for procedural effects
// Includes hash functions, Perlin-style gradient noise, Simplex noise, and FBM

// ============================================================================
// Constants
// ============================================================================

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

// ============================================================================
// Hash Functions (pseudo-random number generation)
// ============================================================================

// Hash function for 1D input, returns value in [0, 1]
fn hash11(p: f32) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3 + 33.33);
    return fract(p3 * (p3 + p3));
}

// Hash function for 2D input, returns value in [0, 1]
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Hash function for 2D input, returns vec2 in [0, 1]
fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

// Hash function for 3D input, returns value in [0, 1]
fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

// Hash function for 3D input, returns vec3 in [0, 1]
fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// ============================================================================
// Gradient Noise (Perlin-style)
// ============================================================================

// 2D gradient function - returns a pseudo-random unit gradient
fn gradient2d(p: vec2<f32>) -> vec2<f32> {
    let angle = hash12(p) * TAU;
    return vec2<f32>(cos(angle), sin(angle));
}

// 2D Perlin-style gradient noise, returns value in approximately [-1, 1]
fn perlin2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Quintic interpolation curve (smoother than cubic)
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    // Four corner gradients
    let g00 = gradient2d(i + vec2<f32>(0.0, 0.0));
    let g10 = gradient2d(i + vec2<f32>(1.0, 0.0));
    let g01 = gradient2d(i + vec2<f32>(0.0, 1.0));
    let g11 = gradient2d(i + vec2<f32>(1.0, 1.0));

    // Distance vectors to corners
    let d00 = f - vec2<f32>(0.0, 0.0);
    let d10 = f - vec2<f32>(1.0, 0.0);
    let d01 = f - vec2<f32>(0.0, 1.0);
    let d11 = f - vec2<f32>(1.0, 1.0);

    // Dot products
    let v00 = dot(g00, d00);
    let v10 = dot(g10, d10);
    let v01 = dot(g01, d01);
    let v11 = dot(g11, d11);

    // Bilinear interpolation
    let x0 = mix(v00, v10, u.x);
    let x1 = mix(v01, v11, u.x);
    return mix(x0, x1, u.y);
}

// ============================================================================
// Simplex-like Noise (2D)
// ============================================================================

// 2D simplex-like noise, returns value in approximately [-1, 1]
fn simplex2d(p: vec2<f32>) -> f32 {
    // Skew constants for 2D
    let F2 = 0.5 * (sqrt(3.0) - 1.0);  // 0.366025
    let G2 = (3.0 - sqrt(3.0)) / 6.0;  // 0.211325

    // Skew input to simplex grid
    let s = (p.x + p.y) * F2;
    let i = floor(p + s);

    // Unskew back
    let t = (i.x + i.y) * G2;
    let x0 = p - (i - t);

    // Determine which simplex
    var i1: vec2<f32>;
    if x0.x > x0.y {
        i1 = vec2<f32>(1.0, 0.0);
    } else {
        i1 = vec2<f32>(0.0, 1.0);
    }

    // Offsets for corners
    let x1 = x0 - i1 + G2;
    let x2 = x0 - 1.0 + 2.0 * G2;

    // Gradients
    let g0 = gradient2d(i);
    let g1 = gradient2d(i + i1);
    let g2 = gradient2d(i + vec2<f32>(1.0, 1.0));

    // Contributions from each corner
    var n0 = 0.0;
    var n1 = 0.0;
    var n2 = 0.0;

    var t0 = 0.5 - dot(x0, x0);
    if t0 >= 0.0 {
        t0 = t0 * t0;
        n0 = t0 * t0 * dot(g0, x0);
    }

    var t1 = 0.5 - dot(x1, x1);
    if t1 >= 0.0 {
        t1 = t1 * t1;
        n1 = t1 * t1 * dot(g1, x1);
    }

    var t2 = 0.5 - dot(x2, x2);
    if t2 >= 0.0 {
        t2 = t2 * t2;
        n2 = t2 * t2 * dot(g2, x2);
    }

    // Scale to approximately [-1, 1]
    return 70.0 * (n0 + n1 + n2);
}

// ============================================================================
// Value Noise
// ============================================================================

// Simple 2D value noise using bilinear interpolation
fn value_noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Smoothstep interpolation
    let u = f * f * (3.0 - 2.0 * f);

    // Four corner values
    let a = hash12(i + vec2<f32>(0.0, 0.0));
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));

    // Bilinear interpolation
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// ============================================================================
// Fractal Brownian Motion (FBM)
// ============================================================================

// FBM using Perlin noise with configurable octaves
fn fbm_perlin(p: vec2<f32>, octaves: i32, lacunarity: f32, gain: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < octaves; i = i + 1) {
        value = value + amplitude * perlin2d(pos * frequency);
        frequency = frequency * lacunarity;
        amplitude = amplitude * gain;
    }

    return value;
}

// FBM using simplex noise
fn fbm_simplex(p: vec2<f32>, octaves: i32, lacunarity: f32, gain: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < octaves; i = i + 1) {
        value = value + amplitude * simplex2d(pos * frequency);
        frequency = frequency * lacunarity;
        amplitude = amplitude * gain;
    }

    return value;
}

// FBM using value noise
fn fbm_value(p: vec2<f32>, octaves: i32, lacunarity: f32, gain: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < octaves; i = i + 1) {
        value = value + amplitude * value_noise2d(pos * frequency);
        frequency = frequency * lacunarity;
        amplitude = amplitude * gain;
    }

    return value;
}

// Standard FBM preset: 5 octaves, lacunarity 2.0, gain 0.5
fn fbm(p: vec2<f32>) -> f32 {
    return fbm_perlin(p, 5, 2.0, 0.5);
}

// Turbulence (absolute value FBM for sharper features)
fn turbulence(p: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < octaves; i = i + 1) {
        value = value + amplitude * abs(perlin2d(pos * frequency));
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// ============================================================================
// Utility Functions
// ============================================================================

// Remap value from one range to another
fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    return to_min + (value - from_min) * (to_max - to_min) / (from_max - from_min);
}

// Remap noise from [-1, 1] to [0, 1]
fn noise_to_01(n: f32) -> f32 {
    return n * 0.5 + 0.5;
}

// Smooth minimum (for soft blending)
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

// Smooth maximum
fn smax(a: f32, b: f32, k: f32) -> f32 {
    return -smin(-a, -b, k);
}
