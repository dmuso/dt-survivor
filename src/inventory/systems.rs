use bevy::prelude::*;
use crate::inventory::resources::*;
use crate::inventory::components::*;
use crate::weapon::components::Weapon;
use crate::player::components::*;

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::combat::components::Health;
        use crate::weapon::components::WeaponType;

    #[test]
    fn test_inventory_initialization_creates_weapon_entities() {
        let mut app = App::new();
        app.add_systems(Update, inventory_initialization_system);
        app.init_resource::<Inventory>();

        // Create player without weapons
        app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Add a pistol to inventory (simulating Whisper collection)
        {
            let mut inventory = app.world_mut().get_resource_mut::<Inventory>().unwrap();
            inventory.add_or_level_weapon(Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: -2.0,
            });
        }

        // Run initialization system
        app.update();

        // Check that weapon entities were created for occupied slots
        let inventory = app.world().get_resource::<Inventory>().unwrap();
        let pistol_type = WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 };
        assert!(inventory.get_weapon(&pistol_type).is_some()); // Pistol should be in inventory
    }

    #[test]
    fn test_weapon_follow_player_system() {
        let mut app = App::new();
        app.add_systems(Update, weapon_follow_player_system);

        // Create player at (100, 200)
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            },
            Health::new(100.0),
            Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        )).id();

        // Create weapon entity at (0, 0)
        let weapon_entity = app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 2.0,
                base_damage: 1.0,
                last_fired: 0.0,
            },
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        )).id();

        // Run follow system
        app.update();

        // Check that weapon moved to player position
        let weapon_transform = app.world().get::<Transform>(weapon_entity).unwrap();
        let player_transform = app.world().get::<Transform>(player_entity).unwrap();
        assert_eq!(weapon_transform.translation, player_transform.translation);
    }

    #[test]
    fn test_inventory_weapon_management() {
        let mut app = App::new();
        // Start with empty inventory for this test
        app.insert_resource(Inventory { weapons: std::collections::HashMap::new() });

        let mut inventory = app.world_mut().get_resource_mut::<Inventory>().unwrap();

        // Add weapons of different types
        let pistol = Weapon {
            weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
            level: 1,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: 0.0,
        };

        let laser = Weapon {
            weapon_type: WeaponType::Laser,
            level: 1,
            fire_rate: 3.0,
            base_damage: 15.0,
            last_fired: 0.0,
        };

        // Add new weapons
        assert!(inventory.add_or_level_weapon(pistol.clone()));
        assert!(inventory.add_or_level_weapon(laser.clone()));

        // Check that weapons are in inventory
        let pistol_type = WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 };
        let laser_type = WeaponType::Laser;
        assert!(inventory.get_weapon(&pistol_type).is_some());
        assert!(inventory.get_weapon(&laser_type).is_some());

        // Try to add the same weapon again - should level up
        let pistol_level_2 = Weapon {
            weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
            level: 1,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: 0.0,
        };
        assert!(inventory.add_or_level_weapon(pistol_level_2));

        // Check that pistol leveled up
        let pistol_weapon = inventory.get_weapon(&pistol_type).unwrap();
        assert_eq!(pistol_weapon.level, 2);
    }
}

pub fn inventory_initialization_system(
    mut commands: Commands,
    weapon_query: Query<(), With<Weapon>>,
    inventory: Res<Inventory>,
) {
    // Only initialize if there are no weapon entities yet
    if weapon_query.is_empty() {
        // Create separate weapon entities for each weapon in inventory
        for (_weapon_id, weapon) in inventory.iter_weapons() {
            commands.spawn((
                weapon.clone(),
                EquippedWeapon { weapon_type: weapon.weapon_type.clone() },
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ));
        }
    }
}

pub fn weapon_follow_player_system(
    mut weapon_query: Query<(&mut Transform, &Weapon), Without<Player>>,
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(player_transform) = player_query.single() {
        for (mut weapon_transform, _) in weapon_query.iter_mut() {
            // Keep weapon entities positioned at the player
            weapon_transform.translation = player_transform.translation;
        }
    }
}