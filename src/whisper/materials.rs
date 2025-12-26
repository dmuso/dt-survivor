use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
        RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dKey},
};

/// Additive blending material for textured sprites (e.g., whisper glow).
/// Uses additive blend mode for fire/energy visual effects.
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct AdditiveTextureMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub color: LinearRgba,
}

impl Material2d for AdditiveTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/additive_textured.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        // Use Blend as base - specialize() overrides with additive blend state
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set additive blend state for glow effects
        if let Some(fragment) = &mut descriptor.fragment {
            if let Some(Some(target_state)) = fragment.targets.first_mut() {
                target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        Ok(())
    }
}

/// Additive blending material for solid-color sprites (e.g., lightning effects).
/// Uses additive blend mode for fire/energy visual effects.
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct AdditiveColorMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for AdditiveColorMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/additive_color.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = &mut descriptor.fragment {
            if let Some(Some(target_state)) = fragment.targets.first_mut() {
                target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_additive_texture_material_creation() {
        let material = AdditiveTextureMaterial {
            texture: Handle::default(),
            color: LinearRgba::new(1.0, 1.0, 1.0, 0.7),
        };
        assert_eq!(material.color.red, 1.0);
        assert_eq!(material.color.alpha, 0.7);
    }

    #[test]
    fn test_additive_color_material_creation() {
        let material = AdditiveColorMaterial {
            color: LinearRgba::new(3.0, 1.5, 1.0, 0.9),
        };
        // HDR values supported
        assert_eq!(material.color.red, 3.0);
        assert_eq!(material.color.green, 1.5);
        assert_eq!(material.color.alpha, 0.9);
    }

    #[test]
    fn test_additive_texture_material_clone() {
        let material = AdditiveTextureMaterial {
            texture: Handle::default(),
            color: LinearRgba::new(0.5, 0.5, 0.5, 1.0),
        };
        let cloned = material.clone();
        assert_eq!(cloned.color, material.color);
    }

    #[test]
    fn test_additive_color_material_clone() {
        let material = AdditiveColorMaterial {
            color: LinearRgba::new(2.0, 1.0, 0.5, 0.8),
        };
        let cloned = material.clone();
        assert_eq!(cloned.color, material.color);
    }
}
