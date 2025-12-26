// Additive blending shader for textured sprites (e.g., whisper glow)
// Used with AdditiveTextureMaterial for fire/energy visual effects
// Blend state: src * SrcAlpha + dst * One (additive)

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var base_texture: texture_2d<f32>;
@group(2) @binding(1) var base_sampler: sampler;
@group(2) @binding(2) var<uniform> color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(base_texture, base_sampler, mesh.uv);

    // Apply color tint and uniform alpha
    // With additive blending, black pixels add nothing regardless of alpha
    return tex * color;
}
