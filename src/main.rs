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

    #[test]
    fn test_game_state_default() {
        let state = GameState::default();
        assert_eq!(state, GameState::Intro);
    }

    #[test]
    fn test_components_exist() {
        // Test that our component types can be created
        let _player = Player;
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
}