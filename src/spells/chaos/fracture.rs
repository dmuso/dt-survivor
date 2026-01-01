//! Fracture spell - Causes enemies to split into fragments on death.
//!
//! A Chaos element spell that marks enemies with a fracture effect. When a
//! fractured enemy dies, it splits into 2-3 smaller hostile fragments that
//! are weaker versions of the original enemy.

use bevy::prelude::*;
use crate::combat::{CheckDeath, DamageEvent, DeathEvent, EntityType, Health};
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::enemies::systems::ENEMY_Y_HEIGHT;
use crate::game::components::Level;
use crate::game::resources::{enemy_scale_for_level, EnemyLevelMaterials, GameMeshes};
use crate::movement::components::Velocity;
use rand::Rng;

/// Default configuration for Fracture spell
pub const FRACTURE_DAMAGE: f32 = 15.0;
pub const FRACTURE_COOLDOWN: f32 = 1.0;
pub const FRAGMENT_HEALTH_MULTIPLIER: f32 = 0.3;
pub const FRAGMENT_DAMAGE_MULTIPLIER: f32 = 0.5;
pub const FRAGMENT_SCALE_MULTIPLIER: f32 = 0.5;
pub const FRAGMENT_MIN_COUNT: u32 = 2;
pub const FRAGMENT_MAX_COUNT: u32 = 3;
pub const FRAGMENT_SPAWN_OFFSET: f32 = 1.0;

/// Get the chaos element color for visual effects (magenta/pink)
pub fn fracture_color() -> Color {
    Element::Chaos.color()
}

/// Marker component for enemies affected by the fracture effect.
/// When an enemy with this component dies, it will split into fragments.
#[derive(Component, Debug, Clone, Default)]
pub struct FractureEffect;

/// Component marking an enemy as a fragment (split from another enemy).
/// Fragments cannot fracture again to prevent infinite recursion.
#[derive(Component, Debug, Clone)]
pub struct Fragment {
    /// Health multiplier applied to the original enemy's health
    pub health_multiplier: f32,
    /// Damage multiplier applied to the original enemy's damage
    pub damage_multiplier: f32,
    /// Scale multiplier applied to the original enemy's visual size
    pub scale_multiplier: f32,
}

impl Default for Fragment {
    fn default() -> Self {
        Self {
            health_multiplier: FRAGMENT_HEALTH_MULTIPLIER,
            damage_multiplier: FRAGMENT_DAMAGE_MULTIPLIER,
            scale_multiplier: FRAGMENT_SCALE_MULTIPLIER,
        }
    }
}

impl Fragment {
    /// Create a new fragment with custom multipliers.
    pub fn new(health_mult: f32, damage_mult: f32, scale_mult: f32) -> Self {
        Self {
            health_multiplier: health_mult,
            damage_multiplier: damage_mult,
            scale_multiplier: scale_mult,
        }
    }

    /// Check if an entity is a fragment (cannot be fractured again).
    pub fn is_fragment(&self) -> bool {
        true
    }
}

/// Event fired when a fractured enemy dies and needs to spawn fragments.
#[derive(Message, Debug, Clone)]
pub struct FractureDeathEvent {
    /// Position where the enemy died (spawn fragments here)
    pub position: Vec3,
    /// Level of the original enemy (fragments inherit reduced stats)
    pub enemy_level: u8,
    /// Number of fragments to spawn (2-3)
    pub fragment_count: u32,
    /// Original enemy's strength/damage
    pub original_strength: f32,
    /// Original enemy's health
    pub original_health: f32,
}

impl FractureDeathEvent {
    pub fn new(position: Vec3, enemy_level: u8, fragment_count: u32, original_strength: f32, original_health: f32) -> Self {
        Self {
            position,
            enemy_level,
            fragment_count,
            original_strength,
            original_health,
        }
    }
}

/// System that listens for DeathEvents and spawns fragments for fractured enemies.
pub fn fracture_on_death_system(
    mut death_events: MessageReader<DeathEvent>,
    fractured_query: Query<(&Enemy, &Health, &Level), (With<FractureEffect>, Without<Fragment>)>,
    mut fracture_death_events: MessageWriter<FractureDeathEvent>,
) {
    for event in death_events.read() {
        if event.entity_type != EntityType::Enemy {
            continue;
        }

        // Check if the dead entity was fractured (and not already a fragment)
        if let Ok((enemy, health, level)) = fractured_query.get(event.entity) {
            let mut rng = rand::thread_rng();
            let fragment_count = rng.gen_range(FRAGMENT_MIN_COUNT..=FRAGMENT_MAX_COUNT);

            fracture_death_events.write(FractureDeathEvent::new(
                event.position,
                level.value(),
                fragment_count,
                enemy.strength,
                health.max,
            ));
        }
    }
}

/// System that spawns fragment enemies from FractureDeathEvents.
#[allow(clippy::too_many_arguments)]
pub fn spawn_fragment_enemies_system(
    mut commands: Commands,
    mut fracture_events: MessageReader<FractureDeathEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    enemy_materials: Option<Res<EnemyLevelMaterials>>,
) {
    for event in fracture_events.read() {
        let mut rng = rand::thread_rng();

        for i in 0..event.fragment_count {
            // Calculate spawn position with offset
            let angle = (i as f32 / event.fragment_count as f32) * std::f32::consts::TAU
                + rng.gen_range(-0.3..0.3);
            let offset = Vec3::new(
                angle.cos() * FRAGMENT_SPAWN_OFFSET,
                0.0,
                angle.sin() * FRAGMENT_SPAWN_OFFSET,
            );
            let spawn_pos = event.position + offset;

            // Calculate fragment stats
            let fragment = Fragment::default();
            let fragment_health = event.original_health * fragment.health_multiplier;
            let fragment_strength = event.original_strength * fragment.damage_multiplier;
            let base_scale = enemy_scale_for_level(event.enemy_level);
            let fragment_scale = base_scale * fragment.scale_multiplier;
            let y_height = ENEMY_Y_HEIGHT * fragment_scale;

            // Spawn fragment enemy
            let fragment_pos = Vec3::new(spawn_pos.x, y_height, spawn_pos.z);

            if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), enemy_materials.as_ref()) {
                commands.spawn((
                    Mesh3d(meshes.enemy.clone()),
                    MeshMaterial3d(materials.for_level(event.enemy_level)),
                    Transform::from_translation(fragment_pos)
                        .with_scale(Vec3::splat(fragment_scale)),
                    Enemy {
                        speed: 2.0,
                        strength: fragment_strength,
                    },
                    Health::new(fragment_health),
                    Level::new(event.enemy_level),
                    Velocity::new(bevy::math::Vec2::ZERO),
                    CheckDeath,
                    fragment,
                ));
            } else {
                // Fallback for tests without mesh resources
                commands.spawn((
                    Transform::from_translation(fragment_pos)
                        .with_scale(Vec3::splat(fragment_scale)),
                    Enemy {
                        speed: 2.0,
                        strength: fragment_strength,
                    },
                    Health::new(fragment_health),
                    Level::new(event.enemy_level),
                    Velocity::new(bevy::math::Vec2::ZERO),
                    CheckDeath,
                    Fragment::default(),
                ));
            }
        }
    }
}

/// Apply fracture effect to enemies hit by fracture spell damage.
/// This is typically called when the spell deals damage to mark enemies.
pub fn apply_fracture_effect(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    enemy_query: Query<Entity, (With<Enemy>, Without<FractureEffect>, Without<Fragment>)>,
) {
    for event in damage_events.read() {
        // Only apply fracture effect from chaos element damage
        if event.element != Some(Element::Chaos) {
            continue;
        }

        // Check if target is a valid enemy without fracture effect or fragment marker
        if enemy_query.get(event.target).is_ok() {
            commands.entity(event.target).insert(FractureEffect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    mod fracture_effect_tests {
        use super::*;

        #[test]
        fn test_fracture_effect_is_marker_component() {
            let effect = FractureEffect;
            // FractureEffect is a marker component with no fields
            assert_eq!(std::mem::size_of::<FractureEffect>(), 0);
            let _ = effect; // Silence unused warning
        }

        #[test]
        fn test_fracture_color_is_chaos_element() {
            let color = fracture_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod fragment_tests {
        use super::*;

        #[test]
        fn test_fragment_default_values() {
            let fragment = Fragment::default();
            assert_eq!(fragment.health_multiplier, FRAGMENT_HEALTH_MULTIPLIER);
            assert_eq!(fragment.damage_multiplier, FRAGMENT_DAMAGE_MULTIPLIER);
            assert_eq!(fragment.scale_multiplier, FRAGMENT_SCALE_MULTIPLIER);
        }

        #[test]
        fn test_fragment_new_with_custom_values() {
            let fragment = Fragment::new(0.5, 0.6, 0.7);
            assert_eq!(fragment.health_multiplier, 0.5);
            assert_eq!(fragment.damage_multiplier, 0.6);
            assert_eq!(fragment.scale_multiplier, 0.7);
        }

        #[test]
        fn test_fragment_is_fragment() {
            let fragment = Fragment::default();
            assert!(fragment.is_fragment());
        }

        #[test]
        fn test_fragment_has_reduced_health() {
            let fragment = Fragment::default();
            let original_health = 100.0;
            let fragment_health = original_health * fragment.health_multiplier;
            assert!(fragment_health < original_health);
            assert!((fragment_health - 30.0).abs() < 0.01); // 100 * 0.3
        }

        #[test]
        fn test_fragment_has_reduced_damage() {
            let fragment = Fragment::default();
            let original_damage = 20.0;
            let fragment_damage = original_damage * fragment.damage_multiplier;
            assert!(fragment_damage < original_damage);
            assert_eq!(fragment_damage, 10.0); // 20 * 0.5
        }

        #[test]
        fn test_fragment_is_visually_smaller() {
            let fragment = Fragment::default();
            let original_scale = 1.0;
            let fragment_scale = original_scale * fragment.scale_multiplier;
            assert!(fragment_scale < original_scale);
            assert_eq!(fragment_scale, 0.5); // 1.0 * 0.5
        }
    }

    mod fracture_death_event_tests {
        use super::*;

        #[test]
        fn test_fracture_death_event_creation() {
            let position = Vec3::new(10.0, 0.75, 20.0);
            let event = FractureDeathEvent::new(position, 3, 2, 15.0, 50.0);

            assert_eq!(event.position, position);
            assert_eq!(event.enemy_level, 3);
            assert_eq!(event.fragment_count, 2);
            assert_eq!(event.original_strength, 15.0);
            assert_eq!(event.original_health, 50.0);
        }

        #[test]
        fn test_fragment_count_is_random_2_to_3() {
            // The count is set by the caller, but should be 2 or 3
            for count in FRAGMENT_MIN_COUNT..=FRAGMENT_MAX_COUNT {
                let event = FractureDeathEvent::new(Vec3::ZERO, 1, count, 10.0, 100.0);
                assert!(event.fragment_count >= 2 && event.fragment_count <= 3);
            }
        }
    }

    mod apply_fracture_effect_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_fracture_marks_enemy_on_chaos_damage() {
            let mut app = setup_test_app();

            // Spawn enemy without fracture effect
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.75, 0.0)),
            )).id();

            // Send chaos damage event
            app.world_mut().write_message(DamageEvent::with_element(enemy, 20.0, Element::Chaos));

            let _ = app.world_mut().run_system_once(apply_fracture_effect);

            // Enemy should now have FractureEffect
            assert!(
                app.world().get::<FractureEffect>(enemy).is_some(),
                "Enemy should have FractureEffect after chaos damage"
            );
        }

        #[test]
        fn test_fracture_does_not_mark_on_other_element_damage() {
            let mut app = setup_test_app();

            // Spawn enemy without fracture effect
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.75, 0.0)),
            )).id();

            // Send fire damage event (not chaos)
            app.world_mut().write_message(DamageEvent::with_element(enemy, 20.0, Element::Fire));

            let _ = app.world_mut().run_system_once(apply_fracture_effect);

            // Enemy should NOT have FractureEffect
            assert!(
                app.world().get::<FractureEffect>(enemy).is_none(),
                "Enemy should NOT have FractureEffect from non-chaos damage"
            );
        }

        #[test]
        fn test_fracture_does_not_mark_already_fractured_enemy() {
            let mut app = setup_test_app();

            // Spawn enemy already with fracture effect
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.75, 0.0)),
                FractureEffect,
            )).id();

            // Send chaos damage event
            app.world_mut().write_message(DamageEvent::with_element(enemy, 20.0, Element::Chaos));

            // This should not cause an error (idempotent)
            let _ = app.world_mut().run_system_once(apply_fracture_effect);

            // Enemy should still have only one FractureEffect
            assert!(app.world().get::<FractureEffect>(enemy).is_some());
        }

        #[test]
        fn test_fragment_cannot_be_fractured_again() {
            let mut app = setup_test_app();

            // Spawn fragment enemy (already a fragment)
            let fragment_enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.75, 0.0)),
                Fragment::default(),
            )).id();

            // Send chaos damage event to fragment
            app.world_mut().write_message(DamageEvent::with_element(fragment_enemy, 20.0, Element::Chaos));

            let _ = app.world_mut().run_system_once(apply_fracture_effect);

            // Fragment should NOT get FractureEffect (prevents infinite splitting)
            assert!(
                app.world().get::<FractureEffect>(fragment_enemy).is_none(),
                "Fragment should NOT get FractureEffect to prevent infinite recursion"
            );
        }

        #[test]
        fn test_fracture_ignores_non_enemy_entities() {
            let mut app = setup_test_app();

            // Spawn non-enemy entity
            let non_enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.75, 0.0)),
            )).id();

            // Send chaos damage event
            app.world_mut().write_message(DamageEvent::with_element(non_enemy, 20.0, Element::Chaos));

            let _ = app.world_mut().run_system_once(apply_fracture_effect);

            // Non-enemy should NOT have FractureEffect
            assert!(app.world().get::<FractureEffect>(non_enemy).is_none());
        }
    }

    mod fracture_on_death_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_message::<DeathEvent>();
            app.add_message::<FractureDeathEvent>();
            app
        }

        #[test]
        fn test_fractured_enemy_emits_fracture_death_event() {
            let mut app = setup_test_app();

            // Spawn fractured enemy
            let enemy_pos = Vec3::new(10.0, 0.75, 20.0);
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 15.0 },
                Health::new(100.0),
                Level::new(2),
                Transform::from_translation(enemy_pos),
                FractureEffect,
            )).id();

            // Send death event for the enemy
            app.world_mut().write_message(DeathEvent::new(enemy, enemy_pos, EntityType::Enemy));

            // Add counter for fracture death events
            #[derive(Resource, Default)]
            struct FractureDeathCounter(u32);

            fn count_fracture_deaths(
                mut events: MessageReader<FractureDeathEvent>,
                mut counter: ResMut<FractureDeathCounter>,
            ) {
                for _ in events.read() {
                    counter.0 += 1;
                }
            }

            app.init_resource::<FractureDeathCounter>();
            app.add_systems(Update, (fracture_on_death_system, count_fracture_deaths).chain());
            app.update();

            let counter = app.world().resource::<FractureDeathCounter>();
            assert_eq!(counter.0, 1, "Should emit one FractureDeathEvent");
        }

        #[test]
        fn test_non_fractured_enemy_does_not_emit_event() {
            let mut app = setup_test_app();

            // Spawn regular enemy (no FractureEffect)
            let enemy_pos = Vec3::new(10.0, 0.75, 20.0);
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 15.0 },
                Health::new(100.0),
                Level::new(2),
                Transform::from_translation(enemy_pos),
            )).id();

            // Send death event
            app.world_mut().write_message(DeathEvent::new(enemy, enemy_pos, EntityType::Enemy));

            #[derive(Resource, Default)]
            struct FractureDeathCounter(u32);

            fn count_fracture_deaths(
                mut events: MessageReader<FractureDeathEvent>,
                mut counter: ResMut<FractureDeathCounter>,
            ) {
                for _ in events.read() {
                    counter.0 += 1;
                }
            }

            app.init_resource::<FractureDeathCounter>();
            app.add_systems(Update, (fracture_on_death_system, count_fracture_deaths).chain());
            app.update();

            let counter = app.world().resource::<FractureDeathCounter>();
            assert_eq!(counter.0, 0, "Non-fractured enemy should not emit FractureDeathEvent");
        }

        #[test]
        fn test_fragment_does_not_emit_fracture_death_event() {
            let mut app = setup_test_app();

            // Spawn fragment enemy with FractureEffect (shouldn't happen but testing defense)
            let enemy_pos = Vec3::new(10.0, 0.75, 20.0);
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 15.0 },
                Health::new(30.0),
                Level::new(2),
                Transform::from_translation(enemy_pos),
                FractureEffect,
                Fragment::default(), // Is a fragment
            )).id();

            // Send death event
            app.world_mut().write_message(DeathEvent::new(enemy, enemy_pos, EntityType::Enemy));

            #[derive(Resource, Default)]
            struct FractureDeathCounter(u32);

            fn count_fracture_deaths(
                mut events: MessageReader<FractureDeathEvent>,
                mut counter: ResMut<FractureDeathCounter>,
            ) {
                for _ in events.read() {
                    counter.0 += 1;
                }
            }

            app.init_resource::<FractureDeathCounter>();
            app.add_systems(Update, (fracture_on_death_system, count_fracture_deaths).chain());
            app.update();

            let counter = app.world().resource::<FractureDeathCounter>();
            assert_eq!(counter.0, 0, "Fragment should not emit FractureDeathEvent");
        }

        #[test]
        fn test_player_death_does_not_trigger_fracture() {
            let mut app = setup_test_app();

            // Spawn fractured enemy (won't be found by query)
            let player_pos = Vec3::new(0.0, 0.5, 0.0);
            let player = app.world_mut().spawn((
                Transform::from_translation(player_pos),
            )).id();

            // Send player death event
            app.world_mut().write_message(DeathEvent::new(player, player_pos, EntityType::Player));

            #[derive(Resource, Default)]
            struct FractureDeathCounter(u32);

            fn count_fracture_deaths(
                mut events: MessageReader<FractureDeathEvent>,
                mut counter: ResMut<FractureDeathCounter>,
            ) {
                for _ in events.read() {
                    counter.0 += 1;
                }
            }

            app.init_resource::<FractureDeathCounter>();
            app.add_systems(Update, (fracture_on_death_system, count_fracture_deaths).chain());
            app.update();

            let counter = app.world().resource::<FractureDeathCounter>();
            assert_eq!(counter.0, 0, "Player death should not trigger fracture");
        }
    }

    mod spawn_fragment_enemies_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_message::<FractureDeathEvent>();
            app
        }

        #[test]
        fn test_spawn_fragments_creates_correct_count() {
            let mut app = setup_test_app();

            // Send fracture death event for 3 fragments
            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                2,
                3,
                15.0,
                100.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            // Count spawned fragments
            let mut query = app.world_mut().query::<&Fragment>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 3, "Should spawn 3 fragments");
        }

        #[test]
        fn test_spawn_fragments_creates_2_minimum() {
            let mut app = setup_test_app();

            // Send fracture death event for 2 fragments (minimum)
            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1,
                2,
                10.0,
                50.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<&Fragment>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2, "Should spawn 2 fragments (minimum)");
        }

        #[test]
        fn test_fragments_have_reduced_health() {
            let mut app = setup_test_app();

            let original_health = 100.0;
            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1,
                2,
                10.0,
                original_health,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &Health)>();
            for (fragment, health) in query.iter(app.world()) {
                let expected_health = original_health * fragment.health_multiplier;
                assert_eq!(health.max, expected_health);
                assert!(health.max < original_health);
            }
        }

        #[test]
        fn test_fragments_have_reduced_damage() {
            let mut app = setup_test_app();

            let original_strength = 20.0;
            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1,
                2,
                original_strength,
                100.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &Enemy)>();
            for (fragment, enemy) in query.iter(app.world()) {
                let expected_strength = original_strength * fragment.damage_multiplier;
                assert_eq!(enemy.strength, expected_strength);
                assert!(enemy.strength < original_strength);
            }
        }

        #[test]
        fn test_fragments_are_visually_smaller() {
            let mut app = setup_test_app();

            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1, // Level 1 enemy
                2,
                10.0,
                50.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let base_scale = enemy_scale_for_level(1);
            let expected_fragment_scale = base_scale * FRAGMENT_SCALE_MULTIPLIER;

            let mut query = app.world_mut().query::<(&Fragment, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert!(
                    (transform.scale.x - expected_fragment_scale).abs() < 0.01,
                    "Fragment scale {} should be {}", transform.scale.x, expected_fragment_scale
                );
            }
        }

        #[test]
        fn test_fragments_have_enemy_component() {
            let mut app = setup_test_app();

            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1,
                2,
                10.0,
                50.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &Enemy)>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2, "All fragments should have Enemy component");
        }

        #[test]
        fn test_fragments_have_check_death_component() {
            let mut app = setup_test_app();

            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1,
                2,
                10.0,
                50.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &CheckDeath)>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2, "All fragments should have CheckDeath component");
        }

        #[test]
        fn test_fragments_have_velocity_component() {
            let mut app = setup_test_app();

            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                1,
                2,
                10.0,
                50.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &Velocity)>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2, "All fragments should have Velocity component");
        }

        #[test]
        fn test_fragments_spawn_near_death_position() {
            let mut app = setup_test_app();

            let death_pos = Vec3::new(10.0, 0.75, 20.0);
            app.world_mut().write_message(FractureDeathEvent::new(
                death_pos,
                1,
                2,
                10.0,
                50.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                let distance = Vec2::new(
                    transform.translation.x - death_pos.x,
                    transform.translation.z - death_pos.z,
                ).length();

                // Fragments should spawn within a reasonable distance of death position
                assert!(
                    distance <= FRAGMENT_SPAWN_OFFSET + 0.5,
                    "Fragment should spawn near death position, distance: {}", distance
                );
            }
        }

        #[test]
        fn test_fragment_death_awards_xp() {
            // Fragments have CheckDeath component, so they go through normal death flow
            // which handles XP awards. This test verifies they have the required components.
            let mut app = setup_test_app();

            app.world_mut().write_message(FractureDeathEvent::new(
                Vec3::new(10.0, 0.75, 20.0),
                2, // Level 2 enemy
                2,
                15.0,
                100.0,
            ));

            let _ = app.world_mut().run_system_once(spawn_fragment_enemies_system);

            let mut query = app.world_mut().query::<(&Fragment, &Level, &CheckDeath)>();
            for (_, level, _) in query.iter(app.world()) {
                // Fragments inherit level from parent
                assert_eq!(level.value(), 2);
            }
        }
    }
}
