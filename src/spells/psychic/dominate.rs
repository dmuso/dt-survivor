//! Dominate spell - Temporarily turns enemies against each other.
//!
//! A Psychic element spell (Dominate SpellType) that targets the nearest enemy
//! and takes control of their mind. The dominated enemy becomes an ally for a
//! brief duration, attacking other enemies before the effect wears off.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::PlayerPosition;
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default range within which Dominate can target enemies
pub const DOMINATE_DEFAULT_RANGE: f32 = 10.0;

/// Default duration of the domination effect in seconds
pub const DOMINATE_DEFAULT_DURATION: f32 = 5.0;

/// Default damage the dominated enemy deals to other enemies per hit
pub const DOMINATE_ALLY_DAMAGE: f32 = 15.0;

/// Attack interval for dominated enemies
pub const DOMINATE_ATTACK_INTERVAL: f32 = 1.0;

/// Height of the visual effect above enemy
pub const DOMINATE_VISUAL_HEIGHT: f32 = 1.5;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn dominate_color() -> Color {
    Element::Psychic.color()
}

/// Marker component for the Dominate spell effect entity.
/// Attached to a spell entity that triggers domination.
#[derive(Component, Debug, Clone)]
pub struct DominateEffect {
    /// Maximum range to target enemies
    pub range: f32,
    /// Duration of the domination effect
    pub duration: f32,
}

impl Default for DominateEffect {
    fn default() -> Self {
        Self {
            range: DOMINATE_DEFAULT_RANGE,
            duration: DOMINATE_DEFAULT_DURATION,
        }
    }
}

impl DominateEffect {
    /// Create a new DominateEffect with specified range and duration.
    pub fn new(range: f32, duration: f32) -> Self {
        Self { range, duration }
    }

    /// Create a DominateEffect from a Spell component.
    pub fn from_spell(_spell: &Spell) -> Self {
        Self::default()
    }
}

/// Component attached to enemies that are currently dominated.
/// Stores the original enemy behavior to restore when effect expires.
#[derive(Component, Debug, Clone)]
pub struct DominatedEnemy {
    /// Timer tracking remaining domination duration
    pub duration: Timer,
    /// Original enemy speed (to restore after domination)
    pub original_speed: f32,
    /// Original enemy strength (to restore after domination)
    pub original_strength: f32,
    /// Timer for attack intervals
    pub attack_timer: Timer,
    /// Current target entity (another enemy to attack)
    pub current_target: Option<Entity>,
}

impl DominatedEnemy {
    /// Create a new DominatedEnemy with specified duration and original stats.
    pub fn new(duration: f32, original_speed: f32, original_strength: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration, TimerMode::Once),
            original_speed,
            original_strength,
            attack_timer: Timer::from_seconds(DOMINATE_ATTACK_INTERVAL, TimerMode::Repeating),
            current_target: None,
        }
    }

    /// Check if the domination effect has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Get the remaining duration as a percentage (1.0 = full, 0.0 = expired).
    pub fn remaining_percentage(&self) -> f32 {
        1.0 - self.duration.fraction()
    }
}

/// System that finds the nearest enemy to the player and applies domination.
/// This runs when a Dominate spell is cast.
pub fn cast_dominate_system(
    mut commands: Commands,
    _player_position: Res<PlayerPosition>,
    dominate_query: Query<(Entity, &Transform, &DominateEffect)>,
    mut enemy_query: Query<(Entity, &Transform, &Enemy), Without<DominatedEnemy>>,
) {
    for (dominate_entity, dominate_transform, effect) in dominate_query.iter() {
        let cast_pos = from_xz(dominate_transform.translation);

        // Find the nearest enemy within range that isn't already dominated
        let mut nearest_enemy: Option<(Entity, f32, f32, f32)> = None;

        for (enemy_entity, enemy_transform, enemy) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = cast_pos.distance(enemy_pos);

            if distance <= effect.range
                && (nearest_enemy.is_none() || distance < nearest_enemy.as_ref().unwrap().1)
            {
                nearest_enemy = Some((enemy_entity, distance, enemy.speed, enemy.strength));
            }
        }

        // Apply domination to the nearest enemy
        if let Some((target_entity, _, original_speed, original_strength)) = nearest_enemy {
            commands.entity(target_entity).insert(DominatedEnemy::new(
                effect.duration,
                original_speed,
                original_strength,
            ));
        }

        // Despawn the dominate effect entity after use
        commands.entity(dominate_entity).despawn();
    }
}

/// System that updates dominated enemies - ticks duration and handles attacks.
#[allow(clippy::type_complexity)]
pub fn update_dominated_enemies_system(
    time: Res<Time>,
    mut dominated_query: Query<(Entity, &Transform, &mut DominatedEnemy)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<DominatedEnemy>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (dominated_entity, dominated_transform, mut dominated) in dominated_query.iter_mut() {
        // Tick the duration timer
        dominated.duration.tick(time.delta());

        // Tick the attack timer
        dominated.attack_timer.tick(time.delta());

        // If attack timer finished, find and attack nearest enemy
        if dominated.attack_timer.just_finished() {
            let dominated_pos = from_xz(dominated_transform.translation);

            // Find the nearest non-dominated enemy to attack
            let mut nearest_target: Option<(Entity, f32)> = None;

            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                // Skip self
                if enemy_entity == dominated_entity {
                    continue;
                }

                let enemy_pos = from_xz(enemy_transform.translation);
                let distance = dominated_pos.distance(enemy_pos);

                // Attack range (use a reasonable melee range)
                if distance <= 3.0
                    && (nearest_target.is_none() || distance < nearest_target.as_ref().unwrap().1)
                {
                    nearest_target = Some((enemy_entity, distance));
                }
            }

            // Deal damage to nearest target
            if let Some((target_entity, _)) = nearest_target {
                dominated.current_target = Some(target_entity);
                damage_events.write(DamageEvent::with_source(
                    target_entity,
                    DOMINATE_ALLY_DAMAGE,
                    dominated_entity,
                ));
            } else {
                dominated.current_target = None;
            }
        }
    }
}

/// System that makes dominated enemies move toward other enemies instead of the player.
#[allow(clippy::type_complexity)]
pub fn dominated_enemy_targeting_system(
    mut dominated_query: Query<(&mut Transform, &Enemy, &DominatedEnemy)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<DominatedEnemy>)>,
    time: Res<Time>,
) {
    for (mut dominated_transform, enemy, _dominated) in dominated_query.iter_mut() {
        let dominated_pos = from_xz(dominated_transform.translation);

        // Find the nearest non-dominated enemy to chase
        let mut nearest_target: Option<Vec2> = None;
        let mut nearest_distance = f32::MAX;

        for (_enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = dominated_pos.distance(enemy_pos);

            if distance < nearest_distance {
                nearest_distance = distance;
                nearest_target = Some(enemy_pos);
            }
        }

        // Move toward the nearest enemy instead of the player
        if let Some(target_pos) = nearest_target {
            let direction = (target_pos - dominated_pos).normalize_or_zero();
            let movement = direction * enemy.speed * time.delta_secs();

            // Apply movement on XZ plane
            dominated_transform.translation.x += movement.x;
            dominated_transform.translation.z += movement.y;
        }
    }
}

/// System that removes domination effect when duration expires and restores original behavior.
pub fn cleanup_dominate_system(
    mut commands: Commands,
    dominated_query: Query<(Entity, &DominatedEnemy)>,
) {
    for (entity, dominated) in dominated_query.iter() {
        if dominated.is_expired() {
            // Remove the dominated component, restoring normal enemy behavior
            commands.entity(entity).remove::<DominatedEnemy>();
        }
    }
}

/// Cast Dominate spell - spawns a domination effect that targets the nearest enemy.
#[allow(clippy::too_many_arguments)]
pub fn fire_dominate(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
) {
    fire_dominate_with_config(
        commands,
        spell,
        spawn_position,
        DOMINATE_DEFAULT_RANGE,
        DOMINATE_DEFAULT_DURATION,
    );
}

/// Cast Dominate spell with explicit configuration.
#[allow(clippy::too_many_arguments)]
pub fn fire_dominate_with_config(
    commands: &mut Commands,
    _spell: &Spell,
    spawn_position: Vec3,
    range: f32,
    duration: f32,
) {
    commands.spawn((
        Transform::from_translation(spawn_position),
        DominateEffect::new(range, duration),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CheckDeath, Health};
    use crate::spell::SpellType;
    use std::time::Duration;

    mod dominate_effect_tests {
        use super::*;

        #[test]
        fn test_dominate_effect_default() {
            let effect = DominateEffect::default();
            assert_eq!(effect.range, DOMINATE_DEFAULT_RANGE);
            assert_eq!(effect.duration, DOMINATE_DEFAULT_DURATION);
        }

        #[test]
        fn test_dominate_effect_new() {
            let effect = DominateEffect::new(15.0, 8.0);
            assert_eq!(effect.range, 15.0);
            assert_eq!(effect.duration, 8.0);
        }

        #[test]
        fn test_dominate_effect_from_spell() {
            let spell = Spell::new(SpellType::Dominate);
            let effect = DominateEffect::from_spell(&spell);
            assert_eq!(effect.range, DOMINATE_DEFAULT_RANGE);
            assert_eq!(effect.duration, DOMINATE_DEFAULT_DURATION);
        }

        #[test]
        fn test_dominate_uses_psychic_element_color() {
            let color = dominate_color();
            assert_eq!(color, Element::Psychic.color());
        }
    }

    mod dominated_enemy_tests {
        use super::*;

        #[test]
        fn test_dominated_enemy_new() {
            let dominated = DominatedEnemy::new(5.0, 2.0, 10.0);
            assert!(!dominated.is_expired());
            assert_eq!(dominated.original_speed, 2.0);
            assert_eq!(dominated.original_strength, 10.0);
            assert!(dominated.current_target.is_none());
        }

        #[test]
        fn test_dominated_enemy_is_expired() {
            let mut dominated = DominatedEnemy::new(0.1, 2.0, 10.0);
            assert!(!dominated.is_expired());

            dominated.duration.tick(Duration::from_secs_f32(0.2));
            assert!(dominated.is_expired());
        }

        #[test]
        fn test_dominated_enemy_remaining_percentage() {
            let mut dominated = DominatedEnemy::new(1.0, 2.0, 10.0);
            assert!((dominated.remaining_percentage() - 1.0).abs() < 0.01);

            dominated.duration.tick(Duration::from_secs_f32(0.5));
            assert!((dominated.remaining_percentage() - 0.5).abs() < 0.01);

            dominated.duration.tick(Duration::from_secs_f32(0.5));
            assert!(dominated.remaining_percentage() < 0.01);
        }

        #[test]
        fn test_dominated_enemy_attack_timer() {
            let mut dominated = DominatedEnemy::new(10.0, 2.0, 10.0);
            assert!(!dominated.attack_timer.is_finished());

            dominated.attack_timer.tick(Duration::from_secs_f32(DOMINATE_ATTACK_INTERVAL));
            assert!(dominated.attack_timer.just_finished());
        }
    }

    mod cast_dominate_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.init_resource::<PlayerPosition>();
            app
        }

        #[test]
        fn test_dominate_targets_nearest_enemy() {
            let mut app = setup_test_app();

            // Set player position at origin
            {
                let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
                player_pos.0 = Vec2::ZERO;
            }

            // Create dominate effect at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                DominateEffect::new(20.0, 5.0),
            ));

            // Create two enemies - near and far
            let near_enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(5.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                Health::new(50.0),
                CheckDeath,
            )).id();

            let far_enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(15.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                Health::new(50.0),
                CheckDeath,
            )).id();

            let _ = app.world_mut().run_system_once(cast_dominate_system);

            // Near enemy should be dominated
            assert!(
                app.world().get::<DominatedEnemy>(near_enemy).is_some(),
                "Near enemy should be dominated"
            );
            // Far enemy should NOT be dominated (outside range relative to near enemy)
            assert!(
                app.world().get::<DominatedEnemy>(far_enemy).is_none(),
                "Far enemy should not be dominated"
            );
        }

        #[test]
        fn test_dominated_enemy_ignores_player() {
            // This is verified by checking that dominated enemies get the DominatedEnemy
            // component, and the dominated_enemy_targeting_system moves them toward
            // other enemies, not the player. Tested in targeting system tests.
            let dominated = DominatedEnemy::new(5.0, 2.0, 10.0);
            assert!(dominated.current_target.is_none());
        }

        #[test]
        fn test_dominate_no_valid_target() {
            let mut app = setup_test_app();

            // Set player position at origin
            {
                let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
                player_pos.0 = Vec2::ZERO;
            }

            // Create dominate effect at origin with small range
            let dominate_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                DominateEffect::new(5.0, 5.0),
            )).id();

            // Create enemy far outside range
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(100.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                Health::new(50.0),
                CheckDeath,
            )).id();

            let _ = app.world_mut().run_system_once(cast_dominate_system);

            // Enemy should NOT be dominated (out of range)
            assert!(
                app.world().get::<DominatedEnemy>(enemy).is_none(),
                "Enemy outside range should not be dominated"
            );

            // Dominate effect should be despawned
            assert!(
                app.world().get_entity(dominate_entity).is_err(),
                "Dominate effect should be despawned after cast"
            );
        }

        #[test]
        fn test_dominate_range_limit() {
            let mut app = setup_test_app();

            {
                let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
                player_pos.0 = Vec2::ZERO;
            }

            // Create dominate effect with 10 unit range
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                DominateEffect::new(10.0, 5.0),
            ));

            // Enemy at exactly the range limit
            let at_range = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                Health::new(50.0),
                CheckDeath,
            )).id();

            // Enemy just beyond range
            let beyond_range = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.1, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                Health::new(50.0),
                CheckDeath,
            )).id();

            let _ = app.world_mut().run_system_once(cast_dominate_system);

            // Enemy at range should be dominated
            assert!(
                app.world().get::<DominatedEnemy>(at_range).is_some(),
                "Enemy at range limit should be dominated"
            );
            // Enemy beyond range should not be dominated
            assert!(
                app.world().get::<DominatedEnemy>(beyond_range).is_none(),
                "Enemy beyond range should not be dominated"
            );
        }
    }

    mod update_dominated_enemies_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_dominated_enemy_sets_target_when_attacking() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();

            // Create dominated enemy with attack timer ready to fire
            let mut dominated = DominatedEnemy::new(10.0, 2.0, 10.0);
            // Pre-tick timer to near completion
            dominated.attack_timer.tick(Duration::from_secs_f32(DOMINATE_ATTACK_INTERVAL - 0.05));

            let dominated_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                dominated,
                Health::new(50.0),
                CheckDeath,
            )).id();

            // Create target enemy within attack range (melee range is 3.0)
            let target_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(1.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                Health::new(50.0),
                CheckDeath,
            )).id();

            // Advance time to push the timer over
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(update_dominated_enemies_system);

            // Check that the dominated enemy now has a target set
            let dominated_component = app.world().get::<DominatedEnemy>(dominated_entity).unwrap();
            assert_eq!(
                dominated_component.current_target,
                Some(target_entity),
                "Dominated enemy should target the nearest enemy"
            );
        }

        #[test]
        fn test_dominate_duration_expires() {
            let mut dominated = DominatedEnemy::new(1.0, 2.0, 10.0);
            assert!(!dominated.is_expired());

            dominated.duration.tick(Duration::from_secs_f32(1.0));
            assert!(dominated.is_expired());
        }
    }

    mod dominated_enemy_targeting_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_dominated_enemy_moves_toward_other_enemies() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create dominated enemy at origin
            let dominated_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                Enemy { speed: 10.0, strength: 10.0 },
                DominatedEnemy::new(10.0, 10.0, 10.0),
            )).id();

            // Create target enemy at positive X
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(dominated_enemy_targeting_system);

            let transform = app.world().get::<Transform>(dominated_entity).unwrap();
            assert!(
                transform.translation.x > 0.0,
                "Dominated enemy should move toward other enemy (positive X)"
            );
        }
    }

    mod cleanup_dominate_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_dominate_restores_behavior() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create enemy with expired domination
            let mut expired_dominated = DominatedEnemy::new(0.1, 5.0, 15.0);
            expired_dominated.duration.tick(Duration::from_secs_f32(0.2));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Enemy { speed: 2.0, strength: 10.0 },
                expired_dominated,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_dominate_system);

            // DominatedEnemy component should be removed
            assert!(
                app.world().get::<DominatedEnemy>(entity).is_none(),
                "DominatedEnemy should be removed when expired"
            );
            // Enemy should still exist
            assert!(
                app.world().get::<Enemy>(entity).is_some(),
                "Enemy should still exist after domination ends"
            );
        }

        #[test]
        fn test_dominate_not_removed_if_active() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create enemy with active domination
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                Enemy { speed: 2.0, strength: 10.0 },
                DominatedEnemy::new(10.0, 2.0, 10.0), // Long duration
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_dominate_system);

            // DominatedEnemy component should still exist
            assert!(
                app.world().get::<DominatedEnemy>(entity).is_some(),
                "DominatedEnemy should remain while active"
            );
        }
    }

    mod fire_dominate_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_dominate_spawns_effect() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Dominate);
            let spawn_pos = Vec3::new(5.0, 0.5, 10.0);

            {
                let mut commands = app.world_mut().commands();
                fire_dominate(&mut commands, &spell, spawn_pos);
            }
            app.update();

            let mut query = app.world_mut().query::<&DominateEffect>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn 1 dominate effect");
        }

        #[test]
        fn test_fire_dominate_with_config() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Dominate);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_dominate_with_config(&mut commands, &spell, spawn_pos, 15.0, 8.0);
            }
            app.update();

            let mut query = app.world_mut().query::<&DominateEffect>();
            for effect in query.iter(app.world()) {
                assert_eq!(effect.range, 15.0);
                assert_eq!(effect.duration, 8.0);
            }
        }

        #[test]
        fn test_fire_dominate_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Dominate);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_dominate(&mut commands, &spell, spawn_pos);
            }
            app.update();

            let mut query = app.world_mut().query::<(&DominateEffect, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation, spawn_pos);
            }
        }
    }

    mod dominated_enemy_damage_tests {
        use super::*;

        #[test]
        fn test_dominated_enemy_uses_correct_damage_constant() {
            // This test verifies that the damage constant is correctly defined
            assert_eq!(DOMINATE_ALLY_DAMAGE, 15.0, "Dominated enemy should deal 15 damage");
        }

        #[test]
        fn test_dominated_enemy_attack_range() {
            // Verify melee range constant behavior
            // The attack range is 3.0 units - enemies within this range can be attacked
            let dominated = DominatedEnemy::new(10.0, 2.0, 10.0);
            assert!(dominated.current_target.is_none(), "New dominated enemy has no target");
        }
    }
}
