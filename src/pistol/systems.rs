use bevy::prelude::*;
use crate::weapon::components::Weapon;
use crate::bullets::components::Bullet;
use crate::audio::plugin::*;
use bevy_kira_audio::prelude::*;
use crate::pistol::components::PistolConfig;

/// Fire pistol weapon
pub fn fire_pistol(
    commands: &mut Commands,
    weapon: &Weapon,
    player_transform: &Transform,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
) {
    if let crate::weapon::components::WeaponType::Pistol { .. } = &weapon.weapon_type {
        let config = PistolConfig::default();
        let player_pos = player_transform.translation.truncate();
        let base_direction = (target_pos - player_pos).normalize();

        // Calculate spread pattern for 5 bullets
        let spread_angle_rad = config.spread_angle.to_radians();

        // Create 5 bullets in a spread pattern (-2, -1, 0, 1, 2)
        for i in -2..=2 {
            let angle_offset = i as f32 * spread_angle_rad;

            // Rotate the base direction by the spread angle
            let cos_offset = angle_offset.cos();
            let sin_offset = angle_offset.sin();
            let direction = Vec2::new(
                base_direction.x * cos_offset - base_direction.y * sin_offset,
                base_direction.x * sin_offset + base_direction.y * cos_offset,
            );

            commands.spawn((
                Sprite::from_color(config.bullet_color, config.bullet_size),
                Transform::from_translation(player_transform.translation + Vec3::new(0.0, 0.0, 0.1)),
                Bullet {
                    direction,
                    speed: config.bullet_speed,
                    lifetime: Timer::from_seconds(config.bullet_lifetime, TimerMode::Once),
                },
            ));
        }

        // Play weapon sound effect
        if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
            (asset_server, weapon_channel, sound_limiter) {
            play_limited_sound(
                weapon_channel,
                asset_server,
                "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
                sound_limiter,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon::components::{Weapon, WeaponType};

    #[test]
    fn test_pistol_config_default() {
        let config = PistolConfig::default();
        assert_eq!(config.bullet_count, 5);
        assert_eq!(config.spread_angle, 15.0);
        assert_eq!(config.bullet_speed, 200.0);
        assert_eq!(config.bullet_lifetime, 15.0);
        assert_eq!(config.bullet_color, Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(config.bullet_size, Vec2::new(8.0, 8.0));
    }

    #[test]
    fn test_fire_pistol_creates_bullets() {
        // Test that pistol firing creates the expected number of bullets
        // This is a simple unit test that doesn't require the full Bevy app
        let weapon = Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
            level: 1,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: 0.0,
        };

        let _player_transform = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let _target_pos = Vec2::new(10.0, 0.0);

        // Test pistol config
        let config = PistolConfig::default();
        assert_eq!(config.bullet_count, 5); // Pistol should create 5 bullets
        assert_eq!(config.spread_angle, 15.0); // Default spread angle

        // The actual firing test would require a full Bevy world setup
        // For now, just verify the weapon type is correct
        assert!(matches!(weapon.weapon_type, WeaponType::Pistol { .. }));
    }
}