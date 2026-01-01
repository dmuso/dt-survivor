//! Corrode spell - Poison damage applies a damage amplification debuff.
//!
//! A Poison element spell (Corrode SpellType) that causes poison damage to apply
//! the Corroded debuff to enemies. Enemies with Corroded take increased damage
//! from all sources. Multiple poison sources refresh duration but don't stack multiplier.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;

/// Default configuration for Corroded debuff
pub const CORRODED_DURATION: f32 = 4.0;
pub const CORRODED_DAMAGE_MULTIPLIER: f32 = 1.2; // 20% more damage taken

/// Get the poison element color for visual effects
pub fn corrode_color() -> Color {
    Element::Poison.color()
}

/// Corroded debuff applied to enemies that have taken poison damage.
/// Causes the enemy to take increased damage from all sources.
/// Duration refreshes on reapplication, but multiplier does not stack.
#[derive(Component, Debug, Clone)]
pub struct CorrodedDebuff {
    /// Remaining duration of the corroded effect
    pub duration: Timer,
    /// Damage multiplier (1.2 = 20% more damage taken)
    pub damage_multiplier: f32,
}

impl CorrodedDebuff {
    pub fn new(duration_secs: f32, damage_multiplier: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            damage_multiplier,
        }
    }

    /// Check if the corroded effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Refresh the corroded duration (for reapplying effect)
    /// Does NOT stack the multiplier - keeps existing multiplier
    pub fn refresh(&mut self, duration_secs: f32) {
        self.duration = Timer::from_seconds(duration_secs, TimerMode::Once);
    }
}

impl Default for CorrodedDebuff {
    fn default() -> Self {
        Self::new(CORRODED_DURATION, CORRODED_DAMAGE_MULTIPLIER)
    }
}

/// System that ticks corroded debuff timers and removes expired debuffs
pub fn corroded_debuff_tick_system(
    mut commands: Commands,
    time: Res<Time>,
    mut corroded_query: Query<(Entity, &mut CorrodedDebuff)>,
) {
    for (entity, mut corroded) in corroded_query.iter_mut() {
        corroded.tick(time.delta());

        if corroded.is_expired() {
            commands.entity(entity).remove::<CorrodedDebuff>();
        }
    }
}

/// System that applies CorrodedDebuff to enemies when they take poison damage.
/// This listens for DamageEvents with Element::Poison and applies/refreshes the debuff.
pub fn apply_corroded_on_poison_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    enemy_query: Query<Entity, With<Enemy>>,
    mut corroded_query: Query<&mut CorrodedDebuff>,
) {
    for event in damage_events.read() {
        // Only process poison damage
        if !event.is_poison() {
            continue;
        }

        // Only apply to enemies
        if !enemy_query.contains(event.target) {
            continue;
        }

        // Check if enemy already has CorrodedDebuff
        if let Ok(mut corroded) = corroded_query.get_mut(event.target) {
            // Refresh duration, don't stack multiplier
            corroded.refresh(CORRODED_DURATION);
        } else {
            // Apply new CorrodedDebuff
            commands
                .entity(event.target)
                .try_insert(CorrodedDebuff::default());
        }
    }
}

// Note: Damage amplification for CorrodedDebuff is handled in
// crate::combat::systems::apply_damage_system which checks for
// damage-modifying debuffs and applies their multipliers.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;

    mod corroded_debuff_component_tests {
        use super::*;

        #[test]
        fn test_corroded_debuff_new() {
            let corroded = CorrodedDebuff::new(4.0, 1.2);
            assert_eq!(corroded.damage_multiplier, 1.2);
            assert!(!corroded.is_expired());
        }

        #[test]
        fn test_corroded_debuff_default() {
            let corroded = CorrodedDebuff::default();
            assert_eq!(corroded.damage_multiplier, CORRODED_DAMAGE_MULTIPLIER);
            assert!(!corroded.is_expired());
        }

        #[test]
        fn test_corroded_debuff_tick_expires() {
            let mut corroded = CorrodedDebuff::new(1.0, 1.2);
            assert!(!corroded.is_expired());

            corroded.tick(Duration::from_secs_f32(1.1));
            assert!(corroded.is_expired());
        }

        #[test]
        fn test_corroded_debuff_tick_not_expired() {
            let mut corroded = CorrodedDebuff::new(2.0, 1.2);
            corroded.tick(Duration::from_secs_f32(0.5));
            assert!(!corroded.is_expired());
        }

        #[test]
        fn test_corroded_debuff_refresh_resets_duration() {
            let mut corroded = CorrodedDebuff::new(1.0, 1.2);
            corroded.tick(Duration::from_secs_f32(0.9));
            assert!(!corroded.is_expired());

            // Refresh to 2 seconds
            corroded.refresh(2.0);
            corroded.tick(Duration::from_secs_f32(1.5));
            assert!(!corroded.is_expired(), "Should still have time after refresh");
        }

        #[test]
        fn test_corroded_debuff_refresh_does_not_stack_multiplier() {
            let mut corroded = CorrodedDebuff::new(1.0, 1.2);
            let original_multiplier = corroded.damage_multiplier;

            // Refresh should NOT increase multiplier
            corroded.refresh(2.0);
            assert_eq!(
                corroded.damage_multiplier, original_multiplier,
                "Refresh should not change damage multiplier"
            );
        }

        #[test]
        fn test_corroded_debuff_uses_poison_element_color() {
            let color = corrode_color();
            assert_eq!(color, Element::Poison.color());
        }
    }

    mod corroded_debuff_tick_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_corroded_debuff_system_ticks_duration() {
            let mut app = setup_test_app();

            // Spawn entity with corroded debuff
            let entity = app.world_mut().spawn(CorrodedDebuff::new(2.0, 1.2)).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(corroded_debuff_tick_system);

            // Debuff should still exist
            assert!(app.world().get::<CorrodedDebuff>(entity).is_some());
        }

        #[test]
        fn test_corroded_debuff_system_removes_expired() {
            let mut app = setup_test_app();

            // Spawn entity with short corroded debuff
            let entity = app.world_mut().spawn(CorrodedDebuff::new(0.5, 1.2)).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(corroded_debuff_tick_system);

            // Debuff should be removed
            assert!(app.world().get::<CorrodedDebuff>(entity).is_none());
        }

        #[test]
        fn test_corroded_debuff_system_preserves_other_components() {
            let mut app = setup_test_app();

            // Spawn entity with corroded debuff and other components
            let entity = app
                .world_mut()
                .spawn((
                    CorrodedDebuff::new(0.5, 1.2),
                    Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
                ))
                .id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(corroded_debuff_tick_system);

            // Entity should still exist with Transform
            assert!(app.world().entities().contains(entity));
            assert!(app.world().get::<Transform>(entity).is_some());
        }

        #[test]
        fn test_corroded_debuff_expires_after_duration() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn(CorrodedDebuff::new(CORRODED_DURATION, CORRODED_DAMAGE_MULTIPLIER))
                .id();

            // Advance time past default duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(CORRODED_DURATION + 0.1));
            }

            let _ = app.world_mut().run_system_once(corroded_debuff_tick_system);

            // Debuff should be removed after default duration
            assert!(
                app.world().get::<CorrodedDebuff>(entity).is_none(),
                "Corroded debuff should be removed after {} seconds",
                CORRODED_DURATION
            );
        }
    }

    mod apply_corroded_on_poison_damage_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_corroded_applied_on_poison_damage() {
            let mut app = setup_test_app();

            // Spawn enemy
            let enemy = app
                .world_mut()
                .spawn(Enemy { speed: 50.0, strength: 10.0 })
                .id();

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_corroded_on_poison_damage);

            // Enemy should have CorrodedDebuff
            let corroded = app.world().get::<CorrodedDebuff>(enemy);
            assert!(corroded.is_some(), "Enemy should have CorrodedDebuff after poison damage");
        }

        #[test]
        fn test_corroded_not_applied_on_non_poison_damage() {
            let mut app = setup_test_app();

            // Spawn enemy
            let enemy = app
                .world_mut()
                .spawn(Enemy { speed: 50.0, strength: 10.0 })
                .id();

            // Send fire damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Fire));

            let _ = app.world_mut().run_system_once(apply_corroded_on_poison_damage);

            // Enemy should NOT have CorrodedDebuff
            assert!(
                app.world().get::<CorrodedDebuff>(enemy).is_none(),
                "Enemy should not have CorrodedDebuff from fire damage"
            );
        }

        #[test]
        fn test_corroded_not_applied_on_no_element_damage() {
            let mut app = setup_test_app();

            // Spawn enemy
            let enemy = app
                .world_mut()
                .spawn(Enemy { speed: 50.0, strength: 10.0 })
                .id();

            // Send damage event without element
            app.world_mut().write_message(DamageEvent::new(enemy, 25.0));

            let _ = app.world_mut().run_system_once(apply_corroded_on_poison_damage);

            // Enemy should NOT have CorrodedDebuff
            assert!(
                app.world().get::<CorrodedDebuff>(enemy).is_none(),
                "Enemy should not have CorrodedDebuff from elementless damage"
            );
        }

        #[test]
        fn test_corroded_not_applied_to_non_enemy() {
            let mut app = setup_test_app();

            // Spawn non-enemy entity
            let entity = app.world_mut().spawn(Transform::default()).id();

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(entity, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_corroded_on_poison_damage);

            // Entity should NOT have CorrodedDebuff (not an enemy)
            assert!(
                app.world().get::<CorrodedDebuff>(entity).is_none(),
                "Non-enemy should not receive CorrodedDebuff"
            );
        }

        #[test]
        fn test_corroded_refreshes_duration_on_reapply() {
            let mut app = setup_test_app();

            // Spawn enemy with existing corroded debuff
            let enemy = app
                .world_mut()
                .spawn((Enemy { speed: 50.0, strength: 10.0 }, CorrodedDebuff::new(1.0, 1.2)))
                .id();

            // Tick down the debuff
            {
                let mut corroded = app.world_mut().get_mut::<CorrodedDebuff>(enemy).unwrap();
                corroded.tick(Duration::from_secs_f32(0.8));
            }

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_corroded_on_poison_damage);

            // Check debuff was refreshed (duration should be close to CORRODED_DURATION)
            let corroded = app.world().get::<CorrodedDebuff>(enemy).unwrap();
            let remaining = corroded.duration.remaining_secs();
            assert!(
                (remaining - CORRODED_DURATION).abs() < 0.1,
                "Corroded duration should be refreshed to {}, got {}",
                CORRODED_DURATION,
                remaining
            );
        }

        #[test]
        fn test_corroded_no_multiplier_stacking_on_reapply() {
            let mut app = setup_test_app();

            // Spawn enemy with existing corroded debuff
            let original_multiplier = 1.2;
            let enemy = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    CorrodedDebuff::new(1.0, original_multiplier),
                ))
                .id();

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_corroded_on_poison_damage);

            // Multiplier should NOT have changed
            let corroded = app.world().get::<CorrodedDebuff>(enemy).unwrap();
            assert_eq!(
                corroded.damage_multiplier, original_multiplier,
                "Multiplier should not stack on reapply"
            );
        }

        #[test]
        fn test_corroded_affects_all_damage_types() {
            // This test verifies the damage amplification works with any damage type
            // by checking the apply_damage_system behavior
            use crate::combat::{Health, apply_damage_system};

            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();

            // Spawn enemy with health and corroded debuff (20% damage increase)
            let enemy = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Health::new(100.0),
                    CorrodedDebuff::new(10.0, 1.2),
                ))
                .id();

            // Send fire damage (not poison) - should still be amplified
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 10.0, Element::Fire));

            let _ = app.world_mut().run_system_once(apply_damage_system);

            // Health should be 100 - (10 * 1.2) = 88
            let health = app.world().get::<Health>(enemy).unwrap();
            assert!(
                (health.current - 88.0).abs() < 0.01,
                "Corroded should amplify fire damage: expected 88, got {}",
                health.current
            );
        }

        #[test]
        fn test_corroded_multiplier_calculation() {
            use crate::combat::{Health, apply_damage_system};

            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();

            // Spawn enemy with health and corroded debuff
            let enemy = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Health::new(100.0),
                    CorrodedDebuff::new(10.0, CORRODED_DAMAGE_MULTIPLIER),
                ))
                .id();

            // Send damage
            let base_damage = 25.0;
            app.world_mut().write_message(DamageEvent::new(enemy, base_damage));

            let _ = app.world_mut().run_system_once(apply_damage_system);

            // Health should be 100 - (25 * 1.2) = 70
            let expected_health = 100.0 - (base_damage * CORRODED_DAMAGE_MULTIPLIER);
            let health = app.world().get::<Health>(enemy).unwrap();
            assert!(
                (health.current - expected_health).abs() < 0.01,
                "Expected health {}, got {}",
                expected_health,
                health.current
            );
        }
    }
}
