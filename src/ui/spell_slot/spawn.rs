//! Spawning logic for spell slot visuals.
//!
//! This module provides unified spawn functions and visual constants for spell slots,
//! used in both the active spell bar and inventory bag.

// Re-export empty slot colors from the shared module
pub use crate::ui::components::empty_slot;

/// Standard size for spell icon slots in pixels.
/// Used consistently in the active spell bar and inventory bag.
pub const SLOT_SIZE: f32 = 50.0;

/// Alpha/opacity for spell element background tint.
/// Applied to the element color when rendering slot backgrounds.
pub const BACKGROUND_ALPHA: f32 = 0.4;

/// Vertical offset for level indicator from the top of the slot.
/// Negative value positions the level box above the slot edge.
pub const LEVEL_TOP_OFFSET: f32 = -6.0;

/// Font size for the level number text.
pub const LEVEL_FONT_SIZE: f32 = 9.0;

/// Horizontal padding for the level indicator box (left/right).
pub const LEVEL_PADDING_X: f32 = 3.0;

/// Vertical padding for the level indicator box (top/bottom).
pub const LEVEL_PADDING_Y: f32 = 1.0;

/// Border radius for slot containers.
pub const BORDER_RADIUS: f32 = 6.0;

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::ui::spell_slot::components::SpellLevelIndicator;

/// Spawns a level indicator badge for a spell slot.
///
/// Creates a small badge positioned at the top of the slot with:
/// - Absolute positioning (top: LEVEL_TOP_OFFSET, centered horizontally)
/// - Black background with white border
/// - Child Text with SpellLevelIndicator marker for refresh system
///
/// The text content is empty initially - the refresh system updates it.
pub fn spawn_level_indicator(parent: &mut ChildSpawnerCommands, index: usize) -> Entity {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(LEVEL_TOP_OFFSET),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-10.0)), // Center the level box
                padding: UiRect::axes(Val::Px(LEVEL_PADDING_X), Val::Px(LEVEL_PADDING_Y)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.0)),
            BorderColor::all(Color::srgb(1.0, 1.0, 1.0)),
            BorderRadius::all(Val::Px(2.0)),
            ZIndex(10),
        ))
        .with_children(|level_box| {
            level_box.spawn((
                Text::new(""),
                TextFont {
                    font_size: LEVEL_FONT_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextLayout::new_with_justify(bevy::text::Justify::Center),
                SpellLevelIndicator { index },
            ));
        })
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_module_exists() {
        // Placeholder test to verify module structure
        assert!(true);
    }

    mod slot_constants_tests {
        use super::*;

        #[test]
        fn slot_size_is_50() {
            assert_eq!(SLOT_SIZE, 50.0);
        }

        #[test]
        fn background_alpha_is_reasonable() {
            assert!(BACKGROUND_ALPHA > 0.0);
            assert!(BACKGROUND_ALPHA <= 1.0);
            assert_eq!(BACKGROUND_ALPHA, 0.4);
        }

        #[test]
        fn level_top_offset_is_negative() {
            assert!(LEVEL_TOP_OFFSET < 0.0);
            assert_eq!(LEVEL_TOP_OFFSET, -6.0);
        }

        #[test]
        fn level_font_size_is_9() {
            assert_eq!(LEVEL_FONT_SIZE, 9.0);
        }

        #[test]
        fn level_padding_values() {
            assert_eq!(LEVEL_PADDING_X, 3.0);
            assert_eq!(LEVEL_PADDING_Y, 1.0);
        }

        #[test]
        fn border_radius_is_6() {
            assert_eq!(BORDER_RADIUS, 6.0);
        }
    }

    mod empty_slot_reexport_tests {
        use super::*;

        #[test]
        fn empty_slot_background_is_accessible() {
            let _color = empty_slot::SLOT_BACKGROUND;
        }

        #[test]
        fn empty_slot_border_is_accessible() {
            let _color = empty_slot::SLOT_BORDER;
        }

        #[test]
        fn empty_slot_hover_background_is_accessible() {
            let _color = empty_slot::SLOT_BACKGROUND_HOVER;
        }

        #[test]
        fn empty_slot_hover_border_is_accessible() {
            let _color = empty_slot::SLOT_BORDER_HOVER;
        }
    }

    mod spawn_level_indicator_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        /// Test marker to find our spawned parent
        #[derive(Component)]
        struct TestParent;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::prelude::TaskPoolPlugin::default());
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app
        }

        fn spawn_test_indicator_with_index(index: usize) -> impl FnMut(Commands) {
            move |mut commands: Commands| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_level_indicator(parent, index);
                });
            }
        }

        #[test]
        fn spawns_entity_with_node_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            // Find the test parent
            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .expect("TestParent should exist");

            let children = app.world().get::<Children>(parent_entity);
            assert!(children.is_some(), "Parent should have children");
            let child_entity = children.unwrap().first().unwrap();
            assert!(
                app.world().get::<Node>(*child_entity).is_some(),
                "Child should have Node component"
            );
        }

        #[test]
        fn spawns_entity_with_absolute_positioning() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let child_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let node = app.world().get::<Node>(child_entity).unwrap();
            assert_eq!(node.position_type, PositionType::Absolute);
        }

        #[test]
        fn spawns_entity_with_correct_top_offset() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let child_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let node = app.world().get::<Node>(child_entity).unwrap();
            assert_eq!(node.top, Val::Px(LEVEL_TOP_OFFSET));
        }

        #[test]
        fn spawns_entity_with_black_background() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let child_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let bg = app.world().get::<BackgroundColor>(child_entity).unwrap();
            assert_eq!(bg.0, Color::srgb(0.0, 0.0, 0.0));
        }

        #[test]
        fn spawns_entity_with_white_border() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let child_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let border = app.world().get::<BorderColor>(child_entity).unwrap();
            assert_eq!(border.top, Color::srgb(1.0, 1.0, 1.0));
        }

        #[test]
        fn spawns_child_text_with_spell_level_indicator() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(3));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            // Get the level indicator entity (child of parent)
            let level_indicator_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();

            // Get the text child of the level indicator
            let text_children = app.world().get::<Children>(level_indicator_entity);
            assert!(text_children.is_some(), "Level indicator should have children");

            let text_entity = *text_children.unwrap().first().unwrap();
            let indicator = app.world().get::<SpellLevelIndicator>(text_entity);
            assert!(indicator.is_some(), "Text should have SpellLevelIndicator component");
            assert_eq!(indicator.unwrap().index, 3, "Indicator should have correct index");
        }

        #[test]
        fn spawns_child_text_with_correct_font_size() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let level_indicator_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let text_entity = *app.world().get::<Children>(level_indicator_entity).unwrap().first().unwrap();
            let text_font = app.world().get::<TextFont>(text_entity);
            assert!(text_font.is_some());
            assert_eq!(text_font.unwrap().font_size, LEVEL_FONT_SIZE);
        }

        #[test]
        fn spawns_child_text_with_white_color() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let level_indicator_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let text_entity = *app.world().get::<Children>(level_indicator_entity).unwrap().first().unwrap();
            let text_color = app.world().get::<TextColor>(text_entity);
            assert!(text_color.is_some());
            assert_eq!(text_color.unwrap().0, Color::srgb(1.0, 1.0, 1.0));
        }

        #[test]
        fn spawns_entity_with_zindex() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let child_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let zindex = app.world().get::<ZIndex>(child_entity);
            assert!(zindex.is_some());
            assert_eq!(*zindex.unwrap(), ZIndex(10));
        }

        #[test]
        fn spawns_entity_with_correct_padding() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let child_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let node = app.world().get::<Node>(child_entity).unwrap();
            assert_eq!(node.padding.left, Val::Px(LEVEL_PADDING_X));
            assert_eq!(node.padding.right, Val::Px(LEVEL_PADDING_X));
            assert_eq!(node.padding.top, Val::Px(LEVEL_PADDING_Y));
            assert_eq!(node.padding.bottom, Val::Px(LEVEL_PADDING_Y));
        }

        #[test]
        fn spawns_child_text_with_empty_initial_content() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(0));

            let (parent_entity, _) = app
                .world_mut()
                .query::<(Entity, &TestParent)>()
                .iter(app.world())
                .next()
                .unwrap();

            let level_indicator_entity = *app.world().get::<Children>(parent_entity).unwrap().first().unwrap();
            let text_entity = *app.world().get::<Children>(level_indicator_entity).unwrap().first().unwrap();
            let text = app.world().get::<Text>(text_entity);
            assert!(text.is_some());
            assert_eq!(text.unwrap().0, "");
        }

        #[test]
        fn indicator_index_matches_input() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_indicator_with_index(5));

            // Find the SpellLevelIndicator directly
            let indicator = app
                .world_mut()
                .query::<&SpellLevelIndicator>()
                .iter(app.world())
                .next()
                .expect("SpellLevelIndicator should exist");

            assert_eq!(indicator.index, 5);
        }
    }
}
