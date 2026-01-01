use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::player::components::Player;
use crate::spell::components::Spell;
use rand::Rng as _;

/// Default configuration for Stormcall spell
pub const STORMCALL_MARKER_COUNT_MIN: u8 = 3;
pub const STORMCALL_MARKER_COUNT_MAX: u8 = 5;
pub const STORMCALL_ROAM_RANGE: f32 = 12.0;
pub const STORMCALL_MOVE_INTERVAL: f32 = 1.5;
pub const STORMCALL_STRIKE_INTERVAL: f32 = 2.0;
pub const STORMCALL_STRIKE_RADIUS: f32 = 3.0;
pub const STORMCALL_DURATION: f32 = 8.0;
pub const STORMCALL_STRIKE_VISUAL_LIFETIME: f32 = 0.3;

/// Get the lightning element color for visual effects (yellow)
pub fn stormcall_color() -> Color {
    Element::Lightning.color()
}

/// Controller component that tracks the overall Stormcall spell instance.
/// Manages duration and marker count for the spell.
#[derive(Component, Debug, Clone)]
pub struct StormcallController {
    /// Number of markers spawned for this controller
    pub marker_count: u8,
    /// Timer for total spell duration
    pub duration: Timer,
}

impl StormcallController {
    pub fn new(marker_count: u8) -> Self {
        Self {
            marker_count,
            duration: Timer::from_seconds(STORMCALL_DURATION, TimerMode::Once),
        }
    }

    /// Check if the spell has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }
}

/// Marker component for individual roaming lightning markers.
/// Each marker moves randomly and periodically triggers lightning strikes.
#[derive(Component, Debug, Clone)]
pub struct StormcallMarker {
    /// Timer for moving to a new position
    pub move_timer: Timer,
    /// Timer for triggering lightning strikes
    pub strike_timer: Timer,
    /// Maximum distance from player that marker can roam
    pub roam_range: f32,
    /// Radius of damage when strike triggers
    pub strike_radius: f32,
    /// Damage dealt per strike
    pub strike_damage: f32,
    /// Current position on XZ plane
    pub position: Vec2,
    /// Entity of the controller this marker belongs to
    pub controller: Entity,
}

impl StormcallMarker {
    pub fn new(position: Vec2, damage: f32, controller: Entity) -> Self {
        Self {
            move_timer: Timer::from_seconds(STORMCALL_MOVE_INTERVAL, TimerMode::Repeating),
            strike_timer: Timer::from_seconds(STORMCALL_STRIKE_INTERVAL, TimerMode::Repeating),
            roam_range: STORMCALL_ROAM_RANGE,
            strike_radius: STORMCALL_STRIKE_RADIUS,
            strike_damage: damage,
            position,
            controller,
        }
    }

    /// Check if marker should move to a new position
    pub fn should_move(&self) -> bool {
        self.move_timer.just_finished()
    }

    /// Check if marker should trigger a lightning strike
    pub fn should_strike(&self) -> bool {
        self.strike_timer.just_finished()
    }
}

/// Lightning strike effect component spawned when a marker triggers.
/// Deals AoE damage to enemies in radius.
#[derive(Component, Debug, Clone)]
pub struct StormcallStrike {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Damage dealt to enemies in area
    pub damage: f32,
    /// Radius of damage area
    pub radius: f32,
    /// Lifetime timer for visual effect
    pub lifetime: Timer,
    /// Whether damage has been applied (only apply once)
    pub damage_applied: bool,
}

impl StormcallStrike {
    pub fn new(center: Vec2, damage: f32, radius: f32) -> Self {
        Self {
            center,
            damage,
            radius,
            lifetime: Timer::from_seconds(STORMCALL_STRIKE_VISUAL_LIFETIME, TimerMode::Once),
            damage_applied: false,
        }
    }

    pub fn from_marker(marker: &StormcallMarker) -> Self {
        Self::new(marker.position, marker.strike_damage, marker.strike_radius)
    }

    /// Check if the visual effect has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// System that updates controller duration and despawns expired controllers
/// along with their associated markers.
pub fn stormcall_duration_system(
    mut commands: Commands,
    time: Res<Time>,
    mut controller_query: Query<(Entity, &mut StormcallController)>,
    marker_query: Query<(Entity, &StormcallMarker)>,
) {
    for (controller_entity, mut controller) in controller_query.iter_mut() {
        controller.duration.tick(time.delta());

        if controller.is_expired() {
            // Despawn all markers belonging to this controller
            for (marker_entity, marker) in marker_query.iter() {
                if marker.controller == controller_entity {
                    commands.entity(marker_entity).despawn();
                }
            }
            // Despawn the controller
            commands.entity(controller_entity).despawn();
        }
    }
}

/// System that moves markers to new random positions around the player.
pub fn stormcall_marker_move_system(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut marker_query: Query<(&mut StormcallMarker, &mut Transform), Without<Player>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = from_xz(player_transform.translation);

    let mut rng = rand::thread_rng();

    for (mut marker, mut transform) in marker_query.iter_mut() {
        marker.move_timer.tick(time.delta());

        if marker.should_move() {
            // Generate new random position within roam_range of player
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(0.0..marker.roam_range);
            let offset = Vec2::new(angle.cos() * distance, angle.sin() * distance);
            let new_position = player_pos + offset;

            marker.position = new_position;
            transform.translation = to_xz(new_position) + Vec3::new(0.0, 0.1, 0.0);
        }
    }
}

/// System that triggers lightning strikes at marker positions.
pub fn stormcall_marker_strike_system(
    mut commands: Commands,
    time: Res<Time>,
    mut marker_query: Query<&mut StormcallMarker>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for mut marker in marker_query.iter_mut() {
        marker.strike_timer.tick(time.delta());

        if marker.should_strike() {
            // Spawn lightning strike at marker position
            let strike = StormcallStrike::from_marker(&marker);
            let strike_pos = to_xz(marker.position) + Vec3::new(0.0, 0.2, 0.0);

            if let (Some(ref meshes), Some(ref materials)) = (&game_meshes, &game_materials) {
                commands.spawn((
                    Mesh3d(meshes.explosion.clone()),
                    MeshMaterial3d(materials.thunder_strike.clone()),
                    Transform::from_translation(strike_pos).with_scale(Vec3::splat(strike.radius)),
                    strike,
                ));
            } else {
                // Fallback for tests without mesh resources
                commands.spawn((
                    Transform::from_translation(strike_pos),
                    strike,
                ));
            }
        }
    }
}

/// System that applies area damage when lightning strike lands.
pub fn stormcall_strike_damage_system(
    mut strike_query: Query<&mut StormcallStrike>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut strike in strike_query.iter_mut() {
        if strike.damage_applied {
            continue;
        }

        // Apply damage to all enemies in radius
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = strike.center.distance(enemy_pos);

            if distance <= strike.radius {
                damage_events.write(DamageEvent::new(enemy_entity, strike.damage));
            }
        }

        strike.damage_applied = true;
    }
}

/// System that updates strike lifetime and despawns expired strikes.
pub fn stormcall_strike_cleanup_system(
    mut commands: Commands,
    time: Res<Time>,
    mut strike_query: Query<(Entity, &mut StormcallStrike, &mut Transform)>,
) {
    for (entity, mut strike, mut transform) in strike_query.iter_mut() {
        strike.lifetime.tick(time.delta());

        // Fade out effect by scaling down
        let progress = strike.lifetime.elapsed_secs() / STORMCALL_STRIKE_VISUAL_LIFETIME;
        let scale = strike.radius * (1.0 - progress * 0.5); // Scale from full to half
        transform.scale = Vec3::splat(scale);

        if strike.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Stormcall spell - spawns a controller and 3-5 roaming markers.
/// `spawn_position` is Whisper's full 3D position (player position).
#[allow(clippy::too_many_arguments)]
pub fn fire_stormcall(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_stormcall_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Stormcall spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_stormcall_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let player_pos = from_xz(spawn_position);
    let mut rng = rand::thread_rng();

    // Random marker count between min and max (inclusive)
    let marker_count = rng.gen_range(STORMCALL_MARKER_COUNT_MIN..=STORMCALL_MARKER_COUNT_MAX);

    // Spawn controller entity
    let controller_entity = commands.spawn(StormcallController::new(marker_count)).id();

    // Spawn markers around the player
    for _ in 0..marker_count {
        // Random initial position within roam range
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let distance = rng.gen_range(0.0..STORMCALL_ROAM_RANGE);
        let offset = Vec2::new(angle.cos() * distance, angle.sin() * distance);
        let marker_position = player_pos + offset;

        let marker = StormcallMarker::new(marker_position, damage, controller_entity);
        let marker_pos = to_xz(marker_position) + Vec3::new(0.0, 0.1, 0.0);

        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.target_marker.clone()),
                MeshMaterial3d(materials.thunder_strike_marker.clone()),
                Transform::from_translation(marker_pos).with_scale(Vec3::splat(1.0)),
                marker,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(marker_pos),
                marker,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod stormcall_controller_tests {
        use super::*;

        #[test]
        fn test_controller_creation() {
            let controller = StormcallController::new(4);

            assert_eq!(controller.marker_count, 4);
            assert!(!controller.is_expired());
        }

        #[test]
        fn test_controller_expires_after_duration() {
            let mut controller = StormcallController::new(3);
            controller.duration.tick(Duration::from_secs_f32(STORMCALL_DURATION + 0.1));

            assert!(controller.is_expired());
        }

        #[test]
        fn test_controller_does_not_expire_before_duration() {
            let mut controller = StormcallController::new(3);
            controller.duration.tick(Duration::from_secs_f32(STORMCALL_DURATION / 2.0));

            assert!(!controller.is_expired());
        }
    }

    mod stormcall_marker_tests {
        use super::*;

        #[test]
        fn test_marker_creation() {
            let controller_entity = Entity::from_bits(1);
            let position = Vec2::new(10.0, 20.0);
            let damage = 35.0;
            let marker = StormcallMarker::new(position, damage, controller_entity);

            assert_eq!(marker.position, position);
            assert_eq!(marker.strike_damage, damage);
            assert_eq!(marker.controller, controller_entity);
            assert_eq!(marker.roam_range, STORMCALL_ROAM_RANGE);
            assert_eq!(marker.strike_radius, STORMCALL_STRIKE_RADIUS);
            assert!(!marker.should_move());
            assert!(!marker.should_strike());
        }

        #[test]
        fn test_marker_move_timer_triggers() {
            let controller_entity = Entity::from_bits(1);
            let mut marker = StormcallMarker::new(Vec2::ZERO, 35.0, controller_entity);

            // Not ready initially
            assert!(!marker.should_move());

            // Tick past move interval
            marker.move_timer.tick(Duration::from_secs_f32(STORMCALL_MOVE_INTERVAL + 0.1));

            assert!(marker.should_move());
        }

        #[test]
        fn test_marker_strike_timer_triggers() {
            let controller_entity = Entity::from_bits(1);
            let mut marker = StormcallMarker::new(Vec2::ZERO, 35.0, controller_entity);

            // Not ready initially
            assert!(!marker.should_strike());

            // Tick past strike interval
            marker.strike_timer.tick(Duration::from_secs_f32(STORMCALL_STRIKE_INTERVAL + 0.1));

            assert!(marker.should_strike());
        }
    }

    mod stormcall_strike_tests {
        use super::*;

        #[test]
        fn test_strike_creation() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 35.0;
            let radius = 3.0;
            let strike = StormcallStrike::new(center, damage, radius);

            assert_eq!(strike.center, center);
            assert_eq!(strike.damage, damage);
            assert_eq!(strike.radius, radius);
            assert!(!strike.damage_applied);
            assert!(!strike.is_expired());
        }

        #[test]
        fn test_strike_from_marker() {
            let controller_entity = Entity::from_bits(1);
            let marker = StormcallMarker::new(Vec2::new(5.0, 10.0), 40.0, controller_entity);
            let strike = StormcallStrike::from_marker(&marker);

            assert_eq!(strike.center, marker.position);
            assert_eq!(strike.damage, marker.strike_damage);
            assert_eq!(strike.radius, marker.strike_radius);
        }

        #[test]
        fn test_strike_expires_after_lifetime() {
            let mut strike = StormcallStrike::new(Vec2::ZERO, 35.0, 3.0);

            // Not expired initially
            assert!(!strike.is_expired());

            // Tick past lifetime
            strike.lifetime.tick(Duration::from_secs_f32(STORMCALL_STRIKE_VISUAL_LIFETIME + 0.1));

            assert!(strike.is_expired());
        }

        #[test]
        fn test_uses_lightning_element_color() {
            let color = stormcall_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }
    }

    mod stormcall_duration_system_tests {
        use super::*;

        #[test]
        fn test_controller_despawns_after_duration() {
            let mut app = App::new();
            app.add_systems(Update, stormcall_duration_system);
            app.init_resource::<Time>();

            let controller_entity = app.world_mut().spawn(StormcallController::new(3)).id();

            // Spawn markers for this controller
            for _ in 0..3 {
                app.world_mut().spawn(StormcallMarker::new(
                    Vec2::ZERO,
                    35.0,
                    controller_entity,
                ));
            }

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_DURATION + 0.1));
            }

            app.update();

            // Controller should be despawned
            assert!(app.world().get_entity(controller_entity).is_err());

            // All markers should be despawned
            let mut marker_query = app.world_mut().query::<&StormcallMarker>();
            let count = marker_query.iter(app.world()).count();
            assert_eq!(count, 0);
        }

        #[test]
        fn test_controller_survives_before_duration() {
            let mut app = App::new();
            app.add_systems(Update, stormcall_duration_system);
            app.init_resource::<Time>();

            let controller_entity = app.world_mut().spawn(StormcallController::new(3)).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_DURATION / 2.0));
            }

            app.update();

            // Controller should still exist
            assert!(app.world().get_entity(controller_entity).is_ok());
        }
    }

    mod stormcall_marker_move_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_marker_moves_when_timer_triggers() {
            let mut app = setup_test_app();

            // Spawn player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            ));

            // Spawn controller and marker
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            let initial_position = Vec2::new(5.0, 5.0);
            let marker_entity = app.world_mut().spawn((
                StormcallMarker::new(initial_position, 35.0, controller_entity),
                Transform::from_translation(to_xz(initial_position) + Vec3::new(0.0, 0.1, 0.0)),
            )).id();

            // Advance time past move interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_MOVE_INTERVAL + 0.1));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_move_system);

            // Marker position should have changed (highly likely to be different)
            let marker = app.world().get::<StormcallMarker>(marker_entity).unwrap();
            // We can't assert exact position due to randomness, but we verify the system ran
            // by checking the timer was triggered (just_finished is true after tick)
            assert!(marker.move_timer.just_finished());
        }

        #[test]
        fn test_marker_does_not_move_before_timer() {
            let mut app = setup_test_app();

            // Spawn player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            ));

            // Spawn controller and marker
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            let initial_position = Vec2::new(5.0, 5.0);
            let marker_entity = app.world_mut().spawn((
                StormcallMarker::new(initial_position, 35.0, controller_entity),
                Transform::from_translation(to_xz(initial_position) + Vec3::new(0.0, 0.1, 0.0)),
            )).id();

            // Advance time but not past move interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_MOVE_INTERVAL / 2.0));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_move_system);

            // Marker position should be unchanged
            let marker = app.world().get::<StormcallMarker>(marker_entity).unwrap();
            assert_eq!(marker.position, initial_position);
        }

        #[test]
        fn test_marker_stays_within_roam_range_of_player() {
            let mut app = setup_test_app();

            let player_pos = Vec3::new(100.0, 0.5, 100.0);
            let player_pos_2d = from_xz(player_pos);

            // Spawn player at specific position
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(player_pos),
            ));

            // Spawn controller and marker
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            let initial_position = player_pos_2d; // Start at player
            let marker_entity = app.world_mut().spawn((
                StormcallMarker::new(initial_position, 35.0, controller_entity),
                Transform::from_translation(to_xz(initial_position) + Vec3::new(0.0, 0.1, 0.0)),
            )).id();

            // Run multiple move cycles
            for _ in 0..10 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(STORMCALL_MOVE_INTERVAL + 0.1));
                }

                // Run the system directly
                let _ = app.world_mut().run_system_once(stormcall_marker_move_system);

                // Verify marker stays within range
                let marker = app.world().get::<StormcallMarker>(marker_entity).unwrap();
                let distance = marker.position.distance(player_pos_2d);
                assert!(
                    distance <= STORMCALL_ROAM_RANGE + 0.01, // Small epsilon for floating point
                    "Marker at distance {} exceeds roam range {}",
                    distance,
                    STORMCALL_ROAM_RANGE
                );
            }
        }

        #[test]
        fn test_no_crash_without_player() {
            let mut app = setup_test_app();

            // Spawn controller and marker (no player)
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            app.world_mut().spawn((
                StormcallMarker::new(Vec2::ZERO, 35.0, controller_entity),
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
            ));

            // Advance time past move interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_MOVE_INTERVAL + 0.1));
            }

            // Should not crash - run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_move_system);
        }
    }

    mod stormcall_marker_strike_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_marker_spawns_strike_when_timer_triggers() {
            let mut app = setup_test_app();

            // Spawn controller and marker
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            app.world_mut().spawn(StormcallMarker::new(Vec2::new(10.0, 20.0), 35.0, controller_entity));

            // Advance time past strike interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_STRIKE_INTERVAL + 0.1));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_strike_system);

            // Apply commands for spawned entities
            app.world_mut().flush();

            // Strike should exist
            let mut query = app.world_mut().query::<&StormcallStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_marker_does_not_spawn_strike_before_timer() {
            let mut app = setup_test_app();

            // Spawn controller and marker
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            app.world_mut().spawn(StormcallMarker::new(Vec2::ZERO, 35.0, controller_entity));

            // Advance time but not past strike interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_STRIKE_INTERVAL / 2.0));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_strike_system);

            // Apply commands for spawned entities
            app.world_mut().flush();

            // No strike yet
            let mut query = app.world_mut().query::<&StormcallStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 0);
        }

        #[test]
        fn test_strike_position_matches_marker() {
            let mut app = setup_test_app();

            // Spawn controller and marker at specific position
            let marker_position = Vec2::new(15.0, 25.0);
            let controller_entity = app.world_mut().spawn(StormcallController::new(1)).id();
            app.world_mut().spawn(StormcallMarker::new(marker_position, 35.0, controller_entity));

            // Advance time past strike interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_STRIKE_INTERVAL + 0.1));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_strike_system);

            // Apply commands for spawned entities
            app.world_mut().flush();

            // Strike should be at marker position
            let mut query = app.world_mut().query::<&StormcallStrike>();
            for strike in query.iter(app.world()) {
                assert_eq!(strike.center, marker_position);
            }
        }

        #[test]
        fn test_multiple_markers_spawn_multiple_strikes() {
            let mut app = setup_test_app();

            // Spawn controller and 3 markers
            let controller_entity = app.world_mut().spawn(StormcallController::new(3)).id();
            for i in 0..3 {
                app.world_mut().spawn(StormcallMarker::new(
                    Vec2::new(i as f32 * 10.0, 0.0),
                    35.0,
                    controller_entity,
                ));
            }

            // Advance time past strike interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_STRIKE_INTERVAL + 0.1));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(stormcall_marker_strike_system);

            // Apply commands for spawned entities
            app.world_mut().flush();

            // 3 strikes should exist
            let mut query = app.world_mut().query::<&StormcallStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 3);
        }
    }

    mod stormcall_strike_damage_system_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_damage_applied_to_enemies_in_radius() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (stormcall_strike_damage_system, count_damage_events).chain());

            // Create strike at origin with radius 3.0
            app.world_mut().spawn(StormcallStrike::new(Vec2::ZERO, 35.0, 3.0));

            // Create enemy within radius (XZ distance = 2)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_no_damage_outside_radius() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (stormcall_strike_damage_system, count_damage_events).chain());

            // Create strike at origin with radius 3.0
            app.world_mut().spawn(StormcallStrike::new(Vec2::ZERO, 35.0, 3.0));

            // Create enemy outside radius (XZ distance = 5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_damage_applied_only_once() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (stormcall_strike_damage_system, count_damage_events).chain());

            // Create strike
            app.world_mut().spawn(StormcallStrike::new(Vec2::ZERO, 35.0, 3.0));

            // Create enemy in radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Should only damage once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_damage_uses_xz_plane_ignores_y() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (stormcall_strike_damage_system, count_damage_events).chain());

            // Create strike at origin
            app.world_mut().spawn(StormcallStrike::new(Vec2::ZERO, 35.0, 3.0));

            // Create enemy close on XZ plane but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 100.0, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }

        #[test]
        fn test_multiple_enemies_in_radius() {
            let mut app = App::new();

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

            app.add_message::<DamageEvent>();
            app.add_systems(Update, (stormcall_strike_damage_system, count_damage_events).chain());

            // Create strike
            app.world_mut().spawn(StormcallStrike::new(Vec2::ZERO, 35.0, 3.0));

            // Create 3 enemies in radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }
    }

    mod stormcall_strike_cleanup_system_tests {
        use super::*;

        #[test]
        fn test_strike_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, stormcall_strike_cleanup_system);
            app.init_resource::<Time>();

            let strike_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                StormcallStrike::new(Vec2::ZERO, 35.0, 3.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_STRIKE_VISUAL_LIFETIME + 0.1));
            }

            app.update();

            // Strike should be despawned
            assert!(app.world().get_entity(strike_entity).is_err());
        }

        #[test]
        fn test_strike_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, stormcall_strike_cleanup_system);
            app.init_resource::<Time>();

            let strike_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                StormcallStrike::new(Vec2::ZERO, 35.0, 3.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STORMCALL_STRIKE_VISUAL_LIFETIME / 2.0));
            }

            app.update();

            // Strike should still exist
            assert!(app.world().get_entity(strike_entity).is_ok());
        }
    }

    mod fire_stormcall_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_stormcall_spawns_controller() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 controller
            let mut query = app.world_mut().query::<&StormcallController>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_stormcall_spawns_markers() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 3-5 markers
            let mut query = app.world_mut().query::<&StormcallMarker>();
            let count = query.iter(app.world()).count();
            assert!(
                count >= STORMCALL_MARKER_COUNT_MIN as usize && count <= STORMCALL_MARKER_COUNT_MAX as usize,
                "Expected {} to {} markers, got {}",
                STORMCALL_MARKER_COUNT_MIN,
                STORMCALL_MARKER_COUNT_MAX,
                count
            );
        }

        #[test]
        fn test_fire_stormcall_marker_count_matches_controller() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut controller_query = app.world_mut().query::<&StormcallController>();
            let controller = controller_query.single(app.world()).unwrap();
            let expected_count = controller.marker_count as usize;

            let mut marker_query = app.world_mut().query::<&StormcallMarker>();
            let actual_count = marker_query.iter(app.world()).count();

            assert_eq!(actual_count, expected_count);
        }

        #[test]
        fn test_fire_stormcall_markers_reference_controller() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Get controller entity
            let mut controller_query = app.world_mut().query::<(Entity, &StormcallController)>();
            let (controller_entity, _) = controller_query.single(app.world()).unwrap();

            // All markers should reference this controller
            let mut marker_query = app.world_mut().query::<&StormcallMarker>();
            for marker in marker_query.iter(app.world()) {
                assert_eq!(marker.controller, controller_entity);
            }
        }

        #[test]
        fn test_fire_stormcall_markers_spawn_within_range() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let spawn_pos = Vec3::new(50.0, 0.5, 50.0);
            let spawn_pos_2d = from_xz(spawn_pos);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // All markers should be within roam range
            let mut marker_query = app.world_mut().query::<&StormcallMarker>();
            for marker in marker_query.iter(app.world()) {
                let distance = marker.position.distance(spawn_pos_2d);
                assert!(
                    distance <= STORMCALL_ROAM_RANGE + 0.01,
                    "Marker at distance {} exceeds roam range {}",
                    distance,
                    STORMCALL_ROAM_RANGE
                );
            }
        }

        #[test]
        fn test_fire_stormcall_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&StormcallMarker>();
            for marker in query.iter(app.world()) {
                assert_eq!(marker.strike_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_stormcall_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StormCall);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_stormcall_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&StormcallMarker>();
            for marker in query.iter(app.world()) {
                assert_eq!(marker.strike_damage, explicit_damage);
            }
        }
    }
}
