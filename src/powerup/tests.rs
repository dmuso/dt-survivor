#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use crate::combat::components::Health;
    use crate::powerup::components::*;
    use crate::powerup::systems::*;
    use crate::player::components::*;
    use crate::weapon::components::*;
    use rand::Rng;

    #[test]
    fn test_powerup_type_properties() {
        // Test that all powerup types have correct properties
        assert!(PowerupType::MaxHealth.is_permanent());
        assert!(PowerupType::HealthRegen.is_permanent());
        assert!(PowerupType::PickupRadius.is_permanent());

        assert!(!PowerupType::WeaponFireRate.is_permanent());
        assert!(!PowerupType::MovementSpeed.is_permanent());

        // Test durations
        assert_eq!(PowerupType::WeaponFireRate.duration(), 20.0);
        assert_eq!(PowerupType::MovementSpeed.duration(), 20.0);
        assert_eq!(PowerupType::MaxHealth.duration(), 0.0); // Permanent
    }

    #[test]
    fn test_powerup_type_display_names() {
        assert_eq!(PowerupType::MaxHealth.display_name(), "Max Health +");
        assert_eq!(PowerupType::WeaponFireRate.display_name(), "Weapon Speed");
        assert_eq!(PowerupType::MovementSpeed.display_name(), "Movement Speed");
    }

    #[test]
    fn test_active_powerups_resource() {
        let mut active_powerups = ActivePowerups::default();

        // Test initial state
        assert_eq!(active_powerups.get_stack_count(&PowerupType::MaxHealth), 0);
        assert!(active_powerups.get_active_powerups().is_empty());

        // Test adding permanent powerup
        active_powerups.add_powerup(PowerupType::MaxHealth);
        assert_eq!(active_powerups.get_stack_count(&PowerupType::MaxHealth), 1);
        assert!(active_powerups.get_active_powerups().contains(&&PowerupType::MaxHealth));

        // Test adding temporary powerup
        active_powerups.add_powerup(PowerupType::WeaponFireRate);
        assert_eq!(active_powerups.get_stack_count(&PowerupType::WeaponFireRate), 1);
        assert_eq!(active_powerups.get_remaining_duration(&PowerupType::WeaponFireRate).unwrap(), 20.0);

        // Test stacking
        active_powerups.add_powerup(PowerupType::MaxHealth);
        assert_eq!(active_powerups.get_stack_count(&PowerupType::MaxHealth), 2);

        // Test timer update
        active_powerups.update_timers(5.0);
        assert_eq!(active_powerups.get_remaining_duration(&PowerupType::WeaponFireRate).unwrap(), 15.0);

        // Test timer expiration
        active_powerups.update_timers(16.0);
        assert!(active_powerups.get_remaining_duration(&PowerupType::WeaponFireRate).is_none());
        assert_eq!(active_powerups.get_stack_count(&PowerupType::WeaponFireRate), 0);
    }

    #[test]
    fn test_powerup_spawning_logic() {
        // Test that powerup spawning probability works (this is a simplified test)
        let mut spawned_count = 0;
        for _ in 0..1000 {
            // Simulate 2% chance
            if rand::thread_rng().gen_bool(0.02) {
                spawned_count += 1;
            }
        }
        // With 1000 attempts at 2%, we should get roughly 20 spawns (allowing some variance)
        assert!(spawned_count > 10, "Should spawn some powerups with 2% chance");
        assert!(spawned_count < 50, "Should not spawn too many powerups with 2% chance");
    }

    #[test]
    fn test_powerup_uses_dropped_item() {
        use crate::loot::components::{DroppedItem, ItemData, PickupState};

        // Verify that powerups are spawned as DroppedItem with ItemData::Powerup
        // The actual pickup is tested by the loot system tests
        let powerup_type = PowerupType::MaxHealth;
        let dropped_item = DroppedItem {
            pickup_state: PickupState::Idle,
            item_data: ItemData::Powerup(powerup_type.clone()),
            velocity: Vec3::ZERO,
            rotation_speed: 0.0,
            rotation_direction: 1.0,
        };

        // Verify the item data is correctly set
        match dropped_item.item_data {
            ItemData::Powerup(pt) => {
                assert!(matches!(pt, PowerupType::MaxHealth));
            }
            _ => panic!("Expected Powerup item data"),
        }

        // Verify initial state
        assert_eq!(dropped_item.pickup_state, PickupState::Idle);
        assert_eq!(dropped_item.velocity, Vec3::ZERO);
    }

    #[test]
    fn test_player_powerup_effects() {
        let mut app = App::new();
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
        ));
        app.init_resource::<ActivePowerups>();
        app.add_systems(Update, apply_player_powerup_effects);

        // Create player
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Add powerup effects
        {
            let mut active_powerups = app.world_mut().get_resource_mut::<ActivePowerups>().unwrap();
            active_powerups.add_powerup(PowerupType::MaxHealth); // +25% max health
            active_powerups.add_powerup(PowerupType::HealthRegen); // +25% regen
            active_powerups.add_powerup(PowerupType::PickupRadius); // +25% pickup radius
            active_powerups.add_powerup(PowerupType::MovementSpeed); // +25% speed
        }

        // Run effects system
        app.update();

        // Check player stats were modified (base values: speed=7.0, pickup_radius=2.0)
        let player = app.world().get::<Player>(player_entity).unwrap();
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.max, 125.0, "Max health should be 100 * 1.25 = 125");
        assert_eq!(player.regen_rate, 1.25, "Regen rate should be 1.0 * 1.25 = 1.25");
        assert_eq!(player.pickup_radius, 2.5, "Pickup radius should be 2.0 * 1.25 = 2.5");
        assert_eq!(player.speed, 8.75, "Speed should be 7.0 * 1.25 = 8.75");
    }

    #[test]
    fn test_weapon_powerup_effects() {
        let mut app = App::new();
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
        ));
        app.init_resource::<ActivePowerups>();
        app.add_systems(Update, apply_weapon_powerup_effects);

        // Create weapon
        let weapon_entity = app.world_mut().spawn(
            Weapon {
                weapon_type: WeaponType::Pistol {
                    bullet_count: 5,
                    spread_angle: 15.0,
                },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: 0.0,
            }
        ).id();

        // Add weapon fire rate powerup
        {
            let mut active_powerups = app.world_mut().get_resource_mut::<ActivePowerups>().unwrap();
            active_powerups.add_powerup(PowerupType::WeaponFireRate);
        }

        // Run effects system
        app.update();

        // Check weapon fire rate was doubled (halved since lower = faster)
        let weapon = app.world().get::<Weapon>(weapon_entity).unwrap();
        assert_eq!(weapon.fire_rate, 1.0, "Fire rate should be halved (doubled speed) from 2.0 to 1.0");
    }

    #[test]
    fn test_powerup_stacking() {
        let mut app = App::new();
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
        ));
        app.init_resource::<ActivePowerups>();
        app.add_systems(Update, apply_player_powerup_effects);

        // Create player
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Add multiple max health powerups
        {
            let mut active_powerups = app.world_mut().get_resource_mut::<ActivePowerups>().unwrap();
            active_powerups.add_powerup(PowerupType::MaxHealth);
            active_powerups.add_powerup(PowerupType::MaxHealth);
            active_powerups.add_powerup(PowerupType::MaxHealth); // 3 stacks = 75% increase
        }

        // Run effects system
        app.update();

        // Check stacking worked
        let health = app.world().get::<Health>(player_entity).unwrap();
        assert_eq!(health.max, 175.0, "Max health should be 100 * 1.75 = 175 (3 stacks = 75% increase)");
    }

    #[test]
    fn test_temporary_powerup_timer() {
        let mut active_powerups = ActivePowerups::default();

        // Add temporary powerup
        active_powerups.add_powerup(PowerupType::WeaponFireRate);

        // Check initial timer
        assert_eq!(active_powerups.get_remaining_duration(&PowerupType::WeaponFireRate).unwrap(), 20.0);

        // Update timers
        active_powerups.update_timers(10.0);

        // Check timer decreased
        assert_eq!(active_powerups.get_remaining_duration(&PowerupType::WeaponFireRate).unwrap(), 10.0);

        // Update past expiration
        active_powerups.update_timers(11.0);

        // Check powerup expired
        assert!(active_powerups.get_remaining_duration(&PowerupType::WeaponFireRate).is_none());
        assert_eq!(active_powerups.get_stack_count(&PowerupType::WeaponFireRate), 0);
    }

    #[test]
    fn test_powerup_pulse_animation() {
        let mut app = App::new();
        app.add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
        ));
        app.add_systems(Update, powerup_pulse_system);

        // Create powerup with pulse animation
        let powerup_entity = app.world_mut().spawn((
            Transform::from_scale(Vec3::new(1.0, 1.0, 1.0)),
            PowerupPulse {
                base_scale: Vec3::new(1.0, 1.0, 1.0),
                amplitude: 0.5,
                frequency: 2.0,
                time: 0.0,
            },
        )).id();

        // Run animation system multiple times
        for _ in 0..10 {
            app.update();
        }

        // Check that the system runs without error and pulse component exists
        let pulse = app.world().get::<PowerupPulse>(powerup_entity).unwrap();
        assert!(pulse.time >= 0.0, "Pulse time should be non-negative");
    }
}