use bevy::prelude::*;

#[derive(Component)]
pub struct ExperienceOrb {
    pub value: u32,
    pub velocity: Vec2,
}

#[derive(Component)]
pub struct PlayerExperience {
    pub current: u32,
    pub level: u32,
}

#[derive(Component)]
pub struct PlayerLevelDisplay;

/// Marker component for experience pickup sound effects
#[derive(Component)]
pub struct ExperiencePickupSound;

/// Timer component for experience pickup audio cleanup
#[derive(Component)]
pub struct ExperienceAudioCleanupTimer(pub Timer);