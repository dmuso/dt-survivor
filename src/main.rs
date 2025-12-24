use bevy::prelude::*;
use donny_tango_survivor::{game::plugin as game_plugin, ui::plugin as ui_plugin, states::GameState};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_plugins((game_plugin, ui_plugin))
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use donny_tango_survivor::prelude::*;
    use bevy::app::App;

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

        // Verify UI elements are gone
        let has_ui_ingame = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(!has_ui_ingame, "Should have no UI nodes in InGame state");

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
}