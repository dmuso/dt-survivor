//! Hoarfrost spell - Cold mist aura that slows enemies in range.
//!
//! A Frost element spell that creates an aura centered on the player.
//! Enemies within the aura radius are continuously slowed while they remain inside.
//! Unlike debuff-based slows, this is a zone effect - enemies return to normal speed
//! immediately upon leaving the aura radius.

use bevy::prelude::*;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Hoarfrost spell
pub const HOARFROST_RADIUS: f32 = 5.0;
pub const HOARFROST_DURATION: f32 = 8.0;
pub const HOARFROST_SLOW_MULTIPLIER: f32 = 0.5; // 50% speed reduction

/// Get the frost element color for visual effects
pub fn hoarfrost_color() -> Color {
    Element::Frost.color()
}

/// Component attached to the player when Hoarfrost aura is active.
/// Defines the aura's properties and tracks its duration.
#[derive(Component, Debug, Clone)]
pub struct HoarfrostAura {
    /// Radius of the slowing aura
    pub radius: f32,
    /// Speed multiplier applied to enemies in range (0.5 = 50% speed)
    pub slow_multiplier: f32,
    /// Duration timer (aura expires when finished)
    pub duration: Timer,
}

impl HoarfrostAura {
    pub fn new(radius: f32, slow_multiplier: f32, duration_secs: f32) -> Self {
        Self {
            radius,
            slow_multiplier,
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }

    pub fn from_spell(_spell: &Spell) -> Self {
        Self::new(
            HOARFROST_RADIUS,
            HOARFROST_SLOW_MULTIPLIER,
            HOARFROST_DURATION,
        )
    }

    /// Check if the aura has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }
}

impl Default for HoarfrostAura {
    fn default() -> Self {
        Self::new(
            HOARFROST_RADIUS,
            HOARFROST_SLOW_MULTIPLIER,
            HOARFROST_DURATION,
        )
    }
}

/// Marker component added to enemies currently inside the Hoarfrost aura.
/// Used to track which enemies should have reduced speed.
#[derive(Component, Debug, Clone, Default)]
pub struct InHoarfrost {
    /// Speed multiplier from the aura (cached for movement system)
    pub slow_multiplier: f32,
}

impl InHoarfrost {
    pub fn new(slow_multiplier: f32) -> Self {
        Self { slow_multiplier }
    }
}

/// System that ticks the Hoarfrost aura duration timer.
pub fn hoarfrost_duration_system(
    mut aura_query: Query<&mut HoarfrostAura>,
    time: Res<Time>,
) {
    for mut aura in aura_query.iter_mut() {
        aura.tick(time.delta());
    }
}

/// System that tracks which enemies are inside the Hoarfrost aura.
/// Adds InHoarfrost marker to enemies within range, removes it from enemies outside.
pub fn hoarfrost_tracking_system(
    mut commands: Commands,
    player_query: Query<(&Transform, &HoarfrostAura), With<Player>>,
    enemy_query: Query<(Entity, &Transform, Option<&InHoarfrost>), With<Enemy>>,
) {
    let Ok((player_transform, aura)) = player_query.single() else {
        // No active aura - remove all InHoarfrost markers
        for (enemy_entity, _, in_hoarfrost) in enemy_query.iter() {
            if in_hoarfrost.is_some() {
                commands.entity(enemy_entity).remove::<InHoarfrost>();
            }
        }
        return;
    };

    let player_pos = from_xz(player_transform.translation);

    for (enemy_entity, enemy_transform, in_hoarfrost) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = player_pos.distance(enemy_pos);

        if distance <= aura.radius {
            // Enemy is in range - add or update InHoarfrost marker
            if in_hoarfrost.is_none() {
                commands.entity(enemy_entity).insert(InHoarfrost::new(aura.slow_multiplier));
            }
        } else {
            // Enemy is out of range - remove InHoarfrost marker if present
            if in_hoarfrost.is_some() {
                commands.entity(enemy_entity).remove::<InHoarfrost>();
            }
        }
    }
}

/// System that removes the Hoarfrost aura when its duration expires.
/// Also cleans up all InHoarfrost markers from enemies.
pub fn hoarfrost_cleanup_system(
    mut commands: Commands,
    aura_query: Query<(Entity, &HoarfrostAura)>,
    enemy_query: Query<Entity, With<InHoarfrost>>,
) {
    for (entity, aura) in aura_query.iter() {
        if aura.is_expired() {
            commands.entity(entity).remove::<HoarfrostAura>();

            // Clean up all InHoarfrost markers
            for enemy_entity in enemy_query.iter() {
                commands.entity(enemy_entity).remove::<InHoarfrost>();
            }
        }
    }
}

/// Activates the Hoarfrost aura on the player.
/// Called when the Hoarfrost spell is cast.
pub fn activate_hoarfrost(
    commands: &mut Commands,
    player_entity: Entity,
    spell: &Spell,
) {
    let aura = HoarfrostAura::from_spell(spell);
    commands.entity(player_entity).insert(aura);
}

/// Activates the Hoarfrost aura with explicit parameters.
pub fn activate_hoarfrost_with_params(
    commands: &mut Commands,
    player_entity: Entity,
    radius: f32,
    slow_multiplier: f32,
    duration: f32,
) {
    let aura = HoarfrostAura::new(radius, slow_multiplier, duration);
    commands.entity(player_entity).insert(aura);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    /// Create a test player with default values
    fn test_player() -> Player {
        Player {
            speed: 200.0,
            regen_rate: 1.0,
            pickup_radius: 50.0,
            last_movement_direction: Vec3::ZERO,
        }
    }

    mod hoarfrost_aura_component_tests {
        use super::*;

        #[test]
        fn test_hoarfrost_aura_new() {
            let aura = HoarfrostAura::new(8.0, 0.4, 10.0);

            assert_eq!(aura.radius, 8.0);
            assert_eq!(aura.slow_multiplier, 0.4);
            assert!(!aura.is_expired());
        }

        #[test]
        fn test_hoarfrost_aura_from_spell() {
            // Hoarfrost uses IceBarrier SpellType as its trigger
            let spell = Spell::new(SpellType::IceBarrier);
            let aura = HoarfrostAura::from_spell(&spell);

            assert_eq!(aura.radius, HOARFROST_RADIUS);
            assert_eq!(aura.slow_multiplier, HOARFROST_SLOW_MULTIPLIER);
            assert!(!aura.is_expired());
        }

        #[test]
        fn test_hoarfrost_aura_default() {
            let aura = HoarfrostAura::default();

            assert_eq!(aura.radius, HOARFROST_RADIUS);
            assert_eq!(aura.slow_multiplier, HOARFROST_SLOW_MULTIPLIER);
        }

        #[test]
        fn test_hoarfrost_aura_tick_expires() {
            let mut aura = HoarfrostAura::new(5.0, 0.5, 2.0);
            assert!(!aura.is_expired());

            aura.tick(Duration::from_secs_f32(2.1));
            assert!(aura.is_expired());
        }

        #[test]
        fn test_hoarfrost_aura_tick_not_expired() {
            let mut aura = HoarfrostAura::new(5.0, 0.5, 5.0);
            aura.tick(Duration::from_secs_f32(2.0));
            assert!(!aura.is_expired());
        }

        #[test]
        fn test_hoarfrost_uses_frost_element_color() {
            let color = hoarfrost_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod in_hoarfrost_component_tests {
        use super::*;

        #[test]
        fn test_in_hoarfrost_new() {
            let marker = InHoarfrost::new(0.6);
            assert_eq!(marker.slow_multiplier, 0.6);
        }

        #[test]
        fn test_in_hoarfrost_default() {
            let marker = InHoarfrost::default();
            assert_eq!(marker.slow_multiplier, 0.0);
        }
    }

    mod hoarfrost_duration_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_hoarfrost_duration_ticks() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                test_player(),
                HoarfrostAura::new(5.0, 0.5, 5.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.0));
            }

            let _ = app.world_mut().run_system_once(hoarfrost_duration_system);

            let aura = app.world().get::<HoarfrostAura>(entity).unwrap();
            assert!(!aura.is_expired());
        }

        #[test]
        fn test_hoarfrost_duration_expires() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn((
                test_player(),
                HoarfrostAura::new(5.0, 0.5, 2.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(2.5));
            }

            let _ = app.world_mut().run_system_once(hoarfrost_duration_system);

            let aura = app.world().get::<HoarfrostAura>(entity).unwrap();
            assert!(aura.is_expired());
        }
    }

    mod hoarfrost_tracking_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_enemy_in_range_gets_marker() {
            let mut app = setup_test_app();

            // Create player with aura at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            ));

            // Create enemy within radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            // Enemy should have InHoarfrost marker
            let in_hoarfrost = app.world().get::<InHoarfrost>(enemy_entity);
            assert!(in_hoarfrost.is_some(), "Enemy in range should have InHoarfrost marker");
            assert_eq!(in_hoarfrost.unwrap().slow_multiplier, 0.5);
        }

        #[test]
        fn test_enemy_out_of_range_no_marker() {
            let mut app = setup_test_app();

            // Create player with aura at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            ));

            // Create enemy outside radius
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            // Enemy should NOT have InHoarfrost marker
            assert!(app.world().get::<InHoarfrost>(enemy_entity).is_none());
        }

        #[test]
        fn test_enemy_marker_removed_when_leaving_range() {
            let mut app = setup_test_app();

            // Create player with aura at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            ));

            // Create enemy with pre-existing InHoarfrost but outside range
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                InHoarfrost::new(0.5),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            // Enemy should have InHoarfrost removed
            assert!(
                app.world().get::<InHoarfrost>(enemy_entity).is_none(),
                "InHoarfrost should be removed when enemy leaves range"
            );
        }

        #[test]
        fn test_multiple_enemies_tracked_correctly() {
            let mut app = setup_test_app();

            // Create player with aura at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            ));

            // Create enemies: 2 in range, 1 out of range
            let enemy_in_1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            let enemy_in_2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 4.0)),
            )).id();

            let enemy_out = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(20.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            assert!(app.world().get::<InHoarfrost>(enemy_in_1).is_some());
            assert!(app.world().get::<InHoarfrost>(enemy_in_2).is_some());
            assert!(app.world().get::<InHoarfrost>(enemy_out).is_none());
        }

        #[test]
        fn test_aura_follows_player_position() {
            let mut app = setup_test_app();

            // Create player with aura at position (10, 0, 10)
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::new(10.0, 0.5, 10.0)),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            ));

            // Create enemy near player position
            let enemy_near = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(12.0, 0.375, 10.0)),
            )).id();

            // Create enemy near origin (far from player)
            let enemy_far = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            assert!(app.world().get::<InHoarfrost>(enemy_near).is_some());
            assert!(app.world().get::<InHoarfrost>(enemy_far).is_none());
        }

        #[test]
        fn test_no_aura_removes_all_markers() {
            let mut app = setup_test_app();

            // Create player WITHOUT aura
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create enemy with pre-existing InHoarfrost
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
                InHoarfrost::new(0.5),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            // Enemy should have InHoarfrost removed since there's no active aura
            assert!(app.world().get::<InHoarfrost>(enemy_entity).is_none());
        }

        #[test]
        fn test_uses_xz_distance_ignores_y() {
            let mut app = setup_test_app();

            // Create player with aura at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            ));

            // Create enemy close on XZ but far on Y - should still be in range
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_tracking_system);

            assert!(
                app.world().get::<InHoarfrost>(enemy_entity).is_some(),
                "Y distance should be ignored for aura range calculation"
            );
        }
    }

    mod hoarfrost_cleanup_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_expired_aura_removed() {
            let mut app = setup_test_app();

            let mut aura = HoarfrostAura::new(5.0, 0.5, 1.0);
            aura.tick(Duration::from_secs_f32(1.1)); // Expire it

            let player_entity = app.world_mut().spawn((
                test_player(),
                aura,
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_cleanup_system);

            assert!(
                app.world().get::<HoarfrostAura>(player_entity).is_none(),
                "Expired aura should be removed"
            );
        }

        #[test]
        fn test_active_aura_not_removed() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                test_player(),
                HoarfrostAura::new(5.0, 0.5, 10.0),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_cleanup_system);

            assert!(
                app.world().get::<HoarfrostAura>(player_entity).is_some(),
                "Active aura should not be removed"
            );
        }

        #[test]
        fn test_all_in_hoarfrost_markers_cleaned_up() {
            let mut app = setup_test_app();

            let mut aura = HoarfrostAura::new(5.0, 0.5, 1.0);
            aura.tick(Duration::from_secs_f32(1.1)); // Expire it

            app.world_mut().spawn((
                test_player(),
                aura,
            ));

            // Create enemies with InHoarfrost markers
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                InHoarfrost::new(0.5),
            )).id();

            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                InHoarfrost::new(0.5),
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_cleanup_system);

            assert!(app.world().get::<InHoarfrost>(enemy1).is_none());
            assert!(app.world().get::<InHoarfrost>(enemy2).is_none());
        }
    }

    mod activate_hoarfrost_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_activate_hoarfrost_adds_aura() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn(test_player()).id();
            // Hoarfrost uses IceBarrier SpellType as its trigger
            let spell = Spell::new(SpellType::IceBarrier);

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost(&mut commands, player_entity, &spell);
            }
            app.update();

            let aura = app.world().get::<HoarfrostAura>(player_entity);
            assert!(aura.is_some(), "Player should have HoarfrostAura after activation");
        }

        #[test]
        fn test_activate_hoarfrost_with_params() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn(test_player()).id();

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost_with_params(&mut commands, player_entity, 8.0, 0.3, 15.0);
            }
            app.update();

            let aura = app.world().get::<HoarfrostAura>(player_entity).unwrap();
            assert_eq!(aura.radius, 8.0);
            assert_eq!(aura.slow_multiplier, 0.3);
        }
    }
}
