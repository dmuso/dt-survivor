//! Wraith Form spell - Player becomes intangible and passes through enemies while damaging them.
//!
//! When activated, the player can pass through enemies without taking damage.
//! Enemies the player passes through take damage. Each enemy is only damaged
//! once per activation. This implements the Nightmare SpellType from the Dark element.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Wraith Form spell
pub const WRAITH_FORM_DURATION: f32 = 3.0;
pub const WRAITH_FORM_COLLISION_RADIUS: f32 = 1.5; // Same as normal player-enemy collision

/// Get the dark element color for visual effects (purple)
pub fn wraith_form_color() -> Color {
    Element::Dark.color()
}

/// Global counter for wraith form activation IDs to ensure uniqueness
static WRAITH_ACTIVATION_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// Generate a unique activation ID for a new wraith form activation
fn next_activation_id() -> u64 {
    WRAITH_ACTIVATION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

/// WraithForm component - makes the player intangible and damages enemies on pass-through.
/// Added to the player entity when the spell activates.
#[derive(Component, Debug, Clone)]
pub struct WraithForm {
    /// Remaining duration of the wraith form
    pub duration: Timer,
    /// Damage dealt to enemies when player passes through them
    pub damage_on_pass: f32,
    /// Unique ID for this activation (used to track which enemies were damaged)
    pub activation_id: u64,
}

impl WraithForm {
    /// Create a new wraith form with the given duration and damage.
    pub fn new(duration_secs: f32, damage_on_pass: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            damage_on_pass,
            activation_id: next_activation_id(),
        }
    }

    /// Create wraith form with default configuration and spell damage.
    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(WRAITH_FORM_DURATION, spell.damage())
    }

    /// Check if the wraith form has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }
}

/// Marker component added to enemies that have been damaged by a specific wraith form activation.
/// Prevents the same enemy from being damaged multiple times by the same activation.
#[derive(Component, Debug, Clone)]
pub struct WraithFormDamagedBy {
    /// The activation ID of the wraith form that damaged this enemy
    pub activation_id: u64,
}

/// System that updates WraithForm duration timers.
pub fn wraith_form_duration_system(
    time: Res<Time>,
    mut player_query: Query<&mut WraithForm, With<Player>>,
) {
    for mut wraith_form in player_query.iter_mut() {
        wraith_form.tick(time.delta());
    }
}

/// System that removes expired WraithForm components from players.
pub fn wraith_form_expiration_system(
    mut commands: Commands,
    player_query: Query<(Entity, &WraithForm), With<Player>>,
) {
    for (player_entity, wraith_form) in player_query.iter() {
        if wraith_form.is_expired() {
            commands.entity(player_entity).remove::<WraithForm>();
        }
    }
}

/// System that detects when the wraith-form player passes through enemies
/// and applies damage to them (once per enemy per activation).
#[allow(clippy::type_complexity)]
pub fn wraith_form_damage_system(
    mut commands: Commands,
    player_query: Query<(&Transform, &WraithForm), With<Player>>,
    enemy_query: Query<
        (Entity, &Transform, Option<&WraithFormDamagedBy>),
        With<Enemy>,
    >,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok((player_transform, wraith_form)) = player_query.single() else {
        return;
    };

    let player_pos = from_xz(player_transform.translation);

    for (enemy_entity, enemy_transform, damaged_marker) in enemy_query.iter() {
        // Skip enemies already damaged by this activation
        if let Some(marker) = damaged_marker {
            if marker.activation_id == wraith_form.activation_id {
                continue;
            }
        }

        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = player_pos.distance(enemy_pos);

        // Check if player is passing through this enemy
        if distance < WRAITH_FORM_COLLISION_RADIUS {
            // Deal damage to the enemy
            damage_events.write(DamageEvent::with_element(
                enemy_entity,
                wraith_form.damage_on_pass,
                Element::Dark,
            ));

            // Mark enemy as damaged by this activation
            commands.entity(enemy_entity).insert(WraithFormDamagedBy {
                activation_id: wraith_form.activation_id,
            });
        }
    }
}

/// System that cleans up WraithFormDamagedBy markers when no wraith form is active.
/// This allows enemies to be damaged again by future wraith form activations.
pub fn wraith_form_cleanup_system(
    mut commands: Commands,
    player_query: Query<&WraithForm, With<Player>>,
    marked_enemies: Query<Entity, With<WraithFormDamagedBy>>,
) {
    // If no player has wraith form active, clean up all markers
    if player_query.is_empty() {
        for enemy_entity in marked_enemies.iter() {
            commands.entity(enemy_entity).remove::<WraithFormDamagedBy>();
        }
    }
}

/// Cast Wraith Form spell - adds WraithForm component to the player.
/// `player_entity` is the player to apply the effect to.
#[allow(clippy::too_many_arguments)]
pub fn fire_wraith_form(
    commands: &mut Commands,
    spell: &Spell,
    player_entity: Entity,
) {
    fire_wraith_form_with_damage(commands, spell, spell.damage(), player_entity);
}

/// Cast Wraith Form spell with explicit damage.
/// `damage` is the pre-calculated final damage per enemy passed (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_wraith_form_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    player_entity: Entity,
) {
    let wraith_form = WraithForm::new(WRAITH_FORM_DURATION, damage);
    commands.entity(player_entity).insert(wraith_form);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;
    use bevy::ecs::system::RunSystemOnce;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    mod wraith_form_component_tests {
        use super::*;

        #[test]
        fn test_wraith_form_new() {
            let wraith_form = WraithForm::new(3.0, 25.0);

            assert_eq!(wraith_form.damage_on_pass, 25.0);
            assert!(!wraith_form.is_expired());
        }

        #[test]
        fn test_wraith_form_from_spell() {
            let spell = Spell::new(SpellType::Nightmare);
            let wraith_form = WraithForm::from_spell(&spell);

            assert_eq!(wraith_form.damage_on_pass, spell.damage());
            assert!(!wraith_form.is_expired());
        }

        #[test]
        fn test_wraith_form_tick_and_expire() {
            let mut wraith_form = WraithForm::new(1.0, 25.0);
            assert!(!wraith_form.is_expired());

            wraith_form.tick(Duration::from_secs_f32(0.5));
            assert!(!wraith_form.is_expired());

            wraith_form.tick(Duration::from_secs_f32(0.6));
            assert!(wraith_form.is_expired());
        }

        #[test]
        fn test_wraith_form_unique_activation_ids() {
            let wf1 = WraithForm::new(3.0, 25.0);
            let wf2 = WraithForm::new(3.0, 25.0);
            let wf3 = WraithForm::new(3.0, 25.0);

            assert_ne!(wf1.activation_id, wf2.activation_id);
            assert_ne!(wf2.activation_id, wf3.activation_id);
            assert_ne!(wf1.activation_id, wf3.activation_id);
        }

        #[test]
        fn test_wraith_form_uses_dark_element_color() {
            let color = wraith_form_color();
            assert_eq!(color, Element::Dark.color());
        }
    }

    mod wraith_form_damaged_by_tests {
        use super::*;

        #[test]
        fn test_wraith_form_damaged_by_stores_activation_id() {
            let marker = WraithFormDamagedBy { activation_id: 42 };
            assert_eq!(marker.activation_id, 42);
        }
    }

    mod wraith_form_duration_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_duration_ticks_down() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
                WraithForm::new(5.0, 25.0),
            )).id();

            // Advance time by 2 seconds
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.0));
            }

            let _ = app.world_mut().run_system_once(wraith_form_duration_system);

            let wraith_form = app.world().get::<WraithForm>(player_entity).unwrap();
            let remaining = wraith_form.duration.remaining_secs();
            assert!(
                remaining < 4.0 && remaining > 2.5,
                "Duration should have ticked down, remaining: {}",
                remaining
            );
        }

        #[test]
        fn test_wraith_form_expires_after_duration() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
                WraithForm::new(1.0, 25.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.5));
            }

            let _ = app.world_mut().run_system_once(wraith_form_duration_system);

            let wraith_form = app.world().get::<WraithForm>(player_entity).unwrap();
            assert!(wraith_form.is_expired());
        }
    }

    mod wraith_form_expiration_system_tests {
        use super::*;

        #[test]
        fn test_expired_wraith_form_removed() {
            let mut app = App::new();

            let mut wraith_form = WraithForm::new(0.5, 25.0);
            wraith_form.duration.tick(Duration::from_secs_f32(1.0)); // Force expired

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
                wraith_form,
            )).id();

            let _ = app.world_mut().run_system_once(wraith_form_expiration_system);

            assert!(
                app.world().get::<WraithForm>(player_entity).is_none(),
                "Expired wraith form should be removed"
            );
        }

        #[test]
        fn test_active_wraith_form_survives() {
            let mut app = App::new();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
                WraithForm::new(10.0, 25.0),
            )).id();

            let _ = app.world_mut().run_system_once(wraith_form_expiration_system);

            assert!(
                app.world().get::<WraithForm>(player_entity).is_some(),
                "Active wraith form should survive"
            );
        }
    }

    mod wraith_form_damage_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_enemies_in_range_take_damage() {
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

            // Create player with wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                WraithForm::new(5.0, 25.0),
            ));

            // Create enemy within collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy in range should take damage");
        }

        #[test]
        fn test_enemies_outside_range_no_damage() {
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

            // Create player with wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                WraithForm::new(5.0, 25.0),
            ));

            // Create enemy outside collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside range should not take damage");
        }

        #[test]
        fn test_enemy_only_damaged_once_per_activation() {
            let mut app = setup_test_app();

            // Create wraith form and capture its activation ID
            let wraith_form = WraithForm::new(5.0, 25.0);
            let activation_id = wraith_form.activation_id;

            // Create player with wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                wraith_form,
            ));

            // Create enemy within collision radius, PRE-MARKED as already damaged by this activation
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
                WraithFormDamagedBy { activation_id },
            ));

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

            // Run the damage system - enemy already has marker with matching activation_id
            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(
                counter.0.load(Ordering::SeqCst), 0,
                "Enemy with matching activation_id marker should not be damaged again"
            );
        }

        #[test]
        fn test_multiple_enemies_damaged() {
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

            // Create player with wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                WraithForm::new(5.0, 25.0),
            ));

            // Create 3 enemies within collision radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(0.5 + i as f32 * 0.1, 0.375, 0.0)),
                ));
            }

            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 3, "All 3 enemies should take damage");
        }

        #[test]
        fn test_enemy_with_different_activation_id_still_damaged() {
            let mut app = setup_test_app();

            // Create NEW wraith form with fresh activation ID
            let wraith_form = WraithForm::new(5.0, 25.0);
            let new_activation_id = wraith_form.activation_id;

            // Create player with wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                wraith_form,
            ));

            // Create enemy marked with a DIFFERENT activation_id (from previous wraith form)
            let old_activation_id = new_activation_id.wrapping_sub(10); // Guaranteed different
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
                WraithFormDamagedBy { activation_id: old_activation_id },
            ));

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

            // Run the damage system - enemy has old marker, should be damaged by new activation
            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(
                counter.0.load(Ordering::SeqCst), 1,
                "Enemy with different activation_id should be damaged by new wraith form"
            );
        }

        #[test]
        fn test_damage_uses_correct_amount() {
            let mut app = setup_test_app();

            #[derive(Resource)]
            struct LastDamageAmount(f32);

            fn capture_damage(
                mut events: MessageReader<DamageEvent>,
                mut damage: ResMut<LastDamageAmount>,
            ) {
                for event in events.read() {
                    damage.0 = event.amount;
                }
            }

            app.insert_resource(LastDamageAmount(0.0));

            // Create player with wraith form with specific damage
            let expected_damage = 42.0;
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                WraithForm::new(5.0, expected_damage),
            ));

            // Create enemy within collision radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(capture_damage);

            let damage = app.world().get_resource::<LastDamageAmount>().unwrap();
            assert_eq!(damage.0, expected_damage, "Damage should match wraith form damage_on_pass");
        }

        #[test]
        fn test_uses_xz_plane_ignores_y() {
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

            // Create player with wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                WraithForm::new(5.0, 25.0),
            ));

            // Create enemy close on XZ but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 100.0, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(wraith_form_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(
                counter.0.load(Ordering::SeqCst), 1,
                "Y distance should be ignored for collision"
            );
        }

        #[test]
        fn test_enemy_marked_with_activation_id() {
            let mut app = setup_test_app();

            // Create player with wraith form at origin
            let wraith_form = WraithForm::new(5.0, 25.0);
            let activation_id = wraith_form.activation_id;

            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                wraith_form,
            ));

            // Create enemy within collision radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(wraith_form_damage_system);

            let marker = app.world().get::<WraithFormDamagedBy>(enemy_entity).unwrap();
            assert_eq!(marker.activation_id, activation_id, "Enemy should be marked with correct activation ID");
        }
    }

    mod wraith_form_cleanup_system_tests {
        use super::*;

        #[test]
        fn test_markers_removed_when_no_wraith_form_active() {
            let mut app = App::new();

            // Create player WITHOUT wraith form
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
            ));

            // Create enemies with damage markers
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
                WraithFormDamagedBy { activation_id: 1 },
            )).id();

            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
                WraithFormDamagedBy { activation_id: 1 },
            )).id();

            let _ = app.world_mut().run_system_once(wraith_form_cleanup_system);

            assert!(
                app.world().get::<WraithFormDamagedBy>(enemy1).is_none(),
                "Marker should be removed when no wraith form active"
            );
            assert!(
                app.world().get::<WraithFormDamagedBy>(enemy2).is_none(),
                "Marker should be removed when no wraith form active"
            );
        }

        #[test]
        fn test_markers_kept_when_wraith_form_active() {
            let mut app = App::new();

            // Create player WITH wraith form
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
                WraithForm::new(5.0, 25.0),
            ));

            // Create enemy with damage marker
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
                WraithFormDamagedBy { activation_id: 1 },
            )).id();

            let _ = app.world_mut().run_system_once(wraith_form_cleanup_system);

            assert!(
                app.world().get::<WraithFormDamagedBy>(enemy_entity).is_some(),
                "Marker should be kept when wraith form is active"
            );
        }
    }

    mod fire_wraith_form_tests {
        use super::*;

        #[test]
        fn test_fire_wraith_form_adds_component() {
            let mut app = App::new();

            let spell = Spell::new(SpellType::Nightmare);

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_wraith_form(&mut commands, &spell, player_entity);
            }
            app.update();

            assert!(
                app.world().get::<WraithForm>(player_entity).is_some(),
                "Wraith form should be added to player"
            );
        }

        #[test]
        fn test_fire_wraith_form_with_correct_damage() {
            let mut app = App::new();

            let spell = Spell::new(SpellType::Nightmare);
            let expected_damage = spell.damage();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_wraith_form(&mut commands, &spell, player_entity);
            }
            app.update();

            let wraith_form = app.world().get::<WraithForm>(player_entity).unwrap();
            assert_eq!(wraith_form.damage_on_pass, expected_damage);
        }

        #[test]
        fn test_fire_wraith_form_with_damage_uses_explicit_damage() {
            let mut app = App::new();

            let spell = Spell::new(SpellType::Nightmare);
            let explicit_damage = 100.0;

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_wraith_form_with_damage(&mut commands, &spell, explicit_damage, player_entity);
            }
            app.update();

            let wraith_form = app.world().get::<WraithForm>(player_entity).unwrap();
            assert_eq!(wraith_form.damage_on_pass, explicit_damage);
        }

        #[test]
        fn test_fire_wraith_form_has_correct_duration() {
            let mut app = App::new();

            let spell = Spell::new(SpellType::Nightmare);

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::default(),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                fire_wraith_form(&mut commands, &spell, player_entity);
            }
            app.update();

            let wraith_form = app.world().get::<WraithForm>(player_entity).unwrap();
            assert_eq!(
                wraith_form.duration.duration().as_secs_f32(),
                WRAITH_FORM_DURATION
            );
        }
    }

    mod collision_immunity_tests {
        use super::*;
        use crate::game::events::PlayerEnemyCollisionEvent;
        use crate::game::systems::player_enemy_collision_detection;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<PlayerEnemyCollisionEvent>();
            app
        }

        #[test]
        fn test_player_with_wraith_form_no_collision_events() {
            let mut app = setup_test_app();

            #[derive(Resource)]
            struct CollisionCount(usize);

            fn count_collision_events(
                mut events: MessageReader<PlayerEnemyCollisionEvent>,
                mut count: ResMut<CollisionCount>,
            ) {
                count.0 = events.read().count();
            }

            app.insert_resource(CollisionCount(0));

            // Create player WITH wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                WraithForm::new(5.0, 25.0),
            ));

            // Create enemy within collision radius (distance < 1.5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(player_enemy_collision_detection);
            let _ = app.world_mut().run_system_once(count_collision_events);

            let count = app.world().get_resource::<CollisionCount>().unwrap().0;
            assert_eq!(
                count, 0,
                "Player with WraithForm should not trigger collision events"
            );
        }

        #[test]
        fn test_player_without_wraith_form_has_collision_events() {
            let mut app = setup_test_app();

            #[derive(Resource)]
            struct CollisionCount(usize);

            fn count_collision_events(
                mut events: MessageReader<PlayerEnemyCollisionEvent>,
                mut count: ResMut<CollisionCount>,
            ) {
                count.0 = events.read().count();
            }

            app.insert_resource(CollisionCount(0));

            // Create player WITHOUT wraith form at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            ));

            // Create enemy within collision radius (distance < 1.5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(player_enemy_collision_detection);
            let _ = app.world_mut().run_system_once(count_collision_events);

            let count = app.world().get_resource::<CollisionCount>().unwrap().0;
            assert_eq!(
                count, 1,
                "Player without WraithForm should trigger collision events"
            );
        }
    }
}
