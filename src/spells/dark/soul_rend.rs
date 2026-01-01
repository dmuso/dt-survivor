use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use std::collections::HashSet;

use crate::audio::plugin::*;
use crate::combat::{DamageEvent, Health};
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Soul Rend spell
pub const SOUL_REND_SPEED: f32 = 22.0;
pub const SOUL_REND_LIFETIME: f32 = 5.0;
pub const SOUL_REND_EXECUTE_THRESHOLD: f32 = 0.5; // 50% health
pub const SOUL_REND_EXECUTE_MULTIPLIER: f32 = 2.0; // Double damage below threshold
pub const SOUL_REND_SPREAD_ANGLE: f32 = 15.0;
pub const SOUL_REND_COLLISION_RADIUS: f32 = 1.0;

/// Get the dark element color for visual effects (purple)
pub fn soul_rend_color() -> Color {
    Element::Dark.color()
}

/// SoulRendProjectile component - a projectile that deals bonus damage to low-health enemies.
/// Implements an execute mechanic where damage is multiplied if the target is below a health threshold.
#[derive(Component, Debug, Clone)]
pub struct SoulRendProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Base damage dealt to enemies above threshold
    pub base_damage: f32,
    /// Health percentage threshold for execute damage (0.0 to 1.0)
    pub execute_threshold: f32,
    /// Damage multiplier applied when target is below threshold
    pub execute_multiplier: f32,
}

impl SoulRendProjectile {
    pub fn new(
        direction: Vec2,
        speed: f32,
        lifetime_secs: f32,
        base_damage: f32,
        execute_threshold: f32,
        execute_multiplier: f32,
    ) -> Self {
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            base_damage,
            execute_threshold,
            execute_multiplier,
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(
            direction,
            SOUL_REND_SPEED,
            SOUL_REND_LIFETIME,
            spell.damage(),
            SOUL_REND_EXECUTE_THRESHOLD,
            SOUL_REND_EXECUTE_MULTIPLIER,
        )
    }

    /// Calculate final damage based on target's health percentage.
    /// Returns execute damage if target is below threshold, base damage otherwise.
    pub fn calculate_damage(&self, health_percentage: f32) -> f32 {
        if health_percentage < self.execute_threshold {
            self.base_damage * self.execute_multiplier
        } else {
            self.base_damage
        }
    }

    /// Check if the target's health percentage qualifies for execute damage
    pub fn is_execute(&self, health_percentage: f32) -> bool {
        health_percentage < self.execute_threshold
    }
}

/// Collision event for soul rend hitting an enemy
#[derive(Message)]
pub struct SoulRendEnemyCollisionEvent {
    pub soul_rend_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves soul rend projectiles
pub fn soul_rend_movement_system(
    mut soul_rend_query: Query<(&mut Transform, &SoulRendProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, soul_rend) in soul_rend_query.iter_mut() {
        let movement = soul_rend.direction * soul_rend.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles soul rend lifetime
pub fn soul_rend_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut soul_rend_query: Query<(Entity, &mut SoulRendProjectile)>,
) {
    for (entity, mut soul_rend) in soul_rend_query.iter_mut() {
        soul_rend.lifetime.tick(time.delta());

        if soul_rend.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects soul rend-enemy collisions and fires events
pub fn soul_rend_collision_detection(
    soul_rend_query: Query<(Entity, &Transform), With<SoulRendProjectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<SoulRendEnemyCollisionEvent>,
) {
    for (soul_rend_entity, soul_rend_transform) in soul_rend_query.iter() {
        let soul_rend_xz = Vec2::new(
            soul_rend_transform.translation.x,
            soul_rend_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = soul_rend_xz.distance(enemy_xz);

            if distance < SOUL_REND_COLLISION_RADIUS {
                collision_events.write(SoulRendEnemyCollisionEvent {
                    soul_rend_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per projectile
            }
        }
    }
}

/// System that applies effects when soul rend projectiles collide with enemies.
/// Checks enemy health percentage and applies execute damage if below threshold.
pub fn soul_rend_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<SoulRendEnemyCollisionEvent>,
    soul_rend_query: Query<&SoulRendProjectile>,
    enemy_health_query: Query<&Health, With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut projectiles_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32)> = Vec::new();

    for event in collision_events.read() {
        projectiles_to_despawn.insert(event.soul_rend_entity);

        // Get soul rend projectile and calculate damage based on enemy health
        if let Ok(soul_rend) = soul_rend_query.get(event.soul_rend_entity) {
            if let Ok(health) = enemy_health_query.get(event.enemy_entity) {
                let health_percentage = health.percentage();
                let damage = soul_rend.calculate_damage(health_percentage);
                effects_to_apply.push((event.enemy_entity, damage));
            }
        }
    }

    // Despawn projectiles
    for projectile_entity in projectiles_to_despawn {
        commands.entity(projectile_entity).try_despawn();
    }

    // Apply damage with Dark element
    for (enemy_entity, damage) in effects_to_apply {
        damage_events.write(DamageEvent::with_element(enemy_entity, damage, Element::Dark));
    }
}

/// Cast soul rend spell - spawns projectiles with dark element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_soul_rend(
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
    fire_soul_rend_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        asset_server,
        weapon_channel,
        sound_limiter,
        game_meshes,
        game_materials,
    );
}

/// Cast soul rend spell with explicit damage - spawns projectiles with dark element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_soul_rend_with_damage(
    commands: &mut Commands,
    spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let base_direction = (target_pos - spawn_xz).normalize();

    // Get projectile count based on spell level (1 at level 1-4, 2 at 5-9, 3 at 10)
    let projectile_count = spell.projectile_count().max(1);
    let spread_angle_rad = SOUL_REND_SPREAD_ANGLE.to_radians();

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

        let soul_rend = SoulRendProjectile::new(
            direction,
            SOUL_REND_SPEED,
            SOUL_REND_LIFETIME,
            damage,
            SOUL_REND_EXECUTE_THRESHOLD,
            SOUL_REND_EXECUTE_MULTIPLIER,
        );

        // Spawn soul rend at Whisper's full 3D position
        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.shadow_bolt.clone()), // Reuse dark material
                Transform::from_translation(spawn_position),
                soul_rend,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                soul_rend,
            ));
        }
    }

    // Play spell sound effect
    if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
        (asset_server, weapon_channel, sound_limiter)
    {
        play_limited_sound(
            weapon_channel,
            asset_server,
            "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
            sound_limiter,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod soul_rend_projectile_tests {
        use super::*;

        #[test]
        fn test_soul_rend_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let soul_rend = SoulRendProjectile::new(direction, 22.0, 5.0, 20.0, 0.5, 2.0);

            assert_eq!(soul_rend.direction, direction);
            assert_eq!(soul_rend.speed, 22.0);
            assert_eq!(soul_rend.base_damage, 20.0);
            assert_eq!(soul_rend.execute_threshold, 0.5);
            assert_eq!(soul_rend.execute_multiplier, 2.0);
        }

        #[test]
        fn test_soul_rend_from_spell() {
            let spell = Spell::new(SpellType::Oblivion);
            let direction = Vec2::new(0.0, 1.0);
            let soul_rend = SoulRendProjectile::from_spell(direction, &spell);

            assert_eq!(soul_rend.direction, direction);
            assert_eq!(soul_rend.speed, SOUL_REND_SPEED);
            assert_eq!(soul_rend.base_damage, spell.damage());
            assert_eq!(soul_rend.execute_threshold, SOUL_REND_EXECUTE_THRESHOLD);
            assert_eq!(soul_rend.execute_multiplier, SOUL_REND_EXECUTE_MULTIPLIER);
        }

        #[test]
        fn test_soul_rend_lifetime_timer() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);
            assert_eq!(soul_rend.lifetime.duration(), Duration::from_secs_f32(5.0));
            assert!(!soul_rend.lifetime.is_finished());
        }

        #[test]
        fn test_soul_rend_uses_dark_element_color() {
            let color = soul_rend_color();
            assert_eq!(color, Element::Dark.color());
            assert_eq!(color, Color::srgb_u8(128, 0, 128)); // Purple
        }
    }

    mod execute_damage_tests {
        use super::*;

        #[test]
        fn test_projectile_deals_base_damage_to_full_health_enemy() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);

            // Full health (100%)
            let damage = soul_rend.calculate_damage(1.0);
            assert_eq!(damage, 20.0, "Full health enemy should take base damage");
        }

        #[test]
        fn test_projectile_deals_execute_damage_to_low_health_enemy() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);

            // Low health (25%)
            let damage = soul_rend.calculate_damage(0.25);
            assert_eq!(damage, 40.0, "Low health enemy should take execute damage (base * 2.0)");
        }

        #[test]
        fn test_execute_threshold_boundary_at_50_percent() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);

            // Exactly at 50% - should NOT trigger execute (needs to be below)
            let damage_at_threshold = soul_rend.calculate_damage(0.5);
            assert_eq!(damage_at_threshold, 20.0, "Enemy at exactly 50% should take base damage");

            // Just below 50%
            let damage_below_threshold = soul_rend.calculate_damage(0.49);
            assert_eq!(damage_below_threshold, 40.0, "Enemy just below 50% should take execute damage");
        }

        #[test]
        fn test_execute_multiplier_correctly_scales_damage() {
            // Test with 3x multiplier
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 30.0, 0.5, 3.0);

            let damage = soul_rend.calculate_damage(0.25);
            assert_eq!(damage, 90.0, "Damage should be base * 3.0 = 90.0");
        }

        #[test]
        fn test_is_execute_returns_true_when_below_threshold() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);

            assert!(soul_rend.is_execute(0.25), "25% health should be execute");
            assert!(soul_rend.is_execute(0.49), "49% health should be execute");
        }

        #[test]
        fn test_is_execute_returns_false_when_at_or_above_threshold() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);

            assert!(!soul_rend.is_execute(0.5), "50% health should NOT be execute");
            assert!(!soul_rend.is_execute(0.75), "75% health should NOT be execute");
            assert!(!soul_rend.is_execute(1.0), "100% health should NOT be execute");
        }

        #[test]
        fn test_execute_calculation_uses_current_vs_max_health_ratio() {
            let soul_rend = SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0);

            // Simulate enemy with 30 current / 100 max = 30% health
            let health_percentage = 30.0 / 100.0;
            let damage = soul_rend.calculate_damage(health_percentage);
            assert_eq!(damage, 40.0, "30% health should trigger execute damage");

            // Simulate enemy with 60 current / 100 max = 60% health
            let health_percentage = 60.0 / 100.0;
            let damage = soul_rend.calculate_damage(health_percentage);
            assert_eq!(damage, 20.0, "60% health should NOT trigger execute damage");
        }
    }

    mod fire_soul_rend_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_soul_rend_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Oblivion);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_rend(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 projectile (Oblivion returns 0 from projectile_count, but we use max(1))
            let mut query = app.world_mut().query::<&SoulRendProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_soul_rend_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Oblivion);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_soul_rend(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulRendProjectile>();
            for soul_rend in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(soul_rend.direction.x > 0.9, "Soul Rend should move toward target");
            }
        }

        #[test]
        fn test_fire_soul_rend_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Oblivion);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_rend(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulRendProjectile>();
            for soul_rend in query.iter(app.world()) {
                assert_eq!(soul_rend.base_damage, expected_damage);
                assert_eq!(soul_rend.execute_threshold, SOUL_REND_EXECUTE_THRESHOLD);
                assert_eq!(soul_rend.execute_multiplier, SOUL_REND_EXECUTE_MULTIPLIER);
            }
        }

        #[test]
        fn test_fire_soul_rend_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Oblivion);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_rend_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulRendProjectile>();
            for soul_rend in query.iter(app.world()) {
                assert_eq!(soul_rend.base_damage, explicit_damage);
            }
        }
    }

    mod soul_rend_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_soul_rend_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create soul rend moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::new(1.0, 0.0), 100.0, 5.0, 20.0, 0.5, 2.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(soul_rend_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_soul_rend_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create soul rend moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::new(0.0, 1.0), 50.0, 5.0, 20.0, 0.5, 2.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(soul_rend_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }

        #[test]
        fn test_soul_rend_movement_at_configured_speed() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create soul rend at default speed
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::new(1.0, 0.0), SOUL_REND_SPEED, 5.0, 20.0, 0.5, 2.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(soul_rend_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, SOUL_REND_SPEED);
        }
    }

    mod soul_rend_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_soul_rend_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(6));
            }

            let _ = app.world_mut().run_system_once(soul_rend_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_soul_rend_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(3));
            }

            let _ = app.world_mut().run_system_once(soul_rend_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }

        #[test]
        fn test_soul_rend_despawns_after_max_range() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Soul rend with 5 second lifetime = max range of 22 * 5 = 110 units
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, SOUL_REND_SPEED, SOUL_REND_LIFETIME, 20.0, 0.5, 2.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_REND_LIFETIME + 1.0));
            }

            let _ = app.world_mut().run_system_once(soul_rend_lifetime_system);

            assert!(!app.world().entities().contains(entity), "Soul Rend should despawn after max range/lifetime");
        }
    }

    mod soul_rend_collision_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<SoulRendEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<SoulRendEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (soul_rend_collision_detection, count_collisions).chain());

            // Spawn soul rend at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            ));

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_collision_detection_no_event_when_far() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<SoulRendEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (soul_rend_collision_detection, count_collisions).chain());

            // Spawn soul rend at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            ));

            // Spawn enemy far away (beyond collision radius)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_collision_effects_despawns_soul_rend() {
            let mut app = setup_test_app();

            // Chain detection and effects so events are processed
            app.add_systems(
                Update,
                (soul_rend_collision_detection, soul_rend_collision_effects).chain(),
            );

            let soul_rend_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
                Health::new(100.0),
            )).id();

            app.update();

            // Soul rend should be despawned
            assert!(!app.world().entities().contains(soul_rend_entity));
            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));
        }

        #[test]
        fn test_collision_effects_deals_base_damage_to_full_health_enemy() {
            let mut app = setup_test_app();

            #[derive(Resource)]
            struct DamageRecorder(Vec<f32>);

            fn record_damage(
                mut events: MessageReader<DamageEvent>,
                mut recorder: ResMut<DamageRecorder>,
            ) {
                for event in events.read() {
                    recorder.0.push(event.amount);
                }
            }

            app.insert_resource(DamageRecorder(Vec::new()));

            app.add_systems(
                Update,
                (soul_rend_collision_detection, soul_rend_collision_effects, record_damage).chain(),
            );

            // Soul rend with base damage 20.0
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            ));

            // Enemy at full health
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
                Health::new(100.0),
            ));

            app.update();

            let recorder = app.world().resource::<DamageRecorder>();
            assert_eq!(recorder.0.len(), 1);
            assert_eq!(recorder.0[0], 20.0, "Full health enemy should take base damage");
        }

        #[test]
        fn test_collision_effects_deals_execute_damage_to_low_health_enemy() {
            let mut app = setup_test_app();

            #[derive(Resource)]
            struct DamageRecorder(Vec<f32>);

            fn record_damage(
                mut events: MessageReader<DamageEvent>,
                mut recorder: ResMut<DamageRecorder>,
            ) {
                for event in events.read() {
                    recorder.0.push(event.amount);
                }
            }

            app.insert_resource(DamageRecorder(Vec::new()));

            app.add_systems(
                Update,
                (soul_rend_collision_detection, soul_rend_collision_effects, record_damage).chain(),
            );

            // Soul rend with base damage 20.0, 2x execute multiplier
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                SoulRendProjectile::new(Vec2::X, 22.0, 5.0, 20.0, 0.5, 2.0),
            ));

            // Enemy at 30% health (below 50% threshold)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
                Health { current: 30.0, max: 100.0 },
            ));

            app.update();

            let recorder = app.world().resource::<DamageRecorder>();
            assert_eq!(recorder.0.len(), 1);
            assert_eq!(recorder.0[0], 40.0, "Low health enemy should take execute damage (base * 2.0)");
        }

        #[test]
        fn test_multiple_soul_rends_can_exist() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Oblivion);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            // Fire multiple soul rends
            for _ in 0..3 {
                let mut commands = app.world_mut().commands();
                fire_soul_rend(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            // Should have 3 soul rends
            let mut query = app.world_mut().query::<&SoulRendProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 3);
        }
    }
}
