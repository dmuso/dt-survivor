use bevy::prelude::*;

#[derive(Component, Clone, Debug)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub fire_rate: f32,  // seconds between shots
    pub damage: f32,
    pub last_fired: f32, // timestamp
}

#[derive(Clone, Debug)]
pub enum WeaponType {
    Pistol { bullet_count: usize, spread_angle: f32 }, // Current bullet system
    Laser,       // Future: instant hit
    RocketLauncher, // Future: homing projectiles
    Bomb,        // Future: area damage
    BouncingLaser, // Future: chain damage
}