//! Mind Cage spell - Traps enemies in a psychic field.
//!
//! A Psychic element spell (MindBlast SpellType) that creates a zone that
//! prevents enemies from leaving. Enemies inside the cage cannot exit until
//! the cage expires, allowing the player to control the battlefield.

use bevy::prelude::*;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default radius of the Mind Cage zone
pub const MIND_CAGE_DEFAULT_RADIUS: f32 = 6.0;

/// Default duration of the Mind Cage in seconds
pub const MIND_CAGE_DEFAULT_DURATION: f32 = 5.0;

/// Height of the visual effect above ground
pub const MIND_CAGE_VISUAL_HEIGHT: f32 = 0.3;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn mind_cage_color() -> Color {
    Element::Psychic.color()
}

/// Component for the Mind Cage zone.
/// Tracks the cage's position, radius, and duration.
#[derive(Component, Debug, Clone)]
pub struct MindCage {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the cage
    pub radius: f32,
    /// Timer tracking remaining duration
    pub duration: Timer,
}

impl MindCage {
    /// Create a new Mind Cage at the given center position with default values.
    pub fn new(center: Vec2) -> Self {
        Self {
            center,
            radius: MIND_CAGE_DEFAULT_RADIUS,
            duration: Timer::from_seconds(MIND_CAGE_DEFAULT_DURATION, TimerMode::Once),
        }
    }

    /// Create a new Mind Cage with custom radius and duration.
    pub fn with_config(center: Vec2, radius: f32, duration: f32) -> Self {
        Self {
            center,
            radius,
            duration: Timer::from_seconds(duration, TimerMode::Once),
        }
    }

    /// Check if the cage has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer.
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Check if a position is inside the cage.
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }

    /// Clamp a position to stay within the cage bounds.
    /// Returns the clamped position if outside, or the original if inside.
    pub fn clamp_position(&self, position: Vec2) -> Vec2 {
        let distance = self.center.distance(position);
        if distance <= self.radius {
            position
        } else {
            // Move position to edge of cage
            let direction = (position - self.center).normalize_or_zero();
            self.center + direction * self.radius
        }
    }
}

/// Marker component for enemies that are currently trapped in a Mind Cage.
/// Stores a reference to the cage entity for cleanup.
#[derive(Component, Debug, Clone)]
pub struct CagedEnemy {
    /// The entity ID of the cage trapping this enemy
    pub cage_entity: Entity,
}

impl CagedEnemy {
    /// Create a new CagedEnemy marker.
    pub fn new(cage_entity: Entity) -> Self {
        Self { cage_entity }
    }
}

/// System that ticks Mind Cage duration timers.
pub fn mind_cage_duration_system(
    time: Res<Time>,
    mut cage_query: Query<&mut MindCage>,
) {
    for mut cage in cage_query.iter_mut() {
        cage.tick(time.delta());
    }
}

/// System that marks enemies as caged when they enter a Mind Cage zone.
pub fn mind_cage_capture_system(
    mut commands: Commands,
    cage_query: Query<(Entity, &MindCage)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<CagedEnemy>)>,
) {
    for (cage_entity, cage) in cage_query.iter() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            if cage.contains(enemy_pos) {
                commands.entity(enemy_entity).insert(CagedEnemy::new(cage_entity));
            }
        }
    }
}

/// System that constrains caged enemies to stay within their cage boundaries.
pub fn mind_cage_constraint_system(
    cage_query: Query<&MindCage>,
    mut enemy_query: Query<(&CagedEnemy, &mut Transform), With<Enemy>>,
) {
    for (caged_enemy, mut enemy_transform) in enemy_query.iter_mut() {
        // Try to get the cage this enemy is trapped in
        if let Ok(cage) = cage_query.get(caged_enemy.cage_entity) {
            let enemy_pos = from_xz(enemy_transform.translation);
            let clamped_pos = cage.clamp_position(enemy_pos);

            if enemy_pos != clamped_pos {
                enemy_transform.translation.x = clamped_pos.x;
                enemy_transform.translation.z = clamped_pos.y;
            }
        }
    }
}

/// System that removes CagedEnemy markers when cages expire or are despawned.
pub fn mind_cage_cleanup_markers_system(
    mut commands: Commands,
    cage_query: Query<Entity, With<MindCage>>,
    caged_query: Query<(Entity, &CagedEnemy)>,
) {
    for (enemy_entity, caged_enemy) in caged_query.iter() {
        // Remove marker if the cage no longer exists
        if cage_query.get(caged_enemy.cage_entity).is_err() {
            commands.entity(enemy_entity).remove::<CagedEnemy>();
        }
    }
}

/// System that despawns Mind Cages when they expire.
pub fn mind_cage_cleanup_system(
    mut commands: Commands,
    cage_query: Query<(Entity, &MindCage)>,
) {
    for (entity, cage) in cage_query.iter() {
        if cage.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates the visual appearance of Mind Cages.
/// Makes the cage flicker/fade as duration nears end.
pub fn mind_cage_visual_system(
    mut cage_query: Query<(&MindCage, &mut Transform)>,
) {
    for (cage, mut transform) in cage_query.iter_mut() {
        // Scale the visual to match the radius
        let base_scale = cage.radius;

        // Add flicker effect when duration is low (last 20%)
        let remaining_fraction = cage.duration.fraction_remaining();
        let flicker = if remaining_fraction < 0.2 {
            // More intense flickering as time runs out
            let intensity = 1.0 - (remaining_fraction / 0.2);
            1.0 - (intensity * 0.3 * ((remaining_fraction * 50.0).sin().abs()))
        } else {
            1.0
        };

        transform.scale = Vec3::splat(base_scale * flicker);
    }
}

/// Cast Mind Cage spell - spawns a cage zone at the target location.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_mind_cage(
    commands: &mut Commands,
    _spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_mind_cage_with_config(
        commands,
        spawn_position,
        MIND_CAGE_DEFAULT_RADIUS,
        MIND_CAGE_DEFAULT_DURATION,
        game_meshes,
        game_materials,
    );
}

/// Cast Mind Cage spell with explicit radius and duration.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_mind_cage_with_config(
    commands: &mut Commands,
    spawn_position: Vec3,
    radius: f32,
    duration: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let cage = MindCage::with_config(center, radius, duration);
    let cage_pos = Vec3::new(spawn_position.x, MIND_CAGE_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.powerup.clone()), // Magenta/pink material for psychic
            Transform::from_translation(cage_pos).with_scale(Vec3::splat(radius)),
            cage,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(cage_pos),
            cage,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    mod mind_cage_component_tests {
        use super::*;

        #[test]
        fn test_mind_cage_new() {
            let center = Vec2::new(10.0, 20.0);
            let cage = MindCage::new(center);

            assert_eq!(cage.center, center);
            assert_eq!(cage.radius, MIND_CAGE_DEFAULT_RADIUS);
            assert!(!cage.is_expired());
        }

        #[test]
        fn test_mind_cage_with_config() {
            let center = Vec2::new(5.0, 15.0);
            let custom_radius = 10.0;
            let custom_duration = 8.0;
            let cage = MindCage::with_config(center, custom_radius, custom_duration);

            assert_eq!(cage.center, center);
            assert_eq!(cage.radius, custom_radius);
            assert!(!cage.is_expired());
        }

        #[test]
        fn test_mind_cage_is_expired() {
            let mut cage = MindCage::with_config(Vec2::ZERO, 5.0, 1.0);
            assert!(!cage.is_expired());

            cage.tick(Duration::from_secs_f32(1.1));
            assert!(cage.is_expired());
        }

        #[test]
        fn test_mind_cage_tick() {
            let mut cage = MindCage::with_config(Vec2::ZERO, 5.0, 2.0);

            cage.tick(Duration::from_secs_f32(1.0));
            assert!(!cage.is_expired());

            cage.tick(Duration::from_secs_f32(1.0));
            assert!(cage.is_expired());
        }

        #[test]
        fn test_mind_cage_contains_inside() {
            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);

            assert!(cage.contains(Vec2::new(0.0, 0.0)), "Center should be inside");
            assert!(cage.contains(Vec2::new(3.0, 0.0)), "Position within radius should be inside");
            assert!(cage.contains(Vec2::new(0.0, 4.0)), "Position within radius should be inside");
            assert!(cage.contains(Vec2::new(5.0, 0.0)), "Position at edge should be inside");
        }

        #[test]
        fn test_mind_cage_contains_outside() {
            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);

            assert!(!cage.contains(Vec2::new(6.0, 0.0)), "Position outside radius should not be inside");
            assert!(!cage.contains(Vec2::new(0.0, 6.0)), "Position outside radius should not be inside");
            assert!(!cage.contains(Vec2::new(4.0, 4.0)), "Diagonal position outside should not be inside");
        }

        #[test]
        fn test_mind_cage_clamp_position_inside() {
            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);
            let inside_pos = Vec2::new(2.0, 2.0);

            let clamped = cage.clamp_position(inside_pos);
            assert_eq!(clamped, inside_pos, "Position inside should not be clamped");
        }

        #[test]
        fn test_mind_cage_clamp_position_outside() {
            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);
            let outside_pos = Vec2::new(10.0, 0.0);

            let clamped = cage.clamp_position(outside_pos);
            assert!((clamped - Vec2::new(5.0, 0.0)).length() < 0.01,
                "Position should be clamped to edge: got {:?}", clamped);
        }

        #[test]
        fn test_mind_cage_clamp_position_at_edge() {
            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);
            let edge_pos = Vec2::new(5.0, 0.0);

            let clamped = cage.clamp_position(edge_pos);
            assert_eq!(clamped, edge_pos, "Position at edge should not change");
        }

        #[test]
        fn test_mind_cage_clamp_position_diagonal() {
            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);
            let outside_pos = Vec2::new(10.0, 10.0);

            let clamped = cage.clamp_position(outside_pos);
            let distance = clamped.length();
            assert!((distance - 5.0).abs() < 0.01,
                "Clamped position should be at radius distance: got {}", distance);
        }

        #[test]
        fn test_mind_cage_uses_psychic_element_color() {
            let color = mind_cage_color();
            assert_eq!(color, Element::Psychic.color());
        }
    }

    mod caged_enemy_tests {
        use super::*;

        #[test]
        fn test_caged_enemy_new() {
            let cage_entity = Entity::from_bits(42);
            let caged = CagedEnemy::new(cage_entity);
            assert_eq!(caged.cage_entity, cage_entity);
        }
    }

    mod mind_cage_duration_system_tests {
        use super::*;

        #[test]
        fn test_mind_cage_duration_ticks_down() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 2.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(mind_cage_duration_system);

            let cage = app.world().get::<MindCage>(entity).unwrap();
            assert!(!cage.is_expired());

            // Advance more time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.5));
            }

            let _ = app.world_mut().run_system_once(mind_cage_duration_system);

            let cage = app.world().get::<MindCage>(entity).unwrap();
            assert!(cage.is_expired());
        }
    }

    mod mind_cage_capture_system_tests {
        use super::*;

        #[test]
        fn test_mind_cage_captures_enemies_inside() {
            let mut app = App::new();

            // Create cage at origin with radius 5.0
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy inside cage (XZ distance = 3)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_capture_system);

            let caged = app.world().get::<CagedEnemy>(enemy_entity);
            assert!(caged.is_some(), "Enemy inside cage should be marked as caged");
            assert_eq!(caged.unwrap().cage_entity, cage_entity);
        }

        #[test]
        fn test_mind_cage_does_not_capture_enemies_outside() {
            let mut app = App::new();

            // Create cage at origin with radius 3.0
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 3.0, 10.0),
            ));

            // Create enemy outside cage (XZ distance = 5)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_capture_system);

            assert!(
                app.world().get::<CagedEnemy>(enemy_entity).is_none(),
                "Enemy outside cage should not be marked as caged"
            );
        }

        #[test]
        fn test_mind_cage_captures_multiple_enemies() {
            let mut app = App::new();

            // Create cage
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create 3 enemies inside cage
            let mut enemies = Vec::new();
            for i in 0..3 {
                let entity = app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                )).id();
                enemies.push(entity);
            }

            let _ = app.world_mut().run_system_once(mind_cage_capture_system);

            for enemy in enemies {
                let caged = app.world().get::<CagedEnemy>(enemy);
                assert!(caged.is_some(), "All enemies in cage should be marked as caged");
                assert_eq!(caged.unwrap().cage_entity, cage_entity);
            }
        }

        #[test]
        fn test_mind_cage_does_not_recapture_already_caged_enemies() {
            let mut app = App::new();

            // Create cage
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy already marked as caged by a different cage
            let other_cage = Entity::from_bits(999);
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
                CagedEnemy::new(other_cage),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_capture_system);

            // Should still reference the original cage, not the new one
            let caged = app.world().get::<CagedEnemy>(enemy_entity).unwrap();
            assert_eq!(caged.cage_entity, other_cage, "Already caged enemy should not be recaptured");
            assert_ne!(caged.cage_entity, cage_entity);
        }

        #[test]
        fn test_mind_cage_uses_xz_plane_ignores_y() {
            let mut app = App::new();

            // Create cage
            let _cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy close on XZ plane but far on Y
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_capture_system);

            assert!(
                app.world().get::<CagedEnemy>(enemy_entity).is_some(),
                "Y distance should be ignored"
            );
        }
    }

    mod mind_cage_constraint_system_tests {
        use super::*;

        #[test]
        fn test_caged_enemy_movement_restricted() {
            let mut app = App::new();

            // Create cage at origin with radius 5.0
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy outside the cage bounds (trying to escape)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                CagedEnemy::new(cage_entity),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_constraint_system);

            let transform = app.world().get::<Transform>(enemy_entity).unwrap();
            // Should be clamped to edge of cage (radius 5.0)
            assert!(
                (transform.translation.x - 5.0).abs() < 0.01,
                "Enemy X should be clamped to 5.0, got {}",
                transform.translation.x
            );
            assert!(
                transform.translation.z.abs() < 0.01,
                "Enemy Z should remain at 0.0"
            );
        }

        #[test]
        fn test_caged_enemy_inside_cage_not_moved() {
            let mut app = App::new();

            // Create cage at origin with radius 5.0
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy inside the cage
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 1.0)),
                CagedEnemy::new(cage_entity),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_constraint_system);

            let transform = app.world().get::<Transform>(enemy_entity).unwrap();
            assert!(
                (transform.translation.x - 2.0).abs() < 0.01,
                "Enemy inside cage should not be moved"
            );
            assert!(
                (transform.translation.z - 1.0).abs() < 0.01,
                "Enemy inside cage should not be moved"
            );
        }

        #[test]
        fn test_caged_enemy_with_missing_cage_not_constrained() {
            let mut app = App::new();

            // Create enemy caged by a non-existent cage
            let fake_cage = Entity::from_bits(9999);
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
                CagedEnemy::new(fake_cage),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_constraint_system);

            // Should not panic and position should be unchanged
            let transform = app.world().get::<Transform>(enemy_entity).unwrap();
            assert!(
                (transform.translation.x - 100.0).abs() < 0.01,
                "Enemy with missing cage should not be constrained"
            );
        }

        #[test]
        fn test_mind_cage_boundary_collision() {
            let mut app = App::new();

            // Create cage at origin with radius 5.0
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy exactly at boundary
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                CagedEnemy::new(cage_entity),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_constraint_system);

            let transform = app.world().get::<Transform>(enemy_entity).unwrap();
            // Should remain at edge
            assert!(
                (transform.translation.x - 5.0).abs() < 0.01,
                "Enemy at boundary should stay at boundary"
            );
        }
    }

    mod mind_cage_cleanup_markers_system_tests {
        use super::*;

        #[test]
        fn test_mind_cage_frees_enemies_when_cage_gone() {
            let mut app = App::new();

            // Create a cage entity ID that doesn't exist
            let fake_cage = Entity::from_bits(9999);

            // Create enemy marked as caged by the non-existent cage
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
                CagedEnemy::new(fake_cage),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_cleanup_markers_system);

            assert!(
                app.world().get::<CagedEnemy>(enemy_entity).is_none(),
                "CagedEnemy marker should be removed when cage doesn't exist"
            );
        }

        #[test]
        fn test_mind_cage_keeps_marker_while_cage_exists() {
            let mut app = App::new();

            // Create an actual cage
            let cage_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                MindCage::with_config(Vec2::ZERO, 5.0, 10.0),
            )).id();

            // Create enemy marked as caged
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
                CagedEnemy::new(cage_entity),
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_cleanup_markers_system);

            assert!(
                app.world().get::<CagedEnemy>(enemy_entity).is_some(),
                "CagedEnemy marker should remain while cage exists"
            );
        }
    }

    mod mind_cage_cleanup_system_tests {
        use super::*;

        #[test]
        fn test_mind_cage_despawns_when_expired() {
            let mut app = App::new();

            let mut cage = MindCage::with_config(Vec2::ZERO, 5.0, 0.1);
            cage.tick(Duration::from_secs_f32(0.2)); // Expire it

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                cage,
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_cleanup_system);

            assert!(app.world().get_entity(entity).is_err(), "Expired cage should be despawned");
        }

        #[test]
        fn test_mind_cage_survives_while_active() {
            let mut app = App::new();

            let cage = MindCage::with_config(Vec2::ZERO, 5.0, 10.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                cage,
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_cleanup_system);

            assert!(app.world().get_entity(entity).is_ok(), "Active cage should not be despawned");
        }
    }

    mod fire_mind_cage_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_mind_cage_spawns_cage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MindBlast);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_cage(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindCage>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_mind_cage_at_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MindBlast);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_cage(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindCage>();
            for cage in query.iter(app.world()) {
                assert_eq!(cage.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_mind_cage_with_custom_config() {
            let mut app = setup_test_app();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let custom_radius = 10.0;
            let custom_duration = 8.0;

            {
                let mut commands = app.world_mut().commands();
                fire_mind_cage_with_config(
                    &mut commands,
                    spawn_pos,
                    custom_radius,
                    custom_duration,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindCage>();
            for cage in query.iter(app.world()) {
                assert_eq!(cage.radius, custom_radius);
            }
        }

        #[test]
        fn test_fire_mind_cage_default_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MindBlast);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_cage(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindCage>();
            for cage in query.iter(app.world()) {
                assert_eq!(cage.radius, MIND_CAGE_DEFAULT_RADIUS);
            }
        }
    }

    mod mind_cage_visual_system_tests {
        use super::*;

        #[test]
        fn test_visual_scale_matches_radius() {
            let mut app = App::new();

            let cage = MindCage::with_config(Vec2::ZERO, 7.0, 10.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                cage,
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale.x, 7.0);
        }

        #[test]
        fn test_visual_flickers_when_expiring() {
            let mut app = App::new();

            // Create cage that's almost expired (10% remaining)
            let mut cage = MindCage::with_config(Vec2::ZERO, 5.0, 1.0);
            cage.tick(Duration::from_secs_f32(0.9)); // 90% expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                cage,
            )).id();

            let _ = app.world_mut().run_system_once(mind_cage_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Should have some flicker effect, so scale won't be exactly 5.0
            assert!(transform.scale.x <= 5.0, "Scale should be reduced by flicker");
            assert!(transform.scale.x >= 3.5, "Scale shouldn't be too small");
        }
    }
}
