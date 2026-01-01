use bevy::prelude::*;
use rand::Rng;

use crate::arena::components::{ArenaWall, TorchLight};
use crate::arena::resources::{ArenaBounds, WallModelHandle, WALL_THICKNESS};

/// Length of each wall segment (should match the wall.glb model dimensions)
pub const WALL_SEGMENT_LENGTH: f32 = 12.0;

/// Load the wall model asset
pub fn load_wall_model(mut commands: Commands, asset_server: Res<AssetServer>) {
    let wall_handle: Handle<Scene> = asset_server.load("models/wall.glb#Scene0");
    commands.insert_resource(WallModelHandle(wall_handle));
}

/// Spawns wall segments around the arena perimeter
pub fn spawn_arena_walls(
    mut commands: Commands,
    bounds: Res<ArenaBounds>,
    wall_model: Option<Res<WallModelHandle>>,
    existing_walls: Query<Entity, With<ArenaWall>>,
) {
    // Only spawn walls once (check if any walls already exist)
    if !existing_walls.is_empty() {
        return;
    }

    // Get the wall model handle
    let wall_scene = wall_model.map(|w| w.0.clone());

    // Calculate how many wall segments we need for each side
    let arena_width = bounds.width();
    let segments_per_side = (arena_width / WALL_SEGMENT_LENGTH).ceil() as i32;

    // Spawn walls on all four sides
    // Wall model front faces +X in Blender, so we rotate to face inward toward arena center

    // North wall (positive Z edge) - front needs to face -Z (inward)
    spawn_wall_line(
        &mut commands,
        bounds.min_x,
        bounds.max_z + WALL_THICKNESS / 2.0,
        segments_per_side,
        Vec3::X,
        std::f32::consts::FRAC_PI_2,  // 90° to face -Z
        wall_scene.clone(),
    );

    // South wall (negative Z edge) - front needs to face +Z (inward)
    spawn_wall_line(
        &mut commands,
        bounds.min_x,
        bounds.min_z - WALL_THICKNESS / 2.0,
        segments_per_side,
        Vec3::X,
        -std::f32::consts::FRAC_PI_2,  // -90° to face +Z
        wall_scene.clone(),
    );

    // East wall (positive X edge) - front needs to face -X (inward)
    spawn_wall_line(
        &mut commands,
        bounds.max_x + WALL_THICKNESS / 2.0,
        bounds.min_z,
        segments_per_side,
        Vec3::Z,
        std::f32::consts::PI,  // 180° to face -X
        wall_scene.clone(),
    );

    // West wall (negative X edge) - front needs to face +X (inward)
    spawn_wall_line(
        &mut commands,
        bounds.min_x - WALL_THICKNESS / 2.0,
        bounds.min_z,
        segments_per_side,
        Vec3::Z,
        0.0,  // 0° to face +X
        wall_scene,
    );
}

/// Spawns a line of wall segments
fn spawn_wall_line(
    commands: &mut Commands,
    start_x: f32,
    start_z: f32,
    count: i32,
    direction: Vec3,
    rotation_y: f32,
    wall_scene: Option<Handle<Scene>>,
) {
    for i in 0..count {
        let offset = direction * (i as f32 * WALL_SEGMENT_LENGTH + WALL_SEGMENT_LENGTH / 2.0);
        let position = Vec3::new(start_x, 0.0, start_z) + offset;

        spawn_wall_segment(commands, position, rotation_y, wall_scene.clone());
    }
}

/// Spawns a single wall segment with lighting (no physics - movement is clamped to arena bounds)
fn spawn_wall_segment(
    commands: &mut Commands,
    position: Vec3,
    rotation_y: f32,
    wall_scene: Option<Handle<Scene>>,
) {
    let mut entity_commands = commands.spawn((
        ArenaWall,
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(rotation_y)),
        Visibility::default(),
    ));

    // Add the wall model as a child if available
    if let Some(scene) = wall_scene {
        entity_commands.with_child((
            SceneRoot(scene),
            Transform::default(),
        ));
    }

    // Add a point light at the wall position (matching Blender model light position)
    // Blender coords (X=1, Y=0, Z=6) → Bevy coords (X=1, Y=6, Z=0)
    entity_commands.with_child((
        PointLight {
            color: Color::srgb(1.0, 0.9, 0.7), // Warm light color
            intensity: 70_000.0,               // Lumens
            range: 20.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 6.0, 0.0),
        TorchLight::default(), // Adds flickering animation
    ));
}

/// Animates torch lights to create a flickering effect
pub fn animate_torch_lights(
    mut query: Query<(&mut PointLight, &mut TorchLight)>,
    time: Res<Time>,
) {
    let mut rng = rand::thread_rng();

    for (mut light, mut torch) in query.iter_mut() {
        // Update the timer
        torch.change_timer.tick(time.delta());

        // When timer fires, pick a new random target intensity
        if torch.change_timer.just_finished() {
            torch.target_intensity = rng.gen_range(torch.min_intensity..torch.max_intensity);
        }

        // Smoothly interpolate current intensity towards target
        let lerp_speed = 8.0 * time.delta_secs();
        light.intensity = light.intensity + (torch.target_intensity - light.intensity) * lerp_speed;
    }
}

/// Cleans up arena walls when leaving the game state
pub fn cleanup_arena_walls(mut commands: Commands, walls: Query<Entity, With<ArenaWall>>) {
    for entity in walls.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ArenaBounds>();
        app
    }

    #[test]
    fn wall_segment_length_is_positive() {
        assert!(WALL_SEGMENT_LENGTH > 0.0);
    }

    #[test]
    fn spawn_arena_walls_creates_wall_entities() {
        let mut app = setup_test_app();

        let _ = app.world_mut().run_system_once(spawn_arena_walls);

        // Count wall entities
        let wall_count = app.world_mut().query::<&ArenaWall>().iter(app.world()).count();

        // Should have spawned walls on all 4 sides
        // With 100 unit arena and 4 unit segments, we need ~25 segments per side = ~100 total
        assert!(wall_count > 0, "Should have spawned wall entities");
    }

    #[test]
    fn spawn_arena_walls_does_not_duplicate_walls() {
        let mut app = setup_test_app();

        // Spawn walls twice
        let _ = app.world_mut().run_system_once(spawn_arena_walls);
        let first_count = app.world_mut().query::<&ArenaWall>().iter(app.world()).count();

        let _ = app.world_mut().run_system_once(spawn_arena_walls);
        let second_count = app.world_mut().query::<&ArenaWall>().iter(app.world()).count();

        assert_eq!(first_count, second_count, "Should not duplicate walls on second call");
    }

    #[test]
    fn wall_entities_have_transform() {
        let mut app = setup_test_app();

        let _ = app.world_mut().run_system_once(spawn_arena_walls);

        let mut query = app.world_mut().query::<(&ArenaWall, &Transform)>();
        for (_, transform) in query.iter(app.world()) {
            // Y position should be at ground level (0)
            assert_eq!(transform.translation.y, 0.0, "Walls should be at ground level");
        }
    }

    #[test]
    fn cleanup_arena_walls_removes_all_walls() {
        let mut app = setup_test_app();

        // Spawn walls
        let _ = app.world_mut().run_system_once(spawn_arena_walls);
        let initial_count = app.world_mut().query::<&ArenaWall>().iter(app.world()).count();
        assert!(initial_count > 0, "Should have walls to clean up");

        // Cleanup walls
        let _ = app.world_mut().run_system_once(cleanup_arena_walls);
        let final_count = app.world_mut().query::<&ArenaWall>().iter(app.world()).count();

        assert_eq!(final_count, 0, "All walls should be removed after cleanup");
    }

    #[test]
    fn walls_are_positioned_at_arena_edges() {
        let mut app = setup_test_app();

        let _ = app.world_mut().run_system_once(spawn_arena_walls);

        let bounds = app.world().resource::<ArenaBounds>();
        let min_x = bounds.min_x;
        let max_x = bounds.max_x;
        let min_z = bounds.min_z;
        let max_z = bounds.max_z;

        let mut found_north = false;
        let mut found_south = false;
        let mut found_east = false;
        let mut found_west = false;

        let mut query = app.world_mut().query::<(&ArenaWall, &Transform)>();
        for (_, transform) in query.iter(app.world()) {
            let pos = transform.translation;

            // Check if wall is at one of the edges (within tolerance)
            if (pos.z - (max_z + WALL_THICKNESS / 2.0)).abs() < 0.1 {
                found_north = true;
            }
            if (pos.z - (min_z - WALL_THICKNESS / 2.0)).abs() < 0.1 {
                found_south = true;
            }
            if (pos.x - (max_x + WALL_THICKNESS / 2.0)).abs() < 0.1 {
                found_east = true;
            }
            if (pos.x - (min_x - WALL_THICKNESS / 2.0)).abs() < 0.1 {
                found_west = true;
            }
        }

        assert!(found_north, "Should have north wall");
        assert!(found_south, "Should have south wall");
        assert!(found_east, "Should have east wall");
        assert!(found_west, "Should have west wall");
    }
}
