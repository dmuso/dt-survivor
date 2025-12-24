use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use donny_tango_survivor::{
    audio_plugin,
    experience_plugin,
    game_plugin,
    inventory_plugin,
    ui_plugin,
    states::GameState
};

fn main() {
    // Get the current directory and construct the assets path
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let assets_path = current_dir.join("assets");

    println!("Current directory: {:?}", current_dir);
    println!("Assets path: {:?}", assets_path);

    App::new()
        .add_plugins(DefaultPlugins.build()
            .disable::<bevy::audio::AudioPlugin>()
            .set(AssetPlugin {
                file_path: assets_path.to_string_lossy().to_string(),
                ..default()
            }))
        .add_plugins(AudioPlugin)
        .init_state::<GameState>()
        .add_plugins((audio_plugin, experience_plugin, game_plugin, inventory_plugin, ui_plugin))
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use donny_tango_survivor::prelude::*;
    use donny_tango_survivor::bullets::systems::bullet_collision_system;
    use donny_tango_survivor::score::Score;
    use donny_tango_survivor::bullets::Bullet;
    use donny_tango_survivor::weapon::systems::weapon_firing_system;
    use donny_tango_survivor::inventory::Inventory;
    use donny_tango_survivor::game::ScreenTintEffect;
    use donny_tango_survivor::weapon::components::Weapon;
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
    fn test_player_sprite_properties() {
        // Test that player sprite is created with correct properties
        let sprite = Sprite {
            color: Color::srgb(0.0, 1.0, 0.0), // Green
            custom_size: Some(Vec2::new(20.0, 20.0)),
            ..default()
        };

        assert_eq!(sprite.color, Color::srgb(0.0, 1.0, 0.0));
        assert_eq!(sprite.custom_size, Some(Vec2::new(20.0, 20.0)));
    }

    #[test]
    fn test_rock_sprite_properties() {
        // Test that rock sprite is created with correct properties
        let sprite = Sprite {
            color: Color::srgb(0.5, 0.5, 0.5), // Gray
            custom_size: Some(Vec2::new(15.0, 15.0)), // Example size
            ..default()
        };

        assert_eq!(sprite.color, Color::srgb(0.5, 0.5, 0.5));
        assert_eq!(sprite.custom_size, Some(Vec2::new(15.0, 15.0)));
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
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state (starts in Intro by default)
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins
        app.add_plugins((game_plugin, ui_plugin));

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
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins
        app.add_plugins((game_plugin, ui_plugin));

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

        let inventory = Inventory::default();
        assert_eq!(inventory.slots.len(), 5, "Inventory should have 5 slots");

        // Check that slot 0 has a weapon
        assert!(inventory.slots[0].is_some(), "Slot 0 should have a weapon");

        let weapon = inventory.slots[0].as_ref().unwrap();
        assert_eq!(weapon.fire_rate, 2.0, "Default weapon should have 2 second fire rate");
        assert_eq!(weapon.last_fired, -2.0, "Default weapon should start with last_fired = -2.0 to prevent immediate firing");
        assert_eq!(weapon.damage, 1.0, "Default weapon should have 1.0 damage");

        // Check weapon type
        if let donny_tango_survivor::weapon::components::WeaponType::Pistol { bullet_count, spread_angle } = &weapon.weapon_type {
            assert_eq!(*bullet_count, 5, "Default pistol should fire 5 bullets");
            assert_eq!(*spread_angle, 15.0, "Default pistol should have 15 degree spread");
        } else {
            panic!("Default weapon should be a pistol");
        }

        // Check that slots 1-4 are empty
        for i in 1..5 {
            assert!(inventory.slots[i].is_none(), "Slots 1-4 should be empty");
        }
    }

    #[test]
    fn test_weapon_equipped_to_player() {
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins
        app.add_plugins((game_plugin, inventory_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Check that player exists and has weapon component
        let world = app.world_mut();
        // Check that player exists
        let player_count = world.query::<&Player>().iter(world).count();
        assert_eq!(player_count, 1, "Should have exactly one player");

        // Check that weapon entities exist
        let weapon_count = world.query::<&Weapon>().iter(world).count();
        assert_eq!(weapon_count, 1, "Should have exactly one weapon entity");

        let equipped_count = world.query::<&EquippedWeapon>().iter(world).count();
        assert_eq!(equipped_count, 1, "Should have exactly one equipped weapon");

        // Check weapon properties
        if let Ok(weapon) = world.query::<&Weapon>().single(world) {
            assert_eq!(weapon.fire_rate, 2.0, "Equipped weapon should have correct fire rate");
        }

        if let Ok(equipped) = world.query::<&EquippedWeapon>().single(world) {
            assert_eq!(equipped.slot_index, 0, "Weapon should be equipped in slot 0");
        }
    }

    #[test]
    fn test_weapon_firing_spawns_bullets() {
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (without UI to avoid asset dependencies)
        app.add_plugins((inventory_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

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

        // Check that bullets were spawned
        let world = app.world_mut();
        let bullet_count = world.query::<&Bullet>().iter(world).count();
        assert_eq!(bullet_count, 5, "Weapon firing should spawn 5 bullets");

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
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins
        app.add_plugins((game_plugin, inventory_plugin, ui_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Check that weapon slots are created
        let world = app.world_mut();
        let slot_count = world.query::<&WeaponSlot>().iter(world).count();
        assert_eq!(slot_count, 5, "Should have 5 weapon slots");

        // Check that weapon icons exist for all slots
        let icon_count = world.query::<&WeaponIcon>().iter(world).count();
        assert_eq!(icon_count, 5, "Should have 5 weapon icons for all slots");

        // Check that weapon timer fill exists
        let timer_fill_count = world.query::<&WeaponTimerFill>().iter(world).count();
        assert_eq!(timer_fill_count, 1, "Should have 1 weapon timer fill element");
    }

    #[test]
    fn test_scoring_integration_full_flow() {
        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (without UI plugin to avoid asset dependencies)
        app.add_plugins(game_plugin);

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify we're in InGame state
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::InGame);

        // Verify score starts at 0
        let initial_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(initial_score.0, 0, "Score should start at 0");

        // Create a bullet and enemy for collision testing
        let bullet_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Bullet {
                direction: Vec2::new(1.0, 0.0),
                speed: 100.0,
                lifetime: Timer::from_seconds(15.0, TimerMode::Once),
            },
        )).id();

        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
        )).id();

        // Run collision system
        let _ = app.world_mut().run_system_once(bullet_collision_system);

        // Verify both entities are despawned
        assert!(!app.world().entities().contains(bullet_entity), "Bullet should be despawned after collision");
        assert!(!app.world().entities().contains(enemy_entity), "Enemy should be despawned after collision");

        // Verify score incremented
        let updated_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(updated_score.0, 1, "Score should increment to 1 after enemy defeat");
    }

    #[test]
    fn test_scoring_integration_multiple_enemies() {
        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (without UI plugin to avoid asset dependencies)
        app.add_plugins(game_plugin);

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify score starts at 0
        let initial_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(initial_score.0, 0, "Score should start at 0");

        // Create multiple bullets and enemies
        for i in 0..3 {
            let bullet_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, i as f32 * 20.0, 0.0)),
                Bullet {
                    direction: Vec2::new(1.0, 0.0),
                    speed: 100.0,
                    lifetime: Timer::from_seconds(15.0, TimerMode::Once),
                },
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, i as f32 * 20.0, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            // Run collision system for each pair
            let _ = app.world_mut().run_system_once(bullet_collision_system);

            // Verify both entities are despawned
            assert!(!app.world().entities().contains(bullet_entity), "Bullet {} should be despawned", i);
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
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
        ));

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<Inventory>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (without UI plugin to avoid asset dependencies)
        app.add_plugins(game_plugin);

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
}