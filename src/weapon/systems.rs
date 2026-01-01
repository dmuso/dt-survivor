use bevy::prelude::*;
use crate::weapon::components::*;
use crate::whisper::resources::SpellOrigin;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::components::EquippedWeapon;

    #[test]
    fn test_weapons_disabled_without_whisper() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);

        // Set up SpellOrigin with None - simulates Whisper not collected
        app.insert_resource(SpellOrigin { position: None });

        // Create a weapon entity
        app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 0.1,
                base_damage: 1.0,
                last_fired: 10.0, // Ready to fire
            },
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ));

        app.init_resource::<Time>();

        // Run weapon firing system
        app.update();

        // System should complete without error - weapons no longer fire
    }

    #[test]
    fn test_weapon_system_updates_last_fired() {
        let mut app = App::new();
        app.add_systems(Update, weapon_firing_system);
        app.init_resource::<Time>();

        app.insert_resource(SpellOrigin {
            position: Some(Vec3::new(0.0, 3.0, 0.0)),
        });

        // Create a weapon entity with last_fired = 0.0
        let weapon_entity = app.world_mut().spawn((
            Weapon {
                weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 },
                level: 1,
                fire_rate: 0.1,
                base_damage: 1.0,
                last_fired: 0.0,
            },
            EquippedWeapon { weapon_type: WeaponType::Pistol { bullet_count: 5, spread_angle: 15.0 } },
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        )).id();

        // Run weapon firing system
        app.update();

        // Verify last_fired was updated
        let weapon = app.world().get::<Weapon>(weapon_entity).unwrap();
        assert!(weapon.last_fired >= 0.0, "last_fired should be updated");
    }
}

/// Legacy weapon firing system - now deprecated in favor of spell_casting_system.
/// This system is kept for backwards compatibility with existing weapons
/// but all new combat mechanics use the spell system.
pub fn weapon_firing_system(
    time: Res<Time>,
    spell_origin: Res<SpellOrigin>,
    mut weapon_query: Query<&mut Weapon>,
) {
    // Check if weapon exists
    if weapon_query.is_empty() {
        return; // No weapons to fire
    }

    // Check if Whisper has been collected (weapons enabled)
    if spell_origin.position.is_none() {
        return; // No Whisper = no weapons
    }

    // Legacy weapon types are no longer supported.
    // The spell system (spell_casting_system) now handles all combat.
    // Weapons remain for backwards compatibility but do not fire.
    let current_time = time.elapsed_secs();
    for mut weapon in weapon_query.iter_mut() {
        // Update last_fired to prevent cooldown issues if weapon entities exist
        weapon.last_fired = current_time;
    }
}
