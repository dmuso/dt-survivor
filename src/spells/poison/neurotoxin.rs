//! Neurotoxin spell - Poison that increases enemy erratic movement.
//!
//! A Poison element spell (Necrosis SpellType) that causes poison damage to apply
//! the NeurotoxinDebuff to enemies. Enemies with the debuff have random jitter
//! added to their movement direction, making it harder for them to reach the player.
//! Multiple poison sources refresh duration but don't stack jitter amount.

use bevy::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::movement::components::to_xz;

/// Default configuration for Neurotoxin debuff
pub const NEUROTOXIN_DURATION: f32 = 5.0;
pub const NEUROTOXIN_BASE_JITTER_AMOUNT: f32 = 0.5; // Radians - max deviation angle
pub const NEUROTOXIN_JITTER_INTERVAL: f32 = 0.2; // Seconds between jitter direction changes

/// Get the poison element color for visual effects
pub fn neurotoxin_color() -> Color {
    Element::Poison.color()
}

/// Neurotoxin debuff applied to enemies that have taken poison damage.
/// Causes the enemy to move erratically by adding random direction offsets.
/// Duration refreshes on reapplication, but jitter amount does not stack.
#[derive(Component, Debug, Clone)]
pub struct NeurotoxinDebuff {
    /// Remaining duration of the neurotoxin effect
    pub duration: Timer,
    /// Maximum deviation angle in radians (scaled by spell level)
    pub jitter_amount: f32,
    /// Timer between jitter direction changes
    pub jitter_timer: Timer,
    /// Current jitter offset applied to movement direction (Vec2 on XZ plane)
    pub current_jitter: Vec2,
}

impl NeurotoxinDebuff {
    pub fn new(duration_secs: f32, jitter_amount: f32, jitter_interval: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            jitter_amount,
            jitter_timer: Timer::from_seconds(jitter_interval, TimerMode::Repeating),
            current_jitter: Vec2::ZERO,
        }
    }

    /// Check if the neurotoxin effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Tick the jitter timer and return true if jitter should be recalculated
    pub fn tick_jitter(&mut self, delta: std::time::Duration) -> bool {
        self.jitter_timer.tick(delta);
        self.jitter_timer.just_finished()
    }

    /// Refresh the neurotoxin duration (for reapplying effect)
    /// Does NOT stack the jitter amount - keeps existing jitter
    pub fn refresh(&mut self, duration_secs: f32) {
        self.duration = Timer::from_seconds(duration_secs, TimerMode::Once);
    }

    /// Calculate and store a new random jitter offset
    pub fn recalculate_jitter(&mut self, rng: &mut impl Rng) {
        // Generate random angle and magnitude for jitter
        let angle = rng.gen_range(-self.jitter_amount..self.jitter_amount);
        let magnitude = rng.gen_range(0.0..1.0);

        // Convert to Vec2 offset on XZ plane
        self.current_jitter = Vec2::new(angle.cos() * magnitude, angle.sin() * magnitude);
    }

    /// Get the current jitter offset as a Vec2
    pub fn jitter_offset(&self) -> Vec2 {
        self.current_jitter
    }
}

impl Default for NeurotoxinDebuff {
    fn default() -> Self {
        Self::new(NEUROTOXIN_DURATION, NEUROTOXIN_BASE_JITTER_AMOUNT, NEUROTOXIN_JITTER_INTERVAL)
    }
}

/// System that ticks neurotoxin debuff timers, updates jitter, and removes expired debuffs
pub fn neurotoxin_debuff_tick_system(
    mut commands: Commands,
    time: Res<Time>,
    mut debuff_query: Query<(Entity, &mut NeurotoxinDebuff)>,
) {
    let mut rng = rand::thread_rng();

    for (entity, mut debuff) in debuff_query.iter_mut() {
        debuff.tick(time.delta());

        if debuff.is_expired() {
            commands.entity(entity).remove::<NeurotoxinDebuff>();
        } else {
            // Update jitter direction at intervals
            if debuff.tick_jitter(time.delta()) {
                debuff.recalculate_jitter(&mut rng);
            }
        }
    }
}

/// System that applies movement jitter to enemies with NeurotoxinDebuff.
/// This runs after the normal enemy movement system to add random offsets.
pub fn neurotoxin_movement_jitter_system(
    mut enemy_query: Query<(&mut Transform, &NeurotoxinDebuff), With<Enemy>>,
    time: Res<Time>,
) {
    for (mut transform, debuff) in enemy_query.iter_mut() {
        // Apply jitter offset to position on XZ plane
        // The jitter is a small random movement added on top of normal movement
        let jitter = debuff.jitter_offset() * time.delta_secs() * 2.0; // Scale jitter effect
        transform.translation += to_xz(jitter);
    }
}

/// System that applies NeurotoxinDebuff to enemies when they take poison damage.
/// This listens for DamageEvents with Element::Poison and applies/refreshes the debuff.
pub fn apply_neurotoxin_on_poison_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    enemy_query: Query<Entity, With<Enemy>>,
    mut debuff_query: Query<&mut NeurotoxinDebuff>,
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

        // Check if enemy already has NeurotoxinDebuff
        if let Ok(mut debuff) = debuff_query.get_mut(event.target) {
            // Refresh duration, don't stack jitter
            debuff.refresh(NEUROTOXIN_DURATION);
        } else {
            // Apply new NeurotoxinDebuff
            commands
                .entity(event.target)
                .try_insert(NeurotoxinDebuff::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    mod neurotoxin_debuff_component_tests {
        use super::*;

        #[test]
        fn test_neurotoxin_debuff_new() {
            let debuff = NeurotoxinDebuff::new(5.0, 0.5, 0.2);
            assert_eq!(debuff.jitter_amount, 0.5);
            assert!(!debuff.is_expired());
            assert_eq!(debuff.current_jitter, Vec2::ZERO);
        }

        #[test]
        fn test_neurotoxin_debuff_default() {
            let debuff = NeurotoxinDebuff::default();
            assert_eq!(debuff.jitter_amount, NEUROTOXIN_BASE_JITTER_AMOUNT);
            assert!(!debuff.is_expired());
        }

        #[test]
        fn test_neurotoxin_debuff_tick_expires() {
            let mut debuff = NeurotoxinDebuff::new(1.0, 0.5, 0.2);
            assert!(!debuff.is_expired());

            debuff.tick(Duration::from_secs_f32(1.1));
            assert!(debuff.is_expired());
        }

        #[test]
        fn test_neurotoxin_debuff_tick_not_expired() {
            let mut debuff = NeurotoxinDebuff::new(2.0, 0.5, 0.2);
            debuff.tick(Duration::from_secs_f32(0.5));
            assert!(!debuff.is_expired());
        }

        #[test]
        fn test_neurotoxin_debuff_refresh_resets_duration() {
            let mut debuff = NeurotoxinDebuff::new(1.0, 0.5, 0.2);
            debuff.tick(Duration::from_secs_f32(0.9));
            assert!(!debuff.is_expired());

            // Refresh to 2 seconds
            debuff.refresh(2.0);
            debuff.tick(Duration::from_secs_f32(1.5));
            assert!(!debuff.is_expired(), "Should still have time after refresh");
        }

        #[test]
        fn test_neurotoxin_debuff_refresh_does_not_stack_jitter() {
            let mut debuff = NeurotoxinDebuff::new(1.0, 0.5, 0.2);
            let original_jitter = debuff.jitter_amount;

            // Refresh should NOT increase jitter
            debuff.refresh(2.0);
            assert_eq!(
                debuff.jitter_amount, original_jitter,
                "Refresh should not change jitter amount"
            );
        }

        #[test]
        fn test_neurotoxin_jitter_timer_triggers() {
            let mut debuff = NeurotoxinDebuff::new(5.0, 0.5, 0.2);

            // Should not trigger immediately
            assert!(!debuff.tick_jitter(Duration::from_secs_f32(0.1)));

            // Should trigger after interval
            assert!(debuff.tick_jitter(Duration::from_secs_f32(0.15)));
        }

        #[test]
        fn test_neurotoxin_recalculate_jitter_changes_value() {
            let mut debuff = NeurotoxinDebuff::new(5.0, 0.5, 0.2);
            let mut rng = StdRng::seed_from_u64(12345);

            assert_eq!(debuff.current_jitter, Vec2::ZERO);

            debuff.recalculate_jitter(&mut rng);
            assert_ne!(debuff.current_jitter, Vec2::ZERO, "Jitter should be non-zero after calculation");
        }

        #[test]
        fn test_neurotoxin_jitter_offset_returns_current_jitter() {
            let mut debuff = NeurotoxinDebuff::new(5.0, 0.5, 0.2);
            debuff.current_jitter = Vec2::new(0.3, 0.4);

            assert_eq!(debuff.jitter_offset(), Vec2::new(0.3, 0.4));
        }

        #[test]
        fn test_neurotoxin_uses_poison_element_color() {
            let color = neurotoxin_color();
            assert_eq!(color, Element::Poison.color());
        }

        #[test]
        fn test_neurotoxin_expires_after_duration() {
            let mut debuff = NeurotoxinDebuff::new(NEUROTOXIN_DURATION, NEUROTOXIN_BASE_JITTER_AMOUNT, NEUROTOXIN_JITTER_INTERVAL);

            // Advance time past default duration
            debuff.tick(Duration::from_secs_f32(NEUROTOXIN_DURATION + 0.1));

            assert!(
                debuff.is_expired(),
                "Neurotoxin debuff should expire after {} seconds",
                NEUROTOXIN_DURATION
            );
        }
    }

    mod neurotoxin_debuff_tick_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_neurotoxin_system_ticks_duration() {
            let mut app = setup_test_app();

            // Spawn entity with neurotoxin debuff
            let entity = app.world_mut().spawn(NeurotoxinDebuff::new(2.0, 0.5, 0.2)).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_debuff_tick_system);

            // Debuff should still exist
            assert!(app.world().get::<NeurotoxinDebuff>(entity).is_some());
        }

        #[test]
        fn test_neurotoxin_system_removes_expired() {
            let mut app = setup_test_app();

            // Spawn entity with short neurotoxin debuff
            let entity = app.world_mut().spawn(NeurotoxinDebuff::new(0.5, 0.5, 0.2)).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_debuff_tick_system);

            // Debuff should be removed
            assert!(app.world().get::<NeurotoxinDebuff>(entity).is_none());
        }

        #[test]
        fn test_neurotoxin_system_preserves_other_components() {
            let mut app = setup_test_app();

            // Spawn entity with neurotoxin debuff and other components
            let entity = app
                .world_mut()
                .spawn((
                    NeurotoxinDebuff::new(0.5, 0.5, 0.2),
                    Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
                ))
                .id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.6));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_debuff_tick_system);

            // Entity should still exist with Transform
            assert!(app.world().entities().contains(entity));
            assert!(app.world().get::<Transform>(entity).is_some());
        }

        #[test]
        fn test_neurotoxin_system_updates_jitter() {
            let mut app = setup_test_app();

            // Spawn entity with neurotoxin debuff
            let entity = app.world_mut().spawn(NeurotoxinDebuff::new(5.0, 0.5, 0.2)).id();

            // Initial jitter should be zero
            {
                let debuff = app.world().get::<NeurotoxinDebuff>(entity).unwrap();
                assert_eq!(debuff.current_jitter, Vec2::ZERO);
            }

            // Advance time past jitter interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.25));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_debuff_tick_system);

            // Jitter should have been updated
            let debuff = app.world().get::<NeurotoxinDebuff>(entity).unwrap();
            assert_ne!(debuff.current_jitter, Vec2::ZERO, "Jitter should be updated after interval");
        }
    }

    mod apply_neurotoxin_on_poison_damage_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_neurotoxin_applied_on_poison_damage() {
            let mut app = setup_test_app();

            // Spawn enemy
            let enemy = app
                .world_mut()
                .spawn(Enemy { speed: 50.0, strength: 10.0 })
                .id();

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_neurotoxin_on_poison_damage);

            // Enemy should have NeurotoxinDebuff
            let debuff = app.world().get::<NeurotoxinDebuff>(enemy);
            assert!(debuff.is_some(), "Enemy should have NeurotoxinDebuff after poison damage");
        }

        #[test]
        fn test_neurotoxin_not_applied_on_non_poison_damage() {
            let mut app = setup_test_app();

            // Spawn enemy
            let enemy = app
                .world_mut()
                .spawn(Enemy { speed: 50.0, strength: 10.0 })
                .id();

            // Send fire damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Fire));

            let _ = app.world_mut().run_system_once(apply_neurotoxin_on_poison_damage);

            // Enemy should NOT have NeurotoxinDebuff
            assert!(
                app.world().get::<NeurotoxinDebuff>(enemy).is_none(),
                "Enemy should not have NeurotoxinDebuff from fire damage"
            );
        }

        #[test]
        fn test_neurotoxin_not_applied_on_no_element_damage() {
            let mut app = setup_test_app();

            // Spawn enemy
            let enemy = app
                .world_mut()
                .spawn(Enemy { speed: 50.0, strength: 10.0 })
                .id();

            // Send damage event without element
            app.world_mut().write_message(DamageEvent::new(enemy, 25.0));

            let _ = app.world_mut().run_system_once(apply_neurotoxin_on_poison_damage);

            // Enemy should NOT have NeurotoxinDebuff
            assert!(
                app.world().get::<NeurotoxinDebuff>(enemy).is_none(),
                "Enemy should not have NeurotoxinDebuff from elementless damage"
            );
        }

        #[test]
        fn test_neurotoxin_not_applied_to_non_enemy() {
            let mut app = setup_test_app();

            // Spawn non-enemy entity
            let entity = app.world_mut().spawn(Transform::default()).id();

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(entity, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_neurotoxin_on_poison_damage);

            // Entity should NOT have NeurotoxinDebuff (not an enemy)
            assert!(
                app.world().get::<NeurotoxinDebuff>(entity).is_none(),
                "Non-enemy should not receive NeurotoxinDebuff"
            );
        }

        #[test]
        fn test_neurotoxin_refreshes_duration_on_reapply() {
            let mut app = setup_test_app();

            // Spawn enemy with existing neurotoxin debuff
            let enemy = app
                .world_mut()
                .spawn((Enemy { speed: 50.0, strength: 10.0 }, NeurotoxinDebuff::new(1.0, 0.5, 0.2)))
                .id();

            // Tick down the debuff
            {
                let mut debuff = app.world_mut().get_mut::<NeurotoxinDebuff>(enemy).unwrap();
                debuff.tick(Duration::from_secs_f32(0.8));
            }

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_neurotoxin_on_poison_damage);

            // Check debuff was refreshed (duration should be close to NEUROTOXIN_DURATION)
            let debuff = app.world().get::<NeurotoxinDebuff>(enemy).unwrap();
            let remaining = debuff.duration.remaining_secs();
            assert!(
                (remaining - NEUROTOXIN_DURATION).abs() < 0.1,
                "Neurotoxin duration should be refreshed to {}, got {}",
                NEUROTOXIN_DURATION,
                remaining
            );
        }

        #[test]
        fn test_neurotoxin_no_jitter_stacking_on_reapply() {
            let mut app = setup_test_app();

            // Spawn enemy with existing neurotoxin debuff
            let original_jitter = 0.5;
            let enemy = app
                .world_mut()
                .spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    NeurotoxinDebuff::new(1.0, original_jitter, 0.2),
                ))
                .id();

            // Send poison damage event
            app.world_mut()
                .write_message(DamageEvent::with_element(enemy, 25.0, Element::Poison));

            let _ = app.world_mut().run_system_once(apply_neurotoxin_on_poison_damage);

            // Jitter amount should NOT have changed
            let debuff = app.world().get::<NeurotoxinDebuff>(enemy).unwrap();
            assert_eq!(
                debuff.jitter_amount, original_jitter,
                "Jitter amount should not stack on reapply"
            );
        }
    }

    mod neurotoxin_movement_jitter_system_tests {
        use super::*;
        use bevy::app::App;
        use crate::game::resources::PlayerPosition;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.init_resource::<PlayerPosition>();
            app
        }

        #[test]
        fn test_neurotoxin_adds_movement_jitter() {
            let mut app = setup_test_app();

            // Create enemy with neurotoxin debuff that has non-zero jitter
            let mut debuff = NeurotoxinDebuff::new(5.0, 0.5, 0.2);
            debuff.current_jitter = Vec2::new(1.0, 0.5);

            let enemy = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                    Enemy { speed: 100.0, strength: 10.0 },
                    debuff,
                ))
                .id();

            // Record initial position
            let initial_pos = app.world().get::<Transform>(enemy).unwrap().translation;

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_movement_jitter_system);

            // Enemy should have moved due to jitter
            let final_pos = app.world().get::<Transform>(enemy).unwrap().translation;
            assert_ne!(final_pos, initial_pos, "Enemy should have moved due to jitter");
            assert_eq!(final_pos.y, initial_pos.y, "Y position should be preserved");
        }

        #[test]
        fn test_neurotoxin_enemy_still_moves_toward_player() {
            // This test verifies that jitter is additive and doesn't prevent movement
            let mut app = setup_test_app();

            // Set player position
            {
                let mut player_pos = app.world_mut().get_resource_mut::<PlayerPosition>().unwrap();
                player_pos.0 = Vec2::new(100.0, 0.0);
            }

            // Create enemy with small jitter
            let mut debuff = NeurotoxinDebuff::new(5.0, 0.1, 0.2);
            debuff.current_jitter = Vec2::new(0.1, 0.0);

            let enemy = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                    Enemy { speed: 100.0, strength: 10.0 },
                    debuff,
                ))
                .id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            // Run jitter system
            let _ = app.world_mut().run_system_once(neurotoxin_movement_jitter_system);

            // Enemy should have moved in positive X direction due to jitter
            let transform = app.world().get::<Transform>(enemy).unwrap();
            assert!(
                transform.translation.x > 0.0,
                "Enemy should move in X direction, got {}",
                transform.translation.x
            );
        }

        #[test]
        fn test_neurotoxin_multiple_enemies_independent_jitter() {
            let mut app = setup_test_app();

            // Create two enemies with different jitter values
            let mut debuff1 = NeurotoxinDebuff::new(5.0, 0.5, 0.2);
            debuff1.current_jitter = Vec2::new(1.0, 0.0);

            let mut debuff2 = NeurotoxinDebuff::new(5.0, 0.5, 0.2);
            debuff2.current_jitter = Vec2::new(-1.0, 0.0);

            let enemy1 = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                    Enemy { speed: 100.0, strength: 10.0 },
                    debuff1,
                ))
                .id();

            let enemy2 = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                    Enemy { speed: 100.0, strength: 10.0 },
                    debuff2,
                ))
                .id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_movement_jitter_system);

            // Enemies should have moved in different directions
            let pos1 = app.world().get::<Transform>(enemy1).unwrap().translation;
            let pos2 = app.world().get::<Transform>(enemy2).unwrap().translation;

            assert!(
                pos1.x > 0.0,
                "Enemy 1 should move in positive X direction"
            );
            assert!(
                pos2.x < 0.0,
                "Enemy 2 should move in negative X direction"
            );
        }

        #[test]
        fn test_neurotoxin_enemy_without_debuff_not_affected() {
            let mut app = setup_test_app();

            // Create enemy without neurotoxin debuff
            let enemy = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                    Enemy { speed: 100.0, strength: 10.0 },
                ))
                .id();

            // Record initial position
            let initial_pos = app.world().get::<Transform>(enemy).unwrap().translation;

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(neurotoxin_movement_jitter_system);

            // Enemy should NOT have moved (no debuff)
            let final_pos = app.world().get::<Transform>(enemy).unwrap().translation;
            assert_eq!(final_pos, initial_pos, "Enemy without debuff should not be affected by jitter");
        }
    }
}
