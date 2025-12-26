use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;
use bevy_hanabi::prelude::{
    Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier, EffectAsset, ExprWriter,
    Gradient as HanabiGradient, ParticleEffect, SetAttributeModifier, SetPositionCircleModifier,
    SetVelocitySphereModifier, ShapeDimension, SizeOverLifetimeModifier, SpawnerSettings,
};
use bevy_lit::prelude::*;
use rand::Rng;

use crate::player::components::Player;
use crate::whisper::components::{
    ArcBurstTimer, LightningBolt, LightningSegment, LightningSpawnTimer, OrbitalParticle,
    OrbitalParticleSpawnTimer, ParticleTrail, TrailSegment, WhisperArc, WhisperCompanion,
    WhisperDrop, WhisperOuterGlow,
};
use crate::whisper::events::*;
use crate::whisper::materials::{AdditiveColorMaterial, AdditiveTextureMaterial};
use crate::whisper::resources::*;

/// Color constants for Whisper visual effects (red mode)
const WHISPER_LIGHT_COLOR: Color = Color::srgb(1.0, 0.3, 0.2); // Red-orange
const WHISPER_LIGHT_INTENSITY: f32 = 5.0;
const WHISPER_LIGHT_OUTER_RADIUS: f32 = 140.0; // 50% of original 280
const WHISPER_LIGHT_FALLOFF: f32 = 2.0;

/// Particle effect constants (50% of original values)
const SPARK_SPAWN_RATE: f32 = 120.0; // particles per second (unchanged)
const SPARK_LIFETIME: f32 = 0.35; // seconds (unchanged)
const SPARK_SPEED: f32 = 90.0; // 50% of original 180
const SPARK_SIZE_START: f32 = 2.0; // 50% of original 4.0
const SPARK_SIZE_END: f32 = 0.0; // pixels

/// Whisper base texture size (50% of original 128)
const WHISPER_TEXTURE_SIZE: f32 = 64.0;

/// Lightning bolt visual constants
const LIGHTNING_BOLTS_PER_SPAWN: u32 = 3;
/// Minimum bolt size as fraction of max (0.2 = 20%)
const BOLT_MIN_SIZE_FRACTION: f32 = 0.2;

/// Creates and inserts the Whisper spark particle effect asset.
/// Should be called once on startup. Silently skips if HanabiPlugin is not loaded.
pub fn setup_whisper_particle_effect(
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
) {
    let Some(mut effects) = effects else {
        return; // HanabiPlugin not loaded, skip particle setup
    };
    // Create a gradient for particle color (red-orange to transparent)
    let mut color_gradient = HanabiGradient::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 0.5, 0.3, 1.0)); // Bright red-orange
    color_gradient.add_key(0.5, Vec4::new(1.0, 0.7, 0.5, 0.6)); // Lighter orange
    color_gradient.add_key(1.0, Vec4::new(1.0, 0.9, 0.8, 0.0)); // Fade to transparent

    // Create a gradient for particle size (shrinks over lifetime)
    let mut size_gradient = HanabiGradient::new();
    size_gradient.add_key(0.0, Vec3::splat(SPARK_SIZE_START));
    size_gradient.add_key(1.0, Vec3::splat(SPARK_SIZE_END));

    let writer = ExprWriter::new();

    // Position: spawn at center (2D, so use Z axis for the circle plane)
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Z).expr(), // Circle in XY plane
        radius: writer.lit(4.0).expr(),   // 50% of original 8.0
        dimension: ShapeDimension::Surface,
    };

    // Velocity: outward from center
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(SPARK_SPEED).expr(),
    };

    // Lifetime
    let lifetime = writer.lit(SPARK_LIFETIME).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    let module = writer.finish();

    // Create the effect with SpawnerSettings
    let spawner = SpawnerSettings::rate(SPARK_SPAWN_RATE.into());
    let effect = EffectAsset::new(1024, spawner, module)
        .with_name("whisper_sparks")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier {
            gradient: color_gradient,
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        });

    let effect_handle = effects.add(effect);
    commands.insert_resource(WhisperSparkEffect(effect_handle));
}

/// Spawns Whisper drop within 1000px of the player spawn position (origin).
/// Uses polar coordinates to ensure uniform distribution in a ring around the player.
/// Runs on OnEnter(GameState::InGame)
pub fn spawn_whisper_drop(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut additive_materials: ResMut<Assets<AdditiveTextureMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // Generate random position within 200px of player spawn (origin)
    // Using polar coordinates for uniform distribution
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let distance = rng.gen_range(50.0..200.0);
    let x = angle.cos() * distance;
    let y = angle.sin() * distance;
    let position = Vec3::new(x, y, 0.5);

    // Load the red-mode texture (or use default if no asset server)
    let texture: Handle<Image> = asset_server
        .as_ref()
        .map(|s| s.load("whisper/red-mode.png"))
        .unwrap_or_default();

    // Create mesh and material for additive blending (50% size)
    let mesh = meshes.add(Rectangle::new(WHISPER_TEXTURE_SIZE, WHISPER_TEXTURE_SIZE));
    let material = additive_materials.add(AdditiveTextureMaterial {
        texture: texture.clone(),
        color: LinearRgba::new(1.0, 1.0, 1.0, 0.7),
    });

    // Spawn the Whisper drop with visual elements
    commands
        .spawn((
            WhisperDrop::default(),
            LightningSpawnTimer::default(),
            OrbitalParticleSpawnTimer::default(),
            Transform::from_translation(position),
            Visibility::default(),
            // Add PointLight2d for 2D lighting effect
            PointLight2d {
                color: WHISPER_LIGHT_COLOR,
                intensity: WHISPER_LIGHT_INTENSITY * 0.5, // Dimmer when not collected
                outer_radius: WHISPER_LIGHT_OUTER_RADIUS * 0.5,
                falloff: WHISPER_LIGHT_FALLOFF,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Base glow using red-mode.png texture with additive blending
            parent.spawn((
                WhisperOuterGlow,
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::from_xyz(0.0, 0.0, -0.1),
            ));
        });
}

/// Detects when player is close enough to collect Whisper.
/// Runs in GameSet::Combat
pub fn detect_whisper_pickup(
    player_query: Query<(Entity, &Transform), With<Player>>,
    whisper_query: Query<(Entity, &Transform, &WhisperDrop)>,
    mut whisper_events: MessageWriter<WhisperCollectedEvent>,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();

    for (whisper_entity, whisper_transform, whisper_drop) in whisper_query.iter() {
        let whisper_pos = whisper_transform.translation.truncate();
        let distance = player_pos.distance(whisper_pos);

        if distance <= whisper_drop.pickup_radius {
            whisper_events.write(WhisperCollectedEvent {
                player_entity,
                whisper_drop_entity: whisper_entity,
                position: whisper_pos,
            });
        }
    }
}

/// Handles WhisperCollectedEvent - transforms drop into companion.
/// Runs in GameSet::Effects
#[allow(clippy::too_many_arguments)]
pub fn handle_whisper_collection(
    mut commands: Commands,
    mut whisper_events: MessageReader<WhisperCollectedEvent>,
    mut whisper_state: ResMut<WhisperState>,
    mut inventory: ResMut<crate::inventory::resources::Inventory>,
    player_query: Query<&Transform, With<Player>>,
    weapon_query: Query<Entity, With<crate::weapon::components::Weapon>>,
    spark_effect: Option<Res<WhisperSparkEffect>>,
    asset_server: Option<Res<AssetServer>>,
    mut audio_channel: Option<ResMut<bevy_kira_audio::prelude::AudioChannel<crate::audio::plugin::LootSoundChannel>>>,
    mut sound_limiter: Option<ResMut<crate::audio::plugin::SoundLimiter>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut additive_materials: ResMut<Assets<AdditiveTextureMaterial>>,
) {
    use crate::inventory::components::EquippedWeapon;
    use crate::weapon::components::{Weapon, WeaponType};

    for event in whisper_events.read() {
        // Skip if already collected (prevents double-processing)
        if whisper_state.collected {
            continue;
        }

        // Despawn the WhisperDrop entity and its children
        commands.entity(event.whisper_drop_entity).despawn();

        // Get player position for spawning companion
        let player_pos = player_query
            .get(event.player_entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        // Spawn WhisperCompanion at player position with offset
        let companion = WhisperCompanion::default();
        let companion_pos = player_pos + companion.follow_offset;

        // Load the red-mode texture for companion
        let texture: Handle<Image> = asset_server
            .as_ref()
            .map(|s| s.load("whisper/red-mode.png"))
            .unwrap_or_default();

        // Create mesh and material for additive blending (50% size)
        let mesh = meshes.add(Rectangle::new(WHISPER_TEXTURE_SIZE, WHISPER_TEXTURE_SIZE));
        let material = additive_materials.add(AdditiveTextureMaterial {
            texture: texture.clone(),
            color: LinearRgba::new(1.0, 1.0, 1.0, 0.9),
        });

        let mut companion_entity = commands.spawn((
            companion,
            ArcBurstTimer::default(),
            LightningSpawnTimer::default(),
            OrbitalParticleSpawnTimer::default(),
            Transform::from_translation(companion_pos),
            Visibility::default(),
            // Full brightness PointLight2d when collected
            PointLight2d {
                color: WHISPER_LIGHT_COLOR,
                intensity: WHISPER_LIGHT_INTENSITY,
                outer_radius: WHISPER_LIGHT_OUTER_RADIUS,
                falloff: WHISPER_LIGHT_FALLOFF,
                ..default()
            },
        ));

        // Add particle effect if available
        if let Some(effect) = spark_effect.as_ref() {
            companion_entity.insert(ParticleEffect::new(effect.0.clone()));
        }

        companion_entity.with_children(|parent| {
            // Base glow using red-mode.png texture with additive blending
            parent.spawn((
                WhisperOuterGlow,
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::from_xyz(0.0, 0.0, -0.1),
            ));
        });

        // Mark as collected
        whisper_state.collected = true;

        // Add default pistol to inventory
        let pistol = Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
            level: 1,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: -2.0, // Prevent immediate firing
        };
        inventory.add_or_level_weapon(pistol.clone());

        // Recreate weapon entities
        let weapon_entities: Vec<Entity> = weapon_query.iter().collect();
        for entity in weapon_entities {
            commands.entity(entity).despawn();
        }

        // Create new weapon entities for all weapons in inventory
        for (_weapon_id, weapon) in inventory.iter_weapons() {
            commands.spawn((
                weapon.clone(),
                EquippedWeapon {
                    weapon_type: weapon.weapon_type.clone(),
                },
                Transform::from_translation(player_pos),
            ));
        }

        // Play collection sound
        if let (Some(asset_server), Some(audio_channel), Some(sound_limiter)) =
            (asset_server.as_ref(), audio_channel.as_mut(), sound_limiter.as_mut())
        {
            crate::audio::plugin::play_limited_sound(
                audio_channel.as_mut(),
                asset_server,
                "sounds/366104__original_sound__confirmation-downward.wav",
                sound_limiter.as_mut(),
            );
        }
    }
}

/// Resets whisper state when entering game.
/// Runs on OnEnter(GameState::InGame)
pub fn reset_whisper_state(
    mut whisper_state: ResMut<WhisperState>,
    mut weapon_origin: ResMut<WeaponOrigin>,
) {
    whisper_state.collected = false;
    weapon_origin.position = None;
}

/// Makes Whisper companion follow the player with bobbing motion.
/// Runs in GameSet::Movement
pub fn whisper_follow_player(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut whisper_query: Query<(&mut Transform, &mut WhisperCompanion), Without<Player>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (mut whisper_transform, mut companion) in whisper_query.iter_mut() {
        // Update bobbing phase
        companion.bob_phase += time.delta_secs() * 3.0; // 3 Hz bobbing frequency

        // Calculate bobbing offset
        let bob_offset = companion.bob_amplitude * companion.bob_phase.sin();

        // Position Whisper at player + follow_offset + bob offset
        whisper_transform.translation.x = player_transform.translation.x + companion.follow_offset.x;
        whisper_transform.translation.y =
            player_transform.translation.y + companion.follow_offset.y + bob_offset;
        whisper_transform.translation.z = player_transform.translation.z + companion.follow_offset.z;
    }
}

/// Updates WeaponOrigin resource with Whisper's current position.
/// Runs in GameSet::Movement (after whisper_follow_player)
pub fn update_weapon_origin(
    whisper_query: Query<&Transform, With<WhisperCompanion>>,
    mut weapon_origin: ResMut<WeaponOrigin>,
) {
    if let Ok(whisper_transform) = whisper_query.single() {
        weapon_origin.position = Some(whisper_transform.translation.truncate());
    } else {
        weapon_origin.position = None;
    }
}

/// Spawns occasional lightning arc effects around Whisper.
/// Runs in GameSet::Effects
pub fn spawn_whisper_arcs(
    mut commands: Commands,
    time: Res<Time>,
    mut whisper_query: Query<(&Transform, &mut ArcBurstTimer), With<WhisperCompanion>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<AdditiveColorMaterial>>,
) {
    for (whisper_transform, mut timer) in whisper_query.iter_mut() {
        timer.0.tick(time.delta());

        if !timer.0.just_finished() {
            continue;
        }

        let center = whisper_transform.translation;

        // Spawn 1-2 arcs per tick
        let mut rng = rand::thread_rng();
        let arc_count = rng.gen_range(1..=2);

        // Create mesh and material for additive blending (50% size)
        let mesh = meshes.add(Rectangle::new(6.0, 2.0)); // 50% of original 12x4
        let material = color_materials.add(AdditiveColorMaterial {
            color: LinearRgba::new(3.0, 1.5, 1.0, 0.9), // HDR red-orange for bloom
        });

        for _ in 0..arc_count {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(10.0..20.0); // 50% of original 20..40
            let dir = Vec2::new(angle.cos(), angle.sin());
            let pos = Vec3::new(
                center.x + dir.x * distance,
                center.y + dir.y * distance,
                center.z + 0.1,
            );

            // Spawn a small lightning arc with additive blending
            commands.spawn((
                WhisperArc::new(0.06),
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_z(angle)),
            ));
        }
    }
}

/// Spawns lightning bolts from center of Whisper that animate outward.
/// Works on both WhisperDrop and WhisperCompanion entities.
/// Bolts are spawned as children of the whisper so they move with it.
/// Timer resets to a random duration after each spawn for varied timing.
/// Runs in GameSet::Effects
#[allow(clippy::type_complexity)]
pub fn spawn_lightning_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_query: Query<
        (Entity, &mut LightningSpawnTimer),
        (With<WhisperDrop>, Without<WhisperCompanion>),
    >,
    mut companion_query: Query<
        (Entity, &mut LightningSpawnTimer),
        (With<WhisperCompanion>, Without<WhisperDrop>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<AdditiveColorMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // Process WhisperDrop entities
    for (whisper_entity, mut timer) in drop_query.iter_mut() {
        timer.timer.tick(time.delta());

        if !timer.timer.just_finished() {
            continue;
        }

        spawn_bolts_as_children(
            &mut commands,
            whisper_entity,
            &mut rng,
            &mut meshes,
            &mut color_materials,
        );

        // Reset timer with a new random duration
        timer.reset_with_random_duration(&mut rng);
    }

    // Process WhisperCompanion entities
    for (whisper_entity, mut timer) in companion_query.iter_mut() {
        timer.timer.tick(time.delta());

        if !timer.timer.just_finished() {
            continue;
        }

        spawn_bolts_as_children(
            &mut commands,
            whisper_entity,
            &mut rng,
            &mut meshes,
            &mut color_materials,
        );

        // Reset timer with a new random duration
        timer.reset_with_random_duration(&mut rng);
    }
}

/// Helper function to spawn lightning bolts as children of the whisper entity.
/// Uses local coordinates so bolts move with the whisper.
fn spawn_bolts_as_children(
    commands: &mut Commands,
    whisper_entity: Entity,
    rng: &mut impl Rng,
    meshes: &mut Assets<Mesh>,
    color_materials: &mut Assets<AdditiveColorMaterial>,
) {
    // Create a unit mesh for segments (will be scaled via transform)
    let unit_mesh = meshes.add(Rectangle::new(1.0, 1.0));

    // Max bolt length is based on texture radius
    let max_bolt_distance = WHISPER_TEXTURE_SIZE / 2.0;

    // Local center (relative to whisper parent)
    let local_center = Vec3::new(0.0, 0.0, 0.1);

    // Spawn multiple bolts at different angles
    for _ in 0..LIGHTNING_BOLTS_PER_SPAWN {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let seed = rng.gen::<u32>();

        // Random size from 20% to 100%, weighted so longer bolts are rarer
        // Using inverse transform: smaller values are more likely
        // raw^2 biases toward 0, then we invert so larger sizes are rarer
        let raw = rng.gen::<f32>();
        // Square it to bias toward smaller values, then map to range
        let size_multiplier =
            BOLT_MIN_SIZE_FRACTION + (1.0 - raw * raw) * (1.0 - BOLT_MIN_SIZE_FRACTION);

        // Use local center (Vec3::ZERO relative to parent)
        let bolt = LightningBolt::new(angle, seed, local_center, max_bolt_distance, size_multiplier);
        let segment_count = bolt.segment_count;

        // Create materials for all segments first
        let segment_materials: Vec<_> = (0..segment_count)
            .map(|_| {
                color_materials.add(AdditiveColorMaterial {
                    color: LinearRgba::new(3.0, 1.5, 1.0, 0.0), // Start invisible
                })
            })
            .collect();

        // Spawn bolt as child of whisper, with segments as children of bolt
        commands.entity(whisper_entity).with_children(|parent| {
            parent
                .spawn((
                    bolt,
                    Transform::from_translation(local_center),
                    Visibility::default(),
                ))
                .with_children(|bolt_parent| {
                    // Spawn segment meshes as children of the bolt
                    for i in 0..segment_count {
                        bolt_parent.spawn((
                            LightningSegment {
                                index: i,
                                bolt_entity: Entity::PLACEHOLDER, // Will be updated after spawn
                            },
                            Mesh2d(unit_mesh.clone()),
                            MeshMaterial2d(segment_materials[i as usize].clone()),
                            Transform::from_translation(Vec3::new(0.0, 0.0, 0.05 + i as f32 * 0.001)),
                        ));
                    }
                });
        });
    }
}

/// Animates lightning bolts moving outward from center and fading.
/// Uses local coordinates since bolts are children of whisper entities.
/// Runs in GameSet::Effects
pub fn animate_lightning_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut bolt_query: Query<(Entity, &mut LightningBolt), Without<LightningSegment>>,
    mut segment_query: Query<(
        Entity,
        &LightningSegment,
        &ChildOf,
        &mut Transform,
        &MeshMaterial2d<AdditiveColorMaterial>,
    )>,
    mut color_materials: ResMut<Assets<AdditiveColorMaterial>>,
) {
    // First, update all bolt distances
    for (entity, mut bolt) in bolt_query.iter_mut() {
        bolt.distance += bolt.speed * time.delta_secs();

        // Check if bolt should be despawned (despawn cleans up children automatically)
        if bolt.is_expired() {
            commands.entity(entity).despawn();
        }
    }

    // Then update all segments based on their parent bolt
    for (_segment_entity, segment, child_of, mut transform, material_handle) in
        segment_query.iter_mut()
    {
        // Get the parent bolt data using the ChildOf component
        let Ok((_, bolt)) = bolt_query.get(child_of.parent()) else {
            // Parent bolt was despawned, segment will be cleaned up automatically
            continue;
        };

        // If bolt is expired, hide segment (bolt will be despawned with children)
        if bolt.is_expired() {
            if let Some(material) = color_materials.get_mut(&material_handle.0) {
                material.color = LinearRgba::new(0.0, 0.0, 0.0, 0.0);
            }
            continue;
        }

        let segment_idx = segment.index as usize;

        // Calculate how much of this segment should be visible based on bolt distance
        let segment_start_dist = bolt.segment_start_distance(segment_idx);
        let segment_end_dist = bolt.segment_end_distance(segment_idx);
        let segment_len = bolt.segment_length(segment_idx);

        // Only show segment if the bolt has reached it
        if bolt.distance < segment_start_dist {
            if let Some(material) = color_materials.get_mut(&material_handle.0) {
                material.color = LinearRgba::new(0.0, 0.0, 0.0, 0.0); // Invisible
            }
            continue;
        }

        // Calculate visibility progress for this segment
        let segment_progress = if bolt.distance >= segment_end_dist {
            1.0 // Fully visible
        } else if segment_len > 0.0 {
            (bolt.distance - segment_start_dist) / segment_len
        } else {
            1.0
        };

        // Calculate opacity based on overall bolt progress and segment position
        let base_opacity = bolt.current_opacity();
        // Segments further from center fade more
        let segment_fade = 1.0 - (segment_idx as f32 / bolt.segment_count as f32) * 0.3;
        let opacity = (base_opacity * segment_fade * segment_progress).clamp(0.0, 1.0);

        // Skip rendering segments that are nearly invisible
        if opacity < 0.01 {
            if let Some(material) = color_materials.get_mut(&material_handle.0) {
                material.color = LinearRgba::new(0.0, 0.0, 0.0, 0.0);
            }
            continue;
        }

        // Get joint positions for this segment (in local coordinates relative to bolt center)
        let start_pos = bolt.joint_position(segment_idx);
        let end_pos = bolt.joint_position(segment_idx + 1);

        // Interpolate end position based on progress
        let current_end = start_pos + (end_pos - start_pos) * segment_progress;

        // Calculate segment midpoint and length (local to bolt parent)
        let midpoint = (start_pos + current_end) / 2.0;
        let segment_vec = current_end - start_pos;
        let length = segment_vec.length().max(0.1);
        let rotation = segment_vec.y.atan2(segment_vec.x);

        // Update local transform position and rotation (relative to bolt parent)
        transform.translation.x = midpoint.x;
        transform.translation.y = midpoint.y;
        transform.translation.z = 0.05 + segment_idx as f32 * 0.001;
        transform.rotation = Quat::from_rotation_z(rotation);

        // Calculate thickness (tapers from center to tip)
        let thickness = bolt.thickness_at_segment(segment.index);

        // Update size via transform scale (mesh is 1x1 unit)
        transform.scale = Vec3::new(length, thickness, 1.0);

        // Update material color with HDR red-orange and calculated opacity
        if let Some(material) = color_materials.get_mut(&material_handle.0) {
            material.color = LinearRgba::new(3.0, 1.5, 1.0, opacity);
        }
    }
}

/// Updates and despawns lightning arcs.
/// Runs in GameSet::Cleanup
pub fn update_whisper_arcs(
    mut commands: Commands,
    time: Res<Time>,
    mut arc_query: Query<(Entity, &mut WhisperArc)>,
) {
    for (entity, mut arc) in arc_query.iter_mut() {
        arc.lifetime.tick(time.delta());

        if arc.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Number of trail segments per orbital particle (3x the original for smoother trails)
const TRAIL_SEGMENT_COUNT: usize = 36;

/// Spawns orbital particles around Whisper at random intervals.
/// Particles orbit in tilted 3D planes projected to 2D.
/// Runs in GameSet::Effects
#[allow(clippy::type_complexity)]
pub fn spawn_orbital_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_query: Query<
        (Entity, &mut OrbitalParticleSpawnTimer),
        (With<WhisperDrop>, Without<WhisperCompanion>),
    >,
    mut companion_query: Query<
        (Entity, &mut OrbitalParticleSpawnTimer),
        (With<WhisperCompanion>, Without<WhisperDrop>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<AdditiveColorMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // Process WhisperDrop entities
    for (whisper_entity, mut timer) in drop_query.iter_mut() {
        timer.timer.tick(time.delta());

        if !timer.timer.just_finished() {
            continue;
        }

        spawn_orbital_particle_as_child(
            &mut commands,
            whisper_entity,
            &mut meshes,
            &mut color_materials,
            &mut rng,
        );

        timer.reset_with_random_duration(&mut rng);
    }

    // Process WhisperCompanion entities
    for (whisper_entity, mut timer) in companion_query.iter_mut() {
        timer.timer.tick(time.delta());

        if !timer.timer.just_finished() {
            continue;
        }

        spawn_orbital_particle_as_child(
            &mut commands,
            whisper_entity,
            &mut meshes,
            &mut color_materials,
            &mut rng,
        );

        timer.reset_with_random_duration(&mut rng);
    }
}

/// Helper function to spawn an orbital particle as a child of the whisper entity.
fn spawn_orbital_particle_as_child(
    commands: &mut Commands,
    whisper_entity: Entity,
    meshes: &mut Assets<Mesh>,
    color_materials: &mut Assets<AdditiveColorMaterial>,
    rng: &mut impl Rng,
) {
    // Random orbital parameters - orbit within texture area (32px radius)
    let radius = rng.gen_range(12.0..28.0);
    let period = rng.gen_range(0.19..0.38); // Very fast orbit (8x original speed)
    let inclination = rng.gen_range(0.3..1.2); // ~17 to ~69 degrees
    let ascending_node = rng.gen_range(0.0..std::f32::consts::TAU);
    let size = rng.gen_range(3.0..5.0);
    let phase = rng.gen_range(0.0..std::f32::consts::TAU);

    let particle = OrbitalParticle::new(
        radius,
        period,
        phase,
        inclination,
        ascending_node,
        size,
    );

    // Trail with more samples for smoother appearance
    let trail = ParticleTrail::new(TRAIL_SEGMENT_COUNT, 0.015);

    // Calculate initial position
    let (pos_2d, _z_depth) = particle.calculate_position();
    let render_z = particle.calculate_render_z();

    // Create mesh and materials for trail segments
    let unit_mesh = meshes.add(Rectangle::new(1.0, 1.0));

    // Spawn particle as child of whisper
    commands.entity(whisper_entity).with_children(|parent| {
        parent
            .spawn((
                particle,
                trail,
                Transform::from_translation(Vec3::new(pos_2d.x, pos_2d.y, render_z)),
                Visibility::default(),
            ))
            .with_children(|particle_parent| {
                // Pre-spawn trail segment meshes
                for i in 0..TRAIL_SEGMENT_COUNT {
                    let material = color_materials.add(AdditiveColorMaterial {
                        color: LinearRgba::new(3.0, 1.0, 0.6, 0.0), // Start invisible
                    });
                    particle_parent.spawn((
                        TrailSegment { index: i },
                        Mesh2d(unit_mesh.clone()),
                        MeshMaterial2d(material),
                        Transform::from_translation(Vec3::new(0.0, 0.0, -0.01 - i as f32 * 0.001)),
                    ));
                }
            });
    });
}

/// Updates orbital particle positions and trails.
/// Runs in GameSet::Effects
pub fn update_orbital_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(
        Entity,
        &mut OrbitalParticle,
        &mut ParticleTrail,
        &mut Transform,
    )>,
) {
    let delta_secs = time.delta_secs();

    for (entity, mut particle, mut trail, mut transform) in particle_query.iter_mut() {
        // Advance age and check if fully transparent (ready for despawn)
        particle.advance_age(delta_secs);
        if particle.is_fully_transparent() {
            commands.entity(entity).despawn();
            continue;
        }

        // Update phase (orbital position)
        particle.advance_phase(delta_secs);

        // Calculate new position
        let (position_2d, z_depth) = particle.calculate_position();

        // Update trail sampling
        trail.tick(delta_secs);
        if trail.should_sample() {
            trail.record_position(position_2d, z_depth);
            trail.reset_sample_timer();
        }

        // Update particle transform (local to whisper parent)
        transform.translation.x = position_2d.x;
        transform.translation.y = position_2d.y;
        transform.translation.z = particle.calculate_render_z();

        // Scale based on z (pseudo-perspective: slightly larger when closer)
        let normalized_z = (z_depth / particle.radius).clamp(-1.0, 1.0);
        let perspective_scale = 1.0 - normalized_z * 0.1; // Closer = larger
        transform.scale = Vec3::splat(perspective_scale);
    }
}

/// Renders particle trails by positioning trail segments between recorded positions.
/// Each segment following the head is progressively more transparent.
/// Runs in GameSet::Effects (after update_orbital_particles)
pub fn render_particle_trails(
    particle_query: Query<(&ParticleTrail, &OrbitalParticle), Without<TrailSegment>>,
    mut segment_query: Query<(
        &TrailSegment,
        &ChildOf,
        &mut Transform,
        &MeshMaterial2d<AdditiveColorMaterial>,
    )>,
    mut color_materials: ResMut<Assets<AdditiveColorMaterial>>,
) {
    for (segment, child_of, mut transform, material_handle) in segment_query.iter_mut() {
        // Get parent particle data
        let Ok((trail, particle)) = particle_query.get(child_of.parent()) else {
            continue;
        };

        let idx = segment.index;

        // Not enough trail points yet - hide segment
        if idx + 1 >= trail.positions.len() {
            if let Some(material) = color_materials.get_mut(&material_handle.0) {
                material.color.alpha = 0.0;
            }
            continue;
        }

        // Trail positions are in Whisper's local space, but segments are children of the particle.
        // Transform trail positions to particle's local space by subtracting particle's current position.
        let (particle_pos, _) = particle.calculate_position();
        let start = trail.positions[idx] - particle_pos;
        let end = trail.positions[idx + 1] - particle_pos;
        let start_z = trail.z_depths[idx];
        let end_z = trail.z_depths[idx + 1];

        // Segment geometry
        let midpoint = (start + end) / 2.0;
        let direction = end - start;
        let length = direction.length().max(0.1);
        let angle = direction.y.atan2(direction.x);

        // Z position (average of segment endpoints)
        let avg_z = (start_z + end_z) / 2.0;

        // Update transform (local to particle parent)
        transform.translation.x = midpoint.x;
        transform.translation.y = midpoint.y;
        // Slightly behind particle head
        transform.translation.z = -0.01 - idx as f32 * 0.001;
        transform.rotation = Quat::from_rotation_z(angle);

        // Thickness tapers along trail
        let progress = idx as f32 / (TRAIL_SEGMENT_COUNT - 1) as f32;
        let thickness = particle.size * 0.8 * (1.0 - progress);
        transform.scale = Vec3::new(length, thickness.max(0.1), 1.0);

        // Calculate depth-based brightness (further away = dimmer)
        // Normalize z_depth: positive = behind (dimmer), negative = in front (brighter)
        let normalized_z = (avg_z / particle.radius).clamp(-1.0, 1.0);
        // Behind the core (positive z) = 0.3-0.6 brightness, in front = 0.8-1.0 brightness
        let depth_brightness = 0.65 - normalized_z * 0.35;

        // Get segment brightness (includes head fade-in/out and progressive trail fade)
        // This controls HDR intensity rather than alpha for additive blending
        let segment_brightness = particle.segment_opacity(idx, TRAIL_SEGMENT_COUNT);

        // Combined brightness: segment fade * depth
        let brightness = (segment_brightness * depth_brightness).clamp(0.0, 1.0);

        // Use brightness to scale HDR color values (not alpha) for additive blending
        if let Some(material) = color_materials.get_mut(&material_handle.0) {
            material.color = LinearRgba::new(
                3.0 * brightness,
                1.0 * brightness,
                0.6 * brightness,
                1.0, // Full alpha, brightness controls visibility
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_spawn_whisper_drop_creates_entity() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveTextureMaterial>>();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify WhisperDrop entity was spawned
        let whisper_count = app
            .world_mut()
            .query::<&WhisperDrop>()
            .iter(app.world())
            .count();
        assert_eq!(whisper_count, 1);
    }

    #[test]
    fn test_spawn_whisper_drop_position_within_200px_of_player() {
        // Run multiple times to verify random positions are within 200px of player origin
        for _ in 0..20 {
            let mut app = App::new();
            app.init_resource::<Assets<Mesh>>();
            app.init_resource::<Assets<AdditiveTextureMaterial>>();
            app.add_systems(Startup, spawn_whisper_drop);

            app.update();

            let mut query = app.world_mut().query::<(&WhisperDrop, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                let pos = transform.translation;
                let distance = (pos.x * pos.x + pos.y * pos.y).sqrt();

                // Whisper should spawn within 200px of origin (where player spawns)
                assert!(
                    distance <= 200.0,
                    "Whisper spawned at distance {} which exceeds 200px",
                    distance
                );

                // Whisper should spawn at least some minimum distance away (not on top of player)
                assert!(
                    distance >= 50.0,
                    "Whisper spawned too close to player at distance {}",
                    distance
                );
            }
        }
    }

    #[test]
    fn test_detect_whisper_pickup_fires_event_when_in_range() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mut app = App::new();
        app.add_message::<WhisperCollectedEvent>();

        // Add a simple event counter system that runs after pickup detection
        let event_count = Arc::new(AtomicUsize::new(0));
        let event_count_clone = event_count.clone();

        app.add_systems(
            Update,
            (
                detect_whisper_pickup,
                move |mut events: MessageReader<WhisperCollectedEvent>| {
                    for _event in events.read() {
                        event_count_clone.fetch_add(1, Ordering::SeqCst);
                    }
                },
            )
                .chain(),
        );

        // Create player at (0, 0)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create WhisperDrop at (10, 10) - within pickup radius
        app.world_mut().spawn((
            WhisperDrop::default(),
            Transform::from_translation(Vec3::new(10.0, 10.0, 0.0)),
        ));

        app.update();

        // Verify event was fired
        assert_eq!(event_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_detect_whisper_pickup_no_event_when_out_of_range() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mut app = App::new();
        app.add_message::<WhisperCollectedEvent>();
        app.add_systems(Update, detect_whisper_pickup);

        // Create player at (0, 0)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Create WhisperDrop far away
        app.world_mut().spawn((
            WhisperDrop::default(),
            Transform::from_translation(Vec3::new(100.0, 100.0, 0.0)),
        ));

        // Add a simple event counter
        let event_count = Arc::new(AtomicUsize::new(0));
        let event_count_clone = event_count.clone();

        app.add_systems(
            Update,
            move |mut events: MessageReader<WhisperCollectedEvent>| {
                for _event in events.read() {
                    event_count_clone.fetch_add(1, Ordering::SeqCst);
                }
            },
        );

        app.update();

        // Verify no event was fired
        assert_eq!(event_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_reset_whisper_state() {
        let mut app = App::new();
        app.init_resource::<WhisperState>();
        app.init_resource::<WeaponOrigin>();

        // Set initial state
        app.world_mut().resource_mut::<WhisperState>().collected = true;
        app.world_mut().resource_mut::<WeaponOrigin>().position = Some(Vec2::new(10.0, 20.0));

        app.add_systems(Update, reset_whisper_state);
        app.update();

        // Verify state was reset
        assert!(!app.world().resource::<WhisperState>().collected);
        assert!(app.world().resource::<WeaponOrigin>().position.is_none());
    }

    #[test]
    fn test_whisper_follow_player() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, whisper_follow_player);

        // Create player at (100, 100)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(100.0, 100.0, 1.0)),
        ));

        // Create WhisperCompanion at origin
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();

        app.update();
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify Whisper followed player
        let whisper_transform = app.world().get::<Transform>(whisper_entity).unwrap();
        assert!(
            (whisper_transform.translation.x - 100.0).abs() < 1.0,
            "Whisper X should be near player X"
        );
        // Y will have follow_offset (30.0) plus some bobbing
        assert!(
            whisper_transform.translation.y > 100.0,
            "Whisper Y should be above player"
        );
    }

    #[test]
    fn test_update_weapon_origin_with_companion() {
        let mut app = App::new();
        app.init_resource::<WeaponOrigin>();
        app.add_systems(Update, update_weapon_origin);

        // Create WhisperCompanion at (50, 60)
        app.world_mut().spawn((
            WhisperCompanion::default(),
            Transform::from_translation(Vec3::new(50.0, 60.0, 0.5)),
        ));

        app.update();

        // Verify WeaponOrigin was updated
        let weapon_origin = app.world().resource::<WeaponOrigin>();
        assert!(weapon_origin.position.is_some());
        assert_eq!(weapon_origin.position.unwrap(), Vec2::new(50.0, 60.0));
    }

    #[test]
    fn test_update_weapon_origin_without_companion() {
        let mut app = App::new();
        app.init_resource::<WeaponOrigin>();
        app.add_systems(Update, update_weapon_origin);

        // Set initial position
        app.world_mut().resource_mut::<WeaponOrigin>().position = Some(Vec2::new(10.0, 20.0));

        app.update();

        // Verify WeaponOrigin was cleared
        let weapon_origin = app.world().resource::<WeaponOrigin>();
        assert!(weapon_origin.position.is_none());
    }

    #[test]
    fn test_update_whisper_arcs_despawns_expired() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, update_whisper_arcs);

        // Create an arc that's already expired
        let arc_entity = app
            .world_mut()
            .spawn((
                WhisperArc {
                    lifetime: Timer::from_seconds(0.0, TimerMode::Once),
                },
                Transform::default(),
            ))
            .id();

        // Tick the timer to mark it finished
        app.world_mut()
            .get_mut::<WhisperArc>(arc_entity)
            .unwrap()
            .lifetime
            .tick(Duration::from_secs_f32(0.1));

        app.update();

        // Verify arc was despawned
        assert!(
            app.world().get_entity(arc_entity).is_err(),
            "Expired arc should be despawned"
        );
    }

    #[test]
    fn test_spawn_whisper_drop_has_point_light() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveTextureMaterial>>();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify WhisperDrop entity was spawned with PointLight2d
        let mut query = app.world_mut().query::<(&WhisperDrop, &PointLight2d)>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1, "WhisperDrop should have PointLight2d component");
    }

    #[test]
    fn test_spawn_whisper_drop_light_properties() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveTextureMaterial>>();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify the light properties
        let mut query = app.world_mut().query::<&PointLight2d>();
        let light = query.single(app.world()).unwrap();

        // Drop has dimmer light (half intensity and radius)
        assert_eq!(light.intensity, WHISPER_LIGHT_INTENSITY * 0.5);
        assert_eq!(light.outer_radius, WHISPER_LIGHT_OUTER_RADIUS * 0.5);
        assert_eq!(light.falloff, WHISPER_LIGHT_FALLOFF);
    }

    // Note: test_setup_whisper_particle_effect_creates_resource requires full HanabiPlugin
    // which has extensive dependencies (mesh, render, etc). The particle effect is
    // integration-tested via the full game setup. The function signature is validated
    // at compile time.

    #[test]
    fn test_particle_constants_are_reasonable() {
        // Verify particle constants have sensible values
        assert!(SPARK_SPAWN_RATE > 0.0, "Spawn rate should be positive");
        assert!(SPARK_LIFETIME > 0.0, "Lifetime should be positive");
        assert!(SPARK_SPEED > 0.0, "Speed should be positive");
        assert!(
            SPARK_SIZE_START >= SPARK_SIZE_END,
            "Size should shrink over lifetime"
        );
    }

    #[test]
    fn test_lightning_bolt_animation_updates_distance() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();
        app.add_systems(Update, animate_lightning_bolts);

        // Create a lightning bolt at origin facing right (angle = 0)
        let center = Vec3::new(0.0, 0.0, 0.5);
        let bolt = LightningBolt::new(0.0, 42, center, 32.0, 1.0);
        let bolt_entity = app
            .world_mut()
            .spawn((bolt, Transform::from_translation(center), Visibility::default()))
            .id();

        // Create mesh and material for segment
        let mesh_handle = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            meshes.add(Rectangle::new(1.0, 1.0))
        };
        let material_handle = {
            let mut materials = app.world_mut().resource_mut::<Assets<AdditiveColorMaterial>>();
            materials.add(AdditiveColorMaterial {
                color: LinearRgba::WHITE,
            })
        };

        // Create a segment for this bolt
        app.world_mut().spawn((
            LightningSegment {
                index: 0,
                bolt_entity,
            },
            Mesh2d(mesh_handle),
            MeshMaterial2d(material_handle),
            Transform::from_translation(center),
        ));

        // First update to initialize time
        app.update();

        // Advance time and update again
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify bolt distance increased
        let bolt = app.world().get::<LightningBolt>(bolt_entity).unwrap();
        assert!(
            bolt.distance > 0.0,
            "Bolt distance should have increased, got {}",
            bolt.distance
        );
    }

    #[test]
    fn test_lightning_bolt_despawns_when_expired() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();
        app.add_systems(Update, animate_lightning_bolts);

        // Create a lightning bolt that's already expired (distance >= max_distance)
        let center = Vec3::ZERO;
        let mut bolt = LightningBolt::new(0.0, 42, center, 32.0, 1.0);
        bolt.distance = bolt.max_distance; // At max, should be despawned

        let bolt_entity = app
            .world_mut()
            .spawn((bolt, Transform::from_translation(center), Visibility::default()))
            .id();

        // First update to initialize time
        app.update();

        // Advance time and update
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.016));
        app.update();

        // Verify bolt was despawned
        assert!(
            app.world().get_entity(bolt_entity).is_err(),
            "Expired lightning bolt should be despawned"
        );
    }

    #[test]
    fn test_lightning_segments_despawn_with_parent_bolt() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();
        app.add_systems(Update, animate_lightning_bolts);

        // Create mesh and material for segment
        let mesh_handle = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            meshes.add(Rectangle::new(1.0, 1.0))
        };
        let material_handle = {
            let mut materials = app.world_mut().resource_mut::<Assets<AdditiveColorMaterial>>();
            materials.add(AdditiveColorMaterial {
                color: LinearRgba::WHITE,
            })
        };

        // Create an expired lightning bolt with a child segment
        let center = Vec3::ZERO;
        let mut bolt = LightningBolt::new(0.0, 42, center, 32.0, 1.0);
        bolt.distance = bolt.max_distance; // Mark as expired

        let bolt_entity = app
            .world_mut()
            .spawn((bolt, Transform::from_translation(center), Visibility::default()))
            .with_children(|parent| {
                parent.spawn((
                    LightningSegment {
                        index: 0,
                        bolt_entity: Entity::PLACEHOLDER,
                    },
                    Mesh2d(mesh_handle.clone()),
                    MeshMaterial2d(material_handle.clone()),
                    Transform::default(),
                ));
            })
            .id();

        // Get the segment entity
        let segment_entity = app
            .world_mut()
            .query::<(Entity, &LightningSegment)>()
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("Segment should exist before update");

        app.update();

        // Verify both bolt and segment were despawned
        assert!(
            app.world().get_entity(bolt_entity).is_err(),
            "Expired bolt should be despawned"
        );
        assert!(
            app.world().get_entity(segment_entity).is_err(),
            "Child segment should be despawned with parent bolt"
        );
    }

    #[test]
    fn test_spawn_whisper_drop_has_lightning_spawn_timer() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveTextureMaterial>>();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify WhisperDrop entity has LightningSpawnTimer
        let mut query = app.world_mut().query::<(&WhisperDrop, &LightningSpawnTimer)>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "WhisperDrop should have LightningSpawnTimer component"
        );
    }

    #[test]
    fn test_spawn_bolts_as_children_creates_bolts_and_segments() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();

        // Create a whisper entity to be the parent
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::new(100.0, 100.0, 0.5)),
            ))
            .id();

        // Spawn bolts as children of the whisper entity
        {
            let world = app.world_mut();
            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(
                    |world, mut color_materials: Mut<Assets<AdditiveColorMaterial>>| {
                        let mut commands = world.commands();
                        spawn_bolts_as_children(
                            &mut commands,
                            whisper_entity,
                            &mut rand::thread_rng(),
                            &mut meshes,
                            &mut color_materials,
                        );
                    },
                );
            });
        }
        app.update();

        // Verify lightning bolts were spawned
        let bolt_count = app
            .world_mut()
            .query::<&LightningBolt>()
            .iter(app.world())
            .count();
        assert_eq!(
            bolt_count,
            LIGHTNING_BOLTS_PER_SPAWN as usize,
            "Should spawn {} lightning bolts",
            LIGHTNING_BOLTS_PER_SPAWN
        );

        // Verify segments were spawned (5 segments per bolt)
        let segment_count = app
            .world_mut()
            .query::<&LightningSegment>()
            .iter(app.world())
            .count();
        assert_eq!(
            segment_count,
            (LIGHTNING_BOLTS_PER_SPAWN * 5) as usize,
            "Should spawn 5 segments per bolt"
        );
    }

    #[test]
    fn test_lightning_bolts_use_local_coordinates() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();

        // Create a whisper entity at some world position
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::new(50.0, 75.0, 0.5)),
            ))
            .id();

        // Spawn bolts as children
        {
            let world = app.world_mut();
            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(
                    |world, mut color_materials: Mut<Assets<AdditiveColorMaterial>>| {
                        let mut commands = world.commands();
                        spawn_bolts_as_children(
                            &mut commands,
                            whisper_entity,
                            &mut rand::thread_rng(),
                            &mut meshes,
                            &mut color_materials,
                        );
                    },
                );
            });
        }
        app.update();

        // Verify bolts store local center (0, 0, 0.1) not world position
        let mut query = app.world_mut().query::<&LightningBolt>();
        for bolt in query.iter(app.world()) {
            assert!(
                bolt.center.x.abs() < 0.001,
                "Bolt center X should be 0 (local coords), got {}",
                bolt.center.x
            );
            assert!(
                bolt.center.y.abs() < 0.001,
                "Bolt center Y should be 0 (local coords), got {}",
                bolt.center.y
            );
        }
    }

    // ==================== Orbital Particle System Tests ====================

    #[test]
    fn test_spawn_orbital_particles_spawns_particle_as_child() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();
        app.add_systems(Update, spawn_orbital_particles);

        // Create a WhisperCompanion with spawn timer set to trigger immediately
        app.world_mut().spawn((
            WhisperCompanion::default(),
            OrbitalParticleSpawnTimer {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                min_interval: 0.5,
                max_interval: 1.0,
            },
            Transform::from_translation(Vec3::new(100.0, 100.0, 0.5)),
            Visibility::default(),
        ));

        // Tick timer so it triggers
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify orbital particle was spawned
        let particle_count = app
            .world_mut()
            .query::<&OrbitalParticle>()
            .iter(app.world())
            .count();
        assert_eq!(particle_count, 1, "Should spawn 1 orbital particle");
    }

    #[test]
    fn test_spawn_orbital_particles_creates_trail_segments() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();
        app.add_systems(Update, spawn_orbital_particles);

        // Create a WhisperCompanion with spawn timer set to trigger immediately
        app.world_mut().spawn((
            WhisperCompanion::default(),
            OrbitalParticleSpawnTimer {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                min_interval: 0.5,
                max_interval: 1.0,
            },
            Transform::from_translation(Vec3::new(100.0, 100.0, 0.5)),
            Visibility::default(),
        ));

        // Tick timer so it triggers
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify trail segments were spawned (12 per particle)
        let segment_count = app
            .world_mut()
            .query::<&TrailSegment>()
            .iter(app.world())
            .count();
        assert_eq!(
            segment_count, TRAIL_SEGMENT_COUNT,
            "Should spawn {} trail segments",
            TRAIL_SEGMENT_COUNT
        );
    }

    #[test]
    fn test_update_orbital_particles_advances_phase() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, update_orbital_particles);

        // Create an orbital particle
        let particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);
        let trail = ParticleTrail::default();
        let particle_entity = app
            .world_mut()
            .spawn((
                particle,
                trail,
                Transform::from_translation(Vec3::new(30.0, 0.0, 0.5)),
            ))
            .id();

        // First update
        app.update();

        // Advance time (short enough to not trigger despawn)
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.05));
        app.update();

        // Verify phase advanced
        let particle = app.world().get::<OrbitalParticle>(particle_entity).unwrap();
        assert!(
            particle.phase > 0.0,
            "Phase should have advanced, got {}",
            particle.phase
        );
    }

    #[test]
    fn test_update_orbital_particles_updates_transform() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, update_orbital_particles);

        // Create an orbital particle
        let particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);
        let trail = ParticleTrail::default();
        let initial_transform = Transform::from_translation(Vec3::new(30.0, 0.0, 0.5));
        let particle_entity = app
            .world_mut()
            .spawn((particle, trail, initial_transform))
            .id();

        // First update initializes time
        app.update();

        // Advance time (short enough to stay in fade-in/visible phase)
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify transform was updated (particle moved along orbit)
        let transform = app.world().get::<Transform>(particle_entity).unwrap();
        let particle = app.world().get::<OrbitalParticle>(particle_entity).unwrap();

        // The particle should have moved, so its position should reflect the new phase
        let (expected_pos, _) = particle.calculate_position();
        assert!(
            (transform.translation.x - expected_pos.x).abs() < 0.01,
            "Transform X should match particle position"
        );
    }

    #[test]
    fn test_update_orbital_particles_despawns_when_fully_transparent() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, update_orbital_particles);

        // Create an orbital particle that's already past fade-in + fade-out duration
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);
        // Set age beyond fade_in + fade_out to make it fully transparent
        particle.age = particle.fade_in_duration + particle.fade_out_duration + 0.1;
        let trail = ParticleTrail::default();
        let particle_entity = app
            .world_mut()
            .spawn((
                particle,
                trail,
                Transform::from_translation(Vec3::new(30.0, 0.0, 0.5)),
            ))
            .id();

        // Update to trigger despawn
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.01));
        app.update();

        // Verify particle was despawned
        assert!(
            app.world().get_entity(particle_entity).is_err(),
            "Fully transparent particle should be despawned"
        );
    }

    #[test]
    fn test_update_orbital_particles_advances_age() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, update_orbital_particles);

        // Create an orbital particle
        let particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);
        let trail = ParticleTrail::default();
        let particle_entity = app
            .world_mut()
            .spawn((
                particle,
                trail,
                Transform::from_translation(Vec3::new(30.0, 0.0, 0.5)),
            ))
            .id();

        // First update
        app.update();

        // Advance time
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.05));
        app.update();

        // Verify age advanced
        let particle = app.world().get::<OrbitalParticle>(particle_entity).unwrap();
        assert!(
            particle.age > 0.0,
            "Age should have advanced, got {}",
            particle.age
        );
    }

    #[test]
    fn test_orbital_particles_work_with_whisper_drop() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<AdditiveColorMaterial>>();
        app.add_systems(Update, spawn_orbital_particles);

        // Create a WhisperDrop (not companion) with spawn timer
        app.world_mut().spawn((
            WhisperDrop::default(),
            OrbitalParticleSpawnTimer {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                min_interval: 0.5,
                max_interval: 1.0,
            },
            Transform::from_translation(Vec3::new(100.0, 100.0, 0.5)),
            Visibility::default(),
        ));

        // Tick timer so it triggers
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify orbital particle was spawned for WhisperDrop too
        let particle_count = app
            .world_mut()
            .query::<&OrbitalParticle>()
            .iter(app.world())
            .count();
        assert_eq!(
            particle_count, 1,
            "WhisperDrop should also spawn orbital particles"
        );
    }
}
