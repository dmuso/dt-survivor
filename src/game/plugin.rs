use bevy::prelude::*;
use crate::states::*;
use crate::arena::plugin as arena_plugin;
use crate::camera::plugin as camera_plugin;
use crate::enemies::systems::*;
use crate::game::systems::{
    cleanup_game, mark_fresh_game_start, player_death_system, player_enemy_collision_detection,
    player_enemy_damage_system, player_enemy_effect_system, reset_game_level, reset_level_stats_system,
    reset_survival_time, setup_game, setup_game_assets, track_enemy_kills_system,
    track_level_kills_system, track_level_xp_system, update_level_time_system,
    update_screen_tint_timer, update_survival_time,
};
use crate::game::sets::GameSet;
use crate::inventory::systems::inventory_initialization_system;
use crate::enemy_death::plugin as enemy_death_plugin;
use crate::loot::plugin as loot_plugin;
use crate::movement::plugin as movement_plugin;
use crate::player::plugin as player_plugin;
use crate::powerup::plugin as powerup_plugin;
use crate::spell::plugin as spell_plugin;
use crate::whisper::plugin as whisper_plugin;
use crate::player::systems::{camera_follow_player, update_slow_modifiers, player_health_regeneration_system};
use crate::whisper::systems::spawn_whisper_drop;
use crate::game::resources::{FreshGameStart, GameLevel, LevelStats, PlayerPosition, EnemySpawnState, PlayerDamageTimer, ScreenTintEffect, SurvivalTime};
use crate::score::*;
use crate::game::events::{PlayerEnemyCollisionEvent, GameOverEvent, GameLevelUpEvent};
use crate::spells::fire::fireball_effects::init_fireball_effects;
use crate::spells::fire::materials::{
    FireballCoreMaterial, FireballChargeMaterial, FireballChargeParticlesMaterial,
    FireballTrailMaterial, ExplosionCoreMaterial, ExplosionFireMaterial,
    ExplosionDarkImpactMaterial,
    ExplosionEmbersMaterial, ExplosionSmokeMaterial, FireballSparksMaterial,
    update_fireball_core_material_time, update_fireball_charge_material_time,
    update_fireball_trail_material_time, update_fireball_sparks_material_time,
    update_explosion_core_material_time, update_explosion_fire_material_time,
    update_explosion_embers_material_time, update_explosion_smoke_material_time,
};

pub fn plugin(app: &mut App) {
    app.add_plugins(MaterialPlugin::<FireballCoreMaterial>::default());
    app.add_plugins(MaterialPlugin::<FireballChargeMaterial>::default());
    app.add_plugins(MaterialPlugin::<FireballChargeParticlesMaterial>::default());
    app.add_plugins(MaterialPlugin::<FireballTrailMaterial>::default());
    app.add_plugins(MaterialPlugin::<FireballSparksMaterial>::default());
    app.add_plugins(MaterialPlugin::<ExplosionCoreMaterial>::default());
    app.add_plugins(MaterialPlugin::<ExplosionFireMaterial>::default());
    app.add_plugins(MaterialPlugin::<ExplosionDarkImpactMaterial>::default());
    app.add_plugins(MaterialPlugin::<ExplosionEmbersMaterial>::default());
    app.add_plugins(MaterialPlugin::<ExplosionSmokeMaterial>::default());
    app.init_resource::<PlayerPosition>()
        .init_resource::<Score>()
        .init_resource::<EnemySpawnState>()
        .init_resource::<PlayerDamageTimer>()
        .init_resource::<ScreenTintEffect>()
        .init_resource::<SurvivalTime>()
        .init_resource::<GameLevel>()
        .init_resource::<LevelStats>()
        .insert_resource(FreshGameStart::new())
        .add_message::<PlayerEnemyCollisionEvent>()
        .add_message::<GameOverEvent>()
        .add_message::<GameLevelUpEvent>()
        .add_plugins((arena_plugin, camera_plugin, enemy_death_plugin, loot_plugin, movement_plugin, player_plugin, powerup_plugin, spell_plugin, whisper_plugin))
        // Configure GameSet ordering: Input -> Movement -> Combat -> Spawning -> Effects -> Cleanup
        .configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Movement,
                GameSet::Combat,
                GameSet::Spawning,
                GameSet::Effects,
                GameSet::Cleanup,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(OnEnter(GameState::InGame), (
            setup_game_assets,
            setup_game,
            init_fireball_effects,
            spawn_whisper_drop,
            inventory_initialization_system,
            reset_survival_time,
            reset_game_level,
            reset_level_stats_system,
        ).chain())
        // Movement systems (player_movement and enemy_movement_system are in movement_plugin)
        // spell_follow_player_system is now in spell_plugin
        .add_systems(
            Update,
            camera_follow_player
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Combat systems
        .add_systems(
            Update,
            (
                player_enemy_collision_detection,
                player_enemy_damage_system,
                player_death_system,
            )
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        // Spawning systems
        .add_systems(
            Update,
            enemy_spawning_system
                .in_set(GameSet::Spawning)
                .run_if(in_state(GameState::InGame)),
        )
        // Effects systems
        .add_systems(
            Update,
            (
                player_enemy_effect_system,
                update_screen_tint_timer,
                update_slow_modifiers,
                player_health_regeneration_system,
                update_survival_time,
                track_enemy_kills_system,
                update_level_time_system,
                track_level_kills_system,
                track_level_xp_system,
                update_fireball_core_material_time,
                update_fireball_charge_material_time,
                update_fireball_trail_material_time,
                update_fireball_sparks_material_time,
                update_explosion_core_material_time,
                update_explosion_fire_material_time,
                update_explosion_embers_material_time,
                update_explosion_smoke_material_time,
            )
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        )
        // spell_casting_system is now in spell_plugin
        // Cleanup game entities only when going to Intro or GameOver (not LevelComplete)
        .add_systems(OnEnter(GameState::Intro), (mark_fresh_game_start, cleanup_game).chain())
        .add_systems(OnEnter(GameState::GameOver), (mark_fresh_game_start, cleanup_game).chain());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::resources::ArenaBounds;

    #[test]
    fn test_arena_plugin_is_included() {
        // Verify that arena_plugin is registered and provides ArenaBounds resource
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.add_plugins(arena_plugin);

        assert!(
            app.world().get_resource::<ArenaBounds>().is_some(),
            "ArenaBounds resource should be registered by arena_plugin"
        );
    }

    #[test]
    fn test_game_set_ordering_can_be_configured() {
        // Test that GameSet can be configured with chain ordering
        // This validates the pattern used in the plugin without requiring
        // full app initialization
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();

        // Configure sets with the same ordering as the plugin
        app.configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Movement,
                GameSet::Combat,
                GameSet::Spawning,
                GameSet::Effects,
                GameSet::Cleanup,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        );

        // Add test systems to verify ordering works
        fn test_input() {}
        fn test_movement() {}
        fn test_combat() {}
        fn test_spawning() {}
        fn test_effects() {}
        fn test_cleanup() {}

        app.add_systems(
            Update,
            (
                test_input.in_set(GameSet::Input),
                test_movement.in_set(GameSet::Movement),
                test_combat.in_set(GameSet::Combat),
                test_spawning.in_set(GameSet::Spawning),
                test_effects.in_set(GameSet::Effects),
                test_cleanup.in_set(GameSet::Cleanup),
            ),
        );

        // Transition to InGame and verify no scheduling conflicts
        app.world_mut()
            .get_resource_mut::<bevy::state::state::NextState<GameState>>()
            .unwrap()
            .set(GameState::InGame);
        app.update();
        app.update();
    }

    #[test]
    fn test_systems_can_be_added_to_game_sets() {
        // Test that systems can be added to each GameSet
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();

        // Configure sets
        app.configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Movement,
                GameSet::Combat,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        );

        // Verify multiple systems can be in the same set
        fn system_a() {}
        fn system_b() {}
        fn system_c() {}

        app.add_systems(
            Update,
            (system_a, system_b).in_set(GameSet::Movement),
        );
        app.add_systems(
            Update,
            system_c.in_set(GameSet::Combat),
        );

        // Transition to InGame and verify no scheduling conflicts
        app.world_mut()
            .get_resource_mut::<bevy::state::state::NextState<GameState>>()
            .unwrap()
            .set(GameState::InGame);
        app.update();
    }
}