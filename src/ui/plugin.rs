use bevy::prelude::*;
use crate::states::*;
use crate::ui::systems::*;
use crate::score::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<DebugHudVisible>()
        .add_systems(Startup, configure_gizmos)
        .add_systems(OnEnter(GameState::Intro), setup_intro)
        .add_systems(Update, button_interactions.run_if(in_state(GameState::Intro)))
        .add_systems(OnExit(GameState::Intro), cleanup_intro)
        .add_systems(OnEnter(GameState::InGame), (setup_score_display, setup_game_ui, setup_spell_slots, setup_debug_hud))
        .add_systems(Update, (
            update_score_display,
            update_health_display,
            update_screen_tint,
            update_spell_icons,
            update_spell_slot_backgrounds,
            update_spell_level_displays,
            update_game_level_display,
            update_kill_progress_display,
            update_xp_progress_bar,
            toggle_debug_hud,
            update_debug_hud,
        ).run_if(in_state(GameState::InGame)))
        .add_systems(Update,
            draw_debug_axis_gizmos
                .run_if(in_state(GameState::InGame))
                .run_if(debug_hud_enabled)
        )
        .add_systems(PostUpdate, update_spell_cooldowns.run_if(in_state(GameState::InGame)))
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