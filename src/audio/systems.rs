use bevy::prelude::*;
use crate::audio::components::*;
use std::collections::HashSet;

/// System to setup and play background music when entering game states
pub fn setup_background_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<Entity, With<BackgroundMusic>>,
) {
    // Only spawn background music if it doesn't already exist
    if query.is_empty() {
        let music_handle: Handle<AudioSource> = asset_server.load("sounds/music/DT Survivor Upbeat.wav");

        commands.spawn((
            AudioPlayer(music_handle),
            PlaybackSettings::LOOP,
            BackgroundMusic,
        ));
    }
}

/// System to ensure music continues playing (if needed for state transitions)
pub fn maintain_background_music(
    query: Query<Entity, With<BackgroundMusic>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // If background music entity was somehow removed, restart it
    if query.is_empty() {
        let music_handle: Handle<AudioSource> = asset_server.load("sounds/music/DT Survivor Upbeat.wav");
        commands.spawn((
            AudioPlayer(music_handle),
            PlaybackSettings::LOOP,
            BackgroundMusic,
        ));
    }
}

/// System to clean up weapon sound entities after their timers expire
pub fn cleanup_weapon_sounds(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AudioCleanupTimer), With<WeaponSound>>,
) {
    let mut entities_to_despawn = HashSet::new();

    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.is_finished() {
            entities_to_despawn.insert(entity);
        }
    }

    for entity in entities_to_despawn {
        commands.entity(entity).despawn();
    }
}

/// System to clean up enemy death sound entities after their timers expire
pub fn cleanup_enemy_death_sounds(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AudioCleanupTimer), With<EnemyDeathSound>>,
) {
    let mut entities_to_despawn = HashSet::new();

    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.is_finished() {
            entities_to_despawn.insert(entity);
        }
    }

    for entity in entities_to_despawn {
        commands.entity(entity).despawn();
    }
}

/// System to clean up loot pickup sound entities after their timers expire
pub fn cleanup_loot_pickup_sounds(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AudioCleanupTimer), With<LootPickupSound>>,
) {
    let mut entities_to_despawn = HashSet::new();

    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.is_finished() {
            entities_to_despawn.insert(entity);
        }
    }

    for entity in entities_to_despawn {
        commands.entity(entity).despawn();
    }
}