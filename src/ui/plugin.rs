use bevy::prelude::*;
use crate::game::sets::GameSet;
use crate::pause::components::SpellCooldownsVisible;
use crate::states::*;
use crate::ui::attunement::*;
use crate::ui::inventory_bag::*;
use crate::ui::materials::RadialCooldownMaterial;
use crate::ui::spell_slot::SpellSlotPlugin;
use crate::ui::systems::*;
use crate::score::*;

/// Run condition: only run if spell cooldowns are visible
fn spell_cooldowns_enabled(visible: Res<SpellCooldownsVisible>) -> bool {
    visible.0
}

pub fn plugin(app: &mut App) {
    app.add_plugins((
        UiMaterialPlugin::<RadialCooldownMaterial>::default(),
        SpellSlotPlugin,
    ))
        .init_resource::<DebugHudVisible>()
        .init_resource::<SelectedBagSlot>()
        .init_resource::<DragState>()
        .add_systems(Startup, configure_gizmos)
        .add_systems(OnEnter(GameState::Intro), setup_intro)
        .add_systems(Update, button_interactions.run_if(in_state(GameState::Intro)))
        .add_systems(OnExit(GameState::Intro), cleanup_intro)
        // Attunement selection state systems
        .add_systems(OnEnter(GameState::AttunementSelect), setup_attunement_screen)
        .add_systems(Update, handle_attunement_selection.run_if(in_state(GameState::AttunementSelect)))
        .add_systems(OnExit(GameState::AttunementSelect), cleanup_attunement_screen)
        .add_systems(OnEnter(GameState::InGame), (setup_score_display, setup_game_ui, setup_spell_slots, setup_debug_hud))
        .add_systems(Update, (
            update_score_display,
            update_health_display,
            update_screen_tint,
            update_game_level_display,
            update_kill_progress_display,
            update_xp_progress_bar,
            toggle_debug_hud,
            update_debug_hud,
            handle_inventory_toggle,
        ).run_if(in_state(GameState::InGame)))
        // Inventory state systems
        .add_systems(OnEnter(GameState::InventoryOpen), setup_inventory_ui)
        .add_systems(Update, (
            handle_inventory_input,
            handle_bag_slot_click,
            handle_active_slot_click,
            // Spell info panel systems
            update_spell_info_on_hover,
            rebuild_spell_info_content,
            // Drag and drop systems
            track_cursor_position,
            start_drag,
            check_drag_threshold,
            update_drag_visual,
            end_drag,
            cancel_drag_on_escape,
        ).run_if(in_state(GameState::InventoryOpen)))
        .add_systems(OnExit(GameState::InventoryOpen), cleanup_inventory_ui)
        .add_systems(Update,
            draw_debug_axis_gizmos
                .run_if(in_state(GameState::InGame))
                .run_if(debug_hud_enabled)
        )
        // Floating damage numbers
        .add_systems(Update, (
            spawn_floating_damage_numbers,
            update_floating_damage_numbers,
        )
            .in_set(GameSet::Effects)
            .run_if(in_state(GameState::InGame)))
        .add_systems(PostUpdate, update_spell_cooldowns
            .run_if(in_state(GameState::InGame))
            .run_if(spell_cooldowns_enabled))
        // Level Complete state systems
        .add_systems(OnEnter(GameState::LevelComplete), (
            setup_level_complete_screen,
            play_level_complete_sound,
        ))
        .add_systems(Update, (
            animate_level_complete_overlay,
            handle_continue_button,
        ).run_if(in_state(GameState::LevelComplete)))
        .add_systems(OnExit(GameState::LevelComplete), cleanup_level_complete_screen)
        // Game Over state systems
        .add_systems(OnEnter(GameState::GameOver), setup_game_over_ui)
        .add_systems(Update, game_over_input.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), cleanup_game_over);
}