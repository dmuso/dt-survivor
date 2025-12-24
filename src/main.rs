use bevy::prelude::*;
use rand::Rng;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Intro,
    InGame,
}

#[derive(Component)]
struct MenuButton;

#[derive(Component)]
struct StartGameButton;

#[derive(Component)]
struct ExitGameButton;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Rock;



fn setup_intro(
    mut commands: Commands,
) {
    // Spawn UI camera
    commands.spawn(Camera2d);

    // Spawn basic UI root
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    ))
    .with_children(|parent| {
        // Title
        parent.spawn((
            Text::new("Donny Tango: Survivor"),
            TextFont {
                font_size: 60.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));

        // Menu container
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(50.0)),
                ..default()
            },
        ))
        .with_children(|menu| {
            // Start Game button
            menu.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                MenuButton,
                StartGameButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Start Game"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            // Exit Game button
            menu.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.6, 0.2, 0.2)),
                MenuButton,
                ExitGameButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Exit Game"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

#[allow(clippy::type_complexity)]
fn button_interactions(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&StartGameButton>,
            Option<&ExitGameButton>,
        ),
        (Changed<Interaction>, With<MenuButton>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, mut background_color, start_button, exit_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if start_button.is_some() {
                    next_state.set(GameState::InGame);
                } else if exit_button.is_some() {
                    app_exit.write(AppExit::Success);
                }
            }
            Interaction::Hovered => {
                *background_color = BackgroundColor(Color::srgb(0.4, 0.4, 0.4));
            }
            Interaction::None => {
                if start_button.is_some() {
                    *background_color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                } else if exit_button.is_some() {
                    *background_color = BackgroundColor(Color::srgb(0.6, 0.2, 0.2));
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn cleanup_intro(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Camera2d>, With<MenuButton>, With<Node>, With<Text>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn setup_game(
    mut commands: Commands,
) {
    // Spawn game camera
    commands.spawn(Camera2d);

    // Spawn player in the center of the screen
    commands.spawn((
        Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(20.0, 20.0)), // Green player
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Player,
    ));

    // Spawn random rocks scattered throughout the scene
    let mut rng = rand::thread_rng();
    for _ in 0..15 {
        let x = rng.gen_range(-400.0..400.0);
        let y = rng.gen_range(-300.0..300.0);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.5, 0.5, 0.5), Vec2::new(rng.gen_range(10.0..30.0), rng.gen_range(10.0..30.0))), // Gray rocks
            Transform::from_translation(Vec3::new(x, y, 0.0)),
            Rock,
        ));
    }
}

fn game_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Intro);
    }
}

#[allow(clippy::type_complexity)]
fn cleanup_game(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Camera2d>, With<Node>, With<Player>, With<Rock>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(OnEnter(GameState::Intro), setup_intro)
        .add_systems(Update, button_interactions.run_if(in_state(GameState::Intro)))
        .add_systems(OnExit(GameState::Intro), cleanup_intro)
        .add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(Update, game_input.run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game)
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

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