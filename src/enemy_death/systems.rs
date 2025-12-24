use bevy::prelude::*;
use rand;
use bevy_kira_audio::AudioChannel;
use crate::audio::plugin::*;
use crate::game::events::{EnemyDeathEvent, LootDropEvent};

pub fn enemy_death_system(
    mut enemy_death_events: MessageReader<EnemyDeathEvent>,
    mut loot_drop_events: MessageWriter<LootDropEvent>,
    asset_server: Option<Res<AssetServer>>,
    mut enemy_channel: Option<ResMut<AudioChannel<EnemySoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
) {
    for event in enemy_death_events.read() {
    // Play enemy death sound
    if let (Some(asset_server), Some(enemy_channel), Some(sound_limiter)) =
        (asset_server.as_ref(), enemy_channel.as_mut(), sound_limiter.as_mut()) {
        let sound_paths = [
            "sounds/397276__whisperbandnumber1__grunt1.wav",
            "sounds/547200__mrfossy__voice_adultmale_paingrunts_04.wav",
        ];
        let random_index = (rand::random::<f32>() * sound_paths.len() as f32) as usize;
        play_limited_sound(
            enemy_channel,
            asset_server,
            sound_paths[random_index],
            sound_limiter,
        );
    }

        // Send loot drop event to notify loot system
        loot_drop_events.write(LootDropEvent {
            position: event.position,
        });
    }
}