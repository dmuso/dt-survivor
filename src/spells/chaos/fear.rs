//! Fear spell - AOE burst that causes enemies to flee uncontrollably.
//!
//! A Chaos element spell (Mayhem SpellType) that creates an expanding burst
//! centered on the player. Enemies within the burst radius receive the FearedEnemy
//! debuff, causing them to flee away from the player for a duration.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Fear spell
pub const FEAR_BURST_RADIUS: f32 = 6.0;
pub const FEAR_DURATION: f32 = 3.0;
pub const FEAR_FLEE_SPEED_MULTIPLIER: f32 = 1.5; // 50% faster when fleeing
pub const FEAR_VISUAL_HEIGHT: f32 = 0.2;

/// Get the chaos element color for visual effects
pub fn fear_color() -> Color {
    Element::Chaos.color()
}

/// Component for the fear AOE burst.
/// Tracks the burst radius and which enemies have been affected.
#[derive(Component, Debug, Clone)]
pub struct FearBurst {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the fear effect
    pub radius: f32,
    /// Duration of fear to apply to affected enemies
    pub fear_duration: f32,
    /// Set of enemy entities already affected by this burst
    pub affected_enemies: HashSet<Entity>,
    /// Whether this burst has been processed (single-frame effect)
    pub processed: bool,
}

impl FearBurst {
    pub fn new(center: Vec2) -> Self {
        Self {
            center,
            radius: FEAR_BURST_RADIUS,
            fear_duration: FEAR_DURATION,
            affected_enemies: HashSet::new(),
            processed: false,
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.fear_duration = duration;
        self
    }

    /// Check if an enemy is within the burst radius and hasn't been affected yet
    pub fn can_affect(&self, entity: Entity, enemy_pos: Vec2) -> bool {
        let distance = self.center.distance(enemy_pos);
        distance <= self.radius && !self.affected_enemies.contains(&entity)
    }

    /// Mark an enemy as affected
    pub fn mark_affected(&mut self, entity: Entity) {
        self.affected_enemies.insert(entity);
    }
}

/// Component applied to enemies affected by fear.
/// Causes them to flee away from the player at increased speed.
#[derive(Component, Debug, Clone)]
pub struct FearedEnemy {
    /// Timer tracking remaining fear duration
    pub duration: Timer,
    /// The initial flee direction (away from player at time of fear)
    pub flee_direction: Vec2,
    /// Speed multiplier while feared
    pub speed_multiplier: f32,
}

impl FearedEnemy {
    pub fn new(duration: f32, flee_direction: Vec2) -> Self {
        Self {
            duration: Timer::from_seconds(duration, TimerMode::Once),
            flee_direction: flee_direction.normalize_or_zero(),
            speed_multiplier: FEAR_FLEE_SPEED_MULTIPLIER,
        }
    }

    /// Check if the fear effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the fear timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Refresh the fear duration with a new timer
    pub fn refresh(&mut self, duration: f32, flee_direction: Vec2) {
        self.duration = Timer::from_seconds(duration, TimerMode::Once);
        self.flee_direction = flee_direction.normalize_or_zero();
    }
}

/// Spawns a fear burst at the given position.
pub fn spawn_fear_burst(
    commands: &mut Commands,
    _spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let burst = FearBurst::new(center);
    let burst_pos = Vec3::new(spawn_position.x, FEAR_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        // Use powerup material (magenta) for chaos element visual
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.powerup.clone()),
            Transform::from_translation(burst_pos).with_scale(Vec3::splat(burst.radius)),
            burst,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(burst_pos),
            burst,
        ));
    }
}

/// System that applies the fear debuff to enemies within burst radius.
/// This is a single-frame effect - once processed, enemies are marked and the burst is cleaned up.
pub fn apply_fear_to_enemies_system(
    mut commands: Commands,
    mut burst_query: Query<&mut FearBurst>,
    mut enemy_query: Query<(Entity, &Transform, Option<&mut FearedEnemy>), With<Enemy>>,
    player_query: Query<&Transform, (With<crate::player::components::Player>, Without<Enemy>)>,
) {
    let player_transform = match player_query.single() {
        Ok(t) => t,
        Err(_) => return,
    };
    let player_pos = from_xz(player_transform.translation);

    for mut burst in burst_query.iter_mut() {
        if burst.processed {
            continue;
        }

        for (enemy_entity, enemy_transform, existing_fear) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if burst.can_affect(enemy_entity, enemy_pos) {
                // Calculate flee direction (away from player)
                let flee_direction = (enemy_pos - player_pos).normalize_or_zero();

                if let Some(mut feared) = existing_fear {
                    // Refresh existing fear duration
                    feared.refresh(burst.fear_duration, flee_direction);
                } else {
                    // Apply new fear debuff
                    commands.entity(enemy_entity).try_insert(
                        FearedEnemy::new(burst.fear_duration, flee_direction)
                    );
                }

                burst.mark_affected(enemy_entity);
            }
        }

        burst.processed = true;
    }
}

/// System that updates feared enemy movement.
/// Feared enemies move away from the player at increased speed.
pub fn update_feared_enemies_system(
    mut enemy_query: Query<(&mut FearedEnemy, &mut Transform, &Enemy)>,
    player_query: Query<&Transform, (With<crate::player::components::Player>, Without<Enemy>)>,
    time: Res<Time>,
) {
    let player_transform = match player_query.single() {
        Ok(t) => t,
        Err(_) => return,
    };
    let player_pos = from_xz(player_transform.translation);

    for (mut feared, mut transform, enemy) in enemy_query.iter_mut() {
        feared.tick(time.delta());

        if !feared.is_expired() {
            let enemy_pos = from_xz(transform.translation);
            // Continuously update flee direction to move away from player
            let flee_direction = (enemy_pos - player_pos).normalize_or_zero();

            // Move at enhanced speed away from player
            let speed = enemy.speed * feared.speed_multiplier;
            let movement = flee_direction * speed * time.delta_secs();

            transform.translation.x += movement.x;
            transform.translation.z += movement.y;
        }
    }
}

/// System that removes expired fear effects from enemies.
pub fn cleanup_fear_effect_system(
    mut commands: Commands,
    query: Query<(Entity, &FearedEnemy)>,
) {
    for (entity, feared) in query.iter() {
        if feared.is_expired() {
            commands.entity(entity).remove::<FearedEnemy>();
        }
    }
}

/// System that despawns fear bursts after they've been processed.
pub fn cleanup_fear_burst_system(
    mut commands: Commands,
    query: Query<(Entity, &FearBurst)>,
) {
    for (entity, burst) in query.iter() {
        if burst.processed {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::player::components::Player;

    mod fear_burst_component_tests {
        use super::*;

        #[test]
        fn test_fear_burst_new() {
            let center = Vec2::new(5.0, 10.0);
            let burst = FearBurst::new(center);

            assert_eq!(burst.center, center);
            assert_eq!(burst.radius, FEAR_BURST_RADIUS);
            assert_eq!(burst.fear_duration, FEAR_DURATION);
            assert!(burst.affected_enemies.is_empty());
            assert!(!burst.processed);
        }

        #[test]
        fn test_fear_burst_with_radius() {
            let burst = FearBurst::new(Vec2::ZERO).with_radius(10.0);
            assert_eq!(burst.radius, 10.0);
        }

        #[test]
        fn test_fear_burst_with_duration() {
            let burst = FearBurst::new(Vec2::ZERO).with_duration(5.0);
            assert_eq!(burst.fear_duration, 5.0);
        }

        #[test]
        fn test_fear_burst_can_affect_in_radius() {
            let burst = FearBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(3.0, 0.0);

            assert!(burst.can_affect(entity, in_range_pos));
        }

        #[test]
        fn test_fear_burst_cannot_affect_outside_radius() {
            let burst = FearBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);
            let out_of_range_pos = Vec2::new(100.0, 0.0);

            assert!(!burst.can_affect(entity, out_of_range_pos));
        }

        #[test]
        fn test_fear_burst_cannot_affect_already_affected() {
            let mut burst = FearBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(3.0, 0.0);

            burst.mark_affected(entity);
            assert!(!burst.can_affect(entity, in_range_pos));
        }

        #[test]
        fn test_fear_burst_mark_affected() {
            let mut burst = FearBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);

            burst.mark_affected(entity);
            assert!(burst.affected_enemies.contains(&entity));
        }

        #[test]
        fn test_uses_chaos_element_color() {
            let color = fear_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod feared_enemy_component_tests {
        use super::*;

        #[test]
        fn test_feared_enemy_new() {
            let flee_direction = Vec2::new(1.0, 0.0);
            let feared = FearedEnemy::new(3.0, flee_direction);

            assert_eq!(feared.flee_direction, flee_direction.normalize());
            assert_eq!(feared.speed_multiplier, FEAR_FLEE_SPEED_MULTIPLIER);
            assert!(!feared.is_expired());
        }

        #[test]
        fn test_feared_enemy_normalizes_direction() {
            let flee_direction = Vec2::new(3.0, 4.0);
            let feared = FearedEnemy::new(3.0, flee_direction);

            let expected = flee_direction.normalize();
            assert!((feared.flee_direction - expected).length() < 0.001);
        }

        #[test]
        fn test_feared_enemy_handles_zero_direction() {
            let feared = FearedEnemy::new(3.0, Vec2::ZERO);
            assert_eq!(feared.flee_direction, Vec2::ZERO);
        }

        #[test]
        fn test_feared_enemy_is_expired() {
            let mut feared = FearedEnemy::new(0.1, Vec2::X);
            assert!(!feared.is_expired());

            feared.tick(Duration::from_secs_f32(0.2));
            assert!(feared.is_expired());
        }

        #[test]
        fn test_feared_enemy_tick() {
            let mut feared = FearedEnemy::new(1.0, Vec2::X);

            feared.tick(Duration::from_secs_f32(0.5));
            assert!(!feared.is_expired());

            feared.tick(Duration::from_secs_f32(0.5));
            assert!(feared.is_expired());
        }

        #[test]
        fn test_feared_enemy_refresh() {
            let mut feared = FearedEnemy::new(1.0, Vec2::X);
            feared.tick(Duration::from_secs_f32(0.9));

            // About to expire, but refresh
            feared.refresh(2.0, Vec2::Y);

            assert!(!feared.is_expired());
            assert_eq!(feared.flee_direction, Vec2::Y);
        }
    }

    mod spawn_fear_burst_tests {
        use super::*;
        use bevy::app::App;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fear_burst_spawns_at_position() {
            let mut app = setup_test_app();
            let spawn_pos = Vec3::new(15.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                let spell = Spell::new(SpellType::Mayhem);
                spawn_fear_burst(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FearBurst>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);

            for burst in query.iter(app.world()) {
                assert_eq!(burst.center, Vec2::new(15.0, 20.0));
            }
        }

        #[test]
        fn test_fear_burst_spawns_with_correct_radius() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                let spell = Spell::new(SpellType::Mayhem);
                spawn_fear_burst(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FearBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.radius, FEAR_BURST_RADIUS);
            }
        }

        #[test]
        fn test_fear_burst_starts_unprocessed() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                let spell = Spell::new(SpellType::Mayhem);
                spawn_fear_burst(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FearBurst>();
            for burst in query.iter(app.world()) {
                assert!(!burst.processed);
            }
        }
    }

    mod apply_fear_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_fear_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        fn test_player() -> Player {
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            }
        }

        #[test]
        fn test_fear_applies_to_enemies_in_radius() {
            let mut app = setup_fear_test_app();

            // Create player at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create fear burst at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FearBurst::new(Vec2::ZERO),
            ));

            // Create enemy in range
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(apply_fear_to_enemies_system);

            // Enemy should have FearedEnemy component
            let feared = app.world().get::<FearedEnemy>(enemy);
            assert!(feared.is_some(), "Enemy should have FearedEnemy component");
        }

        #[test]
        fn test_fear_does_not_affect_enemies_outside_radius() {
            let mut app = setup_fear_test_app();

            // Create player at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create fear burst at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FearBurst::new(Vec2::ZERO),
            ));

            // Create enemy outside range
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(apply_fear_to_enemies_system);

            // Enemy should NOT have FearedEnemy component
            let feared = app.world().get::<FearedEnemy>(enemy);
            assert!(feared.is_none(), "Enemy outside radius should not be feared");
        }

        #[test]
        fn test_feared_enemy_moves_away_from_player() {
            let mut app = setup_fear_test_app();

            // Create player at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create enemy with FearedEnemy component, positioned at (5, 0, 0)
            let initial_x = 5.0;
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(initial_x, 0.375, 0.0)),
                FearedEnemy::new(FEAR_DURATION, Vec2::X),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(update_feared_enemies_system);

            // Enemy should have moved further away (positive X direction)
            let transform = app.world().get::<Transform>(enemy).unwrap();
            assert!(
                transform.translation.x > initial_x,
                "Enemy should move away from player. Initial: {}, Current: {}",
                initial_x,
                transform.translation.x
            );
        }

        #[test]
        fn test_fear_duration_expires_correctly() {
            let mut app = setup_fear_test_app();

            // Create enemy with short fear duration
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                FearedEnemy::new(0.1, Vec2::X),
            )).id();

            // Wait longer than fear duration
            {
                let mut feared = app.world_mut().get_mut::<FearedEnemy>(enemy).unwrap();
                feared.tick(Duration::from_secs_f32(0.2));
            }

            let _ = app.world_mut().run_system_once(cleanup_fear_effect_system);

            // FearedEnemy should be removed
            let feared = app.world().get::<FearedEnemy>(enemy);
            assert!(feared.is_none(), "FearedEnemy should be removed after expiry");
        }

        #[test]
        fn test_fear_effect_removed_on_expiry() {
            let mut app = setup_fear_test_app();

            // Create enemy with expired fear
            let mut feared_component = FearedEnemy::new(0.0, Vec2::X);
            feared_component.duration.tick(Duration::from_secs(1)); // Force expire

            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                feared_component,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_fear_effect_system);

            assert!(app.world().get::<FearedEnemy>(enemy).is_none());
        }

        #[test]
        fn test_multiple_fear_refreshes_duration() {
            let mut app = setup_fear_test_app();

            // Create player at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create enemy with existing fear
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                FearedEnemy::new(0.5, Vec2::X),
            )).id();

            // Progress fear almost to expiry
            {
                let mut feared = app.world_mut().get_mut::<FearedEnemy>(enemy).unwrap();
                feared.tick(Duration::from_secs_f32(0.4));
            }

            // Create new fear burst
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FearBurst::new(Vec2::ZERO),
            ));

            let _ = app.world_mut().run_system_once(apply_fear_to_enemies_system);

            // Fear should be refreshed
            let feared = app.world().get::<FearedEnemy>(enemy).unwrap();
            assert!(!feared.is_expired(), "Fear should be refreshed, not expired");
        }

        #[test]
        fn test_feared_enemy_ignores_normal_ai() {
            // This test verifies that feared enemies use flee movement
            // rather than chasing the player. We do this by checking
            // that the enemy moves AWAY from the player.

            let mut app = setup_fear_test_app();

            // Create player at origin
            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create enemy between player and edge, moving toward player normally
            // but feared enemies should move away
            let initial_pos = Vec3::new(5.0, 0.375, 0.0);
            let enemy = app.world_mut().spawn((
                Enemy { speed: 100.0, strength: 10.0 },
                Transform::from_translation(initial_pos),
                FearedEnemy::new(FEAR_DURATION, Vec2::X),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(update_feared_enemies_system);

            let transform = app.world().get::<Transform>(enemy).unwrap();
            let new_distance = Vec2::new(transform.translation.x, transform.translation.z).length();
            let old_distance = Vec2::new(initial_pos.x, initial_pos.z).length();

            assert!(
                new_distance > old_distance,
                "Feared enemy should move away from player, not toward. Old distance: {}, New distance: {}",
                old_distance,
                new_distance
            );
        }
    }

    mod cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_fear_burst_despawns_after_processing() {
            let mut app = App::new();

            let mut burst = FearBurst::new(Vec2::ZERO);
            burst.processed = true;

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_fear_burst_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_fear_burst_survives_before_processing() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FearBurst::new(Vec2::ZERO),
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_fear_burst_system);

            assert!(app.world().entities().contains(entity));
        }
    }
}
