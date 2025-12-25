use bevy::prelude::*;
use crate::states::*;
use crate::game::sets::GameSet;
use crate::weapon::systems::*;

/// Re-export weapon_follow_player_system from inventory for now
/// This function is semantically about weapon behavior
pub use crate::inventory::systems::weapon_follow_player_system;

pub fn plugin(app: &mut App) {
    app
        // Movement systems - weapon follows player
        .add_systems(
            Update,
            weapon_follow_player_system
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        )
        // Weapon firing runs in PostUpdate to ensure all movement is complete
        .add_systems(
            PostUpdate,
            weapon_firing_system.run_if(in_state(GameState::InGame)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::components::Player;
    use crate::combat::components::Health;
    use crate::weapon::components::{Weapon, WeaponType};
    use crate::inventory::components::EquippedWeapon;

    #[test]
    fn test_weapon_plugin_can_be_added_to_app() {
        // Test that the weapon plugin can be added without panicking
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

        // Add the weapon plugin
        app.add_plugins(plugin);

        // Run update to verify no scheduling conflicts
        app.update();
    }

    #[test]
    fn test_weapon_follow_player_system_runs_in_game_state() {
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
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        ));

        // Create weapon entity at (0, 0)
        let weapon_entity = app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: 0.0,
            },
            EquippedWeapon { weapon_type: "pistol".to_string() },
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

        // Check that weapon moved to player position
        let weapon_transform = app.world().get::<Transform>(weapon_entity).unwrap();
        assert_eq!(
            weapon_transform.translation,
            Vec3::new(100.0, 200.0, 0.0),
            "Weapon should follow player position"
        );
    }

    #[test]
    fn test_weapon_systems_do_not_run_in_menu_state() {
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
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        ));

        // Create weapon entity at (0, 0)
        let weapon_entity = app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: 0.0,
            },
            EquippedWeapon { weapon_type: "pistol".to_string() },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Stay in Menu state (default)
        app.update();
        app.update();

        // Weapon should NOT have moved (system doesn't run in Menu state)
        let weapon_transform = app.world().get::<Transform>(weapon_entity).unwrap();
        assert_eq!(
            weapon_transform.translation,
            Vec3::new(0.0, 0.0, 0.0),
            "Weapon should not move when not in InGame state"
        );
    }
}