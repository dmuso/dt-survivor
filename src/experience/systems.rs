use bevy::prelude::*;

use crate::player::components::Player;
use crate::experience::components::*;
use crate::experience::resources::*;

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
    exp_requirements: Res<ExperienceRequirements>,
    asset_server: Option<Res<AssetServer>>,
    mut audio_channel: Option<ResMut<bevy_kira_audio::AudioChannel<crate::audio::plugin::LootSoundChannel>>>,
    mut sound_limiter: Option<ResMut<crate::audio::plugin::SoundLimiter>>,
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
                player_exp.current += orb.value;
                let exp_for_next_level = exp_requirements.exp_required_for_level(player_exp.level + 1);
                if player_exp.current >= exp_for_next_level {
                    player_exp.level += 1;
                }
                if let (Some(asset_server), Some(audio_channel), Some(sound_limiter)) =
                    (asset_server.as_ref(), audio_channel.as_mut(), sound_limiter.as_mut()) {
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
        let exp = PlayerExperience {
            current: 25,
            level: 2,
        };
        assert_eq!(exp.current, 25);
        assert_eq!(exp.level, 2);
    }

    #[test]
    fn test_experience_requirements_calculation() {
        let requirements = ExperienceRequirements::default();

        // Test level 1 (no experience required)
        assert_eq!(requirements.exp_required_for_level(1), 0);

        // Test level 2 (should require some experience)
        let level2_req = requirements.exp_required_for_level(2);
        assert!(level2_req > 0, "Level 2 should require experience");

        // Test that higher levels require more experience
        let level3_req = requirements.exp_required_for_level(3);
        assert!(level3_req > level2_req, "Higher levels should require more experience");
    }
}