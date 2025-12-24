use bevy::prelude::*;

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
    pub weapon_type: String, // weapon type identifier
}

#[derive(Component)]
pub struct WeaponTimer;

#[derive(Component)]
pub struct WeaponTimerFill;

#[derive(Component)]
pub struct WeaponLevelDisplay {
    pub weapon_type: String,
}