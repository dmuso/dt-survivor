use bevy::prelude::*;
use rand::Rng;

use crate::game::components::*;
use crate::states::*;

pub fn setup_game(
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

pub fn game_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Intro);
    }
}

#[allow(clippy::type_complexity)]
pub fn cleanup_game(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Camera2d>, With<Node>, With<Player>, With<Rock>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}