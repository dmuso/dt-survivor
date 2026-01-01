use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default height of radiant beam center above ground (used for tests)
pub const RADIANT_BEAM_DEFAULT_Y_HEIGHT: f32 = 0.5;

/// Default beam length in world units
pub const RADIANT_BEAM_LENGTH: f32 = 800.0;

/// Collision radius on either side of the beam line
pub const RADIANT_BEAM_COLLISION_RADIUS: f32 = 1.0;

/// Get the light element color for visual effects (white/gold)
pub fn radiant_beam_color() -> Color {
    Element::Light.color()
}

/// Marker component for radiant beam spell entities.
/// A continuous beam of focused light emanating from the player.
#[derive(Component, Debug, Clone)]
pub struct RadiantBeam {
    /// Start position on XZ plane
    pub start_pos: Vec2,
    /// End position on XZ plane
    pub end_pos: Vec2,
    /// Direction of the beam on XZ plane
    pub direction: Vec2,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Maximum lifetime in seconds
    pub max_lifetime: f32,
    /// Damage dealt per frame while beam intersects enemy
    pub damage: f32,
    /// Y height in 3D world (fires from Whisper's height)
    pub y_height: f32,
}

impl RadiantBeam {
    /// Creates a new RadiantBeam at the default height (0.5).
    /// Prefer `with_height` for proper 3D positioning from Whisper.
    pub fn new(start_pos: Vec2, direction: Vec2, damage: f32) -> Self {
        Self::with_height(start_pos, direction, damage, RADIANT_BEAM_DEFAULT_Y_HEIGHT)
    }

    /// Creates a new RadiantBeam at the specified Y height.
    pub fn with_height(start_pos: Vec2, direction: Vec2, damage: f32, y_height: f32) -> Self {
        let end_pos = start_pos + direction * RADIANT_BEAM_LENGTH;
        Self {
            start_pos,
            end_pos,
            direction,
            lifetime: Timer::from_seconds(0.5, TimerMode::Once), // 0.5 second duration
            max_lifetime: 0.5,
            damage,
            y_height,
        }
    }

    /// Creates a RadiantBeam from a Spell component.
    pub fn from_spell(start_pos: Vec2, direction: Vec2, spell: &Spell, y_height: f32) -> Self {
        Self::with_height(start_pos, direction, spell.damage(), y_height)
    }

    /// Calculate the beam thickness based on lifetime progress.
    /// Animates: thin -> medium -> thick -> dissipate
    pub fn get_thickness(&self) -> f32 {
        let progress = self.lifetime.elapsed_secs() / self.max_lifetime;
        if progress < 0.3 {
            // First 30%: thin to medium (2 to 8 pixels)
            2.0 + (progress / 0.3) * 6.0
        } else if progress < 0.7 {
            // Next 40%: medium to thick (8 to 15 pixels)
            8.0 + ((progress - 0.3) / 0.4) * 7.0
        } else {
            // Last 30%: dissipate (15 to 0 pixels)
            let dissipate_progress = (progress - 0.7) / 0.3;
            15.0 * (1.0 - dissipate_progress)
        }
    }

    /// Check if the beam is still active (not expired)
    pub fn is_active(&self) -> bool {
        self.lifetime.elapsed_secs() < self.max_lifetime
    }
}

/// System to update radiant beam lifetime and despawn expired beams.
pub fn update_radiant_beams(
    mut commands: Commands,
    time: Res<Time>,
    mut beam_query: Query<(Entity, &mut RadiantBeam)>,
) {
    for (entity, mut beam) in beam_query.iter_mut() {
        beam.lifetime.tick(time.delta());

        if !beam.is_active() {
            commands.entity(entity).despawn();
        }
    }
}

/// Radiant beam collision system that sends DamageEvent to enemies in the beam path.
/// Uses XZ plane for collision detection in 3D space (Y axis is height).
pub fn radiant_beam_collision_system(
    beam_query: Query<&RadiantBeam>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for beam in beam_query.iter() {
        if !beam.is_active() {
            continue;
        }

        // Beam start_pos and end_pos are Vec2 representing XZ coordinates
        let beam_start = beam.start_pos;
        let beam_end = beam.end_pos;
        let beam_length = (beam_end - beam_start).length();

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            // Extract XZ coordinates from enemy 3D position
            let enemy_pos = from_xz(enemy_transform.translation);

            // Check if enemy is within the beam bounds
            let to_enemy = enemy_pos - beam_start;
            let projection_length = to_enemy.dot(beam.direction);

            // Check if enemy is within the beam segment
            if projection_length >= 0.0 && projection_length <= beam_length {
                let projection_point = beam_start + beam.direction * projection_length;
                let distance_to_line = (enemy_pos - projection_point).length();

                // If enemy is close enough to the beam
                if distance_to_line < RADIANT_BEAM_COLLISION_RADIUS {
                    // Send damage event to enemy
                    damage_events.write(DamageEvent::new(enemy_entity, beam.damage));
                }
            }
        }
    }
}

/// Renders radiant beams as 3D elongated cubes with white/gold coloring.
/// Uses the shared GameMeshes and GameMaterials resources.
pub fn render_radiant_beams(
    mut commands: Commands,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
    beam_query: Query<(Entity, &RadiantBeam), Changed<RadiantBeam>>,
) {
    // Skip if resources not available (e.g., in tests)
    let Some(game_meshes) = game_meshes else { return; };
    let Some(game_materials) = game_materials else { return; };

    for (entity, beam) in beam_query.iter() {
        let thickness = beam.get_thickness();
        let length = (beam.end_pos - beam.start_pos).length();

        // Center position on XZ plane (start_pos and end_pos are Vec2 for XZ)
        let center_xz = (beam.start_pos + beam.end_pos) / 2.0;

        // Rotation around Y axis to point toward target on XZ plane
        let angle = beam.direction.y.atan2(beam.direction.x);
        let rotation = Quat::from_rotation_y(-angle + std::f32::consts::FRAC_PI_2);

        // Scale: base mesh is 0.1 x 0.1 x 1.0
        // X and Y scale for thickness, Z scale for length
        let scale = Vec3::new(thickness / 10.0, thickness / 10.0, length);

        // Update or create the visual representation as 3D mesh
        // Use beam's stored y_height (from Whisper's position)
        commands.entity(entity).insert((
            Mesh3d(game_meshes.laser.clone()),
            MeshMaterial3d(game_materials.radiant_beam.clone()),
            Transform {
                translation: Vec3::new(center_xz.x, beam.y_height, center_xz.y),
                rotation,
                scale,
            },
        ));
    }
}

/// Cast radiant beam spell - spawns a beam with light element visuals.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is target on XZ plane.
#[allow(clippy::too_many_arguments)]
pub fn fire_radiant_beam(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let direction = (target_pos - spawn_xz).normalize();

    let beam = RadiantBeam::from_spell(spawn_xz, direction, spell, spawn_position.y);

    // Spawn beam at Whisper's full 3D position
    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.laser.clone()),
            MeshMaterial3d(materials.radiant_beam.clone()),
            Transform::from_translation(spawn_position),
            beam,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            beam,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod radiant_beam_component_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_radiant_beam_creation() {
            let start_pos = Vec2::new(0.0, 0.0);
            let direction = Vec2::new(1.0, 0.0); // Right
            let damage = 22.0; // RadiantBeam base damage

            let beam = RadiantBeam::new(start_pos, direction, damage);

            assert_eq!(beam.start_pos, start_pos);
            assert_eq!(beam.end_pos, Vec2::new(RADIANT_BEAM_LENGTH, 0.0));
            assert_eq!(beam.direction, direction);
            assert_eq!(beam.damage, damage);
            assert_eq!(beam.max_lifetime, 0.5);
            assert!(beam.is_active());
        }

        #[test]
        fn test_radiant_beam_with_height() {
            let beam = RadiantBeam::with_height(Vec2::ZERO, Vec2::X, 22.0, 2.5);
            assert_eq!(beam.y_height, 2.5, "Beam should store the provided Y height");
        }

        #[test]
        fn test_radiant_beam_new_uses_default_height() {
            let beam = RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0);
            assert_eq!(beam.y_height, RADIANT_BEAM_DEFAULT_Y_HEIGHT, "RadiantBeam::new should use default height");
        }

        #[test]
        fn test_radiant_beam_from_spell() {
            let spell = Spell::new(SpellType::RadiantBeam);
            let direction = Vec2::new(0.0, 1.0);
            let beam = RadiantBeam::from_spell(Vec2::ZERO, direction, &spell, 1.0);

            assert_eq!(beam.direction, direction);
            assert_eq!(beam.damage, spell.damage());
            assert_eq!(beam.y_height, 1.0);
        }

        #[test]
        fn test_radiant_beam_thickness_animation() {
            let beam = RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0);

            // Test initial thickness (thin)
            assert_eq!(beam.get_thickness(), 2.0);

            // Test that thickness logic works
            assert!(beam.get_thickness() >= 2.0);
        }

        #[test]
        fn test_radiant_beam_is_active() {
            let beam = RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0);
            assert!(beam.is_active(), "New beam should be active");
        }

        #[test]
        fn test_radiant_beam_uses_light_element_color() {
            let color = radiant_beam_color();
            assert_eq!(color, Element::Light.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 255)); // White
        }
    }

    mod radiant_beam_update_system_tests {
        use super::*;

        #[test]
        fn test_radiant_beam_update() {
            let mut app = App::new();
            app.add_systems(Update, update_radiant_beams);
            app.init_resource::<Time>();

            // Create beam
            let beam_entity = app.world_mut().spawn(RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0)).id();

            // Initially active
            {
                let beam = app.world().get::<RadiantBeam>(beam_entity).unwrap();
                assert!(beam.is_active());
            }

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }
            app.update();

            // Beam should be despawned
            assert!(app.world().get_entity(beam_entity).is_err());
        }

        #[test]
        fn test_radiant_beam_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_radiant_beams);
            app.init_resource::<Time>();

            let beam_entity = app.world_mut().spawn(RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0)).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.25));
            }
            app.update();

            // Beam should still exist
            assert!(app.world().entities().contains(beam_entity));
        }
    }

    mod radiant_beam_collision_tests {
        use super::*;
        use crate::combat::{CheckDeath, Health};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_radiant_beam_collision_sends_damage_event() {
            let mut app = App::new();

            // Counter for damage events
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
            app.add_systems(Update, (radiant_beam_collision_system, count_damage_events).chain());

            // Create beam along X axis
            app.world_mut().spawn(RadiantBeam {
                start_pos: Vec2::ZERO,
                end_pos: Vec2::new(RADIANT_BEAM_LENGTH, 0.0),
                direction: Vec2::X,
                lifetime: Timer::from_seconds(0.25, TimerMode::Once),
                max_lifetime: 0.5,
                damage: 22.0,
                y_height: RADIANT_BEAM_DEFAULT_Y_HEIGHT,
            });

            // Create enemy on the beam line with Health component
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
                Health::new(50.0),
                CheckDeath,
            )).id();

            app.update();

            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));

            // DamageEvent should have been sent
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_radiant_beam_collision_uses_xz_plane_ignores_y() {
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
            app.add_systems(Update, (radiant_beam_collision_system, count_damage_events).chain());

            // Create beam on XZ plane
            app.world_mut().spawn(RadiantBeam {
                start_pos: Vec2::ZERO,
                end_pos: Vec2::new(RADIANT_BEAM_LENGTH, 0.0),
                direction: Vec2::X,
                lifetime: Timer::from_seconds(0.25, TimerMode::Once),
                max_lifetime: 0.5,
                damage: 22.0,
                y_height: RADIANT_BEAM_DEFAULT_Y_HEIGHT,
            });

            // Create enemy on beam line at X=100 but at very high Y - should still collide
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 100.0, 0.0)),
                Health::new(50.0),
                CheckDeath,
            )).id();

            app.update();

            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));

            // DamageEvent should have been sent (Y is ignored for collision)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Beam collision should use XZ plane, ignoring Y");
        }

        #[test]
        fn test_radiant_beam_collision_on_z_axis() {
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
            app.add_systems(Update, (radiant_beam_collision_system, count_damage_events).chain());

            // Create beam along Z axis (Vec2.y maps to Vec3.z)
            app.world_mut().spawn(RadiantBeam {
                start_pos: Vec2::ZERO,
                end_pos: Vec2::new(0.0, 100.0),
                direction: Vec2::Y,
                lifetime: Timer::from_seconds(0.25, TimerMode::Once),
                max_lifetime: 0.5,
                damage: 22.0,
                y_height: RADIANT_BEAM_DEFAULT_Y_HEIGHT,
            });

            // Create enemy on Z axis at Z=50
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 50.0)),
                Health::new(50.0),
                CheckDeath,
            )).id();

            app.update();

            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));

            // DamageEvent should have been sent
            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Beam should hit enemy on Z axis");
        }

        #[test]
        fn test_radiant_beam_collision_uses_fixed_distance_threshold() {
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
            app.add_systems(Update, (radiant_beam_collision_system, count_damage_events).chain());

            // Create beam along X axis
            app.world_mut().spawn(RadiantBeam {
                start_pos: Vec2::ZERO,
                end_pos: Vec2::new(RADIANT_BEAM_LENGTH, 0.0),
                direction: Vec2::X,
                lifetime: Timer::from_seconds(0.25, TimerMode::Once),
                max_lifetime: 0.5,
                damage: 22.0,
                y_height: RADIANT_BEAM_DEFAULT_Y_HEIGHT,
            });

            // Enemy at 0.9 units from beam line (within threshold) - should be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.9)),
                Health::new(50.0),
                CheckDeath,
            ));

            // Enemy at 1.1 units from beam line (outside threshold) - should NOT be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(200.0, 0.375, 1.1)),
                Health::new(50.0),
                CheckDeath,
            ));

            app.update();

            // Only the close enemy should be hit
            assert_eq!(
                counter.0.load(Ordering::SeqCst),
                1,
                "Only enemy within {} unit of beam line should be hit",
                RADIANT_BEAM_COLLISION_RADIUS
            );
        }
    }

    mod fire_radiant_beam_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_radiant_beam_spawns_beam() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::RadiantBeam);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiant_beam(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 beam
            let mut query = app.world_mut().query::<&RadiantBeam>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_radiant_beam_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::RadiantBeam);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_radiant_beam(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadiantBeam>();
            for beam in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(beam.direction.x > 0.9, "Beam should point toward target");
            }
        }

        #[test]
        fn test_fire_radiant_beam_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::RadiantBeam);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiant_beam(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadiantBeam>();
            for beam in query.iter(app.world()) {
                assert_eq!(beam.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_radiant_beam_uses_spawn_y_height() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::RadiantBeam);
            let spawn_pos = Vec3::new(0.0, 2.5, 0.0); // Custom Y height
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_radiant_beam(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&RadiantBeam>();
            for beam in query.iter(app.world()) {
                assert_eq!(beam.y_height, 2.5, "Beam should use spawn position Y height");
            }
        }
    }

    mod render_radiant_beams_tests {
        use super::*;
        use bevy::asset::Assets;
        use bevy::pbr::StandardMaterial;

        #[test]
        fn test_radiant_beam_does_not_have_sprite_after_render() {
            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::time::TimePlugin::default(),
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();

            // Setup game resources
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let game_meshes = crate::game::resources::GameMeshes::new(&mut meshes);
            app.world_mut().insert_resource(game_meshes);

            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            let game_materials = crate::game::resources::GameMaterials::new(&mut materials);
            app.world_mut().insert_resource(game_materials);

            app.add_systems(Update, render_radiant_beams);

            // Create beam
            let beam_entity = app.world_mut().spawn(RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0)).id();

            app.update();

            // Beam should NOT have Sprite component in 3D mode
            assert!(
                app.world().get::<Sprite>(beam_entity).is_none(),
                "Beam should NOT have Sprite component in 3D mode"
            );
        }

        #[test]
        fn test_radiant_beam_has_mesh3d_after_render() {
            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::time::TimePlugin::default(),
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();

            // Setup game resources
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let game_meshes = crate::game::resources::GameMeshes::new(&mut meshes);
            app.world_mut().insert_resource(game_meshes);

            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            let game_materials = crate::game::resources::GameMaterials::new(&mut materials);
            app.world_mut().insert_resource(game_materials);

            app.add_systems(Update, render_radiant_beams);

            // Create beam
            let beam_entity = app.world_mut().spawn(RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0)).id();

            app.update();

            // Beam should have Mesh3d component
            assert!(
                app.world().get::<Mesh3d>(beam_entity).is_some(),
                "Beam should have Mesh3d component"
            );
        }

        #[test]
        fn test_radiant_beam_has_mesh_material_3d_after_render() {
            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::time::TimePlugin::default(),
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();

            // Setup game resources
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let game_meshes = crate::game::resources::GameMeshes::new(&mut meshes);
            app.world_mut().insert_resource(game_meshes);

            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            let game_materials = crate::game::resources::GameMaterials::new(&mut materials);
            app.world_mut().insert_resource(game_materials);

            app.add_systems(Update, render_radiant_beams);

            // Create beam
            let beam_entity = app.world_mut().spawn(RadiantBeam::new(Vec2::ZERO, Vec2::X, 22.0)).id();

            app.update();

            // Beam should have MeshMaterial3d component
            assert!(
                app.world().get::<MeshMaterial3d<StandardMaterial>>(beam_entity).is_some(),
                "Beam should have MeshMaterial3d component"
            );
        }

        #[test]
        fn test_radiant_beam_renders_at_correct_y_height() {
            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::time::TimePlugin::default(),
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();

            // Setup game resources
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let game_meshes = crate::game::resources::GameMeshes::new(&mut meshes);
            app.world_mut().insert_resource(game_meshes);

            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            let game_materials = crate::game::resources::GameMaterials::new(&mut materials);
            app.world_mut().insert_resource(game_materials);

            app.add_systems(Update, render_radiant_beams);

            // Create beam at some XZ position
            let beam_entity = app.world_mut().spawn(RadiantBeam::new(
                Vec2::new(10.0, 20.0),
                Vec2::X,
                22.0,
            )).id();

            app.update();

            let transform = app.world().get::<Transform>(beam_entity).unwrap();
            let beam = app.world().get::<RadiantBeam>(beam_entity).unwrap();

            // Y should be at the beam's stored y_height
            assert!(
                (transform.translation.y - beam.y_height).abs() < 0.001,
                "Beam Y position should be {}, got {}",
                beam.y_height,
                transform.translation.y
            );
        }

        #[test]
        fn test_radiant_beam_renders_at_correct_xz_position() {
            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::time::TimePlugin::default(),
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();

            // Setup game resources
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let game_meshes = crate::game::resources::GameMeshes::new(&mut meshes);
            app.world_mut().insert_resource(game_meshes);

            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            let game_materials = crate::game::resources::GameMaterials::new(&mut materials);
            app.world_mut().insert_resource(game_materials);

            app.add_systems(Update, render_radiant_beams);

            // Create beam from (0, 0) to (100, 0) on XZ plane
            let start = Vec2::ZERO;
            let end = Vec2::new(100.0, 0.0);
            let beam_entity = app.world_mut().spawn(RadiantBeam {
                start_pos: start,
                end_pos: end,
                direction: Vec2::X,
                lifetime: Timer::from_seconds(0.5, TimerMode::Once),
                max_lifetime: 0.5,
                damage: 22.0,
                y_height: RADIANT_BEAM_DEFAULT_Y_HEIGHT,
            }).id();

            app.update();

            let transform = app.world().get::<Transform>(beam_entity).unwrap();

            // Beam should be centered at the midpoint
            let expected_center_x = (start.x + end.x) / 2.0; // 50
            let expected_center_z = (start.y + end.y) / 2.0; // 0

            assert!(
                (transform.translation.x - expected_center_x).abs() < 0.001,
                "Beam X position should be {}, got {}",
                expected_center_x,
                transform.translation.x
            );
            assert!(
                (transform.translation.z - expected_center_z).abs() < 0.001,
                "Beam Z position should be {}, got {}",
                expected_center_z,
                transform.translation.z
            );
        }
    }
}
