use bevy::prelude::*;

/// Marker component for background music entity
#[derive(Component)]
pub struct BackgroundMusic;

/// Marker component for weapon sound effects
#[derive(Component)]
pub struct WeaponSound;

/// Marker component for enemy death sound effects
#[derive(Component)]
pub struct EnemyDeathSound;

/// Marker component for loot pickup sound effects
#[derive(Component)]
pub struct LootPickupSound;

/// Timer component for audio entity cleanup
#[derive(Component)]
pub struct AudioCleanupTimer(pub Timer);