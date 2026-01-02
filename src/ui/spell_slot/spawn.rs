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

use crate::spell::Spell;
use crate::ui::spell_slot::components::{
    SlotSource, SpellAbbreviation, SpellIconImage, SpellIconVisual, SpellLevelIndicator,
    SpellSlotVisual,
};

/// Returns background and border colors for a spell slot.
///
/// Single source of truth for spell slot color logic:
/// - With spell: background = element color with alpha, border = element color
/// - Without spell: uses empty slot constants
pub fn spell_slot_colors(spell: Option<&Spell>) -> (BackgroundColor, BorderColor) {
    match spell {
        Some(spell) => {
            let element_color = spell.element.color();
            (
                BackgroundColor(element_color.with_alpha(BACKGROUND_ALPHA)),
                BorderColor::all(element_color),
            )
        }
        None => (
            BackgroundColor(empty_slot::SLOT_BACKGROUND),
            BorderColor::all(empty_slot::SLOT_BORDER),
        ),
    }
}

/// Spawns a complete spell slot with all required children.
///
/// Creates a stable entity structure that the refresh system can query and update:
/// - Slot container with BackgroundColor, BorderColor, BorderRadius, SpellSlotVisual marker
/// - ImageNode child for spell textures (with SpellIconImage marker)
/// - Text child for spell abbreviation (with SpellAbbreviation marker, starts hidden)
/// - Level indicator child (with SpellLevelIndicator marker)
///
/// This function creates the structure once - the refresh system updates
/// the visual properties (colors, textures, text, visibility) based on spell state.
///
/// # Arguments
/// * `parent` - The parent to spawn under
/// * `source` - Whether this slot reads from SpellList (Active) or InventoryBag (Bag)
/// * `index` - Index into the source's spell list
/// * `spell` - Initial spell to display (or None for empty slot)
/// * `asset_server` - Asset server for loading textures
pub fn spawn_spell_slot(
    parent: &mut ChildSpawnerCommands,
    source: SlotSource,
    index: usize,
    spell: Option<&Spell>,
    asset_server: &AssetServer,
) -> Entity {
    let (bg_color, border_color) = spell_slot_colors(spell);

    // Load texture if spell has an icon
    let image_handle = spell
        .and_then(|s| s.spell_type.icon_path())
        .map(|path| asset_server.load(path));

    // Determine visibility and abbreviation text
    let (abbrev_visibility, abbrev_text) = match spell {
        Some(s) if s.spell_type.icon_path().is_none() => {
            // Spell without texture - show abbreviation
            (Visibility::Visible, s.spell_type.abbreviation().to_string())
        }
        _ => {
            // Spell with texture or empty slot - hide abbreviation
            (Visibility::Hidden, String::new())
        }
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(SLOT_SIZE),
                height: Val::Px(SLOT_SIZE),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            bg_color,
            border_color,
            BorderRadius::all(Val::Px(BORDER_RADIUS)),
            SpellSlotVisual { source, index },
        ))
        .with_children(|slot| {
            // ImageNode for spell texture
            let mut image_node = ImageNode::default();
            if let Some(handle) = image_handle {
                image_node.image = handle;
            }

            slot.spawn((
                Node {
                    width: Val::Px(SLOT_SIZE),
                    height: Val::Px(SLOT_SIZE),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                image_node,
                BorderRadius::all(Val::Px(4.0)),
                SpellIconImage { index },
            ));

            // Text for spell abbreviation (shown when no texture)
            slot.spawn((
                Text::new(abbrev_text),
                TextFont {
                    font_size: SLOT_SIZE * 0.35,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(bevy::text::Justify::Center),
                abbrev_visibility,
                SpellAbbreviation { index },
            ));

            // Level indicator
            spawn_level_indicator(slot, index);
        })
        .id()
}

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

/// Spawns a standalone spell icon visual that fills the given size.
///
/// Used for drag visuals and other cases where a spell icon is needed
/// outside of the standard slot refresh system.
///
/// For spells with custom textures, the texture fills the entire area with no background.
/// For spells without textures, uses element-colored background with spell abbreviation.
///
/// # Arguments
/// * `parent` - The parent entity to spawn the icon under
/// * `spell` - The spell to render (or None for empty slot)
/// * `size` - The size of the icon in pixels
/// * `asset_server` - Asset server for loading textures (optional)
pub fn spawn_spell_icon_visual(
    parent: &mut ChildSpawnerCommands,
    spell: Option<&Spell>,
    size: f32,
    asset_server: Option<&AssetServer>,
) {
    match spell {
        Some(spell) => {
            // Check if spell has a custom icon texture
            if let (Some(icon_path), Some(asset_server)) =
                (spell.spell_type.icon_path(), asset_server)
            {
                // Use custom texture - no background, just the image
                parent.spawn((
                    Node {
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    ImageNode {
                        image: asset_server.load(icon_path),
                        ..default()
                    },
                    BorderRadius::all(Val::Px(4.0)),
                    SpellIconVisual,
                ));
            } else {
                // Element-colored square with spell abbreviation
                parent
                    .spawn((
                        Node {
                            width: Val::Px(size),
                            height: Val::Px(size),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(spell.element.color()),
                        BorderColor::all(spell.element.color().lighter(0.3)),
                        BorderRadius::all(Val::Px(4.0)),
                        SpellIconVisual,
                    ))
                    .with_children(|icon| {
                        // Spell type abbreviation
                        icon.spawn((
                            Text::new(spell.spell_type.abbreviation()),
                            TextFont {
                                font_size: size * 0.35,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::new_with_justify(bevy::text::Justify::Center),
                        ));
                    });
            }
        }
        None => {
            // Empty slot - don't spawn any icon visual
            // The slot container handles its own empty appearance
        }
    }
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

    mod spell_slot_colors_tests {
        use super::*;
        use crate::element::Element;
        use crate::spell::Spell;
        use crate::spell::spell_type::SpellType;

        #[test]
        fn returns_empty_slot_colors_when_no_spell() {
            let (bg, border) = spell_slot_colors(None);
            assert_eq!(bg.0, empty_slot::SLOT_BACKGROUND);
            assert_eq!(border.top, empty_slot::SLOT_BORDER);
        }

        #[test]
        fn returns_element_background_with_alpha_when_spell_present() {
            let spell = Spell::new(SpellType::Fireball);
            let (bg, _) = spell_slot_colors(Some(&spell));
            let expected = spell.element.color().with_alpha(BACKGROUND_ALPHA);
            assert_eq!(bg.0, expected);
        }

        #[test]
        fn returns_element_border_when_spell_present() {
            let spell = Spell::new(SpellType::Fireball);
            let (_, border) = spell_slot_colors(Some(&spell));
            let expected = spell.element.color();
            assert_eq!(border.top, expected);
        }

        #[test]
        fn works_with_different_elements() {
            for element in Element::all() {
                let spell_types = SpellType::by_element(*element);
                if let Some(spell_type) = spell_types.first() {
                    let spell = Spell::new(*spell_type);
                    let (bg, border) = spell_slot_colors(Some(&spell));

                    let expected_bg = element.color().with_alpha(BACKGROUND_ALPHA);
                    let expected_border = element.color();

                    assert_eq!(
                        bg.0, expected_bg,
                        "Background color should match element {:?}",
                        element
                    );
                    assert_eq!(
                        border.top, expected_border,
                        "Border color should match element {:?}",
                        element
                    );
                }
            }
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

    mod spawn_spell_slot_tests {
        use super::*;
        use crate::spell::spell_type::SpellType;
        use crate::ui::spell_slot::components::{SlotSource, SpellAbbreviation, SpellIconImage, SpellSlotVisual};
        use bevy::ecs::system::RunSystemOnce;

        /// Test marker to find our spawned parent
        #[derive(Component)]
        struct TestParent;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::prelude::TaskPoolPlugin::default());
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.add_plugins(bevy::prelude::ImagePlugin::default());
            app
        }

        fn spawn_test_slot_empty(source: SlotSource, index: usize) -> impl FnMut(Commands, Res<AssetServer>) {
            move |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, source, index, None, &asset_server);
                });
            }
        }

        fn spawn_test_slot_with_spell(source: SlotSource, index: usize, spell: Spell) -> impl FnMut(Commands, Res<AssetServer>) {
            move |mut commands: Commands, asset_server: Res<AssetServer>| {
                commands.spawn((Node::default(), TestParent)).with_children(|parent| {
                    spawn_spell_slot(parent, source, index, Some(&spell), &asset_server);
                });
            }
        }

        #[test]
        fn spawns_entity_with_spell_slot_visual_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let slot_visual = app
                .world_mut()
                .query::<&SpellSlotVisual>()
                .iter(app.world())
                .next();
            assert!(slot_visual.is_some(), "Should spawn entity with SpellSlotVisual");
        }

        #[test]
        fn spell_slot_visual_has_correct_source_and_index() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Bag, 3));

            let slot_visual = app
                .world_mut()
                .query::<&SpellSlotVisual>()
                .iter(app.world())
                .next()
                .expect("SpellSlotVisual should exist");

            assert_eq!(slot_visual.source, SlotSource::Bag);
            assert_eq!(slot_visual.index, 3);
        }

        #[test]
        fn spawns_entity_with_button_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let has_button = app
                .world_mut()
                .query::<(&Button, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .is_some();
            assert!(has_button, "Slot should have Button component");
        }

        #[test]
        fn spawns_entity_with_node_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let node = app
                .world_mut()
                .query::<(&Node, &SpellSlotVisual)>()
                .iter(app.world())
                .next();
            assert!(node.is_some(), "Slot should have Node component");
        }

        #[test]
        fn spawns_entity_with_correct_size() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (node, _) = app
                .world_mut()
                .query::<(&Node, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(node.width, Val::Px(SLOT_SIZE));
            assert_eq!(node.height, Val::Px(SLOT_SIZE));
        }

        #[test]
        fn empty_slot_has_empty_slot_background_color() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (bg, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(bg.0, empty_slot::SLOT_BACKGROUND);
        }

        #[test]
        fn empty_slot_has_empty_slot_border_color() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (border, _) = app
                .world_mut()
                .query::<(&BorderColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(border.top, empty_slot::SLOT_BORDER);
        }

        #[test]
        fn spawns_slot_with_border_radius() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (radius, _) = app
                .world_mut()
                .query::<(&BorderRadius, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(radius.top_left, Val::Px(BORDER_RADIUS));
        }

        #[test]
        fn spawns_child_with_spell_icon_image_marker() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 2));

            let icon_image = app
                .world_mut()
                .query::<&SpellIconImage>()
                .iter(app.world())
                .next();
            assert!(icon_image.is_some(), "Should spawn child with SpellIconImage marker");
            assert_eq!(icon_image.unwrap().index, 2);
        }

        #[test]
        fn spawns_child_with_image_node() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let has_image_node = app
                .world_mut()
                .query::<(&ImageNode, &SpellIconImage)>()
                .iter(app.world())
                .next()
                .is_some();
            assert!(has_image_node, "Icon child should have ImageNode component");
        }

        #[test]
        fn spawns_child_with_spell_abbreviation_marker() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 4));

            let abbrev = app
                .world_mut()
                .query::<&SpellAbbreviation>()
                .iter(app.world())
                .next();
            assert!(abbrev.is_some(), "Should spawn child with SpellAbbreviation marker");
            assert_eq!(abbrev.unwrap().index, 4);
        }

        #[test]
        fn empty_slot_abbreviation_is_hidden() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (visibility, _) = app
                .world_mut()
                .query::<(&Visibility, &SpellAbbreviation)>()
                .iter(app.world())
                .next()
                .expect("Abbreviation should exist");

            assert_eq!(*visibility, Visibility::Hidden);
        }

        #[test]
        fn spawns_child_with_spell_level_indicator() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 1));

            let indicator = app
                .world_mut()
                .query::<&SpellLevelIndicator>()
                .iter(app.world())
                .next();
            assert!(indicator.is_some(), "Should spawn child with SpellLevelIndicator");
            assert_eq!(indicator.unwrap().index, 1);
        }

        #[test]
        fn spell_slot_has_three_children() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            // Find the slot entity
            let (slot_entity, _) = app
                .world_mut()
                .query::<(Entity, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            let children = app.world().get::<Children>(slot_entity);
            assert!(children.is_some(), "Slot should have children");
            assert_eq!(children.unwrap().len(), 3, "Slot should have 3 children (image, abbreviation, level indicator)");
        }

        #[test]
        fn spell_with_texture_has_abbreviation_hidden() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Fireball); // Fireball has a texture

            let _ = app.world_mut().run_system_once(spawn_test_slot_with_spell(SlotSource::Active, 0, spell));

            let (visibility, _) = app
                .world_mut()
                .query::<(&Visibility, &SpellAbbreviation)>()
                .iter(app.world())
                .next()
                .expect("Abbreviation should exist");

            assert_eq!(*visibility, Visibility::Hidden);
        }

        #[test]
        fn spell_with_texture_has_element_background() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Fireball);
            let expected_bg = spell.element.color().with_alpha(BACKGROUND_ALPHA);

            let _ = app.world_mut().run_system_once(spawn_test_slot_with_spell(SlotSource::Active, 0, spell));

            let (bg, _) = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(bg.0, expected_bg);
        }

        #[test]
        fn spell_with_texture_has_element_border() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Fireball);
            let expected_border = spell.element.color();

            let _ = app.world_mut().run_system_once(spawn_test_slot_with_spell(SlotSource::Active, 0, spell));

            let (border, _) = app
                .world_mut()
                .query::<(&BorderColor, &SpellSlotVisual)>()
                .iter(app.world())
                .next()
                .expect("Slot should exist");

            assert_eq!(border.top, expected_border);
        }

        #[test]
        fn active_source_slot_has_correct_source() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let slot_visual = app
                .world_mut()
                .query::<&SpellSlotVisual>()
                .iter(app.world())
                .next()
                .expect("SpellSlotVisual should exist");

            assert_eq!(slot_visual.source, SlotSource::Active);
        }

        #[test]
        fn bag_source_slot_has_correct_source() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Bag, 0));

            let slot_visual = app
                .world_mut()
                .query::<&SpellSlotVisual>()
                .iter(app.world())
                .next()
                .expect("SpellSlotVisual should exist");

            assert_eq!(slot_visual.source, SlotSource::Bag);
        }

        #[test]
        fn icon_image_has_absolute_positioning() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (node, _) = app
                .world_mut()
                .query::<(&Node, &SpellIconImage)>()
                .iter(app.world())
                .next()
                .expect("Icon image should exist");

            assert_eq!(node.position_type, PositionType::Absolute);
        }

        #[test]
        fn icon_image_fills_slot_size() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(spawn_test_slot_empty(SlotSource::Active, 0));

            let (node, _) = app
                .world_mut()
                .query::<(&Node, &SpellIconImage)>()
                .iter(app.world())
                .next()
                .expect("Icon image should exist");

            assert_eq!(node.width, Val::Px(SLOT_SIZE));
            assert_eq!(node.height, Val::Px(SLOT_SIZE));
        }
    }
}
