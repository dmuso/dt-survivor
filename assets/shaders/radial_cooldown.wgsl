#import bevy_ui::ui_vertex_output::UiVertexOutput

struct RadialCooldownMaterial {
    // Progress from 0.0 (full overlay/on cooldown) to 1.0 (no overlay/ready)
    // Using Vec4 for 16-byte alignment (WebGL2 compatibility)
    progress: vec4<f32>,
    // Overlay color (semi-transparent black by default)
    overlay_color: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> material: RadialCooldownMaterial;

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let progress = material.progress.x;

    // Center UV coordinates (0.5, 0.5 becomes origin)
    let centered = in.uv - vec2<f32>(0.5, 0.5);

    // Calculate angle using atan2
    // atan2 returns angle in range [-PI, PI] with 0 pointing right (+x)
    // We need to start from 6 o'clock (bottom, +y direction) and go clockwise
    // Rotate coordinate system: use -x for clockwise, +y for starting at bottom
    let angle = atan2(-centered.x, centered.y);

    // Normalize angle from [-PI, PI] to [0, 1]
    let normalized_angle = (angle / TAU) + 0.5;

    // Show overlay for the portion NOT yet revealed by progress
    // progress=0.0 -> show full overlay (spell on cooldown)
    // progress=1.0 -> show no overlay (spell ready)
    if normalized_angle > progress {
        // Show semi-transparent overlay
        return material.overlay_color;
    } else {
        // Transparent - show the spell icon underneath
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}
