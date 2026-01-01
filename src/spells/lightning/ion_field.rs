use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Ion Field spell
pub const ION_FIELD_RADIUS: f32 = 6.0;
pub const ION_FIELD_DURATION: f32 = 5.0;
pub const ION_FIELD_TICK_INTERVAL: f32 = 0.25;
pub const ION_FIELD_VISUAL_HEIGHT: f32 = 0.1;

/// Get the lightning element color for visual effects (yellow)
pub fn ion_field_color() -> Color {
    Element::Lightning.color()
}

/// Ion Field component - a stationary electric field zone that damages enemies
/// while they remain inside. Enemies take damage at regular intervals (tick damage).
#[derive(Component, Debug, Clone)]
pub struct IonField {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the field
    pub radius: f32,
    /// Total duration the field exists
    pub duration: Timer,
    /// Damage per tick
    pub damage_per_tick: f32,
    /// Timer for damage ticks
    pub tick_timer: Timer,
}

impl IonField {
    pub fn new(center: Vec2, damage: f32, radius: f32, duration: f32) -> Self {
        Self {
            center,
            radius,
            duration: Timer::from_seconds(duration, TimerMode::Once),
            damage_per_tick: damage * ION_FIELD_TICK_INTERVAL, // Damage per second * tick interval
            tick_timer: Timer::from_seconds(ION_FIELD_TICK_INTERVAL, TimerMode::Repeating),
        }
    }

    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage(), ION_FIELD_RADIUS, ION_FIELD_DURATION)
    }

    /// Check if the field has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Check if a position (XZ plane) is inside the field
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }
}

/// Marker component for enemies currently inside an Ion Field.
/// Tracks which field the enemy is inside for proper cleanup.
#[derive(Component, Debug)]
pub struct InIonField {
    pub field_entity: Entity,
}

/// System that updates Ion Field duration timers and despawns expired fields
pub fn ion_field_duration_system(
    mut commands: Commands,
    time: Res<Time>,
    mut field_query: Query<(Entity, &mut IonField)>,
) {
    for (entity, mut field) in field_query.iter_mut() {
        field.duration.tick(time.delta());

        if field.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that tracks which enemies are inside Ion Fields.
/// Adds InIonField marker when enemy enters, removes when they exit.
pub fn ion_field_track_enemies_system(
    mut commands: Commands,
    field_query: Query<(Entity, &IonField)>,
    enemy_query: Query<(Entity, &Transform, Option<&InIonField>), With<Enemy>>,
) {
    for (enemy_entity, enemy_transform, in_field) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);

        // Find if enemy is inside any field
        let mut inside_field: Option<Entity> = None;
        for (field_entity, field) in field_query.iter() {
            if field.contains(enemy_pos) {
                inside_field = Some(field_entity);
                break;
            }
        }

        match (inside_field, in_field) {
            // Enemy entered a field
            (Some(field_entity), None) => {
                commands.entity(enemy_entity).insert(InIonField { field_entity });
            }
            // Enemy exited their field (or field despawned)
            (None, Some(_)) => {
                commands.entity(enemy_entity).remove::<InIonField>();
            }
            // Enemy moved to a different field
            (Some(new_field), Some(current)) if new_field != current.field_entity => {
                commands.entity(enemy_entity).insert(InIonField { field_entity: new_field });
            }
            // No change needed
            _ => {}
        }
    }
}

/// System that applies damage to enemies inside Ion Fields
pub fn ion_field_damage_system(
    time: Res<Time>,
    mut field_query: Query<(Entity, &mut IonField)>,
    enemy_query: Query<(Entity, &InIonField), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (field_entity, mut field) in field_query.iter_mut() {
        field.tick_timer.tick(time.delta());

        if field.tick_timer.just_finished() {
            // Damage all enemies inside this field
            for (enemy_entity, in_field) in enemy_query.iter() {
                if in_field.field_entity == field_entity {
                    damage_events.write(DamageEvent::new(enemy_entity, field.damage_per_tick));
                }
            }
        }
    }
}

/// Cleanup system that removes InIonField markers when the field entity no longer exists
pub fn ion_field_cleanup_markers_system(
    mut commands: Commands,
    field_query: Query<Entity, With<IonField>>,
    enemy_query: Query<(Entity, &InIonField)>,
) {
    for (enemy_entity, in_field) in enemy_query.iter() {
        // If the field entity no longer exists, remove the marker
        if field_query.get(in_field.field_entity).is_err() {
            commands.entity(enemy_entity).remove::<InIonField>();
        }
    }
}

/// Cast Ion Field spell - spawns a stationary electric field that damages enemies inside.
/// `spawn_position` is Whisper's full 3D position (where the field will be centered).
#[allow(clippy::too_many_arguments)]
pub fn fire_ion_field(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_ion_field_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Ion Field spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage per second (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_ion_field_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let field_center = from_xz(spawn_position);
    let field = IonField::new(field_center, damage, ION_FIELD_RADIUS, ION_FIELD_DURATION);
    let field_pos = Vec3::new(spawn_position.x, ION_FIELD_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(field_pos).with_scale(Vec3::splat(ION_FIELD_RADIUS)),
            field,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(field_pos),
            field,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod ion_field_component_tests {
        use super::*;

        #[test]
        fn test_ion_field_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 15.0;
            let field = IonField::new(center, damage, 6.0, 5.0);

            assert_eq!(field.center, center);
            assert_eq!(field.radius, 6.0);
            assert!(!field.is_expired());
            // damage_per_tick = damage * tick_interval = 15.0 * 0.25 = 3.75
            assert!((field.damage_per_tick - 3.75).abs() < 0.01);
        }

        #[test]
        fn test_ion_field_from_spell() {
            let spell = Spell::new(SpellType::Electrocute);
            let center = Vec2::new(5.0, 15.0);
            let field = IonField::from_spell(center, &spell);

            assert_eq!(field.center, center);
            assert_eq!(field.radius, ION_FIELD_RADIUS);
        }

        #[test]
        fn test_ion_field_tick_timer_initial_state() {
            let field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            assert!(!field.tick_timer.just_finished());
            assert_eq!(field.tick_timer.duration(), Duration::from_secs_f32(ION_FIELD_TICK_INTERVAL));
        }

        #[test]
        fn test_ion_field_duration_initial_state() {
            let field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            assert!(!field.is_expired());
            assert_eq!(field.duration.duration(), Duration::from_secs_f32(ION_FIELD_DURATION));
        }

        #[test]
        fn test_ion_field_expires_after_duration() {
            let mut field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            field.duration.tick(Duration::from_secs_f32(ION_FIELD_DURATION + 0.1));
            assert!(field.is_expired());
        }

        #[test]
        fn test_ion_field_does_not_expire_before_duration() {
            let mut field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            field.duration.tick(Duration::from_secs_f32(ION_FIELD_DURATION / 2.0));
            assert!(!field.is_expired());
        }

        #[test]
        fn test_ion_field_contains_position_inside() {
            let field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            assert!(field.contains(Vec2::new(3.0, 0.0)));
            assert!(field.contains(Vec2::new(0.0, 5.0)));
            assert!(field.contains(Vec2::ZERO));
        }

        #[test]
        fn test_ion_field_does_not_contain_position_outside() {
            let field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            assert!(!field.contains(Vec2::new(10.0, 0.0)));
            assert!(!field.contains(Vec2::new(0.0, 10.0)));
            assert!(!field.contains(Vec2::new(7.0, 7.0)));
        }

        #[test]
        fn test_ion_field_contains_position_on_edge() {
            let field = IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0);
            assert!(field.contains(Vec2::new(6.0, 0.0)));
            assert!(field.contains(Vec2::new(0.0, 6.0)));
        }

        #[test]
        fn test_ion_field_uses_lightning_element_color() {
            let color = ion_field_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }
    }

    mod ion_field_duration_system_tests {
        use super::*;

        #[test]
        fn test_field_despawns_after_duration() {
            let mut app = App::new();
            app.add_systems(Update, ion_field_duration_system);
            app.init_resource::<Time>();

            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ION_FIELD_DURATION + 0.1));
            }

            app.update();

            // Field should be despawned
            assert!(app.world().get_entity(field_entity).is_err());
        }

        #[test]
        fn test_field_survives_before_duration() {
            let mut app = App::new();
            app.add_systems(Update, ion_field_duration_system);
            app.init_resource::<Time>();

            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ION_FIELD_DURATION / 2.0));
            }

            app.update();

            // Field should still exist
            assert!(app.world().get_entity(field_entity).is_ok());
        }
    }

    mod ion_field_track_enemies_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_enemy_entering_field_receives_marker() {
            let mut app = setup_test_app();

            // Create field at origin
            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Create enemy inside field
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // Run the tracking system
            let _ = app.world_mut().run_system_once(ion_field_track_enemies_system);

            // Enemy should have InIonField marker
            let in_field = app.world().get::<InIonField>(enemy_entity);
            assert!(in_field.is_some(), "Enemy inside field should have InIonField marker");
            assert_eq!(in_field.unwrap().field_entity, field_entity);
        }

        #[test]
        fn test_enemy_outside_field_has_no_marker() {
            let mut app = setup_test_app();

            // Create field at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            ));

            // Create enemy outside field (distance = 10, radius = 6)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            )).id();

            // Run the tracking system
            let _ = app.world_mut().run_system_once(ion_field_track_enemies_system);

            // Enemy should NOT have InIonField marker
            assert!(app.world().get::<InIonField>(enemy_entity).is_none(),
                "Enemy outside field should not have InIonField marker");
        }

        #[test]
        fn test_enemy_exiting_field_loses_marker() {
            let mut app = setup_test_app();

            // Create field at origin
            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Create enemy with InIonField marker but outside field
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)), // Far outside
                InIonField { field_entity },
            )).id();

            // Run the tracking system
            let _ = app.world_mut().run_system_once(ion_field_track_enemies_system);

            // Enemy should lose InIonField marker
            assert!(app.world().get::<InIonField>(enemy_entity).is_none(),
                "Enemy that exited field should lose InIonField marker");
        }

        #[test]
        fn test_multiple_enemies_tracked_independently() {
            let mut app = setup_test_app();

            // Create field at origin
            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Create enemy inside field
            let enemy_inside = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // Create enemy outside field
            let enemy_outside = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            )).id();

            // Run the tracking system
            let _ = app.world_mut().run_system_once(ion_field_track_enemies_system);

            // Inside enemy should have marker
            let in_field = app.world().get::<InIonField>(enemy_inside);
            assert!(in_field.is_some());
            assert_eq!(in_field.unwrap().field_entity, field_entity);

            // Outside enemy should not have marker
            assert!(app.world().get::<InIonField>(enemy_outside).is_none());
        }
    }

    mod ion_field_damage_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_damage_applies_at_tick_interval() {
            let mut app = setup_test_app();

            // Create field at origin
            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Create enemy with InIonField marker
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                InIonField { field_entity },
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ION_FIELD_TICK_INTERVAL + 0.01));
            }

            // Run the damage system
            let _ = app.world_mut().run_system_once(ion_field_damage_system);

            // Check that field tick timer triggered
            let mut field_query = app.world_mut().query::<&IonField>();
            let field = field_query.single(app.world()).unwrap();
            assert!(field.tick_timer.just_finished(), "Tick timer should have triggered");
        }

        #[test]
        fn test_no_damage_when_enemy_outside_field() {
            let mut app = setup_test_app();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

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

            // Create field at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            ));

            // Create enemy WITHOUT InIonField marker (outside field)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ION_FIELD_TICK_INTERVAL + 0.01));
            }

            // Run damage system then count events
            let _ = app.world_mut().run_system_once(ion_field_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "No damage should be applied to enemy outside field");
        }

        #[test]
        fn test_multiple_enemies_damaged_simultaneously() {
            let mut app = setup_test_app();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

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

            // Create field at origin
            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Create 3 enemies inside field with markers
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                    InIonField { field_entity },
                ));
            }

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ION_FIELD_TICK_INTERVAL + 0.01));
            }

            // Run damage system then count events
            let _ = app.world_mut().run_system_once(ion_field_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 3, "All 3 enemies should be damaged");
        }

        #[test]
        fn test_damage_per_tick_calculates_correctly() {
            // damage_per_tick = damage * tick_interval
            // For damage=20.0: 20.0 * 0.25 = 5.0 per tick
            let field = IonField::new(Vec2::ZERO, 20.0, 6.0, 5.0);
            assert!((field.damage_per_tick - 5.0).abs() < 0.01);
        }
    }

    mod ion_field_cleanup_markers_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_marker_removed_when_field_despawns() {
            let mut app = App::new();

            // Create enemy with InIonField marker pointing to non-existent entity
            let fake_field_entity = Entity::from_raw_u32(99999).unwrap();
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                InIonField { field_entity: fake_field_entity },
            )).id();

            // Run the cleanup system
            let _ = app.world_mut().run_system_once(ion_field_cleanup_markers_system);

            // Enemy should lose InIonField marker since field doesn't exist
            assert!(app.world().get::<InIonField>(enemy_entity).is_none(),
                "Marker should be removed when field entity doesn't exist");
        }

        #[test]
        fn test_marker_preserved_when_field_exists() {
            let mut app = App::new();

            // Create field
            let field_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                IonField::new(Vec2::ZERO, 10.0, 6.0, 5.0),
            )).id();

            // Create enemy with valid InIonField marker
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                InIonField { field_entity },
            )).id();

            // Run the cleanup system
            let _ = app.world_mut().run_system_once(ion_field_cleanup_markers_system);

            // Enemy should still have InIonField marker
            assert!(app.world().get::<InIonField>(enemy_entity).is_some(),
                "Marker should be preserved when field entity exists");
        }
    }

    mod fire_ion_field_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_ion_field_spawns_field() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ion_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 ion field
            let mut query = app.world_mut().query::<&IonField>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_ion_field_at_spawn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ion_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IonField>();
            for field in query.iter(app.world()) {
                // Field center should match spawn XZ (10.0, 20.0)
                assert_eq!(field.center.x, 10.0);
                assert_eq!(field.center.y, 20.0); // Z maps to Y in Vec2
            }
        }

        #[test]
        fn test_fire_ion_field_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let expected_damage_per_tick = spell.damage() * ION_FIELD_TICK_INTERVAL;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ion_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IonField>();
            for field in query.iter(app.world()) {
                assert!((field.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
            }
        }

        #[test]
        fn test_fire_ion_field_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let explicit_damage = 100.0;
            let expected_damage_per_tick = explicit_damage * ION_FIELD_TICK_INTERVAL;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ion_field_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IonField>();
            for field in query.iter(app.world()) {
                assert!((field.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
            }
        }

        #[test]
        fn test_fire_ion_field_has_correct_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Electrocute);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ion_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&IonField>();
            for field in query.iter(app.world()) {
                assert_eq!(field.radius, ION_FIELD_RADIUS);
            }
        }
    }
}
