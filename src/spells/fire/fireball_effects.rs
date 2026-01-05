use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_hanabi::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;

/// Resource containing particle effect handles for the enhanced Fireball spell
#[derive(Resource)]
pub struct FireballEffects {
    /// Swirling charge-up particles during the 0.5s charge phase
    pub charge_effect: Handle<EffectAsset>,
    /// Circle texture for charge particles (renders them as circles instead of squares)
    pub charge_texture: Handle<Image>,
    /// Comet tail trailing particles during flight
    pub trail_effect: Handle<EffectAsset>,
    /// Sparks flying off during flight
    pub spark_effect: Handle<EffectAsset>,
    /// White-hot core flash - blindingly bright initial burst
    pub explosion_core_effect: Handle<EffectAsset>,
    /// Main fire burst - the big orange-red explosion
    pub explosion_fire_effect: Handle<EffectAsset>,
    /// Flying ember sparks - fast debris flying outward
    pub explosion_embers_effect: Handle<EffectAsset>,
    /// Rising smoke - dark smoke plume after the fire
    pub explosion_smoke_effect: Handle<EffectAsset>,
}

/// Fireball mesh radius - particles should match this scale
pub const FIREBALL_RADIUS: f32 = 0.3;

/// Size of the procedural circle texture (pixels)
const CIRCLE_TEXTURE_SIZE: u32 = 64;

/// Creates a procedural circle texture for particles.
/// The texture is R8Unorm format where R represents opacity.
/// A soft circular falloff makes particles appear round instead of square.
pub fn create_circle_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let size = CIRCLE_TEXTURE_SIZE;
    let center = size as f32 / 2.0;
    let radius = center - 1.0; // Leave 1px margin for smooth edges

    let mut data = vec![0u8; (size * size) as usize];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center + 0.5;
            let dy = y as f32 - center + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            // Smooth falloff from center to edge
            let normalized_dist = dist / radius;
            let alpha = if normalized_dist > 1.0 {
                0.0
            } else {
                // Smooth cubic falloff for soft edges
                let t = 1.0 - normalized_dist;
                t * t * (3.0 - 2.0 * t)
            };

            let idx = (y * size + x) as usize;
            data[idx] = (alpha * 255.0) as u8;
        }
    }

    let image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        default(),
    );

    images.add(image)
}

/// Creates the swirling charge effect - particles spiral inward toward fireball center
/// Particles spawn in a sphere, start slow, and accelerate dramatically toward center
pub fn create_charge_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 0.6, 0.1, 1.0)); // Bright orange
    color_gradient.add_key(0.4, Vec4::new(1.0, 0.9, 0.3, 1.0)); // Yellow
    color_gradient.add_key(0.8, Vec4::new(1.0, 1.0, 0.8, 0.9)); // White-hot
    color_gradient.add_key(1.0, Vec4::new(1.0, 1.0, 1.0, 0.0)); // Fade out

    // Small particles that shrink as they approach
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(0.1)); // Small
    size_gradient.add_key(0.7, Vec3::splat(0.06)); // Shrink
    size_gradient.add_key(1.0, Vec3::splat(0.02)); // Tiny at end

    let writer = ExprWriter::new();

    let lifetime = writer.lit(0.35).expr(); // Shorter lifetime for faster particles
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn in a sphere around local center (effect position)
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(2.5).expr(), // Spawn 2.5 units out
        dimension: ShapeDimension::Surface,
    };

    // Fast inward velocity toward local center (3x faster)
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(-4.5).expr(), // 3x faster initial speed
    };

    // Strong radial acceleration toward local center (3x stronger)
    let radial_accel = RadialAccelModifier::new(
        writer.lit(Vec3::ZERO).expr(), // Local center point
        writer.lit(-75.0).expr(), // 3x stronger inward acceleration
    );

    // Texture slot index for circular particle texture (index 0 in EffectMaterial.images)
    let texture_slot = writer.lit(0u32).expr();

    // Finish the module and register the texture slot for GPU shaders
    let mut module = writer.finish();
    module.add_texture_slot("circle");

    let effect = EffectAsset::new(
        1024, // More particles in pool
        SpawnerSettings::rate(100.0.into()), // High spawn rate
        module,
    )
    .with_name("fireball_charge")
    .with_simulation_space(SimulationSpace::Local) // Particles move relative to effect position
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(radial_accel) // Apply acceleration each frame
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    })
    .render(ParticleTextureModifier {
        texture_slot,
        sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
    });

    effects.add(effect)
}

/// Creates the comet trail effect - particles trail behind in world space
/// Sized to match the fireball width
pub fn create_trail_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 0.6, 0.1, 1.0)); // Bright orange
    color_gradient.add_key(0.3, Vec4::new(1.0, 0.3, 0.0, 0.9)); // Orange-red
    color_gradient.add_key(0.6, Vec4::new(0.8, 0.1, 0.0, 0.6)); // Dark red
    color_gradient.add_key(1.0, Vec4::new(0.2, 0.1, 0.05, 0.0)); // Smoke fade out

    // Trail particles match fireball width, shrink as they age
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(FIREBALL_RADIUS * 1.2)); // 0.36 - slightly larger
    size_gradient.add_key(0.5, Vec3::splat(FIREBALL_RADIUS * 0.8)); // 0.24
    size_gradient.add_key(1.0, Vec3::splat(FIREBALL_RADIUS * 0.2)); // 0.06

    let writer = ExprWriter::new();

    let lifetime = writer.lit(0.4).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn within the fireball volume
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(FIREBALL_RADIUS * 0.5).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Slight backward drift
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(-0.5).expr(),
    };

    let drag = LinearDragModifier::new(writer.lit(3.0).expr());

    let effect = EffectAsset::new(
        512,
        SpawnerSettings::rate(60.0.into()),
        writer.finish(),
    )
    .with_name("fireball_trail")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(drag)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    });

    effects.add(effect)
}

/// Creates the spark effect - quick bright sparks flying off
pub fn create_spark_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 0.8, 1.0)); // Bright yellow-white
    color_gradient.add_key(0.5, Vec4::new(1.0, 0.7, 0.2, 0.8)); // Orange
    color_gradient.add_key(1.0, Vec4::new(0.8, 0.3, 0.0, 0.0)); // Fade

    // Sparks are smaller than the main trail
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(FIREBALL_RADIUS * 0.3)); // 0.09
    size_gradient.add_key(1.0, Vec3::splat(FIREBALL_RADIUS * 0.1)); // 0.03

    let writer = ExprWriter::new();

    let lifetime = writer.lit(0.25).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn on fireball surface
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(FIREBALL_RADIUS).expr(),
        dimension: ShapeDimension::Surface,
    };

    // Fast outward burst
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(3.0).expr(),
    };

    // Gravity
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -4.0, 0.0)).expr());

    let effect = EffectAsset::new(
        128,
        SpawnerSettings::rate(12.0.into()),
        writer.finish(),
    )
    .with_name("fireball_sparks")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(gravity)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    });

    effects.add(effect)
}

// ============================================================================
// EXPLOSION EFFECTS - Multi-layer fire eruption system
// ============================================================================

/// Explosion effect radius - should feel massive compared to the fireball
pub const EXPLOSION_RADIUS: f32 = 2.5;

/// Creates the white-hot core flash - blindingly bright instant burst
/// This is the "flash" you see at the moment of impact
pub fn create_explosion_core_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    // Blinding white-yellow at start
    color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 1.0, 1.0));
    color_gradient.add_key(0.1, Vec4::new(1.0, 1.0, 0.8, 1.0));
    color_gradient.add_key(0.3, Vec4::new(1.0, 0.95, 0.5, 0.8));
    color_gradient.add_key(0.5, Vec4::new(1.0, 0.8, 0.2, 0.4));
    color_gradient.add_key(1.0, Vec4::new(1.0, 0.6, 0.0, 0.0));

    // Core expands rapidly then fades
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(EXPLOSION_RADIUS * 0.3));
    size_gradient.add_key(0.15, Vec3::splat(EXPLOSION_RADIUS * 1.5)); // Rapid expansion
    size_gradient.add_key(0.4, Vec3::splat(EXPLOSION_RADIUS * 2.0)); // Peak
    size_gradient.add_key(1.0, Vec3::splat(EXPLOSION_RADIUS * 0.5)); // Shrink as fading

    let writer = ExprWriter::new();
    let lifetime = writer.lit(0.25).expr(); // Quick flash
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Very tight spawn point
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.1).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Minimal movement - core just expands in place
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(1.0).expr(),
    };

    let drag = LinearDragModifier::new(writer.lit(8.0).expr());

    let effect = EffectAsset::new(
        64,
        SpawnerSettings::burst(20.0.into(), 1.0.into()),
        writer.finish(),
    )
    .with_name("fireball_explosion_core")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(drag)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    });

    effects.add(effect)
}

/// Creates the main fire burst - the big angry orange-red explosion
/// This is the "meat" of the explosion that screams DAMAGE
pub fn create_explosion_fire_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    // Starts bright orange-yellow, transitions through angry reds
    color_gradient.add_key(0.0, Vec4::new(1.0, 0.9, 0.3, 1.0)); // Bright yellow-orange
    color_gradient.add_key(0.15, Vec4::new(1.0, 0.6, 0.1, 1.0)); // Orange
    color_gradient.add_key(0.35, Vec4::new(1.0, 0.35, 0.0, 0.95)); // Deep orange-red
    color_gradient.add_key(0.55, Vec4::new(0.9, 0.15, 0.0, 0.8)); // Angry red
    color_gradient.add_key(0.75, Vec4::new(0.5, 0.05, 0.0, 0.5)); // Dark crimson
    color_gradient.add_key(1.0, Vec4::new(0.15, 0.02, 0.0, 0.0)); // Fade to black

    // BIG expanding particles
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(EXPLOSION_RADIUS * 0.4));
    size_gradient.add_key(0.2, Vec3::splat(EXPLOSION_RADIUS * 1.2));
    size_gradient.add_key(0.5, Vec3::splat(EXPLOSION_RADIUS * 1.8)); // Massive at peak
    size_gradient.add_key(0.8, Vec3::splat(EXPLOSION_RADIUS * 1.4));
    size_gradient.add_key(1.0, Vec3::splat(EXPLOSION_RADIUS * 0.8));

    let writer = ExprWriter::new();
    let lifetime = writer.lit(0.6).expr(); // Longer than core for sustained fire
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn from fireball-sized cluster
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(FIREBALL_RADIUS).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Expand outward with force
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(6.0).expr(),
    };

    let drag = LinearDragModifier::new(writer.lit(4.0).expr());
    let heat_rise = AccelModifier::new(writer.lit(Vec3::new(0.0, 2.0, 0.0)).expr());

    let effect = EffectAsset::new(
        512,
        SpawnerSettings::burst(150.0.into(), 1.0.into()), // LOTS of particles
        writer.finish(),
    )
    .with_name("fireball_explosion_fire")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(drag)
    .update(heat_rise)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    });

    effects.add(effect)
}

/// Creates the flying ember sparks - fast debris shooting outward
/// These sell the violence of the impact
pub fn create_explosion_embers_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    // Bright orange-yellow sparks that cool down
    color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 0.6, 1.0)); // Bright yellow
    color_gradient.add_key(0.2, Vec4::new(1.0, 0.7, 0.2, 1.0)); // Orange
    color_gradient.add_key(0.5, Vec4::new(1.0, 0.4, 0.0, 0.9)); // Deep orange
    color_gradient.add_key(0.8, Vec4::new(0.7, 0.2, 0.0, 0.5)); // Red ember
    color_gradient.add_key(1.0, Vec4::new(0.3, 0.05, 0.0, 0.0)); // Dark fade

    // Small, fast sparks
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(0.15));
    size_gradient.add_key(0.3, Vec3::splat(0.12));
    size_gradient.add_key(0.7, Vec3::splat(0.08));
    size_gradient.add_key(1.0, Vec3::splat(0.03));

    let writer = ExprWriter::new();
    let lifetime = writer.lit(0.8).expr(); // Longer lifetime for travel
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn on the fireball surface
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(FIREBALL_RADIUS * 0.5).expr(),
        dimension: ShapeDimension::Surface,
    };

    // FAST outward burst - these fly!
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(15.0).expr(), // Fast!
    };

    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -8.0, 0.0)).expr());
    let drag = LinearDragModifier::new(writer.lit(2.0).expr());

    let effect = EffectAsset::new(
        256,
        SpawnerSettings::burst(80.0.into(), 1.0.into()),
        writer.finish(),
    )
    .with_name("fireball_explosion_embers")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(gravity)
    .update(drag)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    });

    effects.add(effect)
}

/// Creates the rising smoke plume - dark smoke that rises after the fire
/// Adds aftermath and weight to the explosion
pub fn create_explosion_smoke_effect(effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    let mut color_gradient = HanabiGradient::<Vec4>::new();
    // Dark gray smoke with some ember glow
    color_gradient.add_key(0.0, Vec4::new(0.3, 0.15, 0.05, 0.0)); // Invisible at start
    color_gradient.add_key(0.1, Vec4::new(0.25, 0.12, 0.05, 0.4)); // Fade in
    color_gradient.add_key(0.3, Vec4::new(0.2, 0.1, 0.08, 0.5)); // Dark smoke
    color_gradient.add_key(0.6, Vec4::new(0.15, 0.12, 0.1, 0.4)); // Lighter gray
    color_gradient.add_key(1.0, Vec4::new(0.1, 0.1, 0.1, 0.0)); // Fade out

    // Smoke billows and expands as it rises
    let mut size_gradient = HanabiGradient::<Vec3>::new();
    size_gradient.add_key(0.0, Vec3::splat(EXPLOSION_RADIUS * 0.3));
    size_gradient.add_key(0.3, Vec3::splat(EXPLOSION_RADIUS * 0.8));
    size_gradient.add_key(0.7, Vec3::splat(EXPLOSION_RADIUS * 1.2));
    size_gradient.add_key(1.0, Vec3::splat(EXPLOSION_RADIUS * 1.5)); // Expands as it dissipates

    let writer = ExprWriter::new();
    let lifetime = writer.lit(1.2).expr(); // Long-lasting smoke
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn from explosion center with slight delay effect
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(EXPLOSION_RADIUS * 0.5).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Slow initial velocity
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(2.0).expr(),
    };

    // Strong upward acceleration - heat rises!
    let heat_rise = AccelModifier::new(writer.lit(Vec3::new(0.0, 4.0, 0.0)).expr());
    let drag = LinearDragModifier::new(writer.lit(1.5).expr());

    let effect = EffectAsset::new(
        128,
        SpawnerSettings::burst(40.0.into(), 1.0.into()),
        writer.finish(),
    )
    .with_name("fireball_explosion_smoke")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_lifetime)
    .init(init_pos)
    .init(init_vel)
    .update(heat_rise)
    .update(drag)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    });

    effects.add(effect)
}

/// Initialize the FireballEffects resource
/// Uses Option to handle tests that don't have the HanabiPlugin
pub fn init_fireball_effects(
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
    images: Option<ResMut<Assets<Image>>>,
) {
    if let (Some(mut effects), Some(mut images)) = (effects, images) {
        let fireball_effects = FireballEffects {
            charge_effect: create_charge_effect(&mut effects),
            charge_texture: create_circle_texture(&mut images),
            trail_effect: create_trail_effect(&mut effects),
            spark_effect: create_spark_effect(&mut effects),
            // Multi-layer explosion system for massive impact
            explosion_core_effect: create_explosion_core_effect(&mut effects),
            explosion_fire_effect: create_explosion_fire_effect(&mut effects),
            explosion_embers_effect: create_explosion_embers_effect(&mut effects),
            explosion_smoke_effect: create_explosion_smoke_effect(&mut effects),
        };
        commands.insert_resource(fireball_effects);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_charge_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_charge_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_trail_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_trail_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_spark_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_spark_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_explosion_core_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_explosion_core_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_explosion_fire_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_explosion_fire_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_explosion_embers_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_explosion_embers_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_explosion_smoke_effect() {
        let mut effects = Assets::<EffectAsset>::default();
        let handle = create_explosion_smoke_effect(&mut effects);
        assert!(effects.get(&handle).is_some());
    }

    #[test]
    fn test_create_circle_texture() {
        let mut images = Assets::<Image>::default();
        let handle = create_circle_texture(&mut images);
        let image = images.get(&handle).expect("Image should be created");

        // Verify dimensions
        assert_eq!(image.width(), CIRCLE_TEXTURE_SIZE);
        assert_eq!(image.height(), CIRCLE_TEXTURE_SIZE);

        // Verify format is R8Unorm (single channel)
        assert_eq!(image.texture_descriptor.format, TextureFormat::R8Unorm);

        let data = image.data.as_ref().expect("Image should have data");

        // Verify center pixel is fully opaque (or nearly so)
        let center = (CIRCLE_TEXTURE_SIZE / 2) as usize;
        let center_idx = center * CIRCLE_TEXTURE_SIZE as usize + center;
        assert!(data[center_idx] > 200, "Center should be bright");

        // Verify corner pixel is transparent
        assert_eq!(data[0], 0, "Corner should be transparent");
    }

    #[test]
    fn test_circle_texture_circular_falloff() {
        let mut images = Assets::<Image>::default();
        let handle = create_circle_texture(&mut images);
        let image = images.get(&handle).unwrap();
        let data = image.data.as_ref().unwrap();

        let size = CIRCLE_TEXTURE_SIZE as usize;
        let center = size / 2;

        // Sample at center and at various radii
        let center_val = data[center * size + center];

        // Sample at half radius (should still be bright)
        let half_rad = center / 2;
        let half_rad_val = data[(center + half_rad) * size + center];

        // Sample near edge (should be dimmer)
        let near_edge = center - 2;
        let edge_val = data[(center + near_edge) * size + center];

        // Verify falloff: center > half_radius > edge
        assert!(center_val >= half_rad_val, "Center should be >= half radius");
        assert!(half_rad_val >= edge_val, "Half radius should be >= edge");
    }
}
