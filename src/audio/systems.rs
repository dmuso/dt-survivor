use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;

/// System to setup and play background music when entering game states
pub fn setup_background_music(
    background_channel: Res<AudioChannel<BackgroundMusicChannel>>,
    asset_server: Res<AssetServer>,
) {
    background_channel
        .play(asset_server.load("sounds/music/DT Survivor Upbeat.wav"))
        .looped()
        .with_volume(0.3); // Background music at 30% volume
}

/// System to ensure music continues playing (if needed for state transitions)
pub fn maintain_background_music(
    _background_channel: Res<AudioChannel<BackgroundMusicChannel>>,
) {
    // With Kira, we don't need to manually maintain audio - it handles this automatically
    // This system can be used for any additional background music logic if needed
}