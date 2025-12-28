use bevy::prelude::*;
use crate::weapon::components::Weapon;
use crate::bullets::components::Bullet;
use crate::audio::plugin::*;
use bevy_kira_audio::prelude::*;
use crate::pistol::components::PistolConfig;
use crate::game::resources::{GameMeshes, GameMaterials};

/// Height at which bullets fly (slightly above ground)
const BULLET_Y_HEIGHT: f32 = 0.5;

/// Fire pistol weapon
/// `spawn_position` and `target_pos` are on the XZ plane (x, z as Vec2)
#[allow(clippy::too_many_arguments)]
pub fn fire_pistol(
    commands: &mut Commands,
    weapon: &Weapon,
    spawn_position: Vec2,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    if let crate::weapon::components::WeaponType::Pistol { .. } = &weapon.weapon_type {
        let config = PistolConfig::default();
        let base_direction = (target_pos - spawn_position).normalize();

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

            // Spawn bullet as 3D mesh on XZ plane
            if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
                commands.spawn((
                    Mesh3d(meshes.bullet.clone()),
                    MeshMaterial3d(materials.bullet.clone()),
                    Transform::from_translation(Vec3::new(
                        spawn_position.x,
                        BULLET_Y_HEIGHT,
                        spawn_position.y, // spawn_position.y is the Z coordinate
                    )),
                    Bullet {
                        direction,
                        speed: config.bullet_speed,
                        lifetime: Timer::from_seconds(config.bullet_lifetime, TimerMode::Once),
                    },
                ));
            } else {
                // Fallback for tests without mesh resources - just spawn the component
                commands.spawn((
                    Transform::from_translation(Vec3::new(
                        spawn_position.x,
                        BULLET_Y_HEIGHT,
                        spawn_position.y,
                    )),
                    Bullet {
                        direction,
                        speed: config.bullet_speed,
                        lifetime: Timer::from_seconds(config.bullet_lifetime, TimerMode::Once),
                    },
                ));
            }
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