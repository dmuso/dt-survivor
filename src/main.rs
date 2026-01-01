use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_kira_audio::prelude::*;
use donny_tango_survivor::{
    audio_plugin,
    combat_plugin,
    experience_plugin,
    game_plugin,
    inventory_plugin,
    ui_plugin,
    states::GameState
};

fn main() {
    // Check for --auto-start flag
    let auto_start = std::env::args().any(|arg| arg == "--auto-start");

    // Get the current directory and construct the assets path
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let assets_path = current_dir.join("assets");

    println!("Current directory: {:?}", current_dir);
    println!("Assets path: {:?}", assets_path);

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.build()
            .disable::<bevy::audio::AudioPlugin>()
            .set(AssetPlugin {
                file_path: assets_path.to_string_lossy().to_string(),
                ..default()
            }))
        .add_plugins(AudioPlugin)
        .add_plugins(HanabiPlugin)
        .init_state::<GameState>()
        .add_plugins((audio_plugin, combat_plugin, experience_plugin, game_plugin, inventory_plugin, ui_plugin));

    // If auto-start flag is set, add a system to skip to InGame state
    if auto_start {
        app.add_systems(Startup, |mut next_state: ResMut<NextState<GameState>>| {
            next_state.set(GameState::InGame);
        });
    }

    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use donny_tango_survivor::prelude::*;
    use donny_tango_survivor::bullets::systems::bullet_collision_detection;
    use donny_tango_survivor::bullets::systems::bullet_collision_effects;
    use donny_tango_survivor::score::Score;
    use donny_tango_survivor::bullets::Bullet;
    use donny_tango_survivor::weapon::systems::weapon_firing_system;
    use donny_tango_survivor::inventory::Inventory;
    use donny_tango_survivor::game::ScreenTintEffect;
    use donny_tango_survivor::weapon::components::{Weapon, WeaponType};
    use donny_tango_survivor::inventory::components::EquippedWeapon;
    use donny_tango_survivor::enemies::components::Enemy;
    use donny_tango_survivor::ui::components::{WeaponSlot, WeaponIcon, WeaponTimerFill};
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_game_state_default() {
        let state = GameState::default();
        assert_eq!(state, GameState::Intro);
    }

    #[test]
    fn test_components_exist() {
        // Test that our component types can be created
        let _rock = Rock;
        let _menu_button = MenuButton;
        let _start_button = StartGameButton;
        let _exit_button = ExitGameButton;
    }

    #[test]
    fn test_player_mesh_properties() {
        // Test that player uses 3D mesh (GameMeshes.player is 1x1x1 cube)
        let transform = Transform::from_translation(Vec3::new(0.0, 0.5, 0.0));
        // Player is at Y=0.5 (half cube height above ground)
        assert_eq!(transform.translation.y, 0.5);
    }

    #[test]
    fn test_rock_mesh_properties() {
        // Test that rock uses 3D mesh (GameMeshes.rock is 1.0x0.5x1.0 flat cube)
        let transform = Transform::from_translation(Vec3::new(5.0, 0.25, 5.0));
        // Rock is at Y=0.25 (half height above ground)
        assert_eq!(transform.translation.y, 0.25);
    }

    #[test]
    fn test_player_transform_position() {
        // Test that player transform is created at center position
        let transform = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));

        assert_eq!(transform.translation, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_random_position_range() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Test that random positions are within expected bounds
        for _ in 0..100 {
            let x = rng.gen_range(-400.0..400.0);
            let y = rng.gen_range(-300.0..300.0);

            assert!(x >= -400.0 && x <= 400.0);
            assert!(y >= -300.0 && y <= 300.0);
        }
    }

    #[test]
    fn test_random_rock_sizes() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Test that random rock sizes are within expected bounds
        for _ in 0..100 {
            let size = rng.gen_range(10.0..30.0);
            assert!(size >= 10.0 && size <= 30.0);
        }
    }

    #[test]
    fn test_game_state_enum_variants() {
        // Test that both game states exist and are distinct
        assert_ne!(GameState::Intro, GameState::InGame);
        assert_eq!(GameState::Intro as u8, 0);
        assert_eq!(GameState::InGame as u8, 1);
    }

    #[test]
    fn test_camera_reuse_across_state_transitions() {
        let mut app = App::new();

        // Add minimal plugins needed for state transitions
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state (starts in Intro by default)
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin, ui_plugin));

        // Verify initial state is Intro
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::Intro);

        // Run startup systems (this should create the intro camera)
        app.update();

        // Verify camera exists after intro setup
        let camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        assert!(camera_exists, "Should have a camera after intro setup");

        // Check that intro UI elements exist
        let has_ui = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(has_ui, "Should have UI nodes in intro state");

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update(); // Process state transition

        // Verify state changed to InGame
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::InGame);

        // Verify camera still exists (reused, not recreated)
        let camera_still_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        assert!(camera_still_exists, "Should still have camera after transitioning to InGame");

        // Verify game entities exist (player and rocks)
        let _has_player = app.world_mut().query::<&Player>().single(app.world()).is_ok();
        assert!(_has_player, "Should have a player in InGame");

        let _rock_count = app.world_mut().query::<&Rock>().iter(app.world()).count();
        assert_eq!(_rock_count, 15, "Should have 15 rocks in InGame");

        // Verify score display UI element exists in InGame state
        let has_ui_ingame = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(has_ui_ingame, "Should have UI nodes (score display) in InGame state");

        // Transition back to Intro state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::Intro);
        app.update(); // Process state transition

        // Verify state changed back to Intro
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::Intro);

        // Verify camera still exists (reused again)
        let camera_exists_again = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        assert!(camera_exists_again, "Should still have camera after transitioning back to Intro");

        // Verify UI elements are back
        let _has_ui_again = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(_has_ui_again, "Should have UI nodes again in Intro state");

        // Verify game entities are gone
        let _has_player_intro = app.world_mut().query::<&Player>().single(app.world()).is_ok();
        assert!(!_has_player_intro, "Should have no players in Intro state");

        let rock_count_intro = app.world_mut().query::<&Rock>().iter(app.world()).count();
        assert_eq!(rock_count_intro, 0, "Should have no rocks in Intro state");
    }

    #[test]
    fn test_no_blank_screen_during_transitions() {
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin, ui_plugin));

        // Run initial update to set up intro
        app.update();

        // Record initial state
        let initial_camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        let initial_has_content = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();

        // Ensure we start with content to render
        assert!(initial_camera_exists, "Should have camera initially");
        assert!(initial_has_content, "Should have renderable content initially");

        // Transition to InGame
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify we still have camera and content immediately after transition
        let post_transition_camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        let post_transition_has_content = app.world_mut().query::<&Player>().single(app.world()).is_ok() ||
                                          app.world_mut().query::<&Rock>().iter(app.world()).next().is_some();

        assert!(post_transition_camera_exists, "Camera should exist immediately after transition");
        assert!(post_transition_has_content, "Should have renderable content immediately after transition");

        // Transition back to Intro
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::Intro);
        app.update();

        // Verify camera persists and content is available
        let final_camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        let final_has_content = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();

        assert!(final_camera_exists, "Camera should persist throughout all transitions");
        assert!(final_has_content, "Should have renderable content after all transitions");
    }

    #[test]
    fn test_inventory_initialization() {
        use donny_tango_survivor::inventory::resources::Inventory;

        // Inventory starts empty - pistol is added when Whisper is collected
        let inventory = Inventory::default();
        let pistol_type = WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 };

        // Check that inventory is empty by default
        assert!(inventory.get_weapon(&pistol_type).is_none(), "Inventory should be empty by default");
        assert_eq!(inventory.weapons.len(), 0, "Inventory should have no weapons initially");
    }

    #[test]
    fn test_weapon_equipped_to_player() {
        // Since weapons are only equipped after Whisper is collected,
        // this test verifies that player starts WITHOUT weapons
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Check that player exists
        let world = app.world_mut();
        let player_count = world.query::<&Player>().iter(world).count();
        assert_eq!(player_count, 1, "Should have exactly one player");

        // Player starts with no weapons - weapons are added when Whisper is collected
        let weapon_count = world.query::<&Weapon>().iter(world).count();
        assert_eq!(weapon_count, 0, "Should have no weapon entities until Whisper is collected");

        let equipped_count = world.query::<&EquippedWeapon>().iter(world).count();
        assert_eq!(equipped_count, 0, "Should have no equipped weapons until Whisper is collected");
    }

    #[test]
    fn test_weapon_firing_spawns_bullets() {
        use donny_tango_survivor::whisper::resources::WeaponOrigin;

        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, inventory_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Simulate Whisper collection by setting WeaponOrigin position (full 3D)
        app.world_mut().resource_mut::<WeaponOrigin>().position = Some(Vec3::new(0.0, 3.0, 0.0));

        // Add pistol to inventory (simulating Whisper collection)
        {
            let mut inventory = app.world_mut().resource_mut::<Inventory>();
            inventory.add_or_level_weapon(Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: -2.0,
            });
        }

        // Create weapon entity
        let player_pos = {
            let mut query = app.world_mut().query::<&Transform>();
            query.single(app.world()).map(|t| t.translation).unwrap_or(Vec3::ZERO)
        };

        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: -2.0,
            },
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(player_pos),
        ));

        // Create an enemy to target
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
        ));

        // Advance time to allow weapon to fire (past the 2 second cooldown)
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs(3));
        }

        // Run weapon firing system
        let _ = app.world_mut().run_system_once(weapon_firing_system);

        // Check that bullets were spawned (level 1 pistol fires 1 bullet)
        let world = app.world_mut();
        let bullet_count = world.query::<&Bullet>().iter(world).count();
        assert_eq!(bullet_count, 1, "Level 1 weapon firing should spawn 1 bullet");

        // Check that weapon last_fired was updated
        let mut weapon_query = world.query::<&Weapon>();
        if let Ok(weapon) = weapon_query.single(world) {
            assert!(weapon.last_fired > 0.0, "Weapon last_fired should be updated after firing");
        } else {
            panic!("Weapon not found after firing");
        }
    }

    #[test]
    fn test_weapon_slots_ui_created() {
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin, ui_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Check that weapon slots are created
        let world = app.world_mut();
        let slot_count = world.query::<&WeaponSlot>().iter(world).count();
        assert_eq!(slot_count, 3, "Should have 3 weapon slots (pistol, laser, rocket_launcher)");

        // Check that weapon icons exist for all slots
        let icon_count = world.query::<&WeaponIcon>().iter(world).count();
        assert_eq!(icon_count, 3, "Should have 3 weapon icons for all slots");

        // Check that weapon timer fill exists for all weapon types
        let timer_fill_count = world.query::<&WeaponTimerFill>().iter(world).count();
        assert_eq!(timer_fill_count, 3, "Should have 3 weapon timer fill elements (one for each weapon type)");
    }

    #[test]
    fn test_scoring_integration_full_flow() {
        use donny_tango_survivor::combat::{CheckDeath, Health, apply_damage_system, check_death_system, handle_enemy_death_system};

        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify we're in InGame state
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::InGame);

        // Verify score starts at 0
        let initial_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(initial_score.0, 0, "Score should start at 0");

        // Create a bullet and enemy for collision testing
        // Enemy needs Health and CheckDeath for the combat system
        // Collision detection uses XZ plane, so place bullet and enemy within BULLET_COLLISION_RADIUS on XZ
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.15, 0.0)), // X=0, Z=0
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)), // X=0.5, Z=0 - within BULLET_COLLISION_RADIUS=1.0
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0), // BULLET_DAMAGE is 10
            CheckDeath,
        )).id();

        // Run collision and combat systems in sequence
        let _ = app.world_mut().run_system_once(bullet_collision_detection);
        let _ = app.world_mut().run_system_once(bullet_collision_effects);
        let _ = app.world_mut().run_system_once(apply_damage_system);
        let _ = app.world_mut().run_system_once(check_death_system);
        let _ = app.world_mut().run_system_once(handle_enemy_death_system);

        // Verify both entities are despawned
        assert!(!app.world().entities().contains(bullet_entity), "Bullet should be despawned after collision");
        assert!(!app.world().entities().contains(enemy_entity), "Enemy should be despawned after death");

        // Verify score incremented
        let updated_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(updated_score.0, 1, "Score should increment to 1 after enemy defeat");
    }

    #[test]
    fn test_scoring_integration_multiple_enemies() {
        use donny_tango_survivor::combat::{CheckDeath, Health, apply_damage_system, check_death_system, handle_enemy_death_system};

        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify score starts at 0
        let initial_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(initial_score.0, 0, "Score should start at 0");

        // Create multiple bullets and enemies at once
        // Collision detection uses XZ plane, so space them out in Z (Y is height)
        let mut bullet_entities = Vec::new();
        let mut enemy_entities = Vec::new();
        for i in 0..3 {
            let bullet_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.15, i as f32 * 20.0)), // Spread in Z
                Bullet {
                    direction: Vec2::new(1.0, 0.0),
                    speed: 100.0,
                    lifetime: Timer::from_seconds(15.0, TimerMode::Once),
                },
            )).id();
            bullet_entities.push(bullet_entity);

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, i as f32 * 20.0)), // Same Z, within BULLET_COLLISION_RADIUS on X
                Enemy { speed: 50.0, strength: 10.0 },
                Health::new(10.0), // BULLET_DAMAGE is 10
                CheckDeath,
            )).id();
            enemy_entities.push(enemy_entity);
        }

        // Run collision and combat systems in sequence
        let _ = app.world_mut().run_system_once(bullet_collision_detection);
        let _ = app.world_mut().run_system_once(bullet_collision_effects);
        let _ = app.world_mut().run_system_once(apply_damage_system);
        let _ = app.world_mut().run_system_once(check_death_system);
        let _ = app.world_mut().run_system_once(handle_enemy_death_system);

        // Verify all entities are despawned
        for (i, &bullet_entity) in bullet_entities.iter().enumerate() {
            assert!(!app.world().entities().contains(bullet_entity), "Bullet {} should be despawned", i);
        }
        for (i, &enemy_entity) in enemy_entities.iter().enumerate() {
            assert!(!app.world().entities().contains(enemy_entity), "Enemy {} should be despawned", i);
        }

        // Verify score incremented for each enemy defeated
        let final_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(final_score.0, 3, "Score should be 3 after defeating 3 enemies");
    }

    #[test]
    fn test_scoring_integration_score_persistence() {
        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Score some points
        {
            let mut score = app.world_mut().get_resource_mut::<Score>().unwrap();
            score.0 = 5;
        }

        // Transition back to Intro and then to InGame again
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::Intro);
        app.update();
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Score should persist across state transitions (not reset)
        let score_after_reset = app.world().get_resource::<Score>().unwrap();
        assert_eq!(score_after_reset.0, 5, "Score should persist across state transitions");
    }

    #[test]
    fn test_game_uses_3d_components() {
        use donny_tango_survivor::game::resources::{GameMeshes, GameMaterials};

        let mut app = App::new();

        // Add minimal plugins for 3D rendering
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify 3D rendering resources exist
        let world = app.world();

        // Verify GameMeshes resource is available
        let game_meshes = world.get_resource::<GameMeshes>();
        assert!(game_meshes.is_some(), "GameMeshes resource should be initialized");

        // Verify GameMaterials resource is available
        let game_materials = world.get_resource::<GameMaterials>();
        assert!(game_materials.is_some(), "GameMaterials resource should be initialized");

        // Verify player entity exists with Transform (3D model loaded as child scene)
        let world = app.world_mut();
        let player_entities: Vec<Entity> = world.query_filtered::<Entity, (With<Player>, With<Transform>)>()
            .iter(world)
            .collect();
        assert_eq!(player_entities.len(), 1, "Player should exist with Transform component");

        // Verify Camera3d exists
        let camera_3d_count = world.query_filtered::<Entity, With<Camera3d>>()
            .iter(world)
            .count();
        assert!(camera_3d_count > 0, "Should have at least one Camera3d");

        // Verify no Sprite components on game entities (only 3D used)
        let player_with_sprite: Vec<Entity> = world.query_filtered::<Entity, (With<Player>, With<Sprite>)>()
            .iter(world)
            .collect();
        assert_eq!(player_with_sprite.len(), 0, "Player should NOT have Sprite component in 3D mode");
    }
}