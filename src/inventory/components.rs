use bevy::prelude::*;
use crate::weapon::components::WeaponType;

#[derive(Component)]
pub struct EquippedWeapon {
    pub weapon_type: WeaponType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipped_weapon_uses_weapon_type_enum() {
        let equipped = EquippedWeapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
        };
        assert_eq!(equipped.weapon_type.id(), "pistol");
    }

    #[test]
    fn equipped_weapon_with_laser() {
        let equipped = EquippedWeapon {
            weapon_type: WeaponType::Laser,
        };
        assert_eq!(equipped.weapon_type.id(), "laser");
    }
}