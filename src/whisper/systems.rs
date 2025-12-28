use bevy::prelude::*;
use bevy_hanabi::prelude::{
    Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier, EffectAsset, ExprWriter,
    Gradient as HanabiGradient, ParticleEffect, SetAttributeModifier, SetPositionCircleModifier,
    SetVelocitySphereModifier, ShapeDimension, SizeOverLifetimeModifier, SpawnerSettings,
};
use rand::Rng;

use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::whisper::components::{
    ArcBurstTimer, LightningBolt, LightningSegment, LightningSpawnTimer, OrbitalParticle,
    OrbitalParticleSpawnTimer, ParticleTrail, TrailSegment, WhisperArc, WhisperCompanion,
    WhisperDrop, WhisperOuterGlow,
};
use crate::whisper::events::*;
use crate::whisper::resources::*;

/// Color constants for Whisper visual effects (red mode)
const WHISPER_LIGHT_COLOR: Color = Color::srgb(1.0, 0.3, 0.2); // Red-orange
/// 3D PointLight intensity (lumens)
const WHISPER_LIGHT_INTENSITY: f32 = 2000.0;
/// 3D PointLight radius
const WHISPER_LIGHT_RADIUS: f32 = 5.0;

/// Particle effect constants for 3D space
const SPARK_SPAWN_RATE: f32 = 120.0; // particles per second
const SPARK_LIFETIME: f32 = 0.35; // seconds
const SPARK_SPEED: f32 = 2.0; // 3D world units per second
const SPARK_SIZE_START: f32 = 0.05; // 3D world units
const SPARK_SIZE_END: f32 = 0.0;

/// Whisper core radius in 3D world units
const WHISPER_CORE_RADIUS: f32 = 0.5;

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

    // Position: spawn at center (3D, so use Y axis for the circle plane on XZ ground)
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(), // Circle in XZ plane (Y is up)
        radius: writer.lit(0.2).expr(),   // 3D world units
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

/// Spawns Whisper drop within a certain radius of the player spawn position (origin).
/// Uses polar coordinates to ensure uniform distribution in a ring around the player.
/// Runs on OnEnter(GameState::InGame)
pub fn spawn_whisper_drop(
    mut commands: Commands,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(game_meshes) = game_meshes else { return };
    let Some(game_materials) = game_materials else { return };

    let mut rng = rand::thread_rng();

    // Generate random position on XZ plane within 5-10 world units of origin
    // Using polar coordinates for uniform distribution
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let distance = rng.gen_range(3.0..8.0);
    let x = angle.cos() * distance;
    let z = angle.sin() * distance;
    // Y is height - place whisper slightly above ground
    let position = Vec3::new(x, 1.0, z);

    // Spawn the Whisper drop with 3D visual elements
    commands
        .spawn((
            WhisperDrop::default(),
            LightningSpawnTimer::default(),
            OrbitalParticleSpawnTimer::default(),
            Transform::from_translation(position),
            Visibility::default(),
            // Add 3D PointLight for glow effect (dimmer when not collected)
            PointLight {
                color: WHISPER_LIGHT_COLOR,
                intensity: WHISPER_LIGHT_INTENSITY * 0.5,
                radius: WHISPER_LIGHT_RADIUS,
                shadows_enabled: false,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Core glow sphere using 3D mesh
            parent.spawn((
                WhisperOuterGlow,
                Mesh3d(game_meshes.whisper_core.clone()),
                MeshMaterial3d(game_materials.whisper_drop.clone()),
                Transform::default(),
            ));
        });
}

/// Detects when player is close enough to collect Whisper.
/// Uses XZ plane for 3D collision detection.
/// Runs in GameSet::Combat
pub fn detect_whisper_pickup(
    player_query: Query<(Entity, &Transform), With<Player>>,
    whisper_query: Query<(Entity, &Transform, &WhisperDrop)>,
    mut whisper_events: MessageWriter<WhisperCollectedEvent>,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    // Use XZ plane for 3D collision detection
    let player_pos = from_xz(player_transform.translation);

    for (whisper_entity, whisper_transform, whisper_drop) in whisper_query.iter() {
        let whisper_pos = from_xz(whisper_transform.translation);
        let distance = player_pos.distance(whisper_pos);

        // Pickup radius scaled for 3D world units (1.5 = ~1.5 world units)
        if distance <= whisper_drop.pickup_radius_3d() {
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
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    use crate::inventory::components::EquippedWeapon;
    use crate::weapon::components::{Weapon, WeaponType};

    let Some(game_meshes) = game_meshes else { return };
    let Some(game_materials) = game_materials else { return };

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

        let mut companion_entity = commands.spawn((
            companion,
            ArcBurstTimer::default(),
            LightningSpawnTimer::default(),
            OrbitalParticleSpawnTimer::default(),
            Transform::from_translation(companion_pos),
            Visibility::default(),
            // Full brightness 3D PointLight when collected
            PointLight {
                color: WHISPER_LIGHT_COLOR,
                intensity: WHISPER_LIGHT_INTENSITY,
                radius: WHISPER_LIGHT_RADIUS,
                shadows_enabled: false,
                ..default()
            },
        ));

        // Add particle effect if available
        if let Some(effect) = spark_effect.as_ref() {
            companion_entity.insert(ParticleEffect::new(effect.0.clone()));
        }

        companion_entity.with_children(|parent| {
            // Core glow sphere using 3D mesh
            parent.spawn((
                WhisperOuterGlow,
                Mesh3d(game_meshes.whisper_core.clone()),
                MeshMaterial3d(game_materials.whisper_core.clone()),
                Transform::default(),
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
/// Uses XZ plane for 3D position (ignores Y height).
/// Runs in GameSet::Movement (after whisper_follow_player)
pub fn update_weapon_origin(
    whisper_query: Query<&Transform, With<WhisperCompanion>>,
    mut weapon_origin: ResMut<WeaponOrigin>,
) {
    if let Ok(whisper_transform) = whisper_query.single() {
        // Use XZ plane (from_xz extracts X and Z as Vec2)
        weapon_origin.position = Some(from_xz(whisper_transform.translation));
    } else {
        weapon_origin.position = None;
    }
}

/// Spawns occasional lightning arc effects around Whisper.
/// Uses XZ plane for 3D positioning.
/// Runs in GameSet::Effects
pub fn spawn_whisper_arcs(
    mut commands: Commands,
    time: Res<Time>,
    mut whisper_query: Query<(&Transform, &mut ArcBurstTimer), With<WhisperCompanion>>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(game_meshes) = game_meshes else { return };
    let Some(game_materials) = game_materials else { return };

    for (whisper_transform, mut timer) in whisper_query.iter_mut() {
        timer.0.tick(time.delta());

        if !timer.0.just_finished() {
            continue;
        }

        let center = whisper_transform.translation;

        // Spawn 1-2 arcs per tick
        let mut rng = rand::thread_rng();
        let arc_count = rng.gen_range(1..=2);

        for _ in 0..arc_count {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(0.3..0.6); // 3D world units
            // Position on XZ plane around whisper
            let pos = Vec3::new(
                center.x + angle.cos() * distance,
                center.y,
                center.z + angle.sin() * distance,
            );

            // Spawn a small lightning arc with 3D mesh
            commands.spawn((
                WhisperArc::new(0.06),
                Mesh3d(game_meshes.whisper_arc.clone()),
                MeshMaterial3d(game_materials.lightning.clone()),
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(angle)),
            ));
        }
    }
}

/// Spawns lightning bolts from center of Whisper that animate outward.
/// Works on both WhisperDrop and WhisperCompanion entities.
/// Bolts are spawned as children of the whisper so they move with it.
/// Timer resets to a random duration after each spawn for varied timing.
/// Uses 3D meshes and XZ plane for bolt orientation.
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
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(game_meshes) = game_meshes else { return };
    let Some(game_materials) = game_materials else { return };
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
            &game_meshes,
            &game_materials,
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
            &game_meshes,
            &game_materials,
        );

        // Reset timer with a new random duration
        timer.reset_with_random_duration(&mut rng);
    }
}

/// Helper function to spawn lightning bolts as children of the whisper entity.
/// Uses local coordinates so bolts move with the whisper.
/// Bolts radiate outward on the XZ plane in 3D space.
fn spawn_bolts_as_children(
    commands: &mut Commands,
    whisper_entity: Entity,
    rng: &mut impl Rng,
    game_meshes: &GameMeshes,
    game_materials: &GameMaterials,
) {
    // Max bolt length in 3D world units
    let max_bolt_distance = WHISPER_CORE_RADIUS;

    // Local center (relative to whisper parent)
    let local_center = Vec3::new(0.0, 0.0, 0.0);

    // Spawn multiple bolts at different angles
    for _ in 0..LIGHTNING_BOLTS_PER_SPAWN {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let seed = rng.gen::<u32>();

        // Random size from 20% to 100%, weighted so longer bolts are rarer
        let raw = rng.gen::<f32>();
        let size_multiplier =
            BOLT_MIN_SIZE_FRACTION + (1.0 - raw * raw) * (1.0 - BOLT_MIN_SIZE_FRACTION);

        // Use local center (Vec3::ZERO relative to parent)
        let bolt = LightningBolt::new(angle, seed, local_center, max_bolt_distance, size_multiplier);
        let segment_count = bolt.segment_count;

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
                                bolt_entity: Entity::PLACEHOLDER,
                            },
                            Mesh3d(game_meshes.lightning_segment.clone()),
                            MeshMaterial3d(game_materials.lightning.clone()),
                            Transform::from_translation(Vec3::new(0.0, 0.01 + i as f32 * 0.001, 0.0)),
                        ));
                    }
                });
        });
    }
}

/// Animates lightning bolts moving outward from center and fading.
/// Uses local coordinates since bolts are children of whisper entities.
/// Uses 3D transforms on XZ plane - bolts radiate from center.
/// Runs in GameSet::Effects
pub fn animate_lightning_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut bolt_query: Query<(Entity, &mut LightningBolt), Without<LightningSegment>>,
    mut segment_query: Query<(&LightningSegment, &ChildOf, &mut Transform)>,
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
    for (segment, child_of, mut transform) in segment_query.iter_mut() {
        // Get the parent bolt data using the ChildOf component
        let Ok((_, bolt)) = bolt_query.get(child_of.parent()) else {
            // Parent bolt was despawned, segment will be cleaned up automatically
            continue;
        };

        // If bolt is expired, hide segment
        if bolt.is_expired() {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let segment_idx = segment.index as usize;

        // Calculate how much of this segment should be visible based on bolt distance
        let segment_start_dist = bolt.segment_start_distance(segment_idx);
        let segment_end_dist = bolt.segment_end_distance(segment_idx);
        let segment_len = bolt.segment_length(segment_idx);

        // Only show segment if the bolt has reached it
        if bolt.distance < segment_start_dist {
            transform.scale = Vec3::ZERO;
            continue;
        }

        // Calculate visibility progress for this segment
        let segment_progress = if bolt.distance >= segment_end_dist {
            1.0
        } else if segment_len > 0.0 {
            (bolt.distance - segment_start_dist) / segment_len
        } else {
            1.0
        };

        // Calculate opacity based on overall bolt progress and segment position
        let base_opacity = bolt.current_opacity();
        let segment_fade = 1.0 - (segment_idx as f32 / bolt.segment_count as f32) * 0.3;
        let opacity = (base_opacity * segment_fade * segment_progress).clamp(0.0, 1.0);

        // Hide segments that are nearly invisible
        if opacity < 0.01 {
            transform.scale = Vec3::ZERO;
            continue;
        }

        // Get joint positions for this segment (in local coords relative to bolt center)
        // These are in XY space; we need to map to XZ for 3D
        let start_pos = bolt.joint_position(segment_idx);
        let end_pos = bolt.joint_position(segment_idx + 1);

        // Interpolate end position based on progress
        let current_end = start_pos + (end_pos - start_pos) * segment_progress;

        // Calculate segment midpoint and length
        let midpoint = (start_pos + current_end) / 2.0;
        let segment_vec = current_end - start_pos;
        let length = segment_vec.length().max(0.01);
        let rotation = segment_vec.y.atan2(segment_vec.x);

        // Update local transform - map XY to XZ plane
        transform.translation.x = midpoint.x;
        transform.translation.y = 0.01 + segment_idx as f32 * 0.001; // Slight Y offset
        transform.translation.z = midpoint.y; // Y -> Z for 3D

        // Rotate around Y axis (vertical) instead of Z
        transform.rotation = Quat::from_rotation_y(-rotation);

        // Calculate thickness (tapers from center to tip)
        let thickness = bolt.thickness_at_segment(segment.index) * 0.02; // Scale for 3D

        // Update size via transform scale
        // Scale factor for opacity effect
        let scale_factor = opacity;
        transform.scale = Vec3::new(length * 0.03 * scale_factor, thickness * scale_factor, thickness * scale_factor);
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

/// Number of trail segments per orbital particle
const TRAIL_SEGMENT_COUNT: usize = 36;

/// Spawns orbital particles around Whisper at random intervals.
/// Particles orbit in true 3D space around the whisper core.
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
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(game_meshes) = game_meshes else { return };
    let Some(game_materials) = game_materials else { return };
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
            &game_meshes,
            &game_materials,
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
            &game_meshes,
            &game_materials,
            &mut rng,
        );

        timer.reset_with_random_duration(&mut rng);
    }
}

/// Helper function to spawn an orbital particle as a child of the whisper entity.
/// Uses 3D meshes and positions in true 3D space.
fn spawn_orbital_particle_as_child(
    commands: &mut Commands,
    whisper_entity: Entity,
    game_meshes: &GameMeshes,
    game_materials: &GameMaterials,
    rng: &mut impl Rng,
) {
    // Random orbital parameters - orbit in 3D space around core
    let radius = rng.gen_range(0.3..0.7); // 3D world units
    let period = rng.gen_range(0.19..0.38); // Fast orbit
    let inclination = rng.gen_range(0.3..1.2); // ~17 to ~69 degrees
    let ascending_node = rng.gen_range(0.0..std::f32::consts::TAU);
    let size = rng.gen_range(0.03..0.06); // 3D world units
    let phase = rng.gen_range(0.0..std::f32::consts::TAU);

    let particle = OrbitalParticle::new(
        radius,
        period,
        phase,
        inclination,
        ascending_node,
        size,
    );

    // Trail with samples for appearance
    let trail = ParticleTrail::new(TRAIL_SEGMENT_COUNT, 0.015);

    // Calculate initial position (XY -> XZ mapping)
    let (pos_2d, z_depth) = particle.calculate_position();

    // Spawn particle as child of whisper
    commands.entity(whisper_entity).with_children(|parent| {
        parent
            .spawn((
                particle,
                trail,
                // Map XY to XZ plane: x -> x, y -> z, z_depth -> y
                Transform::from_translation(Vec3::new(pos_2d.x, z_depth * 0.1, pos_2d.y)),
                Visibility::default(),
            ))
            .with_children(|particle_parent| {
                // Pre-spawn trail segment meshes
                for i in 0..TRAIL_SEGMENT_COUNT {
                    particle_parent.spawn((
                        TrailSegment { index: i },
                        Mesh3d(game_meshes.orbital_particle.clone()),
                        MeshMaterial3d(game_materials.orbital_particle.clone()),
                        Transform::from_translation(Vec3::new(0.0, -0.001 * i as f32, 0.0)),
                    ));
                }
            });
    });
}

/// Updates orbital particle positions and trails.
/// Uses 3D transforms with XZ as ground plane.
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

        // Calculate new position (in 2D orbital plane)
        let (position_2d, z_depth) = particle.calculate_position();

        // Update trail sampling
        trail.tick(delta_secs);
        if trail.should_sample() {
            trail.record_position(position_2d, z_depth);
            trail.reset_sample_timer();
        }

        // Update particle transform - map XY orbital plane to XZ 3D plane
        // x -> x, y -> z, z_depth -> y (height)
        transform.translation.x = position_2d.x;
        transform.translation.y = z_depth * 0.1; // Height based on orbital depth
        transform.translation.z = position_2d.y;

        // Scale based on z (pseudo-perspective: slightly larger when closer)
        let normalized_z = (z_depth / particle.radius).clamp(-1.0, 1.0);
        let perspective_scale = 1.0 - normalized_z * 0.1;
        transform.scale = Vec3::splat(perspective_scale);
    }
}

/// Renders particle trails by positioning trail segments between recorded positions.
/// Each segment following the head is progressively smaller for fade effect.
/// Uses 3D transforms with XZ as ground plane.
/// Runs in GameSet::Effects (after update_orbital_particles)
pub fn render_particle_trails(
    particle_query: Query<(&ParticleTrail, &OrbitalParticle), Without<TrailSegment>>,
    mut segment_query: Query<(&TrailSegment, &ChildOf, &mut Transform)>,
) {
    for (segment, child_of, mut transform) in segment_query.iter_mut() {
        // Get parent particle data
        let Ok((trail, particle)) = particle_query.get(child_of.parent()) else {
            continue;
        };

        let idx = segment.index;

        // Not enough trail points yet - hide segment
        if idx + 1 >= trail.positions.len() {
            transform.scale = Vec3::ZERO;
            continue;
        }

        // Trail positions are in 2D orbital space (XY), need to map to 3D (XZ + Y height)
        let (particle_pos, _) = particle.calculate_position();
        let start = trail.positions[idx] - particle_pos;
        let end = trail.positions[idx + 1] - particle_pos;
        let start_z = trail.z_depths[idx];
        let end_z = trail.z_depths[idx + 1];

        // Segment geometry in 2D orbital plane
        let midpoint = (start + end) / 2.0;
        let avg_z = (start_z + end_z) / 2.0;

        // Update transform - map XY orbital plane to XZ 3D plane
        transform.translation.x = midpoint.x;
        transform.translation.y = avg_z * 0.1 - 0.001 * idx as f32; // Height with slight offset
        transform.translation.z = midpoint.y;

        // Calculate segment brightness for scale-based fade effect
        let depth_brightness = 0.65 - (avg_z / particle.radius).clamp(-1.0, 1.0) * 0.35;
        let segment_brightness = particle.segment_opacity(idx, TRAIL_SEGMENT_COUNT);
        let brightness = (segment_brightness * depth_brightness).clamp(0.0, 1.0);

        // Thickness tapers along trail, scaled by brightness
        let progress = idx as f32 / (TRAIL_SEGMENT_COUNT - 1) as f32;
        let base_size = particle.size * 0.5 * (1.0 - progress);
        let size = (base_size * brightness).max(0.001);

        transform.scale = Vec3::splat(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::pbr::StandardMaterial;
    use std::time::Duration;

    fn setup_test_app_with_game_resources() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();

        // Set up game meshes and materials
        {
            let world = app.world_mut();
            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                    let game_meshes = GameMeshes::new(&mut meshes);
                    let game_materials = GameMaterials::new(&mut materials);
                    world.insert_resource(game_meshes);
                    world.insert_resource(game_materials);
                });
            });
        }
        app
    }

    #[test]
    fn test_spawn_whisper_drop_creates_entity() {
        let mut app = setup_test_app_with_game_resources();
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
    fn test_spawn_whisper_drop_position_within_range_of_player() {
        // Run multiple times to verify random positions are within expected range of origin
        for _ in 0..20 {
            let mut app = setup_test_app_with_game_resources();
            app.add_systems(Startup, spawn_whisper_drop);

            app.update();

            let mut query = app.world_mut().query::<(&WhisperDrop, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                let pos = transform.translation;
                // Use XZ plane for 3D distance
                let distance = (pos.x * pos.x + pos.z * pos.z).sqrt();

                // Whisper should spawn within 8 world units of origin
                assert!(
                    distance <= 8.0,
                    "Whisper spawned at distance {} which exceeds 8 units",
                    distance
                );

                // Whisper should spawn at least some minimum distance away
                assert!(
                    distance >= 3.0,
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

        // Create player at (0, 0.5, 0) on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        // Create WhisperDrop at (0.5, 1.0, 0.5) - within 3D pickup radius on XZ plane
        app.world_mut().spawn((
            WhisperDrop::default(),
            Transform::from_translation(Vec3::new(0.5, 1.0, 0.5)),
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

        // Create player at (0, 0.5, 0) on XZ plane
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        // Create WhisperDrop far away on XZ plane
        app.world_mut().spawn((
            WhisperDrop::default(),
            Transform::from_translation(Vec3::new(10.0, 1.0, 10.0)),
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

        // Create WhisperCompanion at (50, 1.5, 60) on XZ plane (Y is height)
        app.world_mut().spawn((
            WhisperCompanion::default(),
            Transform::from_translation(Vec3::new(50.0, 1.5, 60.0)),
        ));

        app.update();

        // Verify WeaponOrigin was updated - should use XZ plane (X and Z)
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
        let mut app = setup_test_app_with_game_resources();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify WhisperDrop entity was spawned with 3D PointLight
        let mut query = app.world_mut().query::<(&WhisperDrop, &PointLight)>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1, "WhisperDrop should have PointLight component");
    }

    #[test]
    fn test_spawn_whisper_drop_light_properties() {
        let mut app = setup_test_app_with_game_resources();
        app.add_systems(Startup, spawn_whisper_drop);

        app.update();

        // Verify the light properties
        let mut query = app.world_mut().query::<&PointLight>();
        let light = query.single(app.world()).unwrap();

        // Drop has dimmer light (half intensity)
        assert_eq!(light.intensity, WHISPER_LIGHT_INTENSITY * 0.5);
        assert_eq!(light.radius, WHISPER_LIGHT_RADIUS);
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
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, animate_lightning_bolts);

        // Create a lightning bolt at origin facing right (angle = 0)
        let center = Vec3::new(0.0, 0.0, 0.0);
        let bolt = LightningBolt::new(0.0, 42, center, 0.5, 1.0);
        let bolt_entity = app
            .world_mut()
            .spawn((bolt, Transform::from_translation(center), Visibility::default()))
            .id();

        // Create a segment for this bolt using 3D mesh
        let mesh_handle = {
            let game_meshes = app.world().resource::<GameMeshes>();
            game_meshes.lightning_segment.clone()
        };
        let material_handle = {
            let game_materials = app.world().resource::<GameMaterials>();
            game_materials.lightning.clone()
        };

        app.world_mut().spawn((
            LightningSegment {
                index: 0,
                bolt_entity,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
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
        app.add_systems(Update, animate_lightning_bolts);

        // Create a lightning bolt that's already expired (distance >= max_distance)
        let center = Vec3::ZERO;
        let mut bolt = LightningBolt::new(0.0, 42, center, 0.5, 1.0);
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
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, animate_lightning_bolts);

        // Get mesh and material for segment
        let mesh_handle = {
            let game_meshes = app.world().resource::<GameMeshes>();
            game_meshes.lightning_segment.clone()
        };
        let material_handle = {
            let game_materials = app.world().resource::<GameMaterials>();
            game_materials.lightning.clone()
        };

        // Create an expired lightning bolt with a child segment
        let center = Vec3::ZERO;
        let mut bolt = LightningBolt::new(0.0, 42, center, 0.5, 1.0);
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
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
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
        let mut app = setup_test_app_with_game_resources();
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
        let mut app = setup_test_app_with_game_resources();

        // Create a whisper entity to be the parent
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::new(50.0, 1.5, 50.0)),
            ))
            .id();

        // Spawn bolts as children of the whisper entity
        {
            let world = app.world_mut();
            world.resource_scope(|world, game_meshes: Mut<GameMeshes>| {
                world.resource_scope(|world, game_materials: Mut<GameMaterials>| {
                    let mut commands = world.commands();
                    spawn_bolts_as_children(
                        &mut commands,
                        whisper_entity,
                        &mut rand::thread_rng(),
                        &game_meshes,
                        &game_materials,
                    );
                });
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
        let mut app = setup_test_app_with_game_resources();

        // Create a whisper entity at some world position
        let whisper_entity = app
            .world_mut()
            .spawn((
                WhisperCompanion::default(),
                Transform::from_translation(Vec3::new(50.0, 1.5, 75.0)),
            ))
            .id();

        // Spawn bolts as children
        {
            let world = app.world_mut();
            world.resource_scope(|world, game_meshes: Mut<GameMeshes>| {
                world.resource_scope(|world, game_materials: Mut<GameMaterials>| {
                    let mut commands = world.commands();
                    spawn_bolts_as_children(
                        &mut commands,
                        whisper_entity,
                        &mut rand::thread_rng(),
                        &game_meshes,
                        &game_materials,
                    );
                });
            });
        }
        app.update();

        // Verify bolts store local center (0, 0, 0) not world position
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
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, spawn_orbital_particles);

        // Create a WhisperCompanion with spawn timer set to trigger immediately
        app.world_mut().spawn((
            WhisperCompanion::default(),
            OrbitalParticleSpawnTimer {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                min_interval: 0.5,
                max_interval: 1.0,
            },
            Transform::from_translation(Vec3::new(50.0, 1.5, 50.0)),
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
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, spawn_orbital_particles);

        // Create a WhisperCompanion with spawn timer set to trigger immediately
        app.world_mut().spawn((
            WhisperCompanion::default(),
            OrbitalParticleSpawnTimer {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                min_interval: 0.5,
                max_interval: 1.0,
            },
            Transform::from_translation(Vec3::new(50.0, 1.5, 50.0)),
            Visibility::default(),
        ));

        // Tick timer so it triggers
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();

        // Verify trail segments were spawned
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
        let mut app = setup_test_app_with_game_resources();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_systems(Update, spawn_orbital_particles);

        // Create a WhisperDrop (not companion) with spawn timer
        app.world_mut().spawn((
            WhisperDrop::default(),
            OrbitalParticleSpawnTimer {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                min_interval: 0.5,
                max_interval: 1.0,
            },
            Transform::from_translation(Vec3::new(50.0, 1.5, 50.0)),
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
