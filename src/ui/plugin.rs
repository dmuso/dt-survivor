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
        .add_systems(OnEnter(GameState::InGame), (setup_score_display, setup_game_ui, setup_weapon_slots, setup_debug_hud))
        .add_systems(Update, (
            update_score_display,
            update_health_display,
            update_screen_tint,
            update_weapon_icons,
            update_weapon_level_displays,
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
        .add_systems(PostUpdate, update_weapon_slots.run_if(in_state(GameState::InGame)))
        .add_systems(OnEnter(GameState::GameOver), setup_game_over_ui)
        .add_systems(Update, game_over_input.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), cleanup_game_over);
}