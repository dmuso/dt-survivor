use bevy::prelude::*;
use crate::states::*;
use crate::bullets::systems::*;
use crate::enemies::systems::*;
use crate::game::systems::*;
use crate::player::systems::*;
use crate::game::resources::{PlayerPosition, EnemySpawnState, PlayerDamageTimer, ScreenTintEffect};
use crate::ui::systems::{update_screen_tint, update_health_display};
use crate::game::systems::update_screen_tint_timer;
use crate::score::*;
use crate::bullets::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<PlayerPosition>()
        .init_resource::<BulletSpawnTimer>()
        .init_resource::<Score>()
        .init_resource::<EnemySpawnState>()
        .init_resource::<PlayerDamageTimer>()
        .init_resource::<ScreenTintEffect>()
        .add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(Update, (
            game_input,
            player_movement,
            camera_follow_player,
            update_slow_modifiers,
            player_health_regeneration_system,
            enemy_spawning_system,
            enemy_movement_system,
            bullet_spawning_system,
            bullet_movement_system,
            bullet_collision_system,
            bullet_lifetime_system,
            player_enemy_collision_system,
            player_death_system,
            update_screen_tint_timer,
            update_screen_tint,
            update_health_display,
        ).chain().run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game);

}