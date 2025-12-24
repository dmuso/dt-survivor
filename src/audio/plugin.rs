use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use crate::states::*;
use crate::audio::systems::*;

// Audio channel types for different sound categories
#[derive(Resource)]
pub struct BackgroundMusicChannel;

#[derive(Resource)]
pub struct WeaponSoundChannel;

#[derive(Resource)]
pub struct EnemySoundChannel;

#[derive(Resource)]
pub struct LootSoundChannel;

// Struct to group audio resources and reduce function parameter count
pub struct AudioResources<'a> {
    pub asset_server: Option<Res<'a, AssetServer>>,
    pub enemy_channel: Option<ResMut<'a, AudioChannel<EnemySoundChannel>>>,
    pub sound_limiter: Option<ResMut<'a, SoundLimiter>>,
}

// Sound limiting system to prevent SoundLimitReached errors and clipping
#[derive(Resource)]
pub struct SoundLimiter {
    pub sounds_this_frame: usize,
    pub max_sounds_per_frame: usize,
    pub volume_multiplier: f32,
}

impl Default for SoundLimiter {
    fn default() -> Self {
        Self {
            sounds_this_frame: 0,
            max_sounds_per_frame: 3, // Allow max 3 sounds per frame
            volume_multiplier: 1.0, // Start at full volume
        }
    }
}

pub fn plugin(app: &mut App) {
    app
        // Add audio channels for different sound types
        .add_audio_channel::<BackgroundMusicChannel>()
        .add_audio_channel::<WeaponSoundChannel>()
        .add_audio_channel::<EnemySoundChannel>()
        .add_audio_channel::<LootSoundChannel>()
        // Add sound limiter resource
        .init_resource::<SoundLimiter>()
        .add_systems(Startup, setup_background_music)
        .add_systems(Update, (
            maintain_background_music,
            reset_sound_limiter,
            // Note: Kira handles cleanup automatically, no need for manual cleanup systems
        ).run_if(in_state(GameState::Intro).or(in_state(GameState::InGame))));
}

fn reset_sound_limiter(mut sound_limiter: ResMut<SoundLimiter>) {
    // Calculate volume multiplier based on sounds played last frame
    // This acts as a simple compressor/limiter to prevent clipping
    let sounds_played = sound_limiter.sounds_this_frame;
    sound_limiter.volume_multiplier = match sounds_played {
        0..=2 => 1.0,        // No compression for few sounds
        3 => 0.8,            // Light compression at limit (-2dB)
        4 => 0.6,            // Moderate compression (-4.4dB)
        5 => 0.4,            // Heavy compression (-8dB)
        _ => 0.3,            // Maximum compression for many sounds (-10.5dB)
    };

    // Reset counter for new frame
    sound_limiter.sounds_this_frame = 0;
}

// Helper function to play sounds with limiting and dynamic volume compression
pub fn play_limited_sound<T>(
    channel: &mut AudioChannel<T>,
    asset_server: &Res<AssetServer>,
    sound_path: &'static str,
    sound_limiter: &mut SoundLimiter,
) {
    if sound_limiter.sounds_this_frame < sound_limiter.max_sounds_per_frame {
        // Apply volume compression to prevent clipping
        // Convert linear volume multiplier to decibels
        let volume_db = if sound_limiter.volume_multiplier > 0.0 {
            20.0 * sound_limiter.volume_multiplier.log10()
        } else {
            -60.0 // Very quiet if multiplier is 0
        };
        channel.play(asset_server.load(sound_path))
            .with_volume(Decibels(volume_db));
        sound_limiter.sounds_this_frame += 1;
    }
}