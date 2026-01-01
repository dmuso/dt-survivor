use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::inventory::resources::SpellList;
use crate::states::*;
use crate::game::events::FireballEnemyCollisionEvent;
use crate::game::sets::GameSet;
use crate::spell::systems::*;
use crate::spells::fire::fireball::{
    burn_damage_system, fireball_collision_detection, fireball_collision_effects,
    fireball_lifetime_system, fireball_movement_system,
};
use crate::spells::light::radiant_beam::{
    radiant_beam_collision_system, render_radiant_beams, update_radiant_beams,
};
use crate::spells::fire::fire_nova::{
    fire_nova_cleanup_system, fire_nova_collision_system, fire_nova_expansion_system,
    fire_nova_visual_system,
};
use crate::spells::lightning::chain_lightning::{
    chain_lightning_arc_cleanup_system, chain_lightning_hit_system, chain_lightning_movement_system,
};
use crate::spells::lightning::thunder_strike::{
    thunder_strike_damage_system, update_thunder_strike_markers, update_thunder_strikes,
};
use crate::spells::poison::poison_cloud::{
    poison_cloud_cleanup_system, poison_cloud_damage_system,
    poison_cloud_projectile_movement_system, poison_cloud_spawn_zone_system,
};
use crate::spells::frost::ice_shard::{
    ice_shard_collision_detection, ice_shard_collision_effects,
    ice_shard_lifetime_system, ice_shard_movement_system, slowed_debuff_system,
    IceShardEnemyCollisionEvent,
};
use crate::whisper::resources::{SpellOrigin, WhisperAttunement};

/// Re-export spell_follow_player_system from inventory for now
/// This function is semantically about spell behavior
pub use crate::inventory::systems::spell_follow_player_system;

pub fn plugin(app: &mut App) {
    app
        // Register DamageEvent for burn damage system (safe to call multiple times)
        .add_message::<DamageEvent>()
        // Ensure SpellOrigin resource exists (initialized by whisper plugin, but ensure it here too)
        .init_resource::<SpellOrigin>()
        // Initialize SpellList resource for equipped spells
        .init_resource::<SpellList>()
        // Initialize WhisperAttunement for elemental damage bonus
        .init_resource::<WhisperAttunement>()
        // Register spell collision events
        .add_message::<FireballEnemyCollisionEvent>()
        .add_message::<IceShardEnemyCollisionEvent>()
        // Movement systems - spell follows player
        .add_systems(
            Update,
            spell_follow_player_system
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Spell casting runs in PostUpdate to ensure all movement is complete
        .add_systems(
            PostUpdate,
            spell_casting_system.run_if(in_state(GameState::InGame)),
        )
        // Fireball movement and lifetime systems
        .add_systems(
            Update,
            (
                fireball_movement_system,
                fireball_lifetime_system,
            )
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Fireball collision detection and effects
        .add_systems(
            Update,
            (
                fireball_collision_detection,
                fireball_collision_effects,
            )
                .chain()
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        // Burn damage over time system
        .add_systems(
            Update,
            burn_damage_system
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        )
        // Radiant beam systems
        .add_systems(
            Update,
            (
                radiant_beam_collision_system,
                update_radiant_beams,
                render_radiant_beams,
            )
                .chain()
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        // Thunder strike systems
        .add_systems(
            Update,
            (
                update_thunder_strike_markers,
                thunder_strike_damage_system,
                update_thunder_strikes,
            )
                .chain()
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        // Chain lightning systems - movement in Movement, hit detection in Combat, cleanup in Cleanup
        .add_systems(
            Update,
            chain_lightning_movement_system
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            chain_lightning_hit_system
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            chain_lightning_arc_cleanup_system
                .in_set(GameSet::Cleanup)
                .run_if(in_state(GameState::InGame)),
        )
        // Fire nova systems - expansion in Movement, collision and cleanup in Combat, visual in Effects
        .add_systems(
            Update,
            fire_nova_expansion_system
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            (
                fire_nova_collision_system,
                fire_nova_cleanup_system,
            )
                .chain()
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            fire_nova_visual_system
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        )
        // Poison cloud systems - projectile movement in Movement, spawn zone and damage in Combat, cleanup in Cleanup
        .add_systems(
            Update,
            poison_cloud_projectile_movement_system
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            (
                poison_cloud_spawn_zone_system,
                poison_cloud_damage_system,
            )
                .chain()
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            poison_cloud_cleanup_system
                .in_set(GameSet::Cleanup)
                .run_if(in_state(GameState::InGame)),
        )
        // Ice shard systems - movement and lifetime in Movement, collision in Combat, debuff in Effects
        .add_systems(
            Update,
            (
                ice_shard_movement_system,
                ice_shard_lifetime_system,
            )
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            (
                ice_shard_collision_detection,
                ice_shard_collision_effects,
            )
                .chain()
                .in_set(GameSet::Combat)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            slowed_debuff_system
                .in_set(GameSet::Effects)
                .run_if(in_state(GameState::InGame)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::components::Player;
    use crate::combat::components::Health;
    use crate::spell::{Spell, SpellType};
    use crate::inventory::components::EquippedSpell;

    #[test]
    fn test_spell_plugin_can_be_added_to_app() {
        // Test that the spell plugin can be added without panicking
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();

        // Configure GameSet ordering (normally done by game plugin)
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

        // Add the spell plugin
        app.add_plugins(plugin);

        // Run update to verify no scheduling conflicts
        app.update();
    }

    #[test]
    fn test_spell_follow_player_system_runs_in_game_state() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();

        // Configure GameSet ordering
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

        app.add_plugins(plugin);

        // Create player at (100, 200)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        ));

        // Create spell entity at (0, 0)
        let spell_entity = app.world_mut().spawn((
            Spell::new(SpellType::Fireball),
            EquippedSpell { spell_type: SpellType::Fireball },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Transition to InGame state
        app.world_mut()
            .get_resource_mut::<bevy::state::state::NextState<GameState>>()
            .unwrap()
            .set(GameState::InGame);

        // Run multiple updates to process state transition
        app.update();
        app.update();

        // Check that spell moved to player position
        let spell_transform = app.world().get::<Transform>(spell_entity).unwrap();
        assert_eq!(
            spell_transform.translation,
            Vec3::new(100.0, 200.0, 0.0),
            "Spell should follow player position"
        );
    }

    #[test]
    fn test_spell_systems_do_not_run_in_menu_state() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();

        // Configure GameSet ordering
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

        app.add_plugins(plugin);

        // Create player at (100, 200)
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        ));

        // Create spell entity at (0, 0)
        let spell_entity = app.world_mut().spawn((
            Spell::new(SpellType::Fireball),
            EquippedSpell { spell_type: SpellType::Fireball },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Stay in Menu state (default)
        app.update();
        app.update();

        // Spell should NOT have moved (system doesn't run in Menu state)
        let spell_transform = app.world().get::<Transform>(spell_entity).unwrap();
        assert_eq!(
            spell_transform.translation,
            Vec3::new(0.0, 0.0, 0.0),
            "Spell should not move when not in InGame state"
        );
    }
}
