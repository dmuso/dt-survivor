use bevy::prelude::*;
use crate::states::*;
use crate::bullets::systems::*;
use crate::enemies::systems::*;
use crate::game::systems::*;
use crate::inventory::systems::{inventory_initialization_system, weapon_follow_player_system};
use crate::laser::plugin as laser_plugin;
use crate::loot::plugin as loot_plugin;
use crate::rocket_launcher::plugin as rocket_launcher_plugin;
use crate::player::systems::*;
use crate::weapon::systems::*;
use crate::game::resources::{PlayerPosition, EnemySpawnState, PlayerDamageTimer, ScreenTintEffect};
use crate::game::systems::update_screen_tint_timer;
use crate::score::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<PlayerPosition>()
        .init_resource::<Score>()
        .init_resource::<EnemySpawnState>()
        .init_resource::<PlayerDamageTimer>()
        .init_resource::<ScreenTintEffect>()
        .add_plugins((laser_plugin, loot_plugin, rocket_launcher_plugin))
        .add_systems(OnEnter(GameState::InGame), (
            setup_game,
            inventory_initialization_system,
        ))
        .add_systems(Update, (
            game_input,
            player_movement,
            camera_follow_player,
            update_slow_modifiers,
            player_health_regeneration_system,
            enemy_spawning_system,
            enemy_movement_system,
            weapon_follow_player_system,
            bullet_movement_system,
            bullet_collision_system,
            bullet_lifetime_system,
            player_enemy_collision_system,
            player_death_system,
            update_screen_tint_timer,
        ).run_if(in_state(GameState::InGame)))
        .add_systems(PostUpdate, weapon_firing_system.run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game);

}