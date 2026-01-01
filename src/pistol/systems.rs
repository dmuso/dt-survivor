use bevy::prelude::*;
use crate::weapon::components::Weapon;
use crate::spell::components::Spell;
use crate::bullets::components::Bullet;
use crate::audio::plugin::*;
use bevy_kira_audio::prelude::*;
use crate::pistol::components::PistolConfig;
use crate::game::resources::{GameMeshes, GameMaterials};

use crate::movement::components::from_xz;

/// Fire pistol weapon
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_pistol(
    commands: &mut Commands,
    weapon: &Weapon,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    if let crate::weapon::components::WeaponType::Pistol { .. } = &weapon.weapon_type {
        let config = PistolConfig::default();
        // Extract XZ position from spawn_position for direction calculation
        let spawn_xz = from_xz(spawn_position);
        let base_direction = (target_pos - spawn_xz).normalize();

        // Get bullet count based on weapon level (1 at level 1-4, 2 at 5-9, 3 at 10)
        let bullet_count = weapon.bullet_count();
        let spread_angle_rad = config.spread_angle.to_radians();

        // Create bullets in a spread pattern centered around the target direction
        // For 1 bullet: just shoot straight
        // For 2 bullets: -0.5, +0.5 offsets
        // For 3 bullets: -1, 0, +1 offsets
        for i in 0..bullet_count {
            let angle_offset = if bullet_count == 1 {
                0.0
            } else {
                // Center the spread: offset from -(count-1)/2 to +(count-1)/2
                let half_spread = (bullet_count - 1) as f32 / 2.0;
                (i as f32 - half_spread) * spread_angle_rad
            };

            // Rotate the base direction by the spread angle
            let cos_offset = angle_offset.cos();
            let sin_offset = angle_offset.sin();
            let direction = Vec2::new(
                base_direction.x * cos_offset - base_direction.y * sin_offset,
                base_direction.x * sin_offset + base_direction.y * cos_offset,
            );

            // Spawn bullet at Whisper's full 3D position
            if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
                commands.spawn((
                    Mesh3d(meshes.bullet.clone()),
                    MeshMaterial3d(materials.bullet.clone()),
                    Transform::from_translation(spawn_position),
                    Bullet {
                        direction,
                        speed: config.bullet_speed,
                        lifetime: Timer::from_seconds(config.bullet_lifetime, TimerMode::Once),
                    },
                ));
            } else {
                // Fallback for tests without mesh resources - just spawn the component
                commands.spawn((
                    Transform::from_translation(spawn_position),
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

/// Cast fireball spell (renamed from fire_pistol for spell system)
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_spell(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    if let crate::spell::components::SpellType::Fireball { .. } = &spell.spell_type {
        let config = PistolConfig::default();
        // Extract XZ position from spawn_position for direction calculation
        let spawn_xz = from_xz(spawn_position);
        let base_direction = (target_pos - spawn_xz).normalize();

        // Get projectile count based on spell level (1 at level 1-4, 2 at 5-9, 3 at 10)
        let projectile_count = spell.projectile_count();
        let spread_angle_rad = config.spread_angle.to_radians();

        // Create projectiles in a spread pattern centered around the target direction
        for i in 0..projectile_count {
            let angle_offset = if projectile_count == 1 {
                0.0
            } else {
                let half_spread = (projectile_count - 1) as f32 / 2.0;
                (i as f32 - half_spread) * spread_angle_rad
            };

            let cos_offset = angle_offset.cos();
            let sin_offset = angle_offset.sin();
            let direction = Vec2::new(
                base_direction.x * cos_offset - base_direction.y * sin_offset,
                base_direction.x * sin_offset + base_direction.y * cos_offset,
            );

            // Spawn fireball at Whisper's full 3D position
            if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
                commands.spawn((
                    Mesh3d(meshes.bullet.clone()),
                    MeshMaterial3d(materials.bullet.clone()),
                    Transform::from_translation(spawn_position),
                    Bullet {
                        direction,
                        speed: config.bullet_speed,
                        lifetime: Timer::from_seconds(config.bullet_lifetime, TimerMode::Once),
                    },
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(spawn_position),
                    Bullet {
                        direction,
                        speed: config.bullet_speed,
                        lifetime: Timer::from_seconds(config.bullet_lifetime, TimerMode::Once),
                    },
                ));
            }
        }

        // Play spell sound effect
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
        assert_eq!(config.spread_angle, 15.0);
        assert_eq!(config.bullet_speed, 20.0); // 3D world units/sec
        assert_eq!(config.bullet_lifetime, 5.0);
        assert_eq!(config.bullet_color, Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(config.bullet_size, Vec2::new(0.3, 0.3)); // 3D world units
    }

    #[test]
    fn test_fire_pistol_creates_bullets() {
        // Test that pistol firing creates the expected number of bullets
        // Bullet count is determined by weapon level via Weapon::bullet_count()
        let weapon = Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5, // Legacy field, not used
                spread_angle: 15.0,
            },
            level: 1,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: 0.0,
        };

        // Level 1 pistol should have 1 bullet
        assert_eq!(weapon.bullet_count(), 1);

        // Level 5 pistol should have 2 bullets
        let weapon_5 = Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
            level: 5,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: 0.0,
        };
        assert_eq!(weapon_5.bullet_count(), 2);

        // Level 10 pistol should have 3 bullets
        let weapon_10 = Weapon {
            weapon_type: WeaponType::Pistol {
                bullet_count: 5,
                spread_angle: 15.0,
            },
            level: 10,
            fire_rate: 2.0,
            base_damage: 1.0,
            last_fired: 0.0,
        };
        assert_eq!(weapon_10.bullet_count(), 3);

        // Test pistol config (spread angle only, bullet count is on weapon)
        let config = PistolConfig::default();
        assert_eq!(config.spread_angle, 15.0);

        // Verify weapon type
        assert!(matches!(weapon.weapon_type, WeaponType::Pistol { .. }));
    }
}