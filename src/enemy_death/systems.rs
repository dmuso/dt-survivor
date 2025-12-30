use bevy::prelude::*;
use rand;
use bevy_kira_audio::AudioChannel;
use crate::audio::plugin::{*, play_limited_sound_with_volume};
use crate::enemy_death::components::*;
use crate::game::events::{EnemyDeathEvent, LootDropEvent};

pub fn enemy_death_system(
    mut enemy_death_events: MessageReader<EnemyDeathEvent>,
    mut loot_drop_events: MessageWriter<LootDropEvent>,
    time: Res<Time>,
    asset_server: Option<Res<AssetServer>>,
    mut enemy_channel: Option<ResMut<AudioChannel<EnemySoundChannel>>>,
    mut sound_limiter: Option<ResMut<SoundLimiter>>,
    mut sound_timer: ResMut<EnemyDeathSoundTimer>,
) {
    for event in enemy_death_events.read() {
        // Play enemy death sound (throttled to prevent spam)
        if sound_timer.time_since_last_sound >= 0.2 { // 200ms minimum interval
            if let (Some(asset_server), Some(enemy_channel), Some(sound_limiter)) =
                (asset_server.as_ref(), enemy_channel.as_mut(), sound_limiter.as_mut()) {
                let sound_paths = [
                    "sounds/397276__whisperbandnumber1__grunt1.wav",
                    "sounds/547200__mrfossy__voice_adultmale_paingrunts_04.wav",
                ];
                let random_index = (rand::random::<f32>() * sound_paths.len() as f32) as usize;
                play_limited_sound_with_volume(
                    enemy_channel,
                    asset_server,
                    sound_paths[random_index],
                    sound_limiter,
                    0.7, // Reduce enemy death sound volume by 30%
                );
                // Reset timer when sound is played
                sound_timer.time_since_last_sound = 0.0;
            }
        }

        // Send loot drop event to notify loot system
        loot_drop_events.write(LootDropEvent {
            position: event.position,
            enemy_level: event.enemy_level,
        });
    }

    // Update timer
    sound_timer.time_since_last_sound += time.delta_secs();
}