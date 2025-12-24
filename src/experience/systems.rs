use bevy::prelude::*;

use crate::player::components::Player;
use crate::experience::components::*;
use crate::experience::resources::*;

/// Moves experience orbs towards the player when in pickup range and handles collection
pub fn experience_orb_movement_system(
    mut commands: Commands,
    time: Res<Time>,
    mut player_query: Query<(&Transform, &mut PlayerExperience), With<Player>>,
    mut orb_query: Query<(Entity, &mut Transform, &mut ExperienceOrb), Without<Player>>,
    exp_requirements: Res<ExperienceRequirements>,
    asset_server: Option<Res<AssetServer>>,
    mut audio_channel: Option<ResMut<bevy_kira_audio::AudioChannel<crate::audio::plugin::LootSoundChannel>>>,
    mut sound_limiter: Option<ResMut<crate::audio::plugin::SoundLimiter>>,
) {
    // Get player data
    if let Ok((player_transform, mut player_exp)) = player_query.single_mut() {
        let player_pos = player_transform.translation.truncate();
        let mut orbs_to_despawn = Vec::new();

        // Process each orb
        for (entity, mut orb_transform, mut orb) in orb_query.iter_mut() {
            let orb_pos = orb_transform.translation.truncate();
            let distance = player_pos.distance(orb_pos);

            // Check if orb should be collected
            if distance < 10.0 {
                // Collect orb - add experience and check for level up
                player_exp.current += orb.value;
                let exp_for_next_level = exp_requirements.exp_required_for_level(player_exp.level + 1);
                if player_exp.current >= exp_for_next_level {
                    player_exp.level += 1;
                }

                // Play collection sound
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
            // Calculate direction to player
            let direction_to_player = (player_pos - orb_pos).normalize();

            // Distance-based acceleration and steering
            // Closer to player = faster acceleration and stronger homing
            let max_distance = player_exp.pickup_radius;
            let distance_ratio = (distance / max_distance).max(0.1).min(1.0); // Clamp between 0.1 and 1.0
            let acceleration_multiplier = 1.0 / distance_ratio; // Closer = higher multiplier

            // Base acceleration scales with distance (closer = faster)
            let base_acceleration = 800.0;
            let acceleration = base_acceleration * acceleration_multiplier;

            // Steering strength also scales with distance (closer = stronger steering)
            let base_steering = 1200.0;
            let steering_strength = base_steering * acceleration_multiplier;

            // Always apply acceleration towards player when in range
            if distance <= player_exp.pickup_radius && distance > 5.0 {
                orb.velocity += direction_to_player * acceleration * time.delta_secs();

                // Apply steering to correct direction
                let current_speed = orb.velocity.length();
                if current_speed > 0.1 {
                    let _current_direction = orb.velocity.normalize();
                    let desired_velocity = direction_to_player * current_speed;
                    let steering_vector = desired_velocity - orb.velocity;

                    // Limit steering based on distance
                    let max_steering = steering_strength * time.delta_secs();
                    let steering_magnitude = steering_vector.length();
                    let clamped_steering = if steering_magnitude > max_steering {
                        steering_vector.normalize() * max_steering
                    } else {
                        steering_vector
                    };

                    orb.velocity += clamped_steering;
                }

                // Update position
                let movement = orb.velocity * time.delta_secs();
                orb_transform.translation += movement.extend(0.0);
            }
            // Outside pickup radius - either stationary or continuing towards player
            else {
                let has_started_moving = orb.velocity.length_squared() > 1.0;

                if has_started_moving {
                    // Continue with reduced acceleration but maintain direction towards player
                    let continued_acceleration = 100.0; // Much reduced when outside radius
                    orb.velocity += direction_to_player * continued_acceleration * time.delta_secs();

                    // Apply some steering even outside radius
                    let current_speed = orb.velocity.length();
                    if current_speed > 0.1 {
                        let _current_direction = orb.velocity.normalize();
                        let desired_velocity = direction_to_player * current_speed;
                        let steering_vector = desired_velocity - orb.velocity;

                        let max_steering = 200.0 * time.delta_secs(); // Reduced steering outside radius
                        let steering_magnitude = steering_vector.length();
                        let clamped_steering = if steering_magnitude > max_steering {
                            steering_vector.normalize() * max_steering
                        } else {
                            steering_vector
                        };

                        orb.velocity += clamped_steering;
                    }

                    // Update position
                    let movement = orb.velocity * time.delta_secs();
                    orb_transform.translation += movement.extend(0.0);
                }
                // If orb hasn't started moving yet, don't move
                else {
                    // Orb stays stationary until it enters pickup radius
                }
            }
        } // end for loop

        // Despawn collected orbs
        for entity in orbs_to_despawn {
            commands.entity(entity).despawn();
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
            pickup_radius: 75.0,
            velocity: Vec2::new(10.0, 5.0),
        };
        assert_eq!(orb.value, 15);
        assert_eq!(orb.pickup_radius, 75.0);
        assert_eq!(orb.velocity, Vec2::new(10.0, 5.0));
    }

    #[test]
    fn test_player_experience_creation() {
        // Test that PlayerExperience components can be created correctly
        let exp = PlayerExperience {
            current: 25,
            level: 2,
            pickup_radius: 60.0,
        };
        assert_eq!(exp.current, 25);
        assert_eq!(exp.level, 2);
        assert_eq!(exp.pickup_radius, 60.0);
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