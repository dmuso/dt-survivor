use bevy::prelude::*;

#[derive(Component)]
pub struct ExperienceOrb {
    pub value: u32,
    pub pickup_radius: f32,
    pub velocity: Vec2,
}

#[derive(Component)]
pub struct PlayerExperience {
    pub current: u32,
    pub level: u32,
    pub pickup_radius: f32, // Radius within which orbs are attracted to player
}

#[derive(Component)]
pub struct PlayerLevelDisplay;

/// Marker component for experience pickup sound effects
#[derive(Component)]
pub struct ExperiencePickupSound;

/// Timer component for experience pickup audio cleanup
#[derive(Component)]
pub struct ExperienceAudioCleanupTimer(pub Timer);