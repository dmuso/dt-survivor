//! Hoarfrost spell - Cold mist aura that slows enemies in range.
//!
//! A Frost element spell that creates an aura centered on the player.
//! Enemies within the aura radius are continuously slowed while they remain inside.
//! Unlike debuff-based slows, this is a zone effect - enemies return to normal speed
//! immediately upon leaving the aura radius.

use bevy::prelude::*;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Hoarfrost spell
pub const HOARFROST_RADIUS: f32 = 5.0;
pub const HOARFROST_DURATION: f32 = 8.0;
pub const HOARFROST_SLOW_MULTIPLIER: f32 = 0.5; // 50% speed reduction
/// Height of the visual effect above the ground
pub const HOARFROST_VISUAL_HEIGHT: f32 = 0.15;

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

/// Marker component for the Hoarfrost aura visual effect.
/// Links the visual entity to the player entity that has the aura.
#[derive(Component, Debug, Clone)]
pub struct HoarfrostVisual {
    /// Entity of the player whose aura this visual represents
    pub player_entity: Entity,
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
/// Also cleans up all InHoarfrost markers from enemies and despawns the visual.
pub fn hoarfrost_cleanup_system(
    mut commands: Commands,
    aura_query: Query<(Entity, &HoarfrostAura)>,
    enemy_query: Query<Entity, With<InHoarfrost>>,
    visual_query: Query<(Entity, &HoarfrostVisual)>,
) {
    for (entity, aura) in aura_query.iter() {
        if aura.is_expired() {
            commands.entity(entity).remove::<HoarfrostAura>();

            // Clean up all InHoarfrost markers
            for enemy_entity in enemy_query.iter() {
                commands.entity(enemy_entity).remove::<InHoarfrost>();
            }

            // Despawn the visual effect
            for (visual_entity, visual) in visual_query.iter() {
                if visual.player_entity == entity {
                    commands.entity(visual_entity).despawn();
                }
            }
        }
    }
}

/// Activates the Hoarfrost aura on the player.
/// Called when the Hoarfrost spell is cast.
pub fn activate_hoarfrost(
    commands: &mut Commands,
    player_entity: Entity,
    player_position: Vec3,
    spell: &Spell,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let aura = HoarfrostAura::from_spell(spell);
    commands.entity(player_entity).insert(aura.clone());

    // Spawn visual effect
    spawn_hoarfrost_visual(commands, player_entity, player_position, aura.radius, game_meshes, game_materials);
}

/// Activates the Hoarfrost aura with explicit parameters.
#[allow(clippy::too_many_arguments)]
pub fn activate_hoarfrost_with_params(
    commands: &mut Commands,
    player_entity: Entity,
    player_position: Vec3,
    radius: f32,
    slow_multiplier: f32,
    duration: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let aura = HoarfrostAura::new(radius, slow_multiplier, duration);
    commands.entity(player_entity).insert(aura);

    // Spawn visual effect
    spawn_hoarfrost_visual(commands, player_entity, player_position, radius, game_meshes, game_materials);
}

/// Spawns the visual effect for the Hoarfrost aura.
fn spawn_hoarfrost_visual(
    commands: &mut Commands,
    player_entity: Entity,
    player_position: Vec3,
    radius: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let visual_pos = Vec3::new(player_position.x, HOARFROST_VISUAL_HEIGHT, player_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.hoarfrost_aura.clone()),
            Transform::from_translation(visual_pos).with_scale(Vec3::splat(radius)),
            HoarfrostVisual { player_entity },
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(visual_pos).with_scale(Vec3::splat(radius)),
            HoarfrostVisual { player_entity },
        ));
    }
}

/// System that updates the Hoarfrost visual position to follow the player
/// and updates scale based on the current aura radius.
#[allow(clippy::type_complexity)]
pub fn hoarfrost_visual_system(
    mut visual_query: Query<(&HoarfrostVisual, &mut Transform)>,
    player_query: Query<(&Transform, &HoarfrostAura), (With<Player>, Without<HoarfrostVisual>)>,
) {
    for (visual, mut visual_transform) in visual_query.iter_mut() {
        if let Ok((player_transform, aura)) = player_query.get(visual.player_entity) {
            // Update position to follow player
            visual_transform.translation.x = player_transform.translation.x;
            visual_transform.translation.z = player_transform.translation.z;
            visual_transform.translation.y = HOARFROST_VISUAL_HEIGHT;

            // Update scale based on aura radius
            visual_transform.scale = Vec3::splat(aura.radius);
        }
    }
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
            let player_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost(&mut commands, player_entity, player_pos, &spell, None, None);
            }
            app.update();

            let aura = app.world().get::<HoarfrostAura>(player_entity);
            assert!(aura.is_some(), "Player should have HoarfrostAura after activation");
        }

        #[test]
        fn test_activate_hoarfrost_with_params() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn(test_player()).id();
            let player_pos = Vec3::new(5.0, 0.5, 15.0);

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost_with_params(
                    &mut commands,
                    player_entity,
                    player_pos,
                    8.0,
                    0.3,
                    15.0,
                    None,
                    None,
                );
            }
            app.update();

            let aura = app.world().get::<HoarfrostAura>(player_entity).unwrap();
            assert_eq!(aura.radius, 8.0);
            assert_eq!(aura.slow_multiplier, 0.3);
        }

        #[test]
        fn test_activate_hoarfrost_spawns_visual() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn(test_player()).id();
            let spell = Spell::new(SpellType::IceBarrier);
            let player_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost(&mut commands, player_entity, player_pos, &spell, None, None);
            }
            app.update();

            // Check that visual was spawned
            let mut visual_query = app.world_mut().query::<&HoarfrostVisual>();
            let count = visual_query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn exactly one HoarfrostVisual");

            // Check visual is linked to the player
            for visual in visual_query.iter(app.world()) {
                assert_eq!(visual.player_entity, player_entity);
            }
        }

        #[test]
        fn test_activate_hoarfrost_visual_has_correct_position() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn(test_player()).id();
            let spell = Spell::new(SpellType::IceBarrier);
            let player_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost(&mut commands, player_entity, player_pos, &spell, None, None);
            }
            app.update();

            // Check visual position
            let mut query = app.world_mut().query::<(&HoarfrostVisual, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation.x, player_pos.x);
                assert_eq!(transform.translation.z, player_pos.z);
                assert_eq!(transform.translation.y, HOARFROST_VISUAL_HEIGHT);
            }
        }

        #[test]
        fn test_activate_hoarfrost_visual_has_correct_scale() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn(test_player()).id();
            let spell = Spell::new(SpellType::IceBarrier);
            let player_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                activate_hoarfrost(&mut commands, player_entity, player_pos, &spell, None, None);
            }
            app.update();

            // Check visual scale matches radius
            let mut query = app.world_mut().query::<(&HoarfrostVisual, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.scale, Vec3::splat(HOARFROST_RADIUS));
            }
        }
    }

    mod hoarfrost_visual_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_visual_follows_player_position() {
            let mut app = setup_test_app();

            // Create player with aura at (10, 0.5, 20)
            let player_entity = app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
                HoarfrostAura::default(),
            )).id();

            // Create visual at different position
            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, HOARFROST_VISUAL_HEIGHT, 0.0)),
                HoarfrostVisual { player_entity },
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_visual_system);

            let transform = app.world().get::<Transform>(visual_entity).unwrap();
            assert_eq!(transform.translation.x, 10.0);
            assert_eq!(transform.translation.z, 20.0);
            assert_eq!(transform.translation.y, HOARFROST_VISUAL_HEIGHT);
        }

        #[test]
        fn test_visual_scale_matches_aura_radius() {
            let mut app = setup_test_app();

            let custom_radius = 8.0;
            let player_entity = app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                HoarfrostAura::new(custom_radius, 0.5, 10.0),
            )).id();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, HOARFROST_VISUAL_HEIGHT, 0.0))
                    .with_scale(Vec3::splat(1.0)), // Different initial scale
                HoarfrostVisual { player_entity },
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_visual_system);

            let transform = app.world().get::<Transform>(visual_entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(custom_radius));
        }

        #[test]
        fn test_visual_does_nothing_if_player_has_no_aura() {
            let mut app = setup_test_app();

            // Create player WITHOUT aura
            let player_entity = app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::new(100.0, 0.5, 200.0)),
            )).id();

            // Visual linked to player without aura
            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, HOARFROST_VISUAL_HEIGHT, 0.0)),
                HoarfrostVisual { player_entity },
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_visual_system);

            // Visual should not have moved (player has no aura, so query fails)
            let transform = app.world().get::<Transform>(visual_entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.z, 0.0);
        }
    }

    mod hoarfrost_cleanup_system_visual_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_cleanup_despawns_visual_when_aura_expires() {
            let mut app = setup_test_app();

            let mut aura = HoarfrostAura::new(5.0, 0.5, 1.0);
            aura.tick(Duration::from_secs_f32(1.1)); // Expire it

            let player_entity = app.world_mut().spawn((
                test_player(),
                aura,
            )).id();

            // Create visual linked to player
            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HoarfrostVisual { player_entity },
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_cleanup_system);

            // Visual should be despawned
            assert!(
                app.world().get_entity(visual_entity).is_err(),
                "Visual should be despawned when aura expires"
            );
        }

        #[test]
        fn test_cleanup_preserves_visual_when_aura_active() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                test_player(),
                HoarfrostAura::new(5.0, 0.5, 10.0), // Active aura
            )).id();

            // Create visual linked to player
            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HoarfrostVisual { player_entity },
            )).id();

            let _ = app.world_mut().run_system_once(hoarfrost_cleanup_system);

            // Visual should still exist
            assert!(
                app.world().get_entity(visual_entity).is_ok(),
                "Visual should be preserved when aura is active"
            );
        }
    }
}
