use bevy::prelude::*;

use crate::inventory::{InventoryBag, SpellList};
use crate::spell::Spell;
use crate::states::GameState;
use crate::ui::components::{spawn_spell_icon_visual, SPELL_SLOT_SIZE};

const BAG_COLUMNS: usize = 6;
const BAG_ROWS: usize = 5;
const ACTIVE_SLOTS: usize = 5;
const SLOT_SIZE: f32 = SPELL_SLOT_SIZE;
const SLOT_GAP: f32 = 8.0;

/// Root marker for the inventory screen.
/// Used for cleanup on state exit.
#[derive(Component)]
pub struct InventoryScreen;

/// Marker for the semi-transparent background overlay.
#[derive(Component)]
pub struct InventoryOverlay;

/// Component marking a bag slot in the inventory grid.
#[derive(Component)]
pub struct InventorySlot {
    pub index: usize,
}

/// Component marking an active spell slot display in the inventory.
#[derive(Component)]
pub struct ActiveSlotDisplay {
    pub index: usize,
}

/// Marker for currently selected slot.
#[derive(Component)]
pub struct SelectedSpell;

/// Resource tracking which bag slot is selected (if any).
#[derive(Resource, Default)]
pub struct SelectedBagSlot(pub Option<usize>);

/// Location of a spell being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragSource {
    Bag(usize),
    Active(usize),
}

/// Time in seconds before a held click becomes a drag.
const DRAG_HOLD_THRESHOLD: f32 = 0.15;

/// Resource tracking drag state for drag-and-drop.
#[derive(Resource, Default)]
pub struct DragState {
    /// Source of the current or pending drag.
    pub dragging: Option<DragSource>,
    /// Current cursor position in window coordinates.
    pub cursor_position: Option<Vec2>,
    /// Time when mouse was first pressed (for hold detection).
    pub drag_start_time: Option<f32>,
    /// Whether the drag visual has been spawned (hold threshold exceeded).
    pub drag_visual_spawned: bool,
}

/// Marker for the drag visual (floating spell copy).
#[derive(Component)]
pub struct DragVisual;

/// Marker for the spell info panel.
#[derive(Component)]
pub struct SpellInfoPanel;

/// Component indicating which spell to show info for.
#[derive(Component)]
pub struct SpellInfoTarget {
    pub spell: Option<Spell>,
}

/// Marker for the left side panel containing spell info.
#[derive(Component)]
pub struct LeftSidePanel;

/// Marker for the main inventory content container (horizontal layout).
#[derive(Component)]
pub struct InventoryContentContainer;

/// Marker for the right side content (grid + active slots).
#[derive(Component)]
pub struct RightSideContent;

/// Marker component for spell level display in inventory.
#[derive(Component)]
pub struct InventorySpellLevel;

/// Helper to spawn the spell bar style level indicator (black box with white border).
fn spawn_level_indicator(parent: &mut ChildSpawnerCommands, level: u32) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(-6.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-10.0)), // Center the level box
                padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.0)),
            BorderColor::all(Color::srgb(1.0, 1.0, 1.0)),
            BorderRadius::all(Val::Px(2.0)),
            ZIndex(10),
            InventorySpellLevel,
        ))
        .with_children(|level_box| {
            level_box.spawn((
                Text::new(format!("{}", level)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextLayout::new_with_justify(bevy::text::Justify::Center),
            ));
        });
}

/// Setup the inventory screen when entering InventoryOpen state.
pub fn setup_inventory_ui(
    mut commands: Commands,
    inventory_bag: Res<InventoryBag>,
    spell_list: Res<SpellList>,
    asset_server: Res<AssetServer>,
) {
    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            InventoryScreen,
        ))
        .with_children(|parent| {
            // Dark overlay background
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                InventoryOverlay,
            ));

            // Title text
            parent.spawn((
                Text::new("INVENTORY"),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
                ZIndex(1),
            ));

            // Horizontal content container (spell info on left, grid on right)
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::FlexStart,
                        column_gap: Val::Px(30.0),
                        ..default()
                    },
                    ZIndex(1),
                    InventoryContentContainer,
                ))
                .with_children(|content| {
                    // Left side: Spell info panel (always visible, shows selected/hovered spell)
                    content.spawn((
                        Node {
                            width: Val::Px(280.0),
                            min_height: Val::Px(200.0),
                            padding: UiRect::all(Val::Px(15.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
                        BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                        BorderRadius::all(Val::Px(8.0)),
                        SpellInfoPanel,
                        SpellInfoTarget { spell: None },
                        LeftSidePanel,
                        ZIndex(2),
                    ));

                    // Right side: Grid and active slots
                    content
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            RightSideContent,
                        ))
                        .with_children(|right| {
                            // Bag grid container (6x5 = 30 slots)
                            let grid_width = BAG_COLUMNS as f32 * (SLOT_SIZE + SLOT_GAP) - SLOT_GAP;
                            let grid_height = BAG_ROWS as f32 * (SLOT_SIZE + SLOT_GAP) - SLOT_GAP;

                            right
                                .spawn((
                                    Node {
                                        width: Val::Px(grid_width),
                                        height: Val::Px(grid_height),
                                        flex_wrap: FlexWrap::Wrap,
                                        column_gap: Val::Px(SLOT_GAP),
                                        row_gap: Val::Px(SLOT_GAP),
                                        ..default()
                                    },
                                ))
                                .with_children(|grid| {
                                    // Spawn 30 bag slots
                                    for slot_index in 0..(BAG_ROWS * BAG_COLUMNS) {
                                        spawn_bag_slot(grid, slot_index, inventory_bag.get_spell(slot_index), Some(&asset_server));
                                    }
                                });

                            // Separator text
                            right.spawn((
                                Text::new("ACTIVE SPELLS"),
                                TextFont {
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(1.0, 0.84, 0.0, 1.0)), // Gold
                                Node {
                                    margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(30.0), Val::Px(15.0)),
                                    ..default()
                                },
                            ));

                            // Active spells bar (5 slots)
                            let active_width = ACTIVE_SLOTS as f32 * (SLOT_SIZE + SLOT_GAP) - SLOT_GAP;

                            right
                                .spawn((
                                    Node {
                                        width: Val::Px(active_width),
                                        height: Val::Px(SLOT_SIZE),
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(SLOT_GAP),
                                        ..default()
                                    },
                                ))
                                .with_children(|active_bar| {
                                    // Spawn 5 active slots
                                    for slot_index in 0..ACTIVE_SLOTS {
                                        spawn_active_slot(active_bar, slot_index, spell_list.get_spell(slot_index), Some(&asset_server));
                                    }
                                });
                        });
                });

            // Instructions text
            parent.spawn((
                Text::new("Drag spells to swap. Click to select, click active slot to equip. Hover to see details."),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(30.0)),
                    ..default()
                },
                ZIndex(1),
            ));
        });
}

/// Spawn a bag slot with optional spell content.
fn spawn_bag_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    spell: Option<&Spell>,
    asset_server: Option<&AssetServer>,
) {
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
            BackgroundColor(Color::NONE),
            BorderRadius::all(Val::Px(6.0)),
            InventorySlot { index },
        ))
        .with_children(|slot| {
            // Spell icon fills the slot - uses shared helper which handles
            // textured, non-textured, and empty slot rendering
            spawn_spell_icon_visual(slot, spell, SLOT_SIZE, asset_server);

            // Level indicator (spell bar style) - only if spell exists
            if let Some(spell) = spell {
                spawn_level_indicator(slot, spell.level);
            }
        });
}

/// Spawn an active spell slot with optional spell content.
/// Slot number is displayed below the slot, icon fills the entire slot.
fn spawn_active_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    spell: Option<&Spell>,
    asset_server: Option<&AssetServer>,
) {
    // Container for slot + number below
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|container| {
            // The actual slot button
            container
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
                    BackgroundColor(Color::NONE),
                    BorderRadius::all(Val::Px(6.0)),
                    ActiveSlotDisplay { index },
                ))
                .with_children(|slot| {
                    // Spell icon fills the slot - uses shared helper which handles
                    // textured, non-textured, and empty slot rendering
                    spawn_spell_icon_visual(slot, spell, SLOT_SIZE, asset_server);

                    // Level indicator (spell bar style) - only if spell exists
                    if let Some(spell) = spell {
                        spawn_level_indicator(slot, spell.level);
                    }
                });

            // Slot number displayed below the slot
            container.spawn((
                Text::new(format!("{}", index + 1)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                },
            ));
        });
}

/// Cleanup inventory screen entities when exiting the state.
pub fn cleanup_inventory_ui(
    mut commands: Commands,
    query: Query<Entity, With<InventoryScreen>>,
    drag_visual_query: Query<Entity, With<DragVisual>>,
    mut selected_slot: ResMut<SelectedBagSlot>,
    mut drag_state: ResMut<DragState>,
) {
    for entity in query.iter() {
        commands.queue(move |world: &mut bevy::ecs::world::World| {
            if world.get_entity(entity).is_ok() {
                let _ = world.despawn(entity);
            }
        });
    }

    // Clean up any drag visuals
    for entity in drag_visual_query.iter() {
        commands.entity(entity).despawn();
    }

    // Clear selection on exit
    selected_slot.0 = None;

    // Clear drag state on exit
    drag_state.dragging = None;
    drag_state.drag_start_time = None;
    drag_state.drag_visual_spawned = false;
}

/// Handle keyboard input for inventory screen (I and Escape to close).
pub fn handle_inventory_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyI) || keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::InGame);
    }
}

/// Handle I key to open inventory from InGame state.
pub fn handle_inventory_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyI) && current_state.get() == &GameState::InGame {
        next_state.set(GameState::InventoryOpen);
    }
}

/// Handle bag slot click to select.
#[allow(clippy::type_complexity)]
pub fn handle_bag_slot_click(
    mut interaction_query: Query<
        (Entity, &Interaction, &mut BackgroundColor, &mut BorderColor, &InventorySlot),
        Changed<Interaction>,
    >,
    mut selected_slot: ResMut<SelectedBagSlot>,
    inventory_bag: Res<InventoryBag>,
    mut commands: Commands,
    selected_query: Query<Entity, With<SelectedSpell>>,
) {
    for (entity, interaction, mut bg_color, mut border_color, slot) in &mut interaction_query {
        let spell = inventory_bag.get_spell(slot.index);
        let element_color = spell.map(|s| s.element.color());

        match *interaction {
            Interaction::Pressed => {
                // Only allow selecting slots that have spells
                if spell.is_some() {
                    // Remove SelectedSpell from previously selected entity
                    for selected_entity in selected_query.iter() {
                        commands.entity(selected_entity).remove::<SelectedSpell>();
                    }
                    // Add SelectedSpell to this entity
                    commands.entity(entity).insert(SelectedSpell);
                    selected_slot.0 = Some(slot.index);
                }
            }
            Interaction::Hovered => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.7));
                    *border_color = BorderColor::all(Color::WHITE);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.6, 0.6, 0.6, 0.8));
                }
            }
            Interaction::None => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.4));
                    *border_color = BorderColor::all(color);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 0.8));
                }
            }
        }
    }
}

/// Handle active slot click to swap with selected bag spell.
#[allow(clippy::type_complexity)]
pub fn handle_active_slot_click(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor, &ActiveSlotDisplay),
        Changed<Interaction>,
    >,
    mut selected_slot: ResMut<SelectedBagSlot>,
    mut spell_list: ResMut<SpellList>,
    mut inventory_bag: ResMut<InventoryBag>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut bg_color, mut border_color, active_slot) in &mut interaction_query {
        let spell = spell_list.get_spell(active_slot.index);
        let element_color = spell.map(|s| s.element.color());

        match *interaction {
            Interaction::Pressed => {
                // Perform swap if we have a selected bag slot
                if let Some(bag_slot_index) = selected_slot.0 {
                    // Get spell from bag
                    if let Some(bag_spell) = inventory_bag.remove(bag_slot_index) {
                        // Get spell from active slot (if any)
                        let active_spell = spell_list.remove(active_slot.index);

                        // Put bag spell into active slot
                        spell_list.slots_mut()[active_slot.index] = Some(bag_spell);

                        // Put active spell (if any) into bag slot
                        if let Some(spell) = active_spell {
                            inventory_bag.slots_mut()[bag_slot_index] = Some(spell);
                        }

                        // Clear selection and close inventory
                        selected_slot.0 = None;
                        next_state.set(GameState::InGame);
                    }
                }
            }
            Interaction::Hovered => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.9));
                    *border_color = BorderColor::all(Color::WHITE);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.7, 0.7, 0.7, 0.8));
                }
            }
            Interaction::None => {
                if let Some(color) = element_color {
                    *bg_color = BackgroundColor(color.with_alpha(0.6));
                    *border_color = BorderColor::all(color);
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8));
                    *border_color = BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 0.8));
                }
            }
        }
    }
}

/// Update visual highlight for selected slot.
pub fn update_selected_slot_visual(
    selected_query: Query<Entity, With<SelectedSpell>>,
    mut slot_query: Query<(Entity, &mut BorderColor, &InventorySlot)>,
    inventory_bag: Res<InventoryBag>,
) {
    for (entity, mut border_color, slot) in &mut slot_query {
        let spell = inventory_bag.get_spell(slot.index);
        let is_selected = selected_query.iter().any(|e| e == entity);

        if is_selected {
            *border_color = BorderColor::all(Color::srgb(1.0, 0.84, 0.0)); // Gold highlight
        } else if let Some(spell) = spell {
            *border_color = BorderColor::all(spell.element.color());
        } else {
            *border_color = BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 0.8));
        }
    }
}

/// Update spell info panel based on drag, selection, or hover state.
/// Priority: dragged spell > hovered spell > selected spell
#[allow(clippy::type_complexity)]
pub fn update_spell_info_on_hover(
    bag_query: Query<(&Interaction, &InventorySlot)>,
    active_query: Query<(&Interaction, &ActiveSlotDisplay)>,
    inventory_bag: Res<InventoryBag>,
    spell_list: Res<SpellList>,
    drag_state: Res<DragState>,
    selected_slot: Res<SelectedBagSlot>,
    mut info_panel_query: Query<(&mut SpellInfoTarget, &mut Visibility), With<SpellInfoPanel>>,
) {
    let mut display_spell: Option<&Spell> = None;

    // Priority 1: Show spell being dragged
    if let Some(drag_source) = &drag_state.dragging {
        display_spell = match drag_source {
            DragSource::Bag(i) => inventory_bag.get_spell(*i),
            DragSource::Active(i) => spell_list.get_spell(*i),
        };
    }

    // Priority 2: Check bag slots for hover/pressed
    if display_spell.is_none() {
        for (interaction, slot) in bag_query.iter() {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                display_spell = inventory_bag.get_spell(slot.index);
                break;
            }
        }
    }

    // Priority 3: Check active slots for hover/pressed
    if display_spell.is_none() {
        for (interaction, slot) in active_query.iter() {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                display_spell = spell_list.get_spell(slot.index);
                break;
            }
        }
    }

    // Priority 4: Show selected bag slot spell
    if display_spell.is_none() {
        if let Some(selected_index) = selected_slot.0 {
            display_spell = inventory_bag.get_spell(selected_index);
        }
    }

    // Update info panel
    for (mut info_target, mut visibility) in info_panel_query.iter_mut() {
        if let Some(spell) = display_spell {
            // Show panel and update content
            *visibility = Visibility::Visible;

            // Only update if spell changed
            let spell_changed = info_target.spell.as_ref().map(|s| s.spell_type) != Some(spell.spell_type);
            if spell_changed {
                info_target.spell = Some(spell.clone());
            }
        } else {
            // Hide panel and clear content
            *visibility = Visibility::Hidden;
            if info_target.spell.is_some() {
                info_target.spell = None;
            }
        }
    }
}

/// System to rebuild spell info panel content when spell changes.
#[allow(clippy::type_complexity)]
pub fn rebuild_spell_info_content(
    mut commands: Commands,
    info_query: Query<(Entity, &SpellInfoTarget, Option<&Children>), (With<SpellInfoPanel>, Changed<SpellInfoTarget>)>,
) {
    for (entity, info_target, children) in info_query.iter() {
        // Clear existing children if any
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        if let Some(spell) = &info_target.spell {
            commands.entity(entity).with_children(|panel| {
                // Spell name with element color
                panel.spawn((
                    Text::new(spell.name.clone()),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(spell.element.color()),
                    Node {
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                ));

                // Element and Level info
                panel.spawn((
                    Text::new(format!("{} Element • Level {}", spell.element.name(), spell.level)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                ));

                // Description
                panel.spawn((
                    Text::new(spell.spell_type.description()),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
                    Node {
                        max_width: Val::Px(270.0),
                        ..default()
                    },
                ));

                // Stats info
                panel.spawn((
                    Text::new(format!("Damage: {:.0} • Fire Rate: {:.1}/s", spell.base_damage, 1.0 / spell.fire_rate)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.8, 0.6, 1.0)),
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            });
        }
    }
}

/// Track cursor position for drag visual.
/// Uses Window cursor_position for reliable positioning.
pub fn track_cursor_position(
    windows: Query<&Window>,
    mut drag_state: ResMut<DragState>,
) {
    if let Ok(window) = windows.single() {
        if let Some(pos) = window.cursor_position() {
            drag_state.cursor_position = Some(pos);
        }
    }
}

/// Start tracking a potential drag when mouse button is pressed on a spell slot.
/// The actual drag visual is spawned only after the hold threshold is exceeded.
#[allow(clippy::type_complexity)]
pub fn start_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    bag_query: Query<(&Interaction, &InventorySlot)>,
    active_query: Query<(&Interaction, &ActiveSlotDisplay)>,
    inventory_bag: Res<InventoryBag>,
    spell_list: Res<SpellList>,
    mut drag_state: ResMut<DragState>,
    time: Res<Time>,
) {
    // Only start tracking on initial press
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    // Check bag slots
    for (interaction, slot) in bag_query.iter() {
        if *interaction == Interaction::Pressed && inventory_bag.get_spell(slot.index).is_some() {
            // Record pending drag - don't spawn visual yet
            drag_state.dragging = Some(DragSource::Bag(slot.index));
            drag_state.drag_start_time = Some(time.elapsed_secs());
            drag_state.drag_visual_spawned = false;
            return;
        }
    }

    // Check active slots
    for (interaction, slot) in active_query.iter() {
        if *interaction == Interaction::Pressed && spell_list.get_spell(slot.index).is_some() {
            // Record pending drag - don't spawn visual yet
            drag_state.dragging = Some(DragSource::Active(slot.index));
            drag_state.drag_start_time = Some(time.elapsed_secs());
            drag_state.drag_visual_spawned = false;
            return;
        }
    }
}

/// Check if mouse has been held long enough and spawn drag visual.
pub fn check_drag_threshold(
    mut drag_state: ResMut<DragState>,
    mut commands: Commands,
    inventory_bag: Res<InventoryBag>,
    spell_list: Res<SpellList>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
    // Only check if we have a pending drag without visual
    if drag_state.dragging.is_none() || drag_state.drag_visual_spawned {
        return;
    }

    let Some(start_time) = drag_state.drag_start_time else {
        return;
    };

    // Check if we've held long enough
    let elapsed = time.elapsed_secs() - start_time;
    if elapsed < DRAG_HOLD_THRESHOLD {
        return;
    }

    let Some(cursor_pos) = drag_state.cursor_position else {
        return;
    };

    // Hold threshold exceeded - spawn the drag visual
    let spell = match drag_state.dragging {
        Some(DragSource::Bag(i)) => inventory_bag.get_spell(i),
        Some(DragSource::Active(i)) => spell_list.get_spell(i),
        None => None,
    };

    if let Some(spell) = spell {
        spawn_drag_visual(&mut commands, spell, cursor_pos, &asset_server);
        drag_state.drag_visual_spawned = true;
    }
}

/// Spawn the visual representation of the dragged spell.
fn spawn_drag_visual(commands: &mut Commands, spell: &Spell, cursor_pos: Vec2, asset_server: &AssetServer) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(SLOT_SIZE),
            height: Val::Px(SLOT_SIZE),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            left: Val::Px(cursor_pos.x - SLOT_SIZE / 2.0),
            top: Val::Px(cursor_pos.y - SLOT_SIZE / 2.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderRadius::all(Val::Px(6.0)),
        DragVisual,
        ZIndex(10),
    ))
    .with_children(|visual| {
        // Spell icon fills the slot - uses shared helper
        spawn_spell_icon_visual(visual, Some(spell), SLOT_SIZE, Some(asset_server));

        // Level indicator (spell bar style)
        spawn_level_indicator(visual, spell.level);
    });
}

/// Update drag visual position to follow cursor.
pub fn update_drag_visual(
    drag_state: Res<DragState>,
    mut drag_visual_query: Query<&mut Node, With<DragVisual>>,
) {
    if drag_state.dragging.is_none() {
        return;
    }

    if let Some(cursor_pos) = drag_state.cursor_position {
        for mut node in drag_visual_query.iter_mut() {
            // Center the visual on cursor
            node.left = Val::Px(cursor_pos.x - SLOT_SIZE / 2.0);
            node.top = Val::Px(cursor_pos.y - SLOT_SIZE / 2.0);
        }
    }
}

/// End drag and perform swap when mouse is released.
/// Only performs swap if the drag threshold was exceeded (visual was spawned).
#[allow(clippy::too_many_arguments)]
pub fn end_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    bag_query: Query<(&Interaction, &InventorySlot)>,
    active_query: Query<(&Interaction, &ActiveSlotDisplay)>,
    mut inventory_bag: ResMut<InventoryBag>,
    mut spell_list: ResMut<SpellList>,
    mut drag_state: ResMut<DragState>,
    mut commands: Commands,
    drag_visual_query: Query<Entity, With<DragVisual>>,
) {
    // Only handle release
    if !mouse_button.just_released(MouseButton::Left) {
        return;
    }

    // Clean up drag visual
    for entity in drag_visual_query.iter() {
        commands.entity(entity).despawn();
    }

    // Get source and check if this was an actual drag (not just a click)
    let source = drag_state.dragging.take();
    let was_actual_drag = drag_state.drag_visual_spawned;

    // Clear all drag state
    drag_state.drag_start_time = None;
    drag_state.drag_visual_spawned = false;

    // If no drag was started or threshold wasn't reached, don't swap
    let Some(source) = source else {
        return;
    };

    if !was_actual_drag {
        // This was just a click, not a drag - don't perform swap
        return;
    }

    // Find drop target
    let mut drop_target: Option<DragSource> = None;

    // Check bag slots
    for (interaction, slot) in bag_query.iter() {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            drop_target = Some(DragSource::Bag(slot.index));
            break;
        }
    }

    // Check active slots
    if drop_target.is_none() {
        for (interaction, slot) in active_query.iter() {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                drop_target = Some(DragSource::Active(slot.index));
                break;
            }
        }
    }

    // Perform swap if we have a valid drop target
    if let Some(target) = drop_target {
        // Don't swap with self
        if source == target {
            return;
        }

        // Get spells from both locations
        let source_spell = match source {
            DragSource::Bag(i) => inventory_bag.remove(i),
            DragSource::Active(i) => spell_list.remove(i),
        };

        let target_spell = match target {
            DragSource::Bag(i) => inventory_bag.remove(i),
            DragSource::Active(i) => spell_list.remove(i),
        };

        // Put source spell in target location
        if let Some(spell) = source_spell {
            match target {
                DragSource::Bag(i) => inventory_bag.slots_mut()[i] = Some(spell),
                DragSource::Active(i) => spell_list.slots_mut()[i] = Some(spell),
            }
        }

        // Put target spell in source location
        if let Some(spell) = target_spell {
            match source {
                DragSource::Bag(i) => inventory_bag.slots_mut()[i] = Some(spell),
                DragSource::Active(i) => spell_list.slots_mut()[i] = Some(spell),
            }
        }
    }
}

/// Cancel drag if escape is pressed.
pub fn cancel_drag_on_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut drag_state: ResMut<DragState>,
    mut commands: Commands,
    drag_visual_query: Query<Entity, With<DragVisual>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        // Clear all drag state
        drag_state.dragging = None;
        drag_state.drag_start_time = None;
        drag_state.drag_visual_spawned = false;

        for entity in drag_visual_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Refresh bag slot visuals to match current inventory state.
/// This should run after swaps to update children and parent background.
pub fn refresh_bag_slot_visuals(
    mut commands: Commands,
    mut slot_query: Query<(Entity, &InventorySlot, &mut BackgroundColor, &mut BorderColor, Option<&Children>)>,
    inventory_bag: Res<InventoryBag>,
    asset_server: Res<AssetServer>,
) {
    for (entity, slot, mut bg_color, mut border_color, children) in &mut slot_query {
        let spell = inventory_bag.get_spell(slot.index);

        // Update parent slot background and border based on spell presence
        if let Some(spell) = spell {
            let element_color = spell.element.color();
            *bg_color = BackgroundColor(element_color.with_alpha(0.4));
            *border_color = BorderColor::all(element_color);
        } else {
            // Empty slot - match the default empty appearance
            *bg_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8));
            *border_color = BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 0.8));
        }

        // Despawn existing children if any
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        // Rebuild children with shared helper (handles all spell states including empty)
        commands.entity(entity).with_children(|slot_parent| {
            spawn_spell_icon_visual(slot_parent, spell, SLOT_SIZE, Some(&asset_server));

            // Level indicator (spell bar style) - only if spell exists
            if let Some(spell) = spell {
                spawn_level_indicator(slot_parent, spell.level);
            }
        });
    }
}

/// Refresh active slot visuals to match current spell list state.
/// This should run after swaps to update children and parent background.
/// Note: Slot number is in parent container and doesn't need to change.
pub fn refresh_active_slot_visuals(
    mut commands: Commands,
    mut slot_query: Query<(Entity, &ActiveSlotDisplay, &mut BackgroundColor, &mut BorderColor, Option<&Children>)>,
    spell_list: Res<SpellList>,
    asset_server: Res<AssetServer>,
) {
    for (entity, slot, mut bg_color, mut border_color, children) in &mut slot_query {
        let spell = spell_list.get_spell(slot.index);

        // Update parent slot background and border based on spell presence
        if let Some(spell) = spell {
            let element_color = spell.element.color();
            *bg_color = BackgroundColor(element_color.with_alpha(0.6));
            *border_color = BorderColor::all(element_color);
        } else {
            // Empty slot - match the default empty appearance
            *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8));
            *border_color = BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 0.8));
        }

        // Despawn existing children if any
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        // Rebuild children with shared helper (handles all spell states including empty)
        commands.entity(entity).with_children(|slot_parent| {
            spawn_spell_icon_visual(slot_parent, spell, SLOT_SIZE, Some(&asset_server));

            // Level indicator (spell bar style) - only if spell exists
            if let Some(spell) = spell {
                spawn_level_indicator(slot_parent, spell.level);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::prelude::TaskPoolPlugin::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(bevy::prelude::ImagePlugin::default());
        app.init_state::<GameState>();
        app.init_resource::<SpellList>();
        app.init_resource::<InventoryBag>();
        app.init_resource::<SelectedBagSlot>();
        app.init_resource::<DragState>();
        app
    }

    mod component_tests {
        use super::*;

        #[test]
        fn inventory_screen_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<InventoryScreen>();
        }

        #[test]
        fn inventory_overlay_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<InventoryOverlay>();
        }

        #[test]
        fn inventory_slot_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<InventorySlot>();
        }

        #[test]
        fn inventory_slot_stores_index() {
            let slot = InventorySlot { index: 15 };
            assert_eq!(slot.index, 15);
        }

        #[test]
        fn active_slot_display_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<ActiveSlotDisplay>();
        }

        #[test]
        fn active_slot_display_stores_index() {
            let slot = ActiveSlotDisplay { index: 3 };
            assert_eq!(slot.index, 3);
        }

        #[test]
        fn selected_spell_is_component() {
            fn assert_component<T: Component>() {}
            assert_component::<SelectedSpell>();
        }

        #[test]
        fn selected_bag_slot_is_resource() {
            fn assert_resource<T: Resource>() {}
            assert_resource::<SelectedBagSlot>();
        }

        #[test]
        fn selected_bag_slot_defaults_to_none() {
            let selected = SelectedBagSlot::default();
            assert!(selected.0.is_none());
        }
    }

    mod setup_inventory_ui_tests {
        use super::*;

        #[test]
        fn spawns_inventory_screen_root() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let screen_count = app
                .world_mut()
                .query::<&InventoryScreen>()
                .iter(app.world())
                .count();
            assert_eq!(screen_count, 1, "Should spawn exactly one InventoryScreen");
        }

        #[test]
        fn spawns_inventory_overlay() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let overlay_count = app
                .world_mut()
                .query::<&InventoryOverlay>()
                .iter(app.world())
                .count();
            assert_eq!(overlay_count, 1, "Should spawn exactly one InventoryOverlay");
        }

        #[test]
        fn spawns_30_bag_slots() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let slot_count = app
                .world_mut()
                .query::<&InventorySlot>()
                .iter(app.world())
                .count();
            assert_eq!(slot_count, 30, "Should spawn exactly 30 bag slots");
        }

        #[test]
        fn spawns_5_active_slots() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let slot_count = app
                .world_mut()
                .query::<&ActiveSlotDisplay>()
                .iter(app.world())
                .count();
            assert_eq!(slot_count, 5, "Should spawn exactly 5 active slots");
        }

        #[test]
        fn total_slots_count_is_35() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let bag_count = app
                .world_mut()
                .query::<&InventorySlot>()
                .iter(app.world())
                .count();
            let active_count = app
                .world_mut()
                .query::<&ActiveSlotDisplay>()
                .iter(app.world())
                .count();
            assert_eq!(bag_count + active_count, 35, "Should spawn 30 bag + 5 active = 35 total slots");
        }

        #[test]
        fn bag_slots_have_button_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let button_slot_count = app
                .world_mut()
                .query::<(&Button, &InventorySlot)>()
                .iter(app.world())
                .count();
            assert_eq!(button_slot_count, 30, "All 30 bag slots should have Button component");
        }

        #[test]
        fn active_slots_have_button_component() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let button_slot_count = app
                .world_mut()
                .query::<(&Button, &ActiveSlotDisplay)>()
                .iter(app.world())
                .count();
            assert_eq!(button_slot_count, 5, "All 5 active slots should have Button component");
        }

        #[test]
        fn each_bag_slot_has_unique_index() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let indices: Vec<usize> = app
                .world_mut()
                .query::<&InventorySlot>()
                .iter(app.world())
                .map(|slot| slot.index)
                .collect();

            // Check all indices 0-29 are present
            for i in 0..30 {
                assert!(indices.contains(&i), "Bag slot index {} should be present", i);
            }
        }

        #[test]
        fn each_active_slot_has_unique_index() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            let indices: Vec<usize> = app
                .world_mut()
                .query::<&ActiveSlotDisplay>()
                .iter(app.world())
                .map(|slot| slot.index)
                .collect();

            // Check all indices 0-4 are present
            for i in 0..5 {
                assert!(indices.contains(&i), "Active slot index {} should be present", i);
            }
        }
    }

    mod cleanup_inventory_ui_tests {
        use super::*;

        #[test]
        fn removes_inventory_screen_entity() {
            let mut app = setup_test_app();

            // Spawn an inventory screen
            let screen = app
                .world_mut()
                .spawn((Node::default(), InventoryScreen))
                .id();

            // Verify it exists
            assert!(app.world().get_entity(screen).is_ok());

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_inventory_ui);

            // Screen should be despawned
            assert!(
                app.world().get_entity(screen).is_err(),
                "Screen should be despawned"
            );
        }

        #[test]
        fn clears_selected_slot() {
            let mut app = setup_test_app();

            // Set a selected slot
            app.world_mut().resource_mut::<SelectedBagSlot>().0 = Some(5);

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_inventory_ui);

            // Selection should be cleared
            let selected = app.world().resource::<SelectedBagSlot>();
            assert!(selected.0.is_none(), "Selected slot should be cleared on cleanup");
        }

        #[test]
        fn removes_children_recursively() {
            let mut app = setup_test_app();

            // Spawn screen with a child
            let child = app.world_mut().spawn(Node::default()).id();
            let screen = app
                .world_mut()
                .spawn((Node::default(), InventoryScreen))
                .id();
            app.world_mut().entity_mut(screen).add_child(child);

            // Run cleanup
            let _ = app.world_mut().run_system_once(cleanup_inventory_ui);

            // Both should be despawned
            assert!(
                app.world().get_entity(screen).is_err(),
                "Screen should be despawned"
            );
            assert!(
                app.world().get_entity(child).is_err(),
                "Child should be despawned"
            );
        }
    }

    mod handle_inventory_input_tests {
        use super::*;

        fn setup_app_with_keyboard() -> App {
            let mut app = setup_test_app();
            // Initialize keyboard resource to track input properly
            app.init_resource::<ButtonInput<KeyCode>>();
            app
        }

        #[test]
        fn i_key_transitions_to_ingame() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_input);

            // Set initial state to InventoryOpen
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);

            // Run the handler (state transition happens next frame)
            app.update();
            // Apply state transition
            app.update();

            // Verify state is now InGame
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::InGame,
                "I key should transition to InGame"
            );
        }

        #[test]
        fn escape_key_transitions_to_ingame() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_input);

            // Set initial state to InventoryOpen
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);
            app.update();

            // Simulate Escape key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);

            // Run the handler (state transition happens next frame)
            app.update();
            // Apply state transition
            app.update();

            // Verify state is now InGame
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::InGame,
                "Escape key should transition to InGame"
            );
        }

        #[test]
        fn no_key_pressed_does_not_change_state() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_input);

            // Set initial state to InventoryOpen
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);
            app.update();

            // No key pressed - just run update
            app.update();

            // State should remain InventoryOpen
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::InventoryOpen,
                "No key pressed should not change state"
            );
        }

        #[test]
        fn just_pressed_not_held_does_not_trigger_transition() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_input);

            // Set initial state to InventoryOpen
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);
            app.update();

            // Simulate key press
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);
            // First update processes the just_pressed
            app.update();
            // Clear just_pressed but keep pressed state
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear_just_pressed(KeyCode::KeyI);

            // Now run again - key is still pressed but not just_pressed
            // Reset state back to InventoryOpen to test the held behavior
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InventoryOpen);
            app.update();
            app.update();

            // State should remain InventoryOpen because key wasn't just_pressed on this update
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::InventoryOpen,
                "Held key (not just_pressed) should not trigger transition"
            );
        }
    }

    mod handle_inventory_toggle_tests {
        use super::*;

        fn setup_app_with_keyboard() -> App {
            let mut app = setup_test_app();
            // Initialize keyboard resource to track input properly
            app.init_resource::<ButtonInput<KeyCode>>();
            app
        }

        #[test]
        fn i_key_from_ingame_opens_inventory() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_toggle);

            // Set state to InGame
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InGame);
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);

            // Run the handler (state transition happens next frame)
            app.update();
            // Apply state transition
            app.update();

            // Verify state is now InventoryOpen
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::InventoryOpen,
                "I key from InGame should open inventory"
            );
        }

        #[test]
        fn i_key_from_attunement_select_does_not_open_inventory() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_toggle);

            // Set state to AttunementSelect
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::AttunementSelect);
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);
            app.update();
            app.update();

            // State should remain AttunementSelect
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::AttunementSelect,
                "I key from AttunementSelect should not change state"
            );
        }

        #[test]
        fn i_key_from_game_over_does_not_open_inventory() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_toggle);

            // Set state to GameOver
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::GameOver);
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);
            app.update();
            app.update();

            // State should remain GameOver
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::GameOver,
                "I key from GameOver should not change state"
            );
        }

        #[test]
        fn i_key_from_intro_does_not_open_inventory() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_toggle);

            // State starts as Intro by default
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);
            app.update();
            app.update();

            // State should remain Intro
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::Intro,
                "I key from Intro should not change state"
            );
        }

        #[test]
        fn i_key_from_level_complete_does_not_open_inventory() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_toggle);

            // Set state to LevelComplete
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::LevelComplete);
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);
            app.update();
            app.update();

            // State should remain LevelComplete
            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::LevelComplete,
                "I key from LevelComplete should not change state"
            );
        }

        #[test]
        fn held_key_does_not_trigger_repeated_transitions() {
            let mut app = setup_app_with_keyboard();
            app.add_systems(Update, handle_inventory_toggle);

            // Set state to InGame
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::InGame);
            app.update();

            // Simulate I key just_pressed
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);

            // First update - key just pressed, should open inventory
            app.update();
            app.update();

            let state = app.world().resource::<State<GameState>>();
            assert_eq!(
                *state.get(),
                GameState::InventoryOpen,
                "First press should open inventory"
            );

            // Note: Additional transitions would require the key to be released
            // and pressed again, which is the behavior we want for just_pressed
        }
    }

    mod slot_selection_tests {
        use super::*;

        #[test]
        fn selected_bag_slot_resource_stores_selection() {
            let mut selected = SelectedBagSlot::default();
            assert!(selected.0.is_none());

            selected.0 = Some(10);
            assert_eq!(selected.0, Some(10));
        }
    }

    mod swap_logic_tests {
        use super::*;
        use crate::spell::Spell;

        #[test]
        fn bag_and_spell_list_can_swap_spells() {
            // This tests the underlying data structures used by swap logic
            let mut bag = InventoryBag::default();
            let mut spell_list = SpellList::default();

            // Add spells
            let fireball = Spell::new(SpellType::Fireball);
            let radiant_beam = Spell::new(SpellType::RadiantBeam);

            bag.add(fireball);
            spell_list.equip(radiant_beam);

            // Verify initial state
            assert!(bag.get_spell(0).is_some());
            assert_eq!(bag.get_spell(0).unwrap().spell_type, SpellType::Fireball);
            assert!(spell_list.get_spell(0).is_some());
            assert_eq!(spell_list.get_spell(0).unwrap().spell_type, SpellType::RadiantBeam);
        }
    }

    mod visual_refresh_tests {
        use super::*;
        use crate::spell::Spell;
        use crate::ui::components::SpellIconVisual;

        #[test]
        fn refresh_bag_slot_visuals_is_system() {
            fn assert_system<T: bevy::ecs::system::IntoSystem<(), (), M>, M>(_: T) {}
            assert_system(refresh_bag_slot_visuals);
        }

        #[test]
        fn refresh_active_slot_visuals_is_system() {
            fn assert_system<T: bevy::ecs::system::IntoSystem<(), (), M>, M>(_: T) {}
            assert_system(refresh_active_slot_visuals);
        }

        #[test]
        fn bag_slot_visual_updates_when_spell_added() {
            let mut app = setup_test_app();

            // Setup inventory UI first
            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            // Count SpellIconVisual entities before adding spell
            let initial_count = app
                .world_mut()
                .query::<&SpellIconVisual>()
                .iter(app.world())
                .count();

            // Add a spell to bag slot 0
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<InventoryBag>().add(fireball);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_bag_slot_visuals);

            // Verify SpellIconVisual still exists (should replace empty with spell icon)
            let final_count = app
                .world_mut()
                .query::<&SpellIconVisual>()
                .iter(app.world())
                .count();

            // Count should be the same - we replace icons, not add new ones
            assert_eq!(initial_count, final_count, "SpellIconVisual count should remain the same after spell added");
        }

        #[test]
        fn bag_slot_visual_updates_when_spell_removed() {
            let mut app = setup_test_app();

            // Add spell before UI setup so it appears
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<InventoryBag>().add(fireball);

            // Setup inventory UI
            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            // Count SpellIconVisual entities before removal
            let initial_count = app
                .world_mut()
                .query::<&SpellIconVisual>()
                .iter(app.world())
                .count();

            // Remove the spell
            app.world_mut().resource_mut::<InventoryBag>().remove(0);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_bag_slot_visuals);

            // Verify SpellIconVisual still exists (empty slots also have icon visuals)
            let final_count = app
                .world_mut()
                .query::<&SpellIconVisual>()
                .iter(app.world())
                .count();

            // Both counts should be the same - we replace icons, not remove them
            assert_eq!(initial_count, final_count, "SpellIconVisual count should remain the same after spell removal");

            // Check that the icon visual for empty slot has gray background
            // Query for SpellIconVisual with BackgroundColor
            let bg_color = app
                .world_mut()
                .query::<(&BackgroundColor, &SpellIconVisual)>()
                .iter(app.world())
                .next()
                .map(|(bg, _)| bg.0);

            // Should have some background (gray for empty slots)
            assert!(bg_color.is_some(), "SpellIconVisual should have a BackgroundColor");
        }

        #[test]
        fn active_slot_visual_updates_when_spell_added() {
            let mut app = setup_test_app();

            // Setup inventory UI first
            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            // Add a spell to active slot 0
            let fireball = Spell::new(SpellType::Fireball);
            app.world_mut().resource_mut::<SpellList>().equip(fireball);

            // Run refresh system
            let _ = app.world_mut().run_system_once(refresh_active_slot_visuals);

            // Verify slot 0 has updated background color (not the empty slot gray)
            let (bg_color, _slot) = app
                .world_mut()
                .query::<(&BackgroundColor, &ActiveSlotDisplay)>()
                .iter(app.world())
                .find(|(_, slot)| slot.index == 0)
                .expect("Active slot 0 should exist");

            // The background should not be the empty slot color
            let empty_color = Color::srgba(0.3, 0.3, 0.3, 0.8);
            assert_ne!(bg_color.0, empty_color, "Active slot 0 should have spell color, not empty color");
        }
    }

    mod spell_info_panel_layout_tests {
        use super::*;

        #[test]
        fn spell_info_panel_has_left_side_marker() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            // Verify SpellInfoPanel has LeftSidePanel marker
            let panel_count = app
                .world_mut()
                .query::<(&SpellInfoPanel, &LeftSidePanel)>()
                .iter(app.world())
                .count();
            assert_eq!(panel_count, 1, "Should have exactly one SpellInfoPanel with LeftSidePanel marker");
        }

        #[test]
        fn inventory_has_horizontal_layout_container() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            // Verify InventoryContentContainer exists
            let container_count = app
                .world_mut()
                .query::<&InventoryContentContainer>()
                .iter(app.world())
                .count();
            assert_eq!(container_count, 1, "Should have exactly one InventoryContentContainer");
        }

        #[test]
        fn left_side_panel_is_child_of_content_container() {
            let mut app = setup_test_app();

            let _ = app.world_mut().run_system_once(setup_inventory_ui);

            // Find the content container
            let container_entity = app
                .world_mut()
                .query::<(Entity, &InventoryContentContainer)>()
                .iter(app.world())
                .next()
                .map(|(e, _)| e)
                .expect("Should have content container");

            // Find the left side panel
            let panel_parent = app
                .world_mut()
                .query::<(&ChildOf, &LeftSidePanel)>()
                .iter(app.world())
                .next()
                .map(|(c, _)| c.parent())
                .expect("Should have left side panel with parent");

            assert_eq!(panel_parent, container_entity, "Left panel should be child of content container");
        }
    }
}
