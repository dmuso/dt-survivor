use bevy::prelude::*;
use crate::spell::SpellType;
use crate::weapon::components::WeaponType;

#[derive(Component)]
pub struct EquippedSpell {
    pub spell_type: SpellType,
}

/// Type alias for backward compatibility during migration
#[derive(Component)]
pub struct EquippedWeapon {
    pub weapon_type: WeaponType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipped_spell_uses_spell_type_enum() {
        let equipped = EquippedSpell {
            spell_type: SpellType::Fireball,
        };
        assert_eq!(equipped.spell_type.id(), 0);
    }

    #[test]
    fn equipped_spell_with_radiant_beam() {
        let equipped = EquippedSpell {
            spell_type: SpellType::RadiantBeam,
        };
        assert_eq!(equipped.spell_type.id(), 33);
    }

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
