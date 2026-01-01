//! Entropy Field spell - Area of distortion causing unpredictable damage.
//!
//! A Chaos element spell (Entropy SpellType) that creates a zone at a target
//! location with variable damage where the damage amount is randomized each tick,
//! creating chaotic and unpredictable destruction.

use bevy::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Entropy Field spell
pub const ENTROPY_FIELD_RADIUS: f32 = 5.0;
pub const ENTROPY_FIELD_DURATION: f32 = 4.0;
pub const ENTROPY_FIELD_TICK_INTERVAL: f32 = 0.5;
pub const ENTROPY_FIELD_MIN_DAMAGE_MULT: f32 = 0.25; // 25% of base damage minimum
pub const ENTROPY_FIELD_MAX_DAMAGE_MULT: f32 = 2.0;  // 200% of base damage maximum
pub const ENTROPY_FIELD_VISUAL_HEIGHT: f32 = 0.2;

/// Get the chaos element color for visual effects (magenta/pink)
pub fn entropy_field_color() -> Color {
    Element::Chaos.color()
}

/// Component for the entropy field zone.
/// Creates a field at a location that deals randomized damage each tick.
#[derive(Component, Debug, Clone)]
pub struct EntropyField {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the damage zone
    pub radius: f32,
    /// Remaining duration of the field
    pub duration: Timer,
    /// Base damage (actual damage is randomized around this)
    pub base_damage: f32,
    /// Minimum damage multiplier
    pub min_damage_mult: f32,
    /// Maximum damage multiplier
    pub max_damage_mult: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
}

impl EntropyField {
    /// Create a new entropy field at the given center position.
    pub fn new(center: Vec2, base_damage: f32) -> Self {
        Self {
            center,
            radius: ENTROPY_FIELD_RADIUS,
            duration: Timer::from_seconds(ENTROPY_FIELD_DURATION, TimerMode::Once),
            base_damage,
            min_damage_mult: ENTROPY_FIELD_MIN_DAMAGE_MULT,
            max_damage_mult: ENTROPY_FIELD_MAX_DAMAGE_MULT,
            tick_timer: Timer::from_seconds(ENTROPY_FIELD_TICK_INTERVAL, TimerMode::Repeating),
        }
    }

    /// Create an entropy field from a Spell component.
    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the field has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the field timers.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);
    }

    /// Check if damage should be applied this frame.
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }

    /// Generate a random damage value within the configured range.
    pub fn random_damage(&self) -> f32 {
        let mut rng = rand::thread_rng();
        let mult = rng.gen_range(self.min_damage_mult..=self.max_damage_mult);
        self.base_damage * mult
    }

    /// Check if an entity at the given position is within the field.
    pub fn is_in_field(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }
}

/// System that ticks entropy field timers.
pub fn entropy_field_tick_system(
    mut field_query: Query<&mut EntropyField>,
    time: Res<Time>,
) {
    for mut field in field_query.iter_mut() {
        field.tick(time.delta());
    }
}

/// System that damages enemies within entropy fields with randomized damage.
pub fn entropy_field_damage_system(
    field_query: Query<&EntropyField>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for field in field_query.iter() {
        if !field.should_damage() {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if field.is_in_field(enemy_pos) {
                let damage = field.random_damage();
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    damage,
                    Element::Chaos,
                ));
            }
        }
    }
}

/// System that despawns entropy fields when their duration expires.
pub fn entropy_field_cleanup_system(
    mut commands: Commands,
    field_query: Query<(Entity, &EntropyField)>,
) {
    for (entity, field) in field_query.iter() {
        if field.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual appearance of entropy fields.
pub fn entropy_field_visual_system(
    mut field_query: Query<(&EntropyField, &mut Transform)>,
) {
    for (field, mut transform) in field_query.iter_mut() {
        // Scale the visual to match the radius
        transform.scale = Vec3::splat(field.radius.max(0.1));
    }
}

/// Cast entropy field spell - spawns a zone at target location.
#[allow(clippy::too_many_arguments)]
pub fn fire_entropy_field(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_entropy_field_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast entropy field spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_entropy_field_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    _spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let field = EntropyField::new(target_pos, damage);
    let field_pos = Vec3::new(target_pos.x, ENTROPY_FIELD_VISUAL_HEIGHT, target_pos.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.chaos_bolt.clone()), // Use chaos material for magenta/pink color
            Transform::from_translation(field_pos).with_scale(Vec3::splat(0.1)),
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
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod entropy_field_component_tests {
        use super::*;

        #[test]
        fn test_entropy_field_spawns_at_correct_location() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 25.0;
            let field = EntropyField::new(center, damage);

            assert_eq!(field.center, center);
            assert_eq!(field.base_damage, damage);
            assert_eq!(field.radius, ENTROPY_FIELD_RADIUS);
            assert!(!field.is_expired());
        }

        #[test]
        fn test_entropy_field_has_correct_radius() {
            let field = EntropyField::new(Vec2::ZERO, 20.0);
            assert_eq!(field.radius, ENTROPY_FIELD_RADIUS);
        }

        #[test]
        fn test_entropy_field_damage_is_randomized() {
            let field = EntropyField::new(Vec2::ZERO, 100.0);

            // Collect multiple random damage values
            let mut damages: Vec<f32> = Vec::new();
            for _ in 0..20 {
                damages.push(field.random_damage());
            }

            // At least some values should be different (very unlikely to all be same)
            let unique_count = damages.iter()
                .map(|d| (d * 100.0) as i32)
                .collect::<std::collections::HashSet<_>>()
                .len();
            assert!(unique_count > 1, "Damage should be randomized");
        }

        #[test]
        fn test_entropy_field_damage_within_bounds() {
            let field = EntropyField::new(Vec2::ZERO, 100.0);

            // Check many random samples are within bounds
            for _ in 0..100 {
                let damage = field.random_damage();
                let min_expected = 100.0 * ENTROPY_FIELD_MIN_DAMAGE_MULT;
                let max_expected = 100.0 * ENTROPY_FIELD_MAX_DAMAGE_MULT;
                assert!(
                    damage >= min_expected && damage <= max_expected,
                    "Damage {} should be between {} and {}",
                    damage, min_expected, max_expected
                );
            }
        }

        #[test]
        fn test_entropy_field_is_in_field() {
            let field = EntropyField::new(Vec2::new(10.0, 10.0), 20.0);

            // Inside field
            assert!(field.is_in_field(Vec2::new(10.0, 10.0))); // Center
            assert!(field.is_in_field(Vec2::new(12.0, 10.0))); // Within radius

            // On edge
            assert!(field.is_in_field(Vec2::new(10.0 + ENTROPY_FIELD_RADIUS, 10.0)));

            // Outside field
            assert!(!field.is_in_field(Vec2::new(10.0 + ENTROPY_FIELD_RADIUS + 0.1, 10.0)));
            assert!(!field.is_in_field(Vec2::new(100.0, 100.0)));
        }

        #[test]
        fn test_entropy_field_duration_expires() {
            let mut field = EntropyField::new(Vec2::ZERO, 20.0);
            assert!(!field.is_expired());

            // Tick past duration
            field.tick(Duration::from_secs_f32(ENTROPY_FIELD_DURATION + 0.1));
            assert!(field.is_expired());
        }

        #[test]
        fn test_entropy_field_despawns_after_duration() {
            let mut app = bevy::app::App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create a field that's already expired
            let mut field = EntropyField::new(Vec2::ZERO, 20.0);
            field.duration.tick(Duration::from_secs_f32(ENTROPY_FIELD_DURATION + 0.1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            )).id();

            let _ = app.world_mut().run_system_once(entropy_field_cleanup_system);

            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_entropy_field_from_spell() {
            let spell = Spell::new(SpellType::Entropy);
            let center = Vec2::new(5.0, 15.0);
            let field = EntropyField::from_spell(center, &spell);

            assert_eq!(field.center, center);
            assert_eq!(field.base_damage, spell.damage());
        }

        #[test]
        fn test_entropy_field_uses_chaos_element_color() {
            let color = entropy_field_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod entropy_field_tick_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_entropy_field_tick_updates_timer() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                EntropyField::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(entropy_field_tick_system);

            let field = app.world().get::<EntropyField>(entity).unwrap();
            assert!(
                field.duration.elapsed_secs() > 0.9,
                "Duration timer should have ticked"
            );
        }

        #[test]
        fn test_entropy_field_tick_triggers_should_damage() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                EntropyField::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ENTROPY_FIELD_TICK_INTERVAL));
            }

            let _ = app.world_mut().run_system_once(entropy_field_tick_system);

            let field = app.world().get::<EntropyField>(entity).unwrap();
            assert!(field.should_damage(), "should_damage should be true after tick interval");
        }
    }

    mod entropy_field_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_entropy_field_damages_enemies_in_zone() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (entropy_field_damage_system, count_damage_events).chain());

            // Create field at origin that should damage
            let mut field = EntropyField::new(Vec2::ZERO, 20.0);
            // Force tick timer to just finished
            field.tick_timer.tick(Duration::from_secs_f32(ENTROPY_FIELD_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_entropy_field_ignores_enemies_outside() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (entropy_field_damage_system, count_damage_events).chain());

            // Create field at origin
            let mut field = EntropyField::new(Vec2::ZERO, 20.0);
            field.tick_timer.tick(Duration::from_secs_f32(ENTROPY_FIELD_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            ));

            // Create enemy outside radius (100 units away)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_entropy_field_no_damage_before_tick() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (entropy_field_damage_system, count_damage_events).chain());

            // Create field that hasn't ticked yet
            let field = EntropyField::new(Vec2::ZERO, 20.0);

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            // No damage yet, tick timer hasn't fired
            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_entropy_field_damages_multiple_enemies() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (entropy_field_damage_system, count_damage_events).chain());

            // Create field
            let mut field = EntropyField::new(Vec2::ZERO, 20.0);
            field.tick_timer.tick(Duration::from_secs_f32(ENTROPY_FIELD_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            ));

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
    }

    mod entropy_field_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_entropy_field_survives_before_expiry() {
            let mut app = App::new();

            let field = EntropyField::new(Vec2::ZERO, 20.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            )).id();

            let _ = app.world_mut().run_system_once(entropy_field_cleanup_system);

            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod entropy_field_visual_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_entropy_field_visual_scale_matches_radius() {
            let mut app = App::new();

            let field = EntropyField::new(Vec2::ZERO, 20.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            )).id();

            let _ = app.world_mut().run_system_once(entropy_field_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(ENTROPY_FIELD_RADIUS));
        }

        #[test]
        fn test_entropy_field_visual_minimum_scale() {
            let mut app = App::new();

            let mut field = EntropyField::new(Vec2::ZERO, 20.0);
            field.radius = 0.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                field,
            )).id();

            let _ = app.world_mut().run_system_once(entropy_field_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }

    mod fire_entropy_field_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_entropy_field_spawns_zone() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Entropy);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_entropy_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EntropyField>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_entropy_field_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Entropy);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(15.0, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_entropy_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EntropyField>();
            for field in query.iter(app.world()) {
                assert_eq!(field.center, target_pos);
            }
        }

        #[test]
        fn test_fire_entropy_field_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Entropy);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_entropy_field(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EntropyField>();
            for field in query.iter(app.world()) {
                assert_eq!(field.base_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_entropy_field_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Entropy);
            let explicit_damage = 150.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_entropy_field_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EntropyField>();
            for field in query.iter(app.world()) {
                assert_eq!(field.base_damage, explicit_damage);
            }
        }
    }
}
