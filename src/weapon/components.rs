use bevy::prelude::*;

#[derive(Component, Clone, Debug)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub level: u32, // 1-10
    pub fire_rate: f32, // seconds between shots
    pub base_damage: f32, // base damage at level 1
    pub last_fired: f32, // timestamp
}

#[derive(Clone, Debug)]
pub enum WeaponType {
    Pistol {
        bullet_count: usize,
        spread_angle: f32,
    },
    Laser,
    RocketLauncher, // Future: homing projectiles
    Bomb,           // Future: area damage
    BouncingLaser,  // Future: chain damage
}

impl PartialEq for WeaponType {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for WeaponType {}

impl std::hash::Hash for WeaponType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl WeaponType {
    pub fn id(&self) -> &'static str {
        match self {
            WeaponType::Pistol { .. } => "pistol",
            WeaponType::Laser => "laser",
            WeaponType::RocketLauncher => "rocket_launcher",
            WeaponType::Bomb => "bomb",
            WeaponType::BouncingLaser => "bouncing_laser",
        }
    }
}

impl Weapon {
    pub fn damage(&self) -> f32 {
        self.base_damage * self.level as f32 * 1.25
    }

    pub fn can_level_up(&self) -> bool {
        self.level < 10
    }

    pub fn level_up(&mut self) {
        if self.can_level_up() {
            self.level += 1;
        }
    }
}
