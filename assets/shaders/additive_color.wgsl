// Additive blending shader for solid-color sprites (e.g., lightning effects)
// Used with AdditiveColorMaterial for fire/energy visual effects

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    return color;
}
