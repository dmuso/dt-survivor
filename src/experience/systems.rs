use bevy::prelude::*;

use crate::experience::components::*;
use crate::player::components::Player;

/// Updates experience orb positions based on velocity (attraction)
/// Movement is on XZ plane (3D ground plane) with Y as height
pub fn experience_orb_movement_system(
    time: Res<Time>,
    mut orb_query: Query<(&mut Transform, &ExperienceOrb)>,
) {
    for (mut transform, orb) in orb_query.iter_mut() {
        let movement = orb.velocity * time.delta_secs();
        // Movement is on XZ plane: velocity.x -> translation.x, velocity.y -> translation.z
        transform.translation.x += movement.x;
        transform.translation.z += movement.y;
    }
}

/// Handles collection of experience orbs when they touch the player
pub fn experience_orb_collection_system(
    mut commands: Commands,
    mut player_query: Query<(&Transform, &mut PlayerExperience), With<Player>>,
    orb_query: Query<(Entity, &Transform, &ExperienceOrb)>,
    asset_server: Option<Res<AssetServer>>,
    mut audio_channel: Option<
        ResMut<bevy_kira_audio::AudioChannel<crate::audio::plugin::LootSoundChannel>>,
    >,
    mut sound_limiter: Option<ResMut<crate::audio::plugin::SoundLimiter>>,
    mut level_up_writer: MessageWriter<PlayerLevelUpEvent>,
) {
    // Get player data
    if let Ok((player_transform, mut player_exp)) = player_query.single_mut() {
        // Use XZ plane for distance calculation (3D ground plane)
        let player_pos_xz = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.z,
        );
        let mut orbs_to_despawn = Vec::new();

        // Process each orb
        for (entity, orb_transform, orb) in orb_query.iter() {
            let orb_pos_xz = Vec2::new(
                orb_transform.translation.x,
                orb_transform.translation.z,
            );
            let distance = player_pos_xz.distance(orb_pos_xz);

            // Check if orb should be collected
            if distance < 10.0 {
                // Collect orb - add experience and check for level up
                let levels_gained = player_exp.add_xp(orb.value);

                if levels_gained > 0 {
                    level_up_writer.write(PlayerLevelUpEvent {
                        new_level: player_exp.level,
                        levels_gained,
                    });
                }

                if let (Some(asset_server), Some(audio_channel), Some(sound_limiter)) =
                    (asset_server.as_ref(), audio_channel.as_mut(), sound_limiter.as_mut())
                {
                    crate::audio::plugin::play_limited_sound(
                        audio_channel,
                        asset_server,
                        "sounds/366104__original_sound__confirmation-downward.wav",
                        sound_limiter,
                    );
                }

                orbs_to_despawn.push(entity);
            }
        }

        // Despawn collected orbs
        for entity in orbs_to_despawn {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Updates the player level display in the UI
pub fn update_player_level_display_system(
    player_query: Query<&PlayerExperience, With<Player>>,
    mut text_query: Query<&mut Text, With<PlayerLevelDisplay>>,
) {
    if let Ok(player_exp) = player_query.single() {
        for mut text in text_query.iter_mut() {
            *text = Text::new(format!("Lv. {}", player_exp.level));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experience_orb_creation() {
        // Test that ExperienceOrb components can be created correctly
        let orb = ExperienceOrb {
            value: 15,
            velocity: Vec2::new(10.0, 5.0),
        };
        assert_eq!(orb.value, 15);
        assert_eq!(orb.velocity, Vec2::new(10.0, 5.0));
    }

    #[test]
    fn test_player_experience_creation() {
        // Test that PlayerExperience components can be created correctly
        let exp = PlayerExperience::new();
        assert_eq!(exp.current, 0);
        assert_eq!(exp.level, 1);
        assert_eq!(exp.total_xp, 0);
    }

    #[test]
    fn test_player_experience_with_custom_values() {
        let mut exp = PlayerExperience::new();
        exp.current = 25;
        exp.level = 2;
        exp.total_xp = 125;
        assert_eq!(exp.current, 25);
        assert_eq!(exp.level, 2);
        assert_eq!(exp.total_xp, 125);
    }
}