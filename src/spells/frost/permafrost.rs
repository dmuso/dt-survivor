//! Permafrost spell - Passive synergy that adds freeze stacks on frost damage.
//!
//! A Frost element passive spell that enhances all frost damage. When enemies
//! are hit by ANY frost spell, they accumulate freeze stacks. At max stacks,
//! enemies become frozen (stunned) briefly. Stacks decay over time if not refreshed.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::player::components::Player;

/// Default configuration for Permafrost spell
pub const PERMAFROST_MAX_STACKS: u32 = 5;
pub const PERMAFROST_DECAY_TIME: f32 = 3.0; // Seconds before stacks start decaying
pub const PERMAFROST_FROZEN_DURATION: f32 = 2.0; // Seconds enemy is frozen

/// Get the frost element color for visual effects
pub fn permafrost_color() -> Color {
    Element::Frost.color()
}

/// Marker component added to player when Permafrost passive is equipped.
/// While this marker is present, frost damage applies freeze buildup.
#[derive(Component, Debug, Clone, Default)]
pub struct PermafrostEnabled;

/// Component tracking freeze buildup on an enemy.
/// Stacks increase when hit by frost spells and decay over time.
#[derive(Component, Debug, Clone)]
pub struct FreezeBuildup {
    /// Current number of freeze stacks
    pub stacks: u32,
    /// Maximum stacks before frozen status is applied
    pub max_stacks: u32,
    /// Timer for stack decay (resets on new frost damage)
    pub decay_timer: Timer,
}

impl FreezeBuildup {
    pub fn new(max_stacks: u32, decay_time_secs: f32) -> Self {
        Self {
            stacks: 0,
            max_stacks,
            decay_timer: Timer::from_seconds(decay_time_secs, TimerMode::Repeating),
        }
    }

    /// Add stacks to the buildup, capped at max_stacks.
    /// Resets the decay timer.
    pub fn add_stacks(&mut self, amount: u32) {
        self.stacks = (self.stacks + amount).min(self.max_stacks);
        self.decay_timer.reset();
    }

    /// Remove one stack. Returns true if stacks remain.
    pub fn decay_stack(&mut self) -> bool {
        if self.stacks > 0 {
            self.stacks -= 1;
        }
        self.stacks > 0
    }

    /// Check if freeze buildup has reached max stacks
    pub fn is_max(&self) -> bool {
        self.stacks >= self.max_stacks
    }

    /// Tick the decay timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.decay_timer.tick(delta);
    }

    /// Check if decay timer has finished (stack should decay)
    pub fn should_decay(&self) -> bool {
        self.decay_timer.just_finished()
    }

    /// Reset stacks to zero
    pub fn reset(&mut self) {
        self.stacks = 0;
        self.decay_timer.reset();
    }
}

impl Default for FreezeBuildup {
    fn default() -> Self {
        Self::new(PERMAFROST_MAX_STACKS, PERMAFROST_DECAY_TIME)
    }
}

/// Component applied to enemies when they reach max freeze stacks.
/// Prevents movement and attacks for the duration.
#[derive(Component, Debug, Clone)]
pub struct FrozenStatus {
    /// Total duration of frozen state
    pub duration: f32,
    /// Remaining time in frozen state
    pub remaining: f32,
}

impl FrozenStatus {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            duration: duration_secs,
            remaining: duration_secs,
        }
    }

    /// Check if frozen status has expired
    pub fn is_expired(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Tick the frozen duration
    pub fn tick(&mut self, delta_secs: f32) {
        self.remaining -= delta_secs;
    }
}

impl Default for FrozenStatus {
    fn default() -> Self {
        Self::new(PERMAFROST_FROZEN_DURATION)
    }
}

/// System that applies freeze buildup when frost damage is dealt.
/// Only active when player has PermafrostEnabled marker.
#[allow(clippy::type_complexity)]
pub fn apply_freeze_buildup_system(
    mut commands: Commands,
    player_query: Query<&PermafrostEnabled, With<Player>>,
    mut damage_events: MessageReader<DamageEvent>,
    mut freeze_query: Query<&mut FreezeBuildup, With<Enemy>>,
    enemy_query: Query<Entity, (With<Enemy>, Without<FreezeBuildup>, Without<FrozenStatus>)>,
) {
    // Check if permafrost is enabled
    if player_query.is_empty() {
        return;
    }

    for event in damage_events.read() {
        // Only process frost damage
        if event.element != Some(Element::Frost) {
            continue;
        }

        // Check if target is an enemy
        if let Ok(mut freeze) = freeze_query.get_mut(event.target) {
            // Enemy already has freeze buildup - add a stack
            freeze.add_stacks(1);
        } else if enemy_query.get(event.target).is_ok() {
            // Enemy doesn't have freeze buildup yet - add component with 1 stack
            let mut freeze = FreezeBuildup::default();
            freeze.add_stacks(1);
            commands.entity(event.target).try_insert(freeze);
        }
    }
}

/// System that decays freeze stacks over time when not refreshed.
pub fn decay_freeze_stacks_system(
    mut commands: Commands,
    time: Res<Time>,
    mut freeze_query: Query<(Entity, &mut FreezeBuildup), Without<FrozenStatus>>,
) {
    for (entity, mut freeze) in freeze_query.iter_mut() {
        freeze.tick(time.delta());

        if freeze.should_decay() && !freeze.decay_stack() {
            // No stacks left - remove the component
            commands.entity(entity).remove::<FreezeBuildup>();
        }
    }
}

/// System that checks for max freeze stacks and applies FrozenStatus.
pub fn check_freeze_threshold_system(
    mut commands: Commands,
    freeze_query: Query<(Entity, &FreezeBuildup), Without<FrozenStatus>>,
) {
    for (entity, freeze) in freeze_query.iter() {
        if freeze.is_max() {
            // Apply frozen status and remove freeze buildup
            commands.entity(entity).insert(FrozenStatus::default());
            commands.entity(entity).remove::<FreezeBuildup>();
        }
    }
}

/// System that ticks frozen status duration and removes when expired.
pub fn update_frozen_status_system(
    mut commands: Commands,
    time: Res<Time>,
    mut frozen_query: Query<(Entity, &mut FrozenStatus)>,
) {
    for (entity, mut frozen) in frozen_query.iter_mut() {
        frozen.tick(time.delta_secs());

        if frozen.is_expired() {
            commands.entity(entity).remove::<FrozenStatus>();
        }
    }
}

/// Activates Permafrost passive on the player.
pub fn activate_permafrost(commands: &mut Commands, player_entity: Entity) {
    commands.entity(player_entity).insert(PermafrostEnabled);
}

/// Deactivates Permafrost passive on the player.
pub fn deactivate_permafrost(commands: &mut Commands, player_entity: Entity) {
    commands.entity(player_entity).remove::<PermafrostEnabled>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    /// Create a test player with default values
    fn test_player() -> Player {
        Player {
            speed: 200.0,
            regen_rate: 1.0,
            pickup_radius: 50.0,
            last_movement_direction: Vec3::ZERO,
        }
    }

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<DamageEvent>();
        app
    }

    mod freeze_buildup_component_tests {
        use super::*;

        #[test]
        fn test_freeze_buildup_new() {
            let freeze = FreezeBuildup::new(10, 5.0);

            assert_eq!(freeze.stacks, 0);
            assert_eq!(freeze.max_stacks, 10);
            assert!(!freeze.is_max());
        }

        #[test]
        fn test_freeze_buildup_default() {
            let freeze = FreezeBuildup::default();

            assert_eq!(freeze.stacks, 0);
            assert_eq!(freeze.max_stacks, PERMAFROST_MAX_STACKS);
        }

        #[test]
        fn test_freeze_buildup_add_stacks() {
            let mut freeze = FreezeBuildup::new(5, 3.0);

            freeze.add_stacks(2);
            assert_eq!(freeze.stacks, 2);

            freeze.add_stacks(1);
            assert_eq!(freeze.stacks, 3);
        }

        #[test]
        fn test_freeze_buildup_add_stacks_capped_at_max() {
            let mut freeze = FreezeBuildup::new(5, 3.0);

            freeze.add_stacks(10); // Try to add more than max
            assert_eq!(freeze.stacks, 5); // Should be capped at max
        }

        #[test]
        fn test_freeze_buildup_is_max() {
            let mut freeze = FreezeBuildup::new(3, 3.0);

            assert!(!freeze.is_max());

            freeze.add_stacks(2);
            assert!(!freeze.is_max());

            freeze.add_stacks(1);
            assert!(freeze.is_max());
        }

        #[test]
        fn test_freeze_buildup_decay_stack() {
            let mut freeze = FreezeBuildup::new(5, 3.0);
            freeze.stacks = 3;

            assert!(freeze.decay_stack()); // Returns true - stacks remain
            assert_eq!(freeze.stacks, 2);

            assert!(freeze.decay_stack());
            assert_eq!(freeze.stacks, 1);

            assert!(!freeze.decay_stack()); // Returns false - no stacks remain
            assert_eq!(freeze.stacks, 0);
        }

        #[test]
        fn test_freeze_buildup_decay_stack_at_zero() {
            let mut freeze = FreezeBuildup::new(5, 3.0);
            freeze.stacks = 0;

            assert!(!freeze.decay_stack());
            assert_eq!(freeze.stacks, 0); // Doesn't go negative
        }

        #[test]
        fn test_freeze_buildup_tick_and_should_decay() {
            let mut freeze = FreezeBuildup::new(5, 1.0); // 1 second decay time
            freeze.stacks = 2;

            // Not yet decayed
            freeze.tick(Duration::from_secs_f32(0.5));
            assert!(!freeze.should_decay());

            // Decay triggers after full timer
            freeze.tick(Duration::from_secs_f32(0.6));
            assert!(freeze.should_decay());
        }

        #[test]
        fn test_freeze_buildup_add_stacks_resets_timer() {
            let mut freeze = FreezeBuildup::new(5, 1.0);

            freeze.tick(Duration::from_secs_f32(0.9));
            freeze.add_stacks(1); // Should reset timer

            // After reset, we need full duration again
            freeze.tick(Duration::from_secs_f32(0.5));
            assert!(!freeze.should_decay());
        }

        #[test]
        fn test_freeze_buildup_reset() {
            let mut freeze = FreezeBuildup::new(5, 3.0);
            freeze.stacks = 4;

            freeze.reset();

            assert_eq!(freeze.stacks, 0);
        }

        #[test]
        fn test_permafrost_uses_frost_element_color() {
            let color = permafrost_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod frozen_status_component_tests {
        use super::*;

        #[test]
        fn test_frozen_status_new() {
            let frozen = FrozenStatus::new(3.0);

            assert_eq!(frozen.duration, 3.0);
            assert_eq!(frozen.remaining, 3.0);
            assert!(!frozen.is_expired());
        }

        #[test]
        fn test_frozen_status_default() {
            let frozen = FrozenStatus::default();

            assert_eq!(frozen.duration, PERMAFROST_FROZEN_DURATION);
            assert_eq!(frozen.remaining, PERMAFROST_FROZEN_DURATION);
        }

        #[test]
        fn test_frozen_status_tick() {
            let mut frozen = FrozenStatus::new(2.0);

            frozen.tick(0.5);
            assert_eq!(frozen.remaining, 1.5);
            assert!(!frozen.is_expired());

            frozen.tick(1.0);
            assert_eq!(frozen.remaining, 0.5);
            assert!(!frozen.is_expired());
        }

        #[test]
        fn test_frozen_status_expires() {
            let mut frozen = FrozenStatus::new(1.0);

            frozen.tick(0.9);
            assert!(!frozen.is_expired());

            frozen.tick(0.2);
            assert!(frozen.is_expired());
        }

        #[test]
        fn test_frozen_status_remaining_can_go_negative() {
            let mut frozen = FrozenStatus::new(1.0);

            frozen.tick(2.0);
            assert!(frozen.remaining < 0.0);
            assert!(frozen.is_expired());
        }
    }

    mod apply_freeze_buildup_system_tests {
        use super::*;

        #[test]
        fn test_frost_damage_adds_freeze_buildup() {
            let mut app = setup_test_app();

            // Spawn player with permafrost enabled
            app.world_mut().spawn((
                test_player(),
                PermafrostEnabled,
            ));

            // Spawn enemy without freeze buildup
            let enemy = app.world_mut().spawn(
                Enemy { speed: 50.0, strength: 10.0 }
            ).id();

            // Send frost damage event
            {
                let mut writer = app.world_mut().resource_mut::<Messages<DamageEvent>>();
                writer.write(DamageEvent::with_element(enemy, 10.0, Element::Frost));
            }

            let _ = app.world_mut().run_system_once(apply_freeze_buildup_system);

            // Enemy should have FreezeBuildup with 1 stack
            let freeze = app.world().get::<FreezeBuildup>(enemy);
            assert!(freeze.is_some(), "Enemy should have FreezeBuildup");
            assert_eq!(freeze.unwrap().stacks, 1);
        }

        #[test]
        fn test_frost_damage_increments_existing_stacks() {
            let mut app = setup_test_app();

            // Spawn player with permafrost enabled
            app.world_mut().spawn((
                test_player(),
                PermafrostEnabled,
            ));

            // Spawn enemy with existing freeze buildup (2 stacks)
            let mut freeze = FreezeBuildup::default();
            freeze.stacks = 2;
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                freeze,
            )).id();

            // Send frost damage event
            {
                let mut writer = app.world_mut().resource_mut::<Messages<DamageEvent>>();
                writer.write(DamageEvent::with_element(enemy, 10.0, Element::Frost));
            }

            let _ = app.world_mut().run_system_once(apply_freeze_buildup_system);

            // Enemy should have 3 stacks now
            let freeze = app.world().get::<FreezeBuildup>(enemy).unwrap();
            assert_eq!(freeze.stacks, 3);
        }

        #[test]
        fn test_non_frost_damage_does_not_add_stacks() {
            let mut app = setup_test_app();

            // Spawn player with permafrost enabled
            app.world_mut().spawn((
                test_player(),
                PermafrostEnabled,
            ));

            // Spawn enemy
            let enemy = app.world_mut().spawn(
                Enemy { speed: 50.0, strength: 10.0 }
            ).id();

            // Send fire damage event
            {
                let mut writer = app.world_mut().resource_mut::<Messages<DamageEvent>>();
                writer.write(DamageEvent::with_element(enemy, 10.0, Element::Fire));
            }

            let _ = app.world_mut().run_system_once(apply_freeze_buildup_system);

            // Enemy should NOT have FreezeBuildup
            assert!(app.world().get::<FreezeBuildup>(enemy).is_none());
        }

        #[test]
        fn test_no_freeze_without_permafrost_enabled() {
            let mut app = setup_test_app();

            // Spawn player WITHOUT permafrost enabled
            app.world_mut().spawn(test_player());

            // Spawn enemy
            let enemy = app.world_mut().spawn(
                Enemy { speed: 50.0, strength: 10.0 }
            ).id();

            // Send frost damage event
            {
                let mut writer = app.world_mut().resource_mut::<Messages<DamageEvent>>();
                writer.write(DamageEvent::with_element(enemy, 10.0, Element::Frost));
            }

            let _ = app.world_mut().run_system_once(apply_freeze_buildup_system);

            // Enemy should NOT have FreezeBuildup
            assert!(
                app.world().get::<FreezeBuildup>(enemy).is_none(),
                "Freeze buildup should not be added without PermafrostEnabled"
            );
        }

        #[test]
        fn test_frozen_enemy_does_not_get_buildup() {
            let mut app = setup_test_app();

            // Spawn player with permafrost enabled
            app.world_mut().spawn((
                test_player(),
                PermafrostEnabled,
            ));

            // Spawn enemy that is already frozen
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FrozenStatus::default(),
            )).id();

            // Send frost damage event
            {
                let mut writer = app.world_mut().resource_mut::<Messages<DamageEvent>>();
                writer.write(DamageEvent::with_element(enemy, 10.0, Element::Frost));
            }

            let _ = app.world_mut().run_system_once(apply_freeze_buildup_system);

            // Frozen enemy should NOT get FreezeBuildup
            assert!(
                app.world().get::<FreezeBuildup>(enemy).is_none(),
                "Frozen enemy should not receive new freeze buildup"
            );
        }
    }

    mod decay_freeze_stacks_system_tests {
        use super::*;

        #[test]
        fn test_stacks_decay_after_timer() {
            let mut app = setup_test_app();

            // Spawn enemy with freeze buildup (2 stacks, 1 sec decay)
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FreezeBuildup::new(5, 1.0),
            )).id();

            // Set stacks manually
            {
                let mut freeze = app.world_mut().get_mut::<FreezeBuildup>(enemy).unwrap();
                freeze.stacks = 2;
            }

            // Advance time past decay timer
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.1));
            }

            let _ = app.world_mut().run_system_once(decay_freeze_stacks_system);

            // Should have 1 stack now
            let freeze = app.world().get::<FreezeBuildup>(enemy).unwrap();
            assert_eq!(freeze.stacks, 1);
        }

        #[test]
        fn test_component_removed_when_stacks_depleted() {
            let mut app = setup_test_app();

            // Spawn enemy with freeze buildup (1 stack, 1 sec decay)
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FreezeBuildup::new(5, 1.0),
            )).id();

            // Set to 1 stack
            {
                let mut freeze = app.world_mut().get_mut::<FreezeBuildup>(enemy).unwrap();
                freeze.stacks = 1;
            }

            // Advance time past decay timer
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.1));
            }

            let _ = app.world_mut().run_system_once(decay_freeze_stacks_system);

            // FreezeBuildup component should be removed
            assert!(
                app.world().get::<FreezeBuildup>(enemy).is_none(),
                "FreezeBuildup should be removed when stacks reach 0"
            );
        }

        #[test]
        fn test_stacks_dont_decay_before_timer() {
            let mut app = setup_test_app();

            // Spawn enemy with freeze buildup
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FreezeBuildup::new(5, 2.0), // 2 second decay
            )).id();

            {
                let mut freeze = app.world_mut().get_mut::<FreezeBuildup>(enemy).unwrap();
                freeze.stacks = 3;
            }

            // Advance time but not past decay timer
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(decay_freeze_stacks_system);

            // Should still have 3 stacks
            let freeze = app.world().get::<FreezeBuildup>(enemy).unwrap();
            assert_eq!(freeze.stacks, 3);
        }

        #[test]
        fn test_frozen_enemies_dont_decay() {
            let mut app = setup_test_app();

            // Spawn enemy with both FreezeBuildup AND FrozenStatus
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FreezeBuildup::new(5, 1.0),
                FrozenStatus::default(),
            )).id();

            {
                let mut freeze = app.world_mut().get_mut::<FreezeBuildup>(enemy).unwrap();
                freeze.stacks = 2;
            }

            // Advance time past decay timer
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.1));
            }

            let _ = app.world_mut().run_system_once(decay_freeze_stacks_system);

            // Stacks should not have decayed (frozen enemies are excluded)
            let freeze = app.world().get::<FreezeBuildup>(enemy).unwrap();
            assert_eq!(freeze.stacks, 2);
        }
    }

    mod check_freeze_threshold_system_tests {
        use super::*;

        #[test]
        fn test_max_stacks_applies_frozen_status() {
            let mut app = setup_test_app();

            // Spawn enemy with max freeze stacks
            let mut freeze = FreezeBuildup::new(3, 5.0);
            freeze.stacks = 3; // At max
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                freeze,
            )).id();

            let _ = app.world_mut().run_system_once(check_freeze_threshold_system);

            // Enemy should have FrozenStatus
            assert!(
                app.world().get::<FrozenStatus>(enemy).is_some(),
                "Enemy at max stacks should have FrozenStatus"
            );
        }

        #[test]
        fn test_max_stacks_removes_freeze_buildup() {
            let mut app = setup_test_app();

            // Spawn enemy with max freeze stacks
            let mut freeze = FreezeBuildup::new(3, 5.0);
            freeze.stacks = 3;
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                freeze,
            )).id();

            let _ = app.world_mut().run_system_once(check_freeze_threshold_system);

            // FreezeBuildup should be removed
            assert!(
                app.world().get::<FreezeBuildup>(enemy).is_none(),
                "FreezeBuildup should be removed when frozen"
            );
        }

        #[test]
        fn test_below_max_stacks_no_frozen_status() {
            let mut app = setup_test_app();

            // Spawn enemy below max stacks
            let mut freeze = FreezeBuildup::new(5, 5.0);
            freeze.stacks = 3; // Below max
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                freeze,
            )).id();

            let _ = app.world_mut().run_system_once(check_freeze_threshold_system);

            // Enemy should NOT have FrozenStatus
            assert!(app.world().get::<FrozenStatus>(enemy).is_none());
            // FreezeBuildup should still exist
            assert!(app.world().get::<FreezeBuildup>(enemy).is_some());
        }

        #[test]
        fn test_already_frozen_not_checked() {
            let mut app = setup_test_app();

            // Spawn enemy with both FreezeBuildup and FrozenStatus
            let mut freeze = FreezeBuildup::new(3, 5.0);
            freeze.stacks = 3;
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                freeze,
                FrozenStatus::default(),
            )).id();

            let _ = app.world_mut().run_system_once(check_freeze_threshold_system);

            // Both components should still exist (already frozen enemies are excluded)
            assert!(app.world().get::<FreezeBuildup>(enemy).is_some());
            assert!(app.world().get::<FrozenStatus>(enemy).is_some());
        }
    }

    mod update_frozen_status_system_tests {
        use super::*;

        #[test]
        fn test_frozen_status_ticks_down() {
            let mut app = setup_test_app();

            // Spawn frozen enemy
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FrozenStatus::new(5.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.0));
            }

            let _ = app.world_mut().run_system_once(update_frozen_status_system);

            // Remaining should be reduced
            let frozen = app.world().get::<FrozenStatus>(enemy).unwrap();
            assert!((frozen.remaining - 3.0).abs() < 0.01);
        }

        #[test]
        fn test_frozen_status_removed_when_expired() {
            let mut app = setup_test_app();

            // Spawn frozen enemy with short duration
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FrozenStatus::new(1.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.1));
            }

            let _ = app.world_mut().run_system_once(update_frozen_status_system);

            // FrozenStatus should be removed
            assert!(
                app.world().get::<FrozenStatus>(enemy).is_none(),
                "FrozenStatus should be removed when expired"
            );
        }

        #[test]
        fn test_frozen_status_persists_before_expiry() {
            let mut app = setup_test_app();

            // Spawn frozen enemy
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                FrozenStatus::new(5.0),
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(3.0));
            }

            let _ = app.world_mut().run_system_once(update_frozen_status_system);

            // FrozenStatus should still exist
            assert!(app.world().get::<FrozenStatus>(enemy).is_some());
        }
    }

    mod activate_permafrost_tests {
        use super::*;

        #[test]
        fn test_activate_permafrost_adds_marker() {
            let mut app = setup_test_app();

            let player = app.world_mut().spawn(test_player()).id();

            {
                let mut commands = app.world_mut().commands();
                activate_permafrost(&mut commands, player);
            }
            app.update();

            assert!(
                app.world().get::<PermafrostEnabled>(player).is_some(),
                "Player should have PermafrostEnabled after activation"
            );
        }

        #[test]
        fn test_deactivate_permafrost_removes_marker() {
            let mut app = setup_test_app();

            let player = app.world_mut().spawn((
                test_player(),
                PermafrostEnabled,
            )).id();

            {
                let mut commands = app.world_mut().commands();
                deactivate_permafrost(&mut commands, player);
            }
            app.update();

            assert!(
                app.world().get::<PermafrostEnabled>(player).is_none(),
                "Player should not have PermafrostEnabled after deactivation"
            );
        }
    }
}
