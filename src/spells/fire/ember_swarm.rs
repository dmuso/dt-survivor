use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;
use std::f32::consts::TAU;

/// Default configuration for Ember Swarm spell
pub const EMBER_SWARM_WISP_COUNT_MIN: u8 = 5;
pub const EMBER_SWARM_WISP_COUNT_MAX: u8 = 8;
pub const EMBER_SWARM_ORBIT_DURATION: f32 = 1.5;
pub const EMBER_SWARM_ORBIT_RADIUS: f32 = 2.0;
pub const EMBER_SWARM_ORBIT_SPEED: f32 = 4.0; // radians per second
pub const EMBER_SWARM_LAUNCH_SPEED: f32 = 25.0;
pub const EMBER_SWARM_HIT_RADIUS: f32 = 0.8;
pub const EMBER_SWARM_VISUAL_HEIGHT: f32 = 0.5;
pub const EMBER_SWARM_MAX_FLIGHT_TIME: f32 = 5.0;

/// Get the fire element color for visual effects
pub fn ember_swarm_color() -> Color {
    Element::Fire.color()
}

/// Controller component that manages the ember swarm lifecycle.
/// Tracks all wisps and their orbit phase.
#[derive(Component, Debug, Clone)]
pub struct EmberSwarmController {
    /// Number of wisps spawned
    pub wisp_count: u8,
    /// Time remaining in orbit phase
    pub orbit_duration: Timer,
    /// Entity IDs of all wisps in this swarm
    pub wisps: Vec<Entity>,
    /// Whether wisps have been launched
    pub launched: bool,
}

impl EmberSwarmController {
    pub fn new(wisp_count: u8) -> Self {
        Self {
            wisp_count,
            orbit_duration: Timer::from_seconds(EMBER_SWARM_ORBIT_DURATION, TimerMode::Once),
            wisps: Vec::with_capacity(wisp_count as usize),
            launched: false,
        }
    }

    /// Check if orbit phase is complete
    pub fn orbit_complete(&self) -> bool {
        self.orbit_duration.is_finished()
    }

    /// Check if all wisps have despawned
    pub fn all_wisps_gone(&self) -> bool {
        self.wisps.is_empty()
    }

    /// Remove a wisp from tracking
    pub fn remove_wisp(&mut self, entity: Entity) {
        self.wisps.retain(|&e| e != entity);
    }
}

/// Component for individual ember wisps during orbit phase.
/// Tracks orbit state and damage.
#[derive(Component, Debug, Clone)]
pub struct EmberWisp {
    /// Current phase in orbit (radians)
    pub orbit_phase: f32,
    /// Radius of orbit around player
    pub orbit_radius: f32,
    /// Damage to deal on hit
    pub damage: f32,
    /// Parent controller entity
    pub controller: Entity,
}

impl EmberWisp {
    pub fn new(orbit_phase: f32, damage: f32, controller: Entity) -> Self {
        Self {
            orbit_phase,
            orbit_radius: EMBER_SWARM_ORBIT_RADIUS,
            damage,
            controller,
        }
    }
}

/// Component added to wisps when they transition to launch phase.
/// Contains targeting and movement info.
#[derive(Component, Debug, Clone)]
pub struct LaunchingWisp {
    /// Target enemy entity (if any)
    pub target: Option<Entity>,
    /// Movement speed
    pub speed: f32,
    /// Time since launch (for despawn if no target hit)
    pub flight_time: Timer,
    /// Direction (used when target is lost)
    pub direction: Vec2,
}

impl LaunchingWisp {
    pub fn new(target: Option<Entity>, direction: Vec2) -> Self {
        Self {
            target,
            speed: EMBER_SWARM_LAUNCH_SPEED,
            flight_time: Timer::from_seconds(EMBER_SWARM_MAX_FLIGHT_TIME, TimerMode::Once),
            direction,
        }
    }
}

/// System that ticks the orbit duration timer on controllers
pub fn ember_swarm_orbit_timer_system(
    time: Res<Time>,
    mut controller_query: Query<&mut EmberSwarmController>,
) {
    for mut controller in controller_query.iter_mut() {
        if !controller.launched {
            controller.orbit_duration.tick(time.delta());
        }
    }
}

/// System that updates wisp positions during orbit phase.
/// Wisps orbit around the player position.
pub fn orbit_ember_wisps_system(
    time: Res<Time>,
    mut wisp_query: Query<(&mut EmberWisp, &mut Transform), Without<LaunchingWisp>>,
    controller_query: Query<(&EmberSwarmController, &Transform), Without<EmberWisp>>,
) {
    for (mut wisp, mut wisp_transform) in wisp_query.iter_mut() {
        // Get controller position (follows player)
        let Ok((controller, controller_transform)) = controller_query.get(wisp.controller) else {
            continue;
        };

        // Don't orbit if already launched
        if controller.launched {
            continue;
        }

        // Update orbit phase
        wisp.orbit_phase += EMBER_SWARM_ORBIT_SPEED * time.delta_secs();
        wisp.orbit_phase %= TAU;

        // Calculate orbit position around controller
        let center = from_xz(controller_transform.translation);
        let offset_x = wisp.orbit_phase.cos() * wisp.orbit_radius;
        let offset_z = wisp.orbit_phase.sin() * wisp.orbit_radius;
        let new_pos = center + Vec2::new(offset_x, offset_z);

        wisp_transform.translation = to_xz(new_pos) + Vec3::Y * EMBER_SWARM_VISUAL_HEIGHT;
    }
}

/// System that launches wisps when orbit duration expires.
/// Finds nearest enemies and assigns targets to wisps.
pub fn launch_ember_wisps_system(
    mut commands: Commands,
    mut controller_query: Query<(Entity, &mut EmberSwarmController, &Transform)>,
    wisp_query: Query<(Entity, &EmberWisp, &Transform), Without<LaunchingWisp>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (controller_entity, mut controller, controller_transform) in controller_query.iter_mut() {
        // Skip if already launched or orbit not complete
        if controller.launched || !controller.orbit_complete() {
            continue;
        }

        // Mark as launched
        controller.launched = true;

        // Get controller position for targeting
        let controller_pos = from_xz(controller_transform.translation);

        // Sort enemies by distance to find targets
        let mut enemies: Vec<(Entity, f32)> = enemy_query
            .iter()
            .map(|(entity, transform)| {
                let pos = from_xz(transform.translation);
                let distance = controller_pos.distance(pos);
                (entity, distance)
            })
            .collect();
        enemies.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Assign targets to wisps (cycle through closest enemies if more wisps than enemies)
        let mut target_index = 0;

        for (wisp_entity, wisp, wisp_transform) in wisp_query.iter() {
            // Only process wisps belonging to this controller
            if wisp.controller != controller_entity {
                continue;
            }

            let wisp_pos = from_xz(wisp_transform.translation);

            // Get target and direction
            let (target, direction) = if enemies.is_empty() {
                // No enemies - fly outward from center
                let outward = (wisp_pos - controller_pos).normalize_or_zero();
                (None, if outward == Vec2::ZERO { Vec2::X } else { outward })
            } else {
                let (target_entity, _) = enemies[target_index % enemies.len()];
                target_index += 1;

                // Get direction to target
                let target_transform = enemy_query.get(target_entity).unwrap().1;
                let target_pos = from_xz(target_transform.translation);
                let direction = (target_pos - wisp_pos).normalize_or_zero();

                (Some(target_entity), direction)
            };

            // Add LaunchingWisp component
            commands.entity(wisp_entity).insert(LaunchingWisp::new(target, direction));
        }
    }
}

/// System that moves launched wisps toward their targets.
pub fn move_launched_wisps_system(
    time: Res<Time>,
    mut wisp_query: Query<(&EmberWisp, &mut LaunchingWisp, &mut Transform), Without<Enemy>>,
    enemy_query: Query<&Transform, With<Enemy>>,
) {
    for (wisp, mut launching, mut wisp_transform) in wisp_query.iter_mut() {
        // Tick flight timer
        launching.flight_time.tick(time.delta());

        let wisp_pos = from_xz(wisp_transform.translation);

        // Get direction to target (or last known direction)
        let direction = if let Some(target) = launching.target {
            if let Ok(target_transform) = enemy_query.get(target) {
                let target_pos = from_xz(target_transform.translation);
                let new_dir = (target_pos - wisp_pos).normalize_or_zero();
                // Update stored direction
                launching.direction = new_dir;
                new_dir
            } else {
                // Target despawned - keep flying in last direction
                launching.target = None;
                launching.direction
            }
        } else {
            launching.direction
        };

        // Move wisp
        let movement = direction * launching.speed * time.delta_secs();
        let new_pos = wisp_pos + movement;
        wisp_transform.translation = to_xz(new_pos) + Vec3::Y * EMBER_SWARM_VISUAL_HEIGHT;

        // Track that we have an EmberWisp component
        let _ = wisp;
    }
}

/// System that checks for wisp collisions with enemies.
pub fn ember_wisp_collision_system(
    mut commands: Commands,
    mut controller_query: Query<&mut EmberSwarmController>,
    wisp_query: Query<(Entity, &EmberWisp, &Transform), With<LaunchingWisp>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (wisp_entity, wisp, wisp_transform) in wisp_query.iter() {
        let wisp_pos = from_xz(wisp_transform.translation);

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = wisp_pos.distance(enemy_pos);

            if distance <= EMBER_SWARM_HIT_RADIUS {
                // Deal damage
                damage_events.write(DamageEvent::new(enemy_entity, wisp.damage));

                // Remove wisp from controller tracking
                if let Ok(mut controller) = controller_query.get_mut(wisp.controller) {
                    controller.remove_wisp(wisp_entity);
                }

                // Despawn wisp
                commands.entity(wisp_entity).despawn();
                break;
            }
        }
    }
}

/// System that despawns wisps that have exceeded max flight time.
pub fn ember_wisp_timeout_system(
    mut commands: Commands,
    mut controller_query: Query<&mut EmberSwarmController>,
    wisp_query: Query<(Entity, &EmberWisp, &LaunchingWisp)>,
) {
    for (wisp_entity, wisp, launching) in wisp_query.iter() {
        if launching.flight_time.is_finished() {
            // Remove wisp from controller tracking
            if let Ok(mut controller) = controller_query.get_mut(wisp.controller) {
                controller.remove_wisp(wisp_entity);
            }

            // Despawn wisp
            commands.entity(wisp_entity).despawn();
        }
    }
}

/// System that cleans up ember swarm controllers when all wisps are gone.
pub fn cleanup_ember_swarm_system(
    mut commands: Commands,
    controller_query: Query<(Entity, &EmberSwarmController)>,
) {
    for (entity, controller) in controller_query.iter() {
        // Only cleanup if launched and all wisps gone
        if controller.launched && controller.all_wisps_gone() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast ember swarm spell - spawns orbiting wisps around the player.
/// `spawn_position` is the player/Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_ember_swarm(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_ember_swarm_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast ember swarm spell with explicit damage.
/// `spawn_position` is the player/Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_ember_swarm_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Determine wisp count (random between min and max)
    let wisp_count = rand::random::<u8>() % (EMBER_SWARM_WISP_COUNT_MAX - EMBER_SWARM_WISP_COUNT_MIN + 1)
        + EMBER_SWARM_WISP_COUNT_MIN;

    // Spawn controller at player position
    let controller_entity = commands.spawn((
        Transform::from_translation(spawn_position),
        EmberSwarmController::new(wisp_count),
    )).id();

    // Spawn wisps at evenly distributed orbit positions
    let center = from_xz(spawn_position);
    let phase_increment = TAU / wisp_count as f32;

    for i in 0..wisp_count {
        let phase = i as f32 * phase_increment;
        let offset_x = phase.cos() * EMBER_SWARM_ORBIT_RADIUS;
        let offset_z = phase.sin() * EMBER_SWARM_ORBIT_RADIUS;
        let wisp_pos = center + Vec2::new(offset_x, offset_z);
        let wisp_pos_3d = to_xz(wisp_pos) + Vec3::Y * EMBER_SWARM_VISUAL_HEIGHT;

        let wisp_entity = if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.fireball.clone()),
                Transform::from_translation(wisp_pos_3d).with_scale(Vec3::splat(0.3)),
                EmberWisp::new(phase, damage, controller_entity),
            )).id()
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(wisp_pos_3d),
                EmberWisp::new(phase, damage, controller_entity),
            )).id()
        };

        // Store wisp entity - we'll update the controller via initialize system
        let _ = wisp_entity;
    }

    // Update controller with wisp entities
    // This is a bit awkward but necessary since we can't mutate during spawn
    // We'll re-query and update in a follow-up system or use a different pattern

    // Actually, let's use a simpler approach: spawn wisps with controller reference
    // and let the controller track them via query in systems
}

/// System to initialize controller's wisp list after spawn.
/// This runs once to populate the wisp list from queries.
pub fn initialize_ember_swarm_wisps_system(
    mut controller_query: Query<(Entity, &mut EmberSwarmController)>,
    wisp_query: Query<(Entity, &EmberWisp)>,
) {
    for (controller_entity, mut controller) in controller_query.iter_mut() {
        // Skip if already initialized
        if !controller.wisps.is_empty() {
            continue;
        }

        // Find all wisps belonging to this controller
        for (wisp_entity, wisp) in wisp_query.iter() {
            if wisp.controller == controller_entity {
                controller.wisps.push(wisp_entity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use crate::spell::SpellType;

    mod ember_swarm_controller_tests {
        use super::*;

        #[test]
        fn test_controller_new() {
            let controller = EmberSwarmController::new(6);

            assert_eq!(controller.wisp_count, 6);
            assert!(!controller.orbit_complete());
            assert!(controller.wisps.is_empty());
            assert!(!controller.launched);
        }

        #[test]
        fn test_controller_orbit_complete() {
            let mut controller = EmberSwarmController::new(5);
            assert!(!controller.orbit_complete());

            controller.orbit_duration.tick(Duration::from_secs_f32(EMBER_SWARM_ORBIT_DURATION + 0.1));
            assert!(controller.orbit_complete());
        }

        #[test]
        fn test_controller_all_wisps_gone() {
            let mut controller = EmberSwarmController::new(3);
            assert!(controller.all_wisps_gone());

            controller.wisps.push(Entity::from_bits(1));
            assert!(!controller.all_wisps_gone());

            controller.wisps.clear();
            assert!(controller.all_wisps_gone());
        }

        #[test]
        fn test_controller_remove_wisp() {
            let mut controller = EmberSwarmController::new(3);
            let e1 = Entity::from_bits(1);
            let e2 = Entity::from_bits(2);
            let e3 = Entity::from_bits(3);

            controller.wisps.push(e1);
            controller.wisps.push(e2);
            controller.wisps.push(e3);

            controller.remove_wisp(e2);

            assert_eq!(controller.wisps.len(), 2);
            assert!(controller.wisps.contains(&e1));
            assert!(!controller.wisps.contains(&e2));
            assert!(controller.wisps.contains(&e3));
        }
    }

    mod ember_wisp_tests {
        use super::*;

        #[test]
        fn test_wisp_new() {
            let controller = Entity::from_bits(1);
            let wisp = EmberWisp::new(1.5, 25.0, controller);

            assert_eq!(wisp.orbit_phase, 1.5);
            assert_eq!(wisp.orbit_radius, EMBER_SWARM_ORBIT_RADIUS);
            assert_eq!(wisp.damage, 25.0);
            assert_eq!(wisp.controller, controller);
        }
    }

    mod launching_wisp_tests {
        use super::*;

        #[test]
        fn test_launching_wisp_new_with_target() {
            let target = Entity::from_bits(42);
            let direction = Vec2::new(1.0, 0.0);
            let launching = LaunchingWisp::new(Some(target), direction);

            assert_eq!(launching.target, Some(target));
            assert_eq!(launching.speed, EMBER_SWARM_LAUNCH_SPEED);
            assert!(!launching.flight_time.is_finished());
            assert_eq!(launching.direction, direction);
        }

        #[test]
        fn test_launching_wisp_new_without_target() {
            let direction = Vec2::new(0.0, 1.0);
            let launching = LaunchingWisp::new(None, direction);

            assert_eq!(launching.target, None);
            assert_eq!(launching.direction, direction);
        }
    }

    mod ember_swarm_spawn_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_ember_swarm_spawns_controller() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball); // Using Fireball as placeholder
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ember_swarm(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 controller
            let mut query = app.world_mut().query::<&EmberSwarmController>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_ember_swarm_spawns_wisps() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ember_swarm(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 5-8 wisps
            let mut query = app.world_mut().query::<&EmberWisp>();
            let count = query.iter(app.world()).count();
            assert!(count >= EMBER_SWARM_WISP_COUNT_MIN as usize, "Expected at least {} wisps, got {}", EMBER_SWARM_WISP_COUNT_MIN, count);
            assert!(count <= EMBER_SWARM_WISP_COUNT_MAX as usize, "Expected at most {} wisps, got {}", EMBER_SWARM_WISP_COUNT_MAX, count);
        }

        #[test]
        fn test_fire_ember_swarm_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ember_swarm(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EmberWisp>();
            for wisp in query.iter(app.world()) {
                assert_eq!(wisp.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_ember_swarm_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ember_swarm_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EmberWisp>();
            for wisp in query.iter(app.world()) {
                assert_eq!(wisp.damage, explicit_damage);
            }
        }

        #[test]
        fn test_wisps_have_distributed_phases() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_ember_swarm(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&EmberWisp>();
            let phases: Vec<f32> = query.iter(app.world()).map(|w| w.orbit_phase).collect();

            // Verify phases are distributed (not all the same)
            if phases.len() > 1 {
                let first = phases[0];
                let has_different = phases.iter().any(|&p| (p - first).abs() > 0.1);
                assert!(has_different, "Wisp phases should be distributed, but all are near {}", first);
            }
        }
    }

    mod orbit_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ember_wisps_orbit_player() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create controller at origin
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                EmberSwarmController::new(1),
            )).id();

            // Create wisp at phase 0 (should be at +X direction)
            let wisp_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(EMBER_SWARM_ORBIT_RADIUS, EMBER_SWARM_VISUAL_HEIGHT, 0.0)),
                EmberWisp::new(0.0, 25.0, controller_entity),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(orbit_ember_wisps_system);

            // Wisp should have moved (phase increased)
            let wisp = app.world().get::<EmberWisp>(wisp_entity).unwrap();
            assert!(wisp.orbit_phase > 0.0, "Orbit phase should have increased");
        }

        #[test]
        fn test_ember_wisps_follow_player() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create controller at non-origin position
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 20.0)),
                EmberSwarmController::new(1),
            )).id();

            // Create wisp
            let wisp_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                EmberWisp::new(0.0, 25.0, controller_entity),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(orbit_ember_wisps_system);

            // Wisp should be near controller position (within orbit radius)
            let wisp_transform = app.world().get::<Transform>(wisp_entity).unwrap();
            let controller_transform = app.world().get::<Transform>(controller_entity).unwrap();

            let wisp_pos = from_xz(wisp_transform.translation);
            let controller_pos = from_xz(controller_transform.translation);
            let distance = wisp_pos.distance(controller_pos);

            assert!(
                (distance - EMBER_SWARM_ORBIT_RADIUS).abs() < 0.5,
                "Wisp should be at orbit radius from controller. Distance: {}, Expected: {}",
                distance,
                EMBER_SWARM_ORBIT_RADIUS
            );
        }
    }

    mod launch_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ember_wisps_launch_after_duration() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create controller with expired orbit duration
            let mut controller = EmberSwarmController::new(1);
            controller.orbit_duration.tick(Duration::from_secs_f32(EMBER_SWARM_ORBIT_DURATION + 0.1));
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            // Create wisp
            let wisp_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                EmberWisp::new(0.0, 25.0, controller_entity),
            )).id();

            // Create enemy for targeting
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(launch_ember_wisps_system);

            // Wisp should have LaunchingWisp component
            assert!(app.world().get::<LaunchingWisp>(wisp_entity).is_some(), "Wisp should have LaunchingWisp component after launch");

            // Controller should be marked as launched
            let controller = app.world().get::<EmberSwarmController>(controller_entity).unwrap();
            assert!(controller.launched, "Controller should be marked as launched");
        }

        #[test]
        fn test_launched_wisps_target_enemies() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create controller with expired orbit duration
            let mut controller = EmberSwarmController::new(1);
            controller.orbit_duration.tick(Duration::from_secs_f32(EMBER_SWARM_ORBIT_DURATION + 0.1));
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            // Create wisp
            let wisp_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                EmberWisp::new(0.0, 25.0, controller_entity),
            )).id();

            // Create enemy
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(launch_ember_wisps_system);

            // Wisp should target the enemy
            let launching = app.world().get::<LaunchingWisp>(wisp_entity).unwrap();
            assert_eq!(launching.target, Some(enemy_entity), "Wisp should target the enemy");
        }

        #[test]
        fn test_launched_wisps_no_enemy_fly_outward() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create controller with expired orbit duration
            let mut controller = EmberSwarmController::new(1);
            controller.orbit_duration.tick(Duration::from_secs_f32(EMBER_SWARM_ORBIT_DURATION + 0.1));
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            // Create wisp at +X position
            let wisp_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                EmberWisp::new(0.0, 25.0, controller_entity),
            )).id();

            // No enemies

            let _ = app.world_mut().run_system_once(launch_ember_wisps_system);

            // Wisp should have no target but valid direction
            let launching = app.world().get::<LaunchingWisp>(wisp_entity).unwrap();
            assert_eq!(launching.target, None, "Wisp should have no target");
            assert!(launching.direction.length() > 0.9, "Wisp should have valid direction");
        }
    }

    mod collision_system_tests {
        use super::*;

        #[test]
        fn test_ember_wisp_deals_damage() {
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
            app.add_systems(Update, (ember_wisp_collision_system, count_damage_events).chain());

            // Create controller
            let mut controller = EmberSwarmController::new(1);
            controller.launched = true;
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            // Create wisp very close to enemy
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
                EmberWisp::new(0.0, 30.0, controller_entity),
                LaunchingWisp::new(None, Vec2::X),
            ));

            // Create enemy at origin
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Should deal damage on collision");
        }

        #[test]
        fn test_ember_wisp_despawns_on_hit() {
            let mut app = App::new();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, ember_wisp_collision_system);

            // Create controller
            let mut controller = EmberSwarmController::new(1);
            controller.launched = true;
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            // Create wisp very close to enemy
            let wisp_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
                EmberWisp::new(0.0, 30.0, controller_entity),
                LaunchingWisp::new(None, Vec2::X),
            )).id();

            // Create enemy
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            // Update controller wisps list
            {
                let mut controller = app.world_mut().get_mut::<EmberSwarmController>(controller_entity).unwrap();
                controller.wisps.push(wisp_entity);
            }

            app.update();

            // Wisp should be despawned
            assert!(app.world().get_entity(wisp_entity).is_err(), "Wisp should be despawned after hit");
        }
    }

    mod cleanup_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_ember_swarm_controller_cleanup() {
            let mut app = App::new();

            // Create controller with no wisps, marked as launched
            let mut controller = EmberSwarmController::new(1);
            controller.launched = true;
            // wisps vec is empty
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_ember_swarm_system);

            // Controller should be despawned
            assert!(app.world().get_entity(controller_entity).is_err(), "Controller should be despawned when all wisps gone");
        }

        #[test]
        fn test_ember_swarm_controller_survives_with_wisps() {
            let mut app = App::new();

            // Create controller with wisps still tracked
            let mut controller = EmberSwarmController::new(1);
            controller.launched = true;
            controller.wisps.push(Entity::from_bits(1));
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_ember_swarm_system);

            // Controller should still exist
            assert!(app.world().get_entity(controller_entity).is_ok(), "Controller should survive while wisps exist");
        }

        #[test]
        fn test_ember_swarm_controller_survives_before_launch() {
            let mut app = App::new();

            // Create controller not yet launched
            let controller = EmberSwarmController::new(1);
            // launched = false by default
            let controller_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                controller,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_ember_swarm_system);

            // Controller should still exist
            assert!(app.world().get_entity(controller_entity).is_ok(), "Controller should survive before launch");
        }
    }

    mod element_color_tests {
        use super::*;

        #[test]
        fn test_ember_swarm_uses_fire_element_color() {
            let color = ember_swarm_color();
            assert_eq!(color, Element::Fire.color());
            assert_eq!(color, Color::srgb_u8(255, 128, 0));
        }
    }
}
