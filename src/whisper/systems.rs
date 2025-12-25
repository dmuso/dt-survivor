use bevy::prelude::*;
use bevy_hanabi::prelude::{
    Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier, EffectAsset, ExprWriter,
    Gradient as HanabiGradient, ParticleEffect, SetAttributeModifier, SetPositionCircleModifier,
    SetVelocitySphereModifier, ShapeDimension, SizeOverLifetimeModifier, SpawnerSettings,
};
use bevy_lit::prelude::*;
use rand::Rng;

use crate::player::components::Player;
use crate::whisper::components::*;
use crate::whisper::events::*;
use crate::whisper::resources::*;

/// Color constants for Whisper visual effects
const WHISPER_LIGHT_COLOR: Color = Color::srgb(0.4, 0.8, 1.0); // Cyan-white
const WHISPER_LIGHT_INTENSITY: f32 = 4.0;
const WHISPER_LIGHT_OUTER_RADIUS: f32 = 240.0;
const WHISPER_LIGHT_FALLOFF: f32 = 2.0;

/// Particle effect constants
const SPARK_SPAWN_RATE: f32 = 120.0; // particles per second
const SPARK_LIFETIME: f32 = 0.35; // seconds
const SPARK_SPEED: f32 = 180.0; // pixels per second
const SPARK_SIZE_START: f32 = 4.0; // pixels
const SPARK_SIZE_END: f32 = 0.0; // pixels

/// Creates and inserts the Whisper spark particle effect asset.
/// Should be called once on startup. Silently skips if HanabiPlugin is not loaded.
pub fn setup_whisper_particle_effect(
    mut commands: Commands,
    effects: Option<ResMut<Assets<EffectAsset>>>,
) {
    let Some(mut effects) = effects else {
        return; // HanabiPlugin not loaded, skip particle setup
    };
    // Create a gradient for particle color (cyan-white to transparent)
    let mut color_gradient = HanabiGradient::new();
    color_gradient.add_key(0.0, Vec4::new(0.4, 0.9, 1.0, 1.0)); // Bright cyan
    color_gradient.add_key(0.5, Vec4::new(0.8, 0.95, 1.0, 0.6)); // Lighter cyan
    color_gradient.add_key(1.0, Vec4::new(1.0, 1.0, 1.0, 0.0)); // Fade to transparent

    // Create a gradient for particle size (shrinks over lifetime)
    let mut size_gradient = HanabiGradient::new();
    size_gradient.add_key(0.0, Vec3::splat(SPARK_SIZE_START));
    size_gradient.add_key(1.0, Vec3::splat(SPARK_SIZE_END));

    let writer = ExprWriter::new();

    // Position: spawn at center (2D, so use Z axis for the circle plane)
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Z).expr(), // Circle in XY plane
        radius: writer.lit(8.0).expr(),   // Small initial radius
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

/// Spawns Whisper drop at random position within 4000x4000px of origin.
/// Runs on OnEnter(GameState::InGame)
pub fn spawn_whisper_drop(mut commands: Commands) {
    let mut rng = rand::thread_rng();

    // Generate random position within 4000x4000px area centered at origin
    let x = rng.gen_range(-2000.0..2000.0);
    let y = rng.gen_range(-2000.0..2000.0);
    let position = Vec3::new(x, y, 0.5);

    // Spawn the Whisper drop with visual elements
    commands
        .spawn((
            WhisperDrop::default(),
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
            // Outer glow (larger, more transparent) - using emissive color for bloom
            parent.spawn((
                WhisperOuterGlow,
                Sprite::from_color(
                    Color::srgba(0.3, 0.5, 1.0, 0.4),
                    Vec2::new(48.0, 48.0),
                ),
                Transform::from_xyz(0.0, 0.0, -0.1),
            ));

            // Core glow (inner bright part) - using HDR color for bloom effect
            parent.spawn((
                WhisperCoreGlow,
                Sprite::from_color(
                    Color::srgb(2.0, 2.5, 3.0), // HDR color for bloom
                    Vec2::new(16.0, 16.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.0),
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

        let mut companion_entity = commands.spawn((
            companion,
            ArcBurstTimer::default(),
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
            // Outer glow with HDR color for bloom
            parent.spawn((
                WhisperOuterGlow,
                Sprite::from_color(
                    Color::srgba(1.5, 2.0, 3.0, 0.4), // HDR for bloom
                    Vec2::new(48.0, 48.0),
                ),
                Transform::from_xyz(0.0, 0.0, -0.1),
            ));

            // Core glow with HDR color for bloom
            parent.spawn((
                WhisperCoreGlow,
                Sprite::from_color(
                    Color::srgb(3.0, 3.5, 4.0), // Bright HDR for strong bloom
                    Vec2::new(16.0, 16.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.0),
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

        for _ in 0..arc_count {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(20.0..40.0);
            let dir = Vec2::new(angle.cos(), angle.sin());
            let pos = Vec3::new(
                center.x + dir.x * distance,
                center.y + dir.y * distance,
                center.z + 0.1,
            );

            // Spawn a small lightning arc sprite with HDR color for bloom
            commands.spawn((
                WhisperArc::new(0.06),
                Sprite::from_color(
                    Color::srgba(2.5, 3.5, 4.0, 0.9), // HDR cyan-white for bloom
                    Vec2::new(12.0, 4.0),
                ),
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_z(angle)),
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_spawn_whisper_drop_creates_entity() {
        let mut app = App::new();
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
    fn test_spawn_whisper_drop_position_in_valid_range() {
        // Run multiple times to verify random positions are in valid range
        for _ in 0..10 {
            let mut app = App::new();
            app.add_systems(Startup, spawn_whisper_drop);

            app.update();

            let mut query = app.world_mut().query::<(&WhisperDrop, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                let pos = transform.translation;
                assert!(
                    pos.x >= -2000.0 && pos.x <= 2000.0,
                    "X position {} out of range",
                    pos.x
                );
                assert!(
                    pos.y >= -2000.0 && pos.y <= 2000.0,
                    "Y position {} out of range",
                    pos.y
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
}
