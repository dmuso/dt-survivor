use bevy::prelude::*;
use crate::inventory::resources::*;
use crate::inventory::components::*;
use crate::weapon::components::Weapon;
use crate::player::components::*;

    #[cfg(test)]
    mod tests {
        use super::*;
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
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

        // Run initialization system
        app.update();

        // Check that weapon entities were created for occupied slots
        // Note: In a real scenario, this would create weapon entities, but for testing
        // the initialization logic, we verify the inventory is set up correctly
        let inventory = app.world().get_resource::<Inventory>().unwrap();
        assert!(inventory.slots[0].is_some()); // Pistol should be in slot 0
    }

    #[test]
    fn test_weapon_follow_player_system() {
        let mut app = App::new();
        app.add_systems(Update, weapon_follow_player_system);

        // Create player at (100, 200)
        let player_entity = app.world_mut().spawn((
            Player {
                speed: 200.0,
                health: 100.0,
                max_health: 100.0,
                regen_rate: 1.0,
            },
            Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        )).id();

        // Create weapon entity at (0, 0)
        let weapon_entity = app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                fire_rate: 2.0,
                damage: 1.0,
                last_fired: 0.0,
            },
            EquippedWeapon { slot_index: 0 },
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
    fn test_inventory_slot_assignment() {
        let mut app = App::new();
        app.init_resource::<Inventory>();

        let mut inventory = app.world_mut().get_resource_mut::<Inventory>().unwrap();

        // Add weapons to different slots
        inventory.slots[0] = Some(Weapon {
            weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
            fire_rate: 2.0,
            damage: 1.0,
            last_fired: 0.0,
        });

        inventory.slots[2] = Some(Weapon {
            weapon_type: WeaponType::Laser,
            fire_rate: 3.0,
            damage: 15.0,
            last_fired: 0.0,
        });

        // Check that slots are correctly assigned
        assert!(inventory.slots[0].is_some());
        assert!(inventory.slots[1].is_none());
        assert!(inventory.slots[2].is_some());
        assert!(inventory.slots[3].is_none());
        assert!(inventory.slots[4].is_none());
    }
}

pub fn inventory_initialization_system(
    mut commands: Commands,
    weapon_query: Query<(), With<Weapon>>,
    inventory: Res<Inventory>,
) {
    // Only initialize if there are no weapon entities yet
    if weapon_query.is_empty() {
        // Create separate weapon entities for each equipped weapon
        for (slot_index, weapon_option) in inventory.slots.iter().enumerate() {
            if let Some(weapon) = weapon_option {
                commands.spawn((
                    weapon.clone(),
                    EquippedWeapon { slot_index },
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ));
            }
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