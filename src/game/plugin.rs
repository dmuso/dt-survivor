use bevy::prelude::*;
use crate::states::*;
use crate::bullets::systems::*;
use crate::enemies::systems::*;
use crate::game::systems::*;
use crate::inventory::systems::{inventory_initialization_system, weapon_follow_player_system};
use crate::enemy_death::plugin as enemy_death_plugin;
use crate::laser::plugin as laser_plugin;
use crate::loot::plugin as loot_plugin;
use crate::rocket_launcher::plugin as rocket_launcher_plugin;
use crate::player::systems::*;
use crate::weapon::systems::*;
use crate::game::resources::{PlayerPosition, EnemySpawnState, PlayerDamageTimer, ScreenTintEffect};
use crate::game::systems::update_screen_tint_timer;
use crate::score::*;
use crate::game::events::{PlayerEnemyCollisionEvent, BulletEnemyCollisionEvent};

pub fn plugin(app: &mut App) {
    app.init_resource::<PlayerPosition>()
        .init_resource::<Score>()
        .init_resource::<EnemySpawnState>()
        .init_resource::<PlayerDamageTimer>()
        .init_resource::<ScreenTintEffect>()
        .add_message::<PlayerEnemyCollisionEvent>()
        .add_message::<BulletEnemyCollisionEvent>()
        .add_plugins((enemy_death_plugin, laser_plugin, loot_plugin, rocket_launcher_plugin))
        .add_systems(OnEnter(GameState::InGame), (
            setup_game,
            inventory_initialization_system,
        ))
        .add_systems(Update, (
            game_input.run_if(in_state(GameState::InGame)),
            player_movement.run_if(in_state(GameState::InGame)),
            camera_follow_player.run_if(in_state(GameState::InGame)),
            update_slow_modifiers.run_if(in_state(GameState::InGame)),
            player_health_regeneration_system.run_if(in_state(GameState::InGame)),
            enemy_spawning_system.run_if(in_state(GameState::InGame)),
            enemy_movement_system.run_if(in_state(GameState::InGame)),
            weapon_follow_player_system.run_if(in_state(GameState::InGame)),
            bullet_movement_system.run_if(in_state(GameState::InGame)),
            bullet_collision_detection.run_if(in_state(GameState::InGame)),
            bullet_collision_effects.run_if(in_state(GameState::InGame)),
            bullet_lifetime_system.run_if(in_state(GameState::InGame)),
            player_enemy_collision_detection.run_if(in_state(GameState::InGame)),
            player_enemy_damage_system.run_if(in_state(GameState::InGame)),
            player_enemy_effect_system.run_if(in_state(GameState::InGame)),
            player_death_system.run_if(in_state(GameState::InGame)),
            update_screen_tint_timer.run_if(in_state(GameState::InGame)),
        ))
        .add_systems(PostUpdate, weapon_firing_system.run_if(in_state(GameState::InGame)))
        .add_systems(OnExit(GameState::InGame), cleanup_game);

}