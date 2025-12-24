use bevy::prelude::*;
use crate::score::components::*;
use crate::score::resources::*;

pub fn setup_score_display(
    mut commands: Commands,
) {
    // Spawn score display at the top-center of the screen
    commands.spawn((
        Text::new("Score: 0"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Percent(50.0),
            ..default()
        },
        ScoreDisplay,
    ));
}

pub fn update_score_display(
    score: Res<Score>,
    mut query: Query<&mut Text, With<ScoreDisplay>>,
) {
    if score.is_changed() {
        for mut text in &mut query {
            **text = format!("Score: {}", score.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_setup_score_display_creates_ui_element() {
        let mut app = App::new();

        // Add required plugins for UI
        app.add_plugins(bevy::ui::UiPlugin::default());

        let _ = app.world_mut().run_system_once(setup_score_display);

        // Check that ScoreDisplay component exists
        let score_display_exists = app.world_mut().query::<&ScoreDisplay>().iter(app.world()).next().is_some();
        assert!(score_display_exists, "ScoreDisplay component should be created");

        // Check that Text component exists
        let text_exists = app.world_mut().query::<&Text>().iter(app.world()).next().is_some();
        assert!(text_exists, "Text component should be created");

        // Check the initial text content
        let text_content = app.world_mut().query::<&Text>().single(app.world()).unwrap();
        assert_eq!(text_content.as_str(), "Score: 0", "Initial score display should show 'Score: 0'");
    }

    #[test]
    fn test_update_score_display_changes_text() {
        let mut app = App::new();

        // Add required plugins
        app.add_plugins(bevy::ui::UiPlugin::default());

        // Initialize score resource
        app.init_resource::<Score>();

        // Create a score display entity
        let score_display_entity = app.world_mut().spawn((
            Text::new("Score: 0"),
            ScoreDisplay,
        )).id();

        // Update score
        {
            let mut score = app.world_mut().get_resource_mut::<Score>().unwrap();
            score.0 = 5;
        }

        // Run the update system
        let _ = app.world_mut().run_system_once(update_score_display);

        // Check that text was updated
        let updated_text = app.world().get::<Text>(score_display_entity).unwrap();
        assert_eq!(updated_text.as_str(), "Score: 5", "Score display should update to show 'Score: 5'");
    }

    #[test]
    fn test_update_score_display_no_change_when_score_unchanged() {
        let mut app = App::new();

        // Add required plugins
        app.add_plugins(bevy::ui::UiPlugin::default());

        // Initialize score resource
        app.init_resource::<Score>();

        // Create a score display entity
        let score_display_entity = app.world_mut().spawn((
            Text::new("Score: 0"),
            ScoreDisplay,
        )).id();

        // Score remains unchanged (default 0)

        // Run the update system
        let _ = app.world_mut().run_system_once(update_score_display);

        // Check that text was not updated (since score.is_changed() should be false initially)
        let text = app.world().get::<Text>(score_display_entity).unwrap();
        assert_eq!(text.as_str(), "Score: 0", "Score display should remain unchanged when score hasn't changed");
    }

    #[test]
    fn test_score_display_component_exists() {
        let mut app = App::new();

        // Create an entity with ScoreDisplay component
        app.world_mut().spawn(ScoreDisplay);

        // Verify the component exists
        let score_display_exists = app.world_mut().query::<&ScoreDisplay>().iter(app.world()).count() == 1;
        assert!(score_display_exists, "ScoreDisplay component should exist on entity");
    }
}