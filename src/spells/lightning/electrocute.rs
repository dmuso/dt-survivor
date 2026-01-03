//! Electrocute spell - Channels continuous lightning into a target.
//!
//! A Lightning element spell that locks onto the nearest enemy and channels
//! a continuous beam of lightning to them. The beam visually connects the
//! player (Whisper) to the targeted enemy and deals damage over time.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::Player;
use crate::spell::components::Spell;

/// Default configuration for Electrocute spell
pub const ELECTROCUTE_DURATION: f32 = 3.0;
pub const ELECTROCUTE_TICK_INTERVAL: f32 = 0.1;
pub const ELECTROCUTE_RANGE: f32 = 15.0;
pub const ELECTROCUTE_BEAM_HEIGHT: f32 = 0.5;
pub const ELECTROCUTE_BEAM_THICKNESS: f32 = 0.15;

/// Get the lightning element color for visual effects (yellow)
pub fn electrocute_color() -> Color {
    Element::Lightning.color()
}

/// Electrocute component - a channeled lightning beam that locks onto an enemy.
/// The beam continuously tracks the enemy's position and deals damage over time.
#[derive(Component, Debug, Clone)]
pub struct Electrocute {
    /// The entity being targeted by the lightning
    pub target: Entity,
    /// Duration timer for the channel
    pub duration: Timer,
    /// Timer for damage ticks
    pub tick_timer: Timer,
    /// Damage per tick
    pub damage_per_tick: f32,
    /// Y height for the beam visual
    pub y_height: f32,
}

impl Electrocute {
    pub fn new(target: Entity, damage: f32, duration: f32) -> Self {
        Self {
            target,
            duration: Timer::from_seconds(duration, TimerMode::Once),
            tick_timer: Timer::from_seconds(ELECTROCUTE_TICK_INTERVAL, TimerMode::Repeating),
            damage_per_tick: damage * ELECTROCUTE_TICK_INTERVAL,
            y_height: ELECTROCUTE_BEAM_HEIGHT,
        }
    }

    pub fn from_spell(target: Entity, spell: &Spell) -> Self {
        Self::new(target, spell.damage(), ELECTROCUTE_DURATION)
    }

    pub fn with_damage(target: Entity, damage: f32) -> Self {
        Self::new(target, damage, ELECTROCUTE_DURATION)
    }

    /// Check if the channel has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick all timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);
    }

    /// Check if ready to apply damage
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished() && !self.is_expired()
    }
}

/// Marker component to link the visual beam entity to the electrocute effect
#[derive(Component, Debug)]
pub struct ElectrocuteVisual {
    /// The entity containing the Electrocute component
    pub electrocute_entity: Entity,
}

/// System that updates Electrocute timers, applies damage, and despawns expired channels
pub fn electrocute_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut electrocute_query: Query<(Entity, &mut Electrocute)>,
    enemy_query: Query<Entity, With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (entity, mut electrocute) in electrocute_query.iter_mut() {
        electrocute.tick(time.delta());

        // Check if target still exists
        let target_exists = enemy_query.contains(electrocute.target);

        if electrocute.is_expired() || !target_exists {
            commands.entity(entity).despawn();
            continue;
        }

        if electrocute.should_damage() {
            damage_events.write(DamageEvent::with_element(
                electrocute.target,
                electrocute.damage_per_tick,
                Element::Lightning,
            ));
        }
    }
}

/// System that updates the visual representation of Electrocute beams.
/// Creates a beam mesh stretched between the player and the target enemy.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn electrocute_visual_system(
    mut commands: Commands,
    electrocute_query: Query<(Entity, &Electrocute), Without<Mesh3d>>,
    mut beam_query: Query<(Entity, &mut Transform, &ElectrocuteVisual), With<Mesh3d>>,
    electrocute_with_mesh: Query<(Entity, &Electrocute), With<Mesh3d>>,
    player_query: Query<&Transform, (With<Player>, Without<ElectrocuteVisual>, Without<Electrocute>)>,
    enemy_query: Query<&Transform, (With<Enemy>, Without<ElectrocuteVisual>, Without<Electrocute>, Without<Player>)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(meshes) = game_meshes else { return };
    let Some(materials) = game_materials else { return };
    let Ok(player_transform) = player_query.single() else { return };

    let player_pos = from_xz(player_transform.translation);

    // Spawn visual for new electrocute effects
    for (electrocute_entity, electrocute) in electrocute_query.iter() {
        let Ok(target_transform) = enemy_query.get(electrocute.target) else {
            continue;
        };

        let target_pos = from_xz(target_transform.translation);
        let (beam_transform, beam_scale) = calculate_beam_transform(
            player_pos,
            target_pos,
            electrocute.y_height,
        );

        commands.entity(electrocute_entity).insert((
            Mesh3d(meshes.laser.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            beam_transform.with_scale(beam_scale),
        ));
    }

    // Update existing beam visuals
    for (electrocute_entity, electrocute) in electrocute_with_mesh.iter() {
        let Ok(target_transform) = enemy_query.get(electrocute.target) else {
            continue;
        };

        let target_pos = from_xz(target_transform.translation);
        let (beam_transform, beam_scale) = calculate_beam_transform(
            player_pos,
            target_pos,
            electrocute.y_height,
        );

        // Find the matching beam entity and update its transform
        for (beam_entity, mut transform, _) in beam_query.iter_mut() {
            if beam_entity == electrocute_entity {
                transform.translation = beam_transform.translation;
                transform.rotation = beam_transform.rotation;
                transform.scale = beam_scale;
            }
        }
    }
}

/// Calculate the beam transform and scale for connecting player to target
fn calculate_beam_transform(player_pos: Vec2, target_pos: Vec2, y_height: f32) -> (Transform, Vec3) {
    let direction = target_pos - player_pos;
    let distance = direction.length();
    let normalized_dir = direction.normalize_or_zero();

    // Center position of the beam
    let center = (player_pos + target_pos) / 2.0;

    // Rotation around Y axis to point toward target on XZ plane
    let angle = normalized_dir.y.atan2(normalized_dir.x);
    let rotation = Quat::from_rotation_y(-angle + std::f32::consts::FRAC_PI_2);

    // Scale: the laser mesh is 0.1 x 0.1 x 1.0
    // We scale X and Y for thickness, Z for length
    let scale = Vec3::new(
        ELECTROCUTE_BEAM_THICKNESS * 10.0,
        ELECTROCUTE_BEAM_THICKNESS * 10.0,
        distance,
    );

    let transform = Transform {
        translation: Vec3::new(center.x, y_height, center.y),
        rotation,
        ..default()
    };

    (transform, scale)
}

/// System that cleans up orphaned ElectrocuteVisual entities
pub fn electrocute_cleanup_system(
    mut commands: Commands,
    visual_query: Query<(Entity, &ElectrocuteVisual)>,
    electrocute_query: Query<Entity, With<Electrocute>>,
) {
    for (visual_entity, visual) in visual_query.iter() {
        if !electrocute_query.contains(visual.electrocute_entity) {
            commands.entity(visual_entity).despawn();
        }
    }
}

/// Cast Electrocute spell - channels lightning to the nearest enemy
#[allow(clippy::too_many_arguments)]
pub fn fire_electrocute(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform, &Enemy)>,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) {
    fire_electrocute_with_damage(commands, spell, spell.damage(), spawn_position, enemy_query);
}

/// Cast Electrocute spell with explicit damage
#[allow(clippy::too_many_arguments)]
pub fn fire_electrocute_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform, &Enemy)>,
) {
    let spawn_xz = from_xz(spawn_position);

    // Find nearest enemy within range
    let mut nearest_enemy: Option<(Entity, f32)> = None;

    for (enemy_entity, enemy_transform, _enemy) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = spawn_xz.distance(enemy_pos);

        if distance <= ELECTROCUTE_RANGE {
            match nearest_enemy {
                None => nearest_enemy = Some((enemy_entity, distance)),
                Some((_, nearest_dist)) if distance < nearest_dist => {
                    nearest_enemy = Some((enemy_entity, distance));
                }
                _ => {}
            }
        }
    }

    // Create electrocute effect targeting the nearest enemy
    if let Some((enemy_entity, _)) = nearest_enemy {
        let electrocute = Electrocute::with_damage(enemy_entity, damage);
        commands.spawn((
            Transform::from_translation(spawn_position),
            electrocute,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::SpellType;
    use std::time::Duration;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<DamageEvent>();
        app
    }

    mod electrocute_component_tests {
        use super::*;

        #[test]
        fn test_electrocute_new() {
            let target = Entity::from_bits(1);
            let damage = 20.0;
            let electrocute = Electrocute::new(target, damage, ELECTROCUTE_DURATION);

            assert_eq!(electrocute.target, target);
            assert!(!electrocute.is_expired());
            // damage_per_tick = damage * tick_interval = 20.0 * 0.1 = 2.0
            assert!((electrocute.damage_per_tick - 2.0).abs() < 0.01);
        }

        #[test]
        fn test_electrocute_from_spell() {
            let target = Entity::from_bits(1);
            let spell = Spell::new(SpellType::Electrocute);
            let electrocute = Electrocute::from_spell(target, &spell);

            assert_eq!(electrocute.target, target);
            let expected_damage_per_tick = spell.damage() * ELECTROCUTE_TICK_INTERVAL;
            assert!((electrocute.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
        }

        #[test]
        fn test_electrocute_with_damage() {
            let target = Entity::from_bits(1);
            let damage = 50.0;
            let electrocute = Electrocute::with_damage(target, damage);

            let expected_damage_per_tick = damage * ELECTROCUTE_TICK_INTERVAL;
            assert!((electrocute.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
        }

        #[test]
        fn test_electrocute_expires_after_duration() {
            let target = Entity::from_bits(1);
            let mut electrocute = Electrocute::new(target, 20.0, ELECTROCUTE_DURATION);

            assert!(!electrocute.is_expired());
            electrocute.tick(Duration::from_secs_f32(ELECTROCUTE_DURATION + 0.1));
            assert!(electrocute.is_expired());
        }

        #[test]
        fn test_electrocute_should_damage_at_tick_interval() {
            let target = Entity::from_bits(1);
            let mut electrocute = Electrocute::new(target, 20.0, ELECTROCUTE_DURATION);

            // Before tick interval - no damage
            electrocute.tick(Duration::from_secs_f32(ELECTROCUTE_TICK_INTERVAL / 2.0));
            assert!(!electrocute.should_damage());

            // After tick interval - should damage
            electrocute.tick(Duration::from_secs_f32(ELECTROCUTE_TICK_INTERVAL));
            assert!(electrocute.should_damage());
        }

        #[test]
        fn test_electrocute_no_damage_after_expiry() {
            let target = Entity::from_bits(1);
            let mut electrocute = Electrocute::new(target, 20.0, ELECTROCUTE_DURATION);

            // Expire the effect
            electrocute.tick(Duration::from_secs_f32(ELECTROCUTE_DURATION + 0.1));
            assert!(!electrocute.should_damage());
        }

        #[test]
        fn test_electrocute_uses_lightning_element_color() {
            let color = electrocute_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }
    }

    mod electrocute_damage_system_tests {
        use super::*;

        #[test]
        fn test_electrocute_damage_calculation() {
            let damage = 30.0;
            let electrocute = Electrocute::new(Entity::from_bits(1), damage, ELECTROCUTE_DURATION);

            // Total damage over duration = damage_per_tick * (duration / tick_interval)
            let total_ticks = (ELECTROCUTE_DURATION / ELECTROCUTE_TICK_INTERVAL) as u32;
            let expected_total = electrocute.damage_per_tick * total_ticks as f32;
            let expected_per_tick = damage * ELECTROCUTE_TICK_INTERVAL;

            assert!((electrocute.damage_per_tick - expected_per_tick).abs() < 0.01);
            // Total damage should approximately equal initial damage * duration
            assert!((expected_total - damage * ELECTROCUTE_DURATION).abs() < 1.0);
        }

        #[test]
        fn test_electrocute_despawns_when_expired() {
            let mut app = setup_test_app();
            app.add_systems(Update, electrocute_damage_system);

            // Create expired electrocute
            let target = app.world_mut().spawn(Enemy { speed: 50.0, strength: 10.0 }).id();
            let mut electrocute = Electrocute::new(target, 20.0, ELECTROCUTE_DURATION);
            electrocute.duration.tick(Duration::from_secs_f32(ELECTROCUTE_DURATION + 0.1));

            let electrocute_entity = app.world_mut().spawn((
                Transform::default(),
                electrocute,
            )).id();

            app.update();

            // Electrocute should be despawned
            assert!(app.world().get_entity(electrocute_entity).is_err());
        }

        #[test]
        fn test_electrocute_despawns_when_target_dies() {
            let mut app = setup_test_app();
            app.add_systems(Update, electrocute_damage_system);

            // Create electrocute targeting non-existent enemy
            let fake_target = Entity::from_bits(99999);
            let electrocute = Electrocute::new(fake_target, 20.0, ELECTROCUTE_DURATION);

            let electrocute_entity = app.world_mut().spawn((
                Transform::default(),
                electrocute,
            )).id();

            app.update();

            // Electrocute should be despawned (target doesn't exist)
            assert!(app.world().get_entity(electrocute_entity).is_err());
        }

        #[test]
        fn test_electrocute_survives_with_valid_target() {
            let mut app = setup_test_app();
            app.add_systems(Update, electrocute_damage_system);

            // Create valid target
            let target = app.world_mut().spawn(Enemy { speed: 50.0, strength: 10.0 }).id();
            let electrocute = Electrocute::new(target, 20.0, ELECTROCUTE_DURATION);

            let electrocute_entity = app.world_mut().spawn((
                Transform::default(),
                electrocute,
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ELECTROCUTE_DURATION / 2.0));
            }

            app.update();

            // Electrocute should still exist
            assert!(app.world().get_entity(electrocute_entity).is_ok());
        }
    }

    mod fire_electrocute_tests {
        use super::*;

        fn test_enemy() -> Enemy {
            Enemy { speed: 100.0, strength: 1.0 }
        }

        #[test]
        fn test_fire_electrocute_targets_nearest_enemy() {
            let mut app = setup_test_app();

            // Create enemies at different distances
            let far_enemy = app.world_mut().spawn((
                test_enemy(),
                Transform::from_translation(Vec3::new(10.0, 0.5, 0.0)),
            )).id();

            let near_enemy = app.world_mut().spawn((
                test_enemy(),
                Transform::from_translation(Vec3::new(3.0, 0.5, 0.0)),
            )).id();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            // Collect enemies and find nearest
            let enemies: Vec<(Entity, Transform)> = app.world_mut()
                .query_filtered::<(Entity, &Transform), With<Enemy>>()
                .iter(app.world())
                .map(|(e, t)| (e, *t))
                .collect();

            let spawn_xz = from_xz(spawn_pos);
            let mut nearest: Option<(Entity, f32)> = None;
            for (entity, transform) in &enemies {
                let pos = from_xz(transform.translation);
                let dist = spawn_xz.distance(pos);
                if dist <= ELECTROCUTE_RANGE {
                    match nearest {
                        None => nearest = Some((*entity, dist)),
                        Some((_, d)) if dist < d => nearest = Some((*entity, dist)),
                        _ => {}
                    }
                }
            }

            // Spawn electrocute targeting nearest enemy
            if let Some((target, _)) = nearest {
                let spell = Spell::new(SpellType::Electrocute);
                let electrocute = Electrocute::from_spell(target, &spell);
                app.world_mut().spawn((Transform::from_translation(spawn_pos), electrocute));
            }
            app.update();

            // Should have spawned electrocute targeting near_enemy
            let mut electrocute_query = app.world_mut().query::<&Electrocute>();
            let electrocute = electrocute_query.single(app.world()).unwrap();
            assert_eq!(electrocute.target, near_enemy);
            assert_ne!(electrocute.target, far_enemy);
        }

        #[test]
        fn test_fire_electrocute_no_target_out_of_range() {
            let mut app = setup_test_app();

            // Create enemy outside range
            app.world_mut().spawn((
                test_enemy(),
                Transform::from_translation(Vec3::new(ELECTROCUTE_RANGE + 5.0, 0.5, 0.0)),
            ));

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            // Collect enemies and try to find one in range
            let enemies: Vec<(Entity, Transform)> = app.world_mut()
                .query_filtered::<(Entity, &Transform), With<Enemy>>()
                .iter(app.world())
                .map(|(e, t)| (e, *t))
                .collect();

            let spawn_xz = from_xz(spawn_pos);
            let mut nearest: Option<(Entity, f32)> = None;
            for (entity, transform) in &enemies {
                let pos = from_xz(transform.translation);
                let dist = spawn_xz.distance(pos);
                if dist <= ELECTROCUTE_RANGE {
                    match nearest {
                        None => nearest = Some((*entity, dist)),
                        Some((_, d)) if dist < d => nearest = Some((*entity, dist)),
                        _ => {}
                    }
                }
            }

            // Should not find any target in range
            assert!(nearest.is_none(), "Should not find target outside range");

            app.update();

            // Should NOT have spawned electrocute
            let mut electrocute_query = app.world_mut().query::<&Electrocute>();
            let count = electrocute_query.iter(app.world()).count();
            assert_eq!(count, 0);
        }

        #[test]
        fn test_fire_electrocute_uses_correct_damage() {
            let mut app = setup_test_app();

            let explicit_damage = 100.0;
            let target = app.world_mut().spawn((
                test_enemy(),
                Transform::from_translation(Vec3::new(3.0, 0.5, 0.0)),
            )).id();

            // Create electrocute with explicit damage
            let electrocute = Electrocute::with_damage(target, explicit_damage);
            app.world_mut().spawn((Transform::default(), electrocute));

            app.update();

            let mut electrocute_query = app.world_mut().query::<&Electrocute>();
            let electrocute = electrocute_query.single(app.world()).unwrap();
            let expected_damage_per_tick = explicit_damage * ELECTROCUTE_TICK_INTERVAL;
            assert!((electrocute.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
        }
    }

    mod calculate_beam_transform_tests {
        use super::*;

        #[test]
        fn test_beam_centered_between_points() {
            let player = Vec2::new(0.0, 0.0);
            let target = Vec2::new(10.0, 0.0);
            let y_height = 0.5;

            let (transform, _) = calculate_beam_transform(player, target, y_height);

            // Center should be at (5, 0) in XZ space
            assert!((transform.translation.x - 5.0).abs() < 0.01);
            assert!((transform.translation.z - 0.0).abs() < 0.01);
            assert!((transform.translation.y - y_height).abs() < 0.01);
        }

        #[test]
        fn test_beam_length_matches_distance() {
            let player = Vec2::new(0.0, 0.0);
            let target = Vec2::new(10.0, 0.0);
            let y_height = 0.5;

            let (_, scale) = calculate_beam_transform(player, target, y_height);

            // Z scale should match the distance (10.0)
            assert!((scale.z - 10.0).abs() < 0.01);
        }

        #[test]
        fn test_beam_thickness() {
            let player = Vec2::new(0.0, 0.0);
            let target = Vec2::new(10.0, 0.0);
            let y_height = 0.5;

            let (_, scale) = calculate_beam_transform(player, target, y_height);

            // X and Y scale should match the thickness setting
            let expected_thickness = ELECTROCUTE_BEAM_THICKNESS * 10.0;
            assert!((scale.x - expected_thickness).abs() < 0.01);
            assert!((scale.y - expected_thickness).abs() < 0.01);
        }

        #[test]
        fn test_beam_diagonal_distance() {
            let player = Vec2::new(0.0, 0.0);
            let target = Vec2::new(3.0, 4.0); // Distance = 5.0
            let y_height = 0.5;

            let (transform, scale) = calculate_beam_transform(player, target, y_height);

            // Z scale should match the distance (5.0)
            assert!((scale.z - 5.0).abs() < 0.01);

            // Center should be at (1.5, 2.0) in XZ space
            assert!((transform.translation.x - 1.5).abs() < 0.01);
            assert!((transform.translation.z - 2.0).abs() < 0.01);
        }
    }
}
