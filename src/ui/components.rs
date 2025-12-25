use bevy::prelude::*;
use crate::weapon::components::WeaponType;

#[derive(Component)]
pub struct MenuButton;

#[derive(Component)]
pub struct StartGameButton;

#[derive(Component)]
pub struct ExitGameButton;

#[derive(Component)]
pub struct HealthDisplay;

#[derive(Component)]
pub struct HealthBar;

#[derive(Component)]
pub struct ScreenTint;

#[derive(Component)]
pub struct WeaponSlot {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct WeaponIcon {
    pub weapon_type: WeaponType,
}

#[derive(Component)]
pub struct WeaponTimer;

#[derive(Component)]
pub struct WeaponTimerFill;

#[derive(Component)]
pub struct WeaponLevelDisplay {
    pub weapon_type: WeaponType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_icon_uses_weapon_type_enum() {
        let icon = WeaponIcon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
        };
        assert_eq!(icon.weapon_type.id(), "pistol");
    }

    #[test]
    fn weapon_level_display_uses_weapon_type_enum() {
        let display = WeaponLevelDisplay {
            weapon_type: WeaponType::Laser,
        };
        assert_eq!(display.weapon_type.id(), "laser");
    }

    #[test]
    fn weapon_icon_compares_by_id_not_variant_data() {
        let icon1 = WeaponIcon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
        };
        let icon2 = WeaponIcon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 10, // Different values
                spread_angle: 30.0,
            },
        };
        // WeaponType equality is based on id(), so these should be equal
        assert_eq!(icon1.weapon_type, icon2.weapon_type);
    }
}