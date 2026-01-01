use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Inferno Pulse spell
pub const INFERNO_PULSE_RADIUS: f32 = 6.0;
pub const INFERNO_PULSE_WAVE_EXPANSION_RATE: f32 = 20.0; // Units per second
pub const INFERNO_PULSE_VISUAL_HEIGHT: f32 = 0.2;

/// Get the fire element color for visual effects
pub fn inferno_pulse_color() -> Color {
    Element::Fire.color()
}

/// Visual component for the expanding pulse wave effect.
/// This is purely visual - damage is applied instantly when pulse triggers.
#[derive(Component, Debug, Clone)]
pub struct InfernoPulseWave {
    /// Current radius of the visual wave
    pub current_radius: f32,
    /// Maximum radius the wave will expand to
    pub max_radius: f32,
    /// Center position on XZ plane
    pub center: Vec2,
}

impl InfernoPulseWave {
    pub fn new(center: Vec2, max_radius: f32) -> Self {
        Self {
            current_radius: 0.0,
            max_radius,
            center,
        }
    }

    /// Check if the wave has finished expanding
    pub fn is_finished(&self) -> bool {
        self.current_radius >= self.max_radius
    }

    /// Expand the wave
    pub fn expand(&mut self, delta_secs: f32) {
        self.current_radius = (self.current_radius + INFERNO_PULSE_WAVE_EXPANSION_RATE * delta_secs)
            .min(self.max_radius);
    }
}

/// System that expands the visual pulse waves over time
pub fn animate_inferno_pulse_wave_system(
    mut wave_query: Query<&mut InfernoPulseWave>,
    time: Res<Time>,
) {
    for mut wave in wave_query.iter_mut() {
        wave.expand(time.delta_secs());
    }
}

/// System that updates the visual scale of pulse waves
pub fn inferno_pulse_wave_visual_system(
    mut wave_query: Query<(&InfernoPulseWave, &mut Transform)>,
) {
    for (wave, mut transform) in wave_query.iter_mut() {
        transform.scale = Vec3::splat(wave.current_radius.max(0.1));
    }
}

/// System that despawns finished pulse wave visuals
pub fn cleanup_inferno_pulse_wave_system(
    mut commands: Commands,
    wave_query: Query<(Entity, &InfernoPulseWave)>,
) {
    for (entity, wave) in wave_query.iter() {
        if wave.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Inferno Pulse (Hellfire) spell - spawns a damage nova at the caster position.
/// Unlike Fire Nova which expands over time, Inferno Pulse deals instant damage.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_inferno_pulse(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform, &Enemy)>,
    damage_events: &mut MessageWriter<DamageEvent>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_inferno_pulse_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        enemy_query,
        damage_events,
        game_meshes,
        game_materials,
    );
}

/// Cast Inferno Pulse (Hellfire) spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_inferno_pulse_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform, &Enemy)>,
    damage_events: &mut MessageWriter<DamageEvent>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let pulse_center = from_xz(spawn_position);

    // Damage all enemies within pulse radius instantly
    for (enemy_entity, enemy_transform, _) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = pulse_center.distance(enemy_pos);

        if distance <= INFERNO_PULSE_RADIUS {
            damage_events.write(DamageEvent::new(enemy_entity, damage));
        }
    }

    // Spawn visual wave effect
    let wave = InfernoPulseWave::new(pulse_center, INFERNO_PULSE_RADIUS);
    let wave_pos = Vec3::new(spawn_position.x, INFERNO_PULSE_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.fire_nova.clone()),
            Transform::from_translation(wave_pos).with_scale(Vec3::splat(0.1)),
            wave,
        ));
    } else {
        commands.spawn((
            Transform::from_translation(wave_pos),
            wave,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod inferno_pulse_wave_tests {
        use super::*;

        #[test]
        fn test_inferno_pulse_wave_new() {
            let center = Vec2::new(10.0, 20.0);
            let wave = InfernoPulseWave::new(center, 6.0);

            assert_eq!(wave.center, center);
            assert_eq!(wave.max_radius, 6.0);
            assert_eq!(wave.current_radius, 0.0);
            assert!(!wave.is_finished());
        }

        #[test]
        fn test_inferno_pulse_wave_expand() {
            let mut wave = InfernoPulseWave::new(Vec2::ZERO, 10.0);

            wave.expand(0.1); // 20 * 0.1 = 2.0 units
            assert!((wave.current_radius - 2.0).abs() < 0.01);

            wave.expand(0.1);
            assert!((wave.current_radius - 4.0).abs() < 0.01);
        }

        #[test]
        fn test_inferno_pulse_wave_caps_at_max() {
            let mut wave = InfernoPulseWave::new(Vec2::ZERO, 5.0);

            wave.expand(10.0); // Would be 200 units, capped at 5.0
            assert_eq!(wave.current_radius, 5.0);
            assert!(wave.is_finished());
        }

        #[test]
        fn test_inferno_pulse_wave_is_finished() {
            let mut wave = InfernoPulseWave::new(Vec2::ZERO, 5.0);
            assert!(!wave.is_finished());

            wave.current_radius = 5.0;
            assert!(wave.is_finished());
        }

        #[test]
        fn test_inferno_pulse_uses_fire_element_color() {
            let color = inferno_pulse_color();
            assert_eq!(color, Element::Fire.color());
            assert_eq!(color, Color::srgb_u8(255, 128, 0));
        }
    }

    mod animate_inferno_pulse_wave_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_animate_inferno_pulse_wave_expands() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                InfernoPulseWave::new(Vec2::ZERO, 10.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(animate_inferno_pulse_wave_system);

            let wave = app.world().get::<InfernoPulseWave>(entity).unwrap();
            // 20 * 0.1 = 2.0 units
            assert!((wave.current_radius - 2.0).abs() < 0.1);
        }
    }

    mod inferno_pulse_wave_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_inferno_pulse_wave_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut wave = InfernoPulseWave::new(Vec2::ZERO, 10.0);
            wave.current_radius = 5.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                wave,
            )).id();

            let _ = app.world_mut().run_system_once(inferno_pulse_wave_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(5.0));
        }

        #[test]
        fn test_inferno_pulse_wave_visual_minimum_scale() {
            let mut app = App::new();

            let wave = InfernoPulseWave::new(Vec2::ZERO, 10.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                wave,
            )).id();

            let _ = app.world_mut().run_system_once(inferno_pulse_wave_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }

    mod cleanup_inferno_pulse_wave_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_cleanup_inferno_pulse_wave_despawns_finished() {
            let mut app = App::new();

            let mut wave = InfernoPulseWave::new(Vec2::ZERO, 5.0);
            wave.current_radius = 5.0; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                wave,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_inferno_pulse_wave_system);

            assert!(app.world().get_entity(entity).is_err(), "Wave should be despawned");
        }

        #[test]
        fn test_cleanup_inferno_pulse_wave_survives_before_finished() {
            let mut app = App::new();

            let mut wave = InfernoPulseWave::new(Vec2::ZERO, 10.0);
            wave.current_radius = 5.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                wave,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_inferno_pulse_wave_system);

            assert!(app.world().get_entity(entity).is_ok(), "Wave should still exist");
        }
    }

    mod fire_inferno_pulse_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        /// Resource to hold spawn position for test system
        #[derive(Resource)]
        struct TestSpawnPos(Vec3);

        /// Resource to hold damage value for test system
        #[derive(Resource)]
        struct TestDamage(f32);

        /// Test system that fires inferno pulse at the configured position
        fn test_fire_inferno_pulse_system(
            mut commands: Commands,
            spawn_pos: Res<TestSpawnPos>,
            enemy_query: Query<(Entity, &Transform, &Enemy)>,
            mut damage_events: MessageWriter<DamageEvent>,
        ) {
            let spell = Spell::new(SpellType::Hellfire);
            fire_inferno_pulse(
                &mut commands,
                &spell,
                spawn_pos.0,
                &enemy_query,
                &mut damage_events,
                None,
                None,
            );
        }

        /// Test system that fires inferno pulse with explicit damage
        fn test_fire_inferno_pulse_with_damage_system(
            mut commands: Commands,
            spawn_pos: Res<TestSpawnPos>,
            test_damage: Res<TestDamage>,
            enemy_query: Query<(Entity, &Transform, &Enemy)>,
            mut damage_events: MessageWriter<DamageEvent>,
        ) {
            let spell = Spell::new(SpellType::Hellfire);
            fire_inferno_pulse_with_damage(
                &mut commands,
                &spell,
                test_damage.0,
                spawn_pos.0,
                &enemy_query,
                &mut damage_events,
                None,
                None,
            );
        }

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_fire_inferno_pulse_spawns_visual_wave() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(0.0, 0.5, 0.0)));
            app.add_systems(Update, test_fire_inferno_pulse_system);

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            // Check wave was spawned
            let mut wave_query = app.world_mut().query::<&InfernoPulseWave>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1, "One visual wave should be spawned");
            assert_eq!(waves[0].center, Vec2::new(0.0, 0.0));
        }

        #[test]
        fn test_fire_inferno_pulse_damages_enemies_in_radius() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(0.0, 0.5, 0.0)));

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
            app.add_systems(Update, (test_fire_inferno_pulse_system, count_damage_events).chain());

            // Create enemy within radius (XZ distance = 3)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_fire_inferno_pulse_ignores_enemies_outside_radius() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(0.0, 0.5, 0.0)));

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
            app.add_systems(Update, (test_fire_inferno_pulse_system, count_damage_events).chain());

            // Create enemy outside radius (XZ distance = 10, radius is 6)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_fire_inferno_pulse_damages_multiple_enemies() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(0.0, 0.5, 0.0)));

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
            app.add_systems(Update, (test_fire_inferno_pulse_system, count_damage_events).chain());

            // Create 3 enemies within radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn test_fire_inferno_pulse_uses_xz_plane_ignores_y() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(0.0, 0.5, 0.0)));

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
            app.add_systems(Update, (test_fire_inferno_pulse_system, count_damage_events).chain());

            // Create enemy close on XZ plane but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }

        #[test]
        fn test_fire_inferno_pulse_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(0.0, 0.5, 0.0)));
            app.insert_resource(TestDamage(100.0));

            #[derive(Resource, Clone)]
            struct LastDamage(Arc<std::sync::Mutex<f32>>);

            fn track_damage(
                mut events: MessageReader<DamageEvent>,
                last_damage: Res<LastDamage>,
            ) {
                for event in events.read() {
                    *last_damage.0.lock().unwrap() = event.amount;
                }
            }

            let last_damage = LastDamage(Arc::new(std::sync::Mutex::new(0.0)));
            app.insert_resource(last_damage.clone());
            app.add_systems(Update, (test_fire_inferno_pulse_with_damage_system, track_damage).chain());

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(*last_damage.0.lock().unwrap(), 100.0);
        }

        #[test]
        fn test_fire_inferno_pulse_wave_at_correct_position() {
            let mut app = setup_test_app();
            app.insert_resource(TestSpawnPos(Vec3::new(5.0, 0.5, 10.0)));
            app.add_systems(Update, test_fire_inferno_pulse_system);

            // Create enemy (doesn't matter where for this test, we just need one to trigger wave)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            // Check wave position
            let mut wave_query = app.world_mut().query::<(&InfernoPulseWave, &Transform)>();
            let waves: Vec<_> = wave_query.iter(app.world()).collect();
            assert_eq!(waves.len(), 1);
            assert_eq!(waves[0].0.center, Vec2::new(5.0, 10.0));
            assert_eq!(waves[0].1.translation.x, 5.0);
            assert_eq!(waves[0].1.translation.z, 10.0);
        }
    }
}
