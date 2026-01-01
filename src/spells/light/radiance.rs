use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Radiance spell
pub const RADIANCE_PULSE_RADIUS: f32 = 8.0;
pub const RADIANCE_PULSE_INTERVAL: f32 = 0.5;
pub const RADIANCE_VISUAL_HEIGHT: f32 = 0.1;

/// Get the light element color for visual effects (white/gold)
pub fn radiance_color() -> Color {
    Element::Light.color()
}

/// RadianceAura component - a pulsing radiant energy aura centered on the player
/// that damages all enemies within the pulse radius at regular intervals.
#[derive(Component, Debug, Clone)]
pub struct RadianceAura {
    /// Center position on XZ plane (follows player/Whisper)
    pub center: Vec2,
    /// Radius of the pulse effect
    pub pulse_radius: f32,
    /// Damage dealt per pulse
    pub damage: f32,
    /// Timer for pulse intervals
    pub pulse_timer: Timer,
}

impl RadianceAura {
    pub fn new(center: Vec2, damage: f32, radius: f32) -> Self {
        Self {
            center,
            pulse_radius: radius,
            damage,
            pulse_timer: Timer::from_seconds(RADIANCE_PULSE_INTERVAL, TimerMode::Repeating),
        }
    }

    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage(), RADIANCE_PULSE_RADIUS)
    }

    /// Check if a position (XZ plane) is inside the aura radius
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.pulse_radius
    }
}

/// Marker component for the visual pulse effect that expands outward
#[derive(Component, Debug, Clone)]
pub struct RadiancePulseVisual {
    /// Current radius of the visual effect
    pub current_radius: f32,
    /// Maximum radius before despawn
    pub max_radius: f32,
    /// Expansion speed
    pub expansion_speed: f32,
    /// Center position on XZ plane
    pub center: Vec2,
}

impl RadiancePulseVisual {
    pub fn new(center: Vec2, max_radius: f32) -> Self {
        Self {
            current_radius: 0.0,
            max_radius,
            expansion_speed: max_radius * 2.0, // Complete expansion in 0.5 seconds
            center,
        }
    }
}

/// System that updates RadianceAura pulse timers and spawns pulse visuals
pub fn radiance_pulse_system(
    mut commands: Commands,
    time: Res<Time>,
    mut aura_query: Query<(Entity, &mut RadianceAura, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (_aura_entity, mut aura, aura_transform) in aura_query.iter_mut() {
        // Update aura center to follow the attached entity
        aura.center = from_xz(aura_transform.translation);

        aura.pulse_timer.tick(time.delta());

        if aura.pulse_timer.just_finished() {
            // Damage all enemies within pulse radius
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);
                if aura.contains(enemy_pos) {
                    damage_events.write(DamageEvent::new(enemy_entity, aura.damage));
                }
            }

            // Spawn visual pulse effect
            let pulse_visual = RadiancePulseVisual::new(aura.center, aura.pulse_radius);
            let visual_pos = Vec3::new(aura.center.x, RADIANCE_VISUAL_HEIGHT, aura.center.y);

            if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), game_materials.as_ref()) {
                commands.spawn((
                    Mesh3d(meshes.explosion.clone()),
                    MeshMaterial3d(materials.radiant_beam.clone()),
                    Transform::from_translation(visual_pos).with_scale(Vec3::splat(0.1)),
                    pulse_visual,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(visual_pos),
                    pulse_visual,
                ));
            }
        }
    }
}

/// System that updates RadiancePulseVisual expansion and despawns when complete
pub fn radiance_pulse_visual_system(
    mut commands: Commands,
    time: Res<Time>,
    mut visual_query: Query<(Entity, &mut RadiancePulseVisual, &mut Transform)>,
) {
    for (entity, mut visual, mut transform) in visual_query.iter_mut() {
        visual.current_radius += visual.expansion_speed * time.delta_secs();

        // Update scale based on current radius
        let scale = visual.current_radius / visual.max_radius * visual.max_radius;
        transform.scale = Vec3::splat(scale.max(0.1));

        // Despawn when fully expanded
        if visual.current_radius >= visual.max_radius {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Radiance spell - spawns a pulsing radiant aura centered on the spell origin.
/// `spawn_position` is Whisper's full 3D position (where the aura will be centered).
#[allow(clippy::too_many_arguments)]
pub fn fire_radiance(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_radiance_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Radiance spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage per pulse (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_radiance_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let aura_center = from_xz(spawn_position);
    let aura = RadianceAura::new(aura_center, damage, RADIANCE_PULSE_RADIUS);
    let aura_pos = Vec3::new(spawn_position.x, RADIANCE_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.radiant_beam.clone()),
            Transform::from_translation(aura_pos).with_scale(Vec3::splat(RADIANCE_PULSE_RADIUS)),
            aura,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(aura_pos),
            aura,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod radiance_aura_component_tests {
        use super::*;

        #[test]
        fn test_radiance_aura_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 15.0;
            let aura = RadianceAura::new(center, damage, 8.0);

            assert_eq!(aura.center, center);
            assert_eq!(aura.pulse_radius, 8.0);
            assert_eq!(aura.damage, damage);
            assert!(!aura.pulse_timer.just_finished());
        }

        #[test]
        fn test_radiance_aura_from_spell() {
            let spell = Spell::new(SpellType::Radiance);
            let center = Vec2::new(5.0, 15.0);
            let aura = RadianceAura::from_spell(center, &spell);

            assert_eq!(aura.center, center);
            assert_eq!(aura.pulse_radius, RADIANCE_PULSE_RADIUS);
            assert_eq!(aura.damage, spell.damage());
        }

        #[test]
        fn test_radiance_aura_pulse_timer_initial_state() {
            let aura = RadianceAura::new(Vec2::ZERO, 10.0, 8.0);
            assert!(!aura.pulse_timer.just_finished());
            assert_eq!(aura.pulse_timer.duration(), Duration::from_secs_f32(RADIANCE_PULSE_INTERVAL));
        }

        #[test]
        fn test_radiance_aura_contains_position_inside() {
            let aura = RadianceAura::new(Vec2::ZERO, 10.0, 8.0);
            assert!(aura.contains(Vec2::new(3.0, 0.0)));
            assert!(aura.contains(Vec2::new(0.0, 5.0)));
            assert!(aura.contains(Vec2::ZERO));
        }

        #[test]
        fn test_radiance_aura_does_not_contain_position_outside() {
            let aura = RadianceAura::new(Vec2::ZERO, 10.0, 8.0);
            assert!(!aura.contains(Vec2::new(10.0, 0.0)));
            assert!(!aura.contains(Vec2::new(0.0, 10.0)));
            assert!(!aura.contains(Vec2::new(9.0, 9.0)));
        }

        #[test]
        fn test_radiance_aura_contains_position_on_edge() {
            let aura = RadianceAura::new(Vec2::ZERO, 10.0, 8.0);
            assert!(aura.contains(Vec2::new(8.0, 0.0)));
            assert!(aura.contains(Vec2::new(0.0, 8.0)));
        }

        #[test]
        fn test_radiance_uses_light_element_color() {
            let color = radiance_color();
            assert_eq!(color, Element::Light.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 255)); // White
        }
    }

    mod radiance_pulse_visual_tests {
        use super::*;

        #[test]
        fn test_pulse_visual_new() {
            let center = Vec2::new(5.0, 10.0);
            let visual = RadiancePulseVisual::new(center, 8.0);

            assert_eq!(visual.center, center);
            assert_eq!(visual.current_radius, 0.0);
            assert_eq!(visual.max_radius, 8.0);
            assert!(visual.expansion_speed > 0.0);
        }

        #[test]
        fn test_pulse_visual_starts_at_zero_radius() {
            let visual = RadiancePulseVisual::new(Vec2::ZERO, 8.0);
            assert_eq!(visual.current_radius, 0.0);
        }
    }

    mod radiance_pulse_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_pulse_damages_enemies_in_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadianceAura::new(Vec2::ZERO, 10.0, 8.0),
            ));

            // Create enemy inside aura radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time past pulse interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(RADIANCE_PULSE_INTERVAL + 0.01));
            }

            // Run the pulse system then count events
            let _ = app.world_mut().run_system_once(radiance_pulse_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy inside radius should take damage");
        }

        #[test]
        fn test_pulse_does_not_damage_enemies_outside_radius() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Create aura at origin with radius 8
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadianceAura::new(Vec2::ZERO, 10.0, 8.0),
            ));

            // Create enemy outside aura radius (distance 10, radius 8)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            // Advance time past pulse interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(RADIANCE_PULSE_INTERVAL + 0.01));
            }

            // Run the pulse system then count events
            let _ = app.world_mut().run_system_once(radiance_pulse_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside radius should not take damage");
        }

        #[test]
        fn test_pulse_damages_multiple_enemies() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadianceAura::new(Vec2::ZERO, 10.0, 8.0),
            ));

            // Create 3 enemies inside radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            // Advance time past pulse interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(RADIANCE_PULSE_INTERVAL + 0.01));
            }

            // Run the pulse system then count events
            let _ = app.world_mut().run_system_once(radiance_pulse_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 3, "All 3 enemies inside radius should take damage");
        }

        #[test]
        fn test_pulse_spawns_visual_on_trigger() {
            let mut app = setup_test_app();

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadianceAura::new(Vec2::ZERO, 10.0, 8.0),
            ));

            // Advance time past pulse interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(RADIANCE_PULSE_INTERVAL + 0.01));
            }

            // Run the pulse system
            let _ = app.world_mut().run_system_once(radiance_pulse_system);

            // Should spawn a pulse visual
            let mut visual_query = app.world_mut().query::<&RadiancePulseVisual>();
            let visual_count = visual_query.iter(app.world()).count();
            assert_eq!(visual_count, 1, "Pulse should spawn a visual effect");
        }

        #[test]
        fn test_no_pulse_before_interval() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadianceAura::new(Vec2::ZERO, 10.0, 8.0),
            ));

            // Create enemy inside radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time but NOT past pulse interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(RADIANCE_PULSE_INTERVAL / 2.0));
            }

            // Run the pulse system then count events
            let _ = app.world_mut().run_system_once(radiance_pulse_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "No damage before pulse interval");
        }
    }

    mod radiance_pulse_visual_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_systems(Update, radiance_pulse_visual_system);
            app.init_resource::<Time>();
            app
        }

        #[test]
        fn test_visual_expands_over_time() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadiancePulseVisual::new(Vec2::ZERO, 8.0),
            )).id();

            // Initial radius should be 0
            {
                let visual = app.world().get::<RadiancePulseVisual>(visual_entity).unwrap();
                assert_eq!(visual.current_radius, 0.0);
            }

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            // Radius should have increased
            {
                let visual = app.world().get::<RadiancePulseVisual>(visual_entity).unwrap();
                assert!(visual.current_radius > 0.0, "Visual radius should increase over time");
            }
        }

        #[test]
        fn test_visual_despawns_at_max_radius() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadiancePulseVisual::new(Vec2::ZERO, 8.0),
            )).id();

            // Advance time past full expansion (0.5 seconds for max_radius * 2.0 speed)
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }
            app.update();

            // Visual should be despawned
            assert!(app.world().get_entity(visual_entity).is_err(), "Visual should despawn at max radius");
        }

        #[test]
        fn test_visual_survives_before_max_radius() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                RadiancePulseVisual::new(Vec2::ZERO, 8.0),
            )).id();

            // Advance time but not to max radius
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            // Visual should still exist
            assert!(app.world().get_entity(visual_entity).is_ok(), "Visual should exist before max radius");
        }

        #[test]
        fn test_visual_scale_increases_with_radius() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)).with_scale(Vec3::splat(0.1)),
                RadiancePulseVisual::new(Vec2::ZERO, 8.0),
            )).id();

            let initial_scale = app.world().get::<Transform>(visual_entity).unwrap().scale;

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            let new_scale = app.world().get::<Transform>(visual_entity).unwrap().scale;
            assert!(new_scale.x > initial_scale.x, "Scale should increase over time");
        }
    }

    mod fire_radiance_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_radiance_spawns_aura() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Radiance);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 radiance aura
            let mut query = app.world_mut().query::<&RadianceAura>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_radiance_at_spawn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Radiance);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadianceAura>();
            for aura in query.iter(app.world()) {
                // Aura center should match spawn XZ (10.0, 20.0)
                assert_eq!(aura.center.x, 10.0);
                assert_eq!(aura.center.y, 20.0); // Z maps to Y in Vec2
            }
        }

        #[test]
        fn test_fire_radiance_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Radiance);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadianceAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_radiance_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Radiance);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiance_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadianceAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_radiance_has_correct_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Radiance);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiance(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadianceAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.pulse_radius, RADIANCE_PULSE_RADIUS);
            }
        }
    }
}
