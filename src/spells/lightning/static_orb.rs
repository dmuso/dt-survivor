use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::frost::ice_shard::SlowedDebuff;

/// Default configuration for Static Orb spell
pub const STATIC_ORB_ZAP_INTERVAL: f32 = 0.5;
pub const STATIC_ORB_ZAP_RANGE: f32 = 8.0;
pub const STATIC_ORB_DURATION: f32 = 5.0;

/// Slow effect configuration for Static Orb
pub const STATIC_ORB_SLOW_DURATION: f32 = 1.5;
pub const STATIC_ORB_SLOW_MULTIPLIER: f32 = 0.6; // 40% slow

/// Get the lightning element color for visual effects (yellow)
pub fn static_orb_color() -> Color {
    Element::Lightning.color()
}

/// Static Orb component - a stationary orb that periodically zaps nearby enemies.
/// The orb remains at its cast location and deals damage to the nearest enemy
/// within range at regular intervals.
#[derive(Component, Debug, Clone)]
pub struct StaticOrb {
    /// Timer for periodic zap attacks
    pub zap_timer: Timer,
    /// Range within which enemies can be targeted
    pub zap_range: f32,
    /// Total duration the orb exists
    pub duration: Timer,
    /// Damage dealt per zap
    pub damage_per_zap: f32,
    /// Position on XZ plane where orb is placed
    pub position: Vec2,
    /// Duration of slow effect to apply to targets
    pub slow_duration: f32,
    /// Speed multiplier for slow effect
    pub slow_multiplier: f32,
}

impl StaticOrb {
    pub fn new(position: Vec2, damage: f32) -> Self {
        Self {
            zap_timer: Timer::from_seconds(STATIC_ORB_ZAP_INTERVAL, TimerMode::Repeating),
            zap_range: STATIC_ORB_ZAP_RANGE,
            duration: Timer::from_seconds(STATIC_ORB_DURATION, TimerMode::Once),
            damage_per_zap: damage,
            position,
            slow_duration: STATIC_ORB_SLOW_DURATION,
            slow_multiplier: STATIC_ORB_SLOW_MULTIPLIER,
        }
    }

    pub fn from_spell(position: Vec2, spell: &Spell) -> Self {
        Self::new(position, spell.damage())
    }

    /// Check if the orb has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }
}

/// System that updates Static Orb duration timers and despawns expired orbs
pub fn static_orb_duration_system(
    mut commands: Commands,
    time: Res<Time>,
    mut orb_query: Query<(Entity, &mut StaticOrb)>,
) {
    for (entity, mut orb) in orb_query.iter_mut() {
        orb.duration.tick(time.delta());

        if orb.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that handles Static Orb zapping the nearest enemy
pub fn static_orb_zap_system(
    mut commands: Commands,
    time: Res<Time>,
    mut orb_query: Query<&mut StaticOrb>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut orb in orb_query.iter_mut() {
        orb.zap_timer.tick(time.delta());

        if orb.zap_timer.just_finished() {
            // Find the nearest enemy within range
            let mut nearest: Option<(Entity, f32)> = None;

            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);
                let distance = orb.position.distance(enemy_pos);

                if distance <= orb.zap_range {
                    match nearest {
                        Some((_, best_distance)) if distance < best_distance => {
                            nearest = Some((enemy_entity, distance));
                        }
                        None => {
                            nearest = Some((enemy_entity, distance));
                        }
                        _ => {}
                    }
                }
            }

            // Zap the nearest enemy if found
            if let Some((enemy_entity, _)) = nearest {
                damage_events.write(DamageEvent::new(enemy_entity, orb.damage_per_zap));
                // Apply slow effect
                commands.entity(enemy_entity).try_insert(SlowedDebuff::new(orb.slow_duration, orb.slow_multiplier));
            }
        }
    }
}

/// Cast Static Orb spell - spawns a stationary orb that zaps nearby enemies.
/// `spawn_position` is Whisper's full 3D position (where the orb will be placed).
#[allow(clippy::too_many_arguments)]
pub fn fire_static_orb(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_static_orb_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Static Orb spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_static_orb_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let orb_xz = from_xz(spawn_position);
    let orb = StaticOrb::new(orb_xz, damage);
    let orb_pos = spawn_position + Vec3::new(0.0, 0.3, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(orb_pos).with_scale(Vec3::splat(1.0)),
            orb,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(orb_pos),
            orb,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod static_orb_component_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_static_orb_new() {
            let position = Vec2::new(10.0, 20.0);
            let damage = 15.0;
            let orb = StaticOrb::new(position, damage);

            assert_eq!(orb.position, position);
            assert_eq!(orb.damage_per_zap, damage);
            assert_eq!(orb.zap_range, STATIC_ORB_ZAP_RANGE);
            assert!(!orb.is_expired());
        }

        #[test]
        fn test_static_orb_from_spell() {
            let spell = Spell::new(SpellType::StaticField);
            let position = Vec2::new(5.0, 15.0);
            let orb = StaticOrb::from_spell(position, &spell);

            assert_eq!(orb.position, position);
            assert_eq!(orb.damage_per_zap, spell.damage());
        }

        #[test]
        fn test_static_orb_zap_timer_initial_state() {
            let orb = StaticOrb::new(Vec2::ZERO, 10.0);
            assert!(!orb.zap_timer.just_finished());
            assert_eq!(orb.zap_timer.duration(), Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL));
        }

        #[test]
        fn test_static_orb_duration_initial_state() {
            let orb = StaticOrb::new(Vec2::ZERO, 10.0);
            assert!(!orb.is_expired());
            assert_eq!(orb.duration.duration(), Duration::from_secs_f32(STATIC_ORB_DURATION));
        }

        #[test]
        fn test_static_orb_expires_after_duration() {
            let mut orb = StaticOrb::new(Vec2::ZERO, 10.0);
            orb.duration.tick(Duration::from_secs_f32(STATIC_ORB_DURATION + 0.1));
            assert!(orb.is_expired());
        }

        #[test]
        fn test_static_orb_does_not_expire_before_duration() {
            let mut orb = StaticOrb::new(Vec2::ZERO, 10.0);
            orb.duration.tick(Duration::from_secs_f32(STATIC_ORB_DURATION / 2.0));
            assert!(!orb.is_expired());
        }

        #[test]
        fn test_static_orb_uses_lightning_element_color() {
            let color = static_orb_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }

        #[test]
        fn test_static_orb_slow_effect_values() {
            let orb = StaticOrb::new(Vec2::ZERO, 10.0);
            assert_eq!(orb.slow_duration, STATIC_ORB_SLOW_DURATION);
            assert_eq!(orb.slow_multiplier, STATIC_ORB_SLOW_MULTIPLIER);
        }
    }

    mod static_orb_duration_system_tests {
        use super::*;

        #[test]
        fn test_orb_despawns_after_duration() {
            let mut app = App::new();
            app.add_systems(Update, static_orb_duration_system);
            app.init_resource::<Time>();

            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 10.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_DURATION + 0.1));
            }

            app.update();

            // Orb should be despawned
            assert!(app.world().get_entity(orb_entity).is_err());
        }

        #[test]
        fn test_orb_survives_before_duration() {
            let mut app = App::new();
            app.add_systems(Update, static_orb_duration_system);
            app.init_resource::<Time>();

            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 10.0),
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_DURATION / 2.0));
            }

            app.update();

            // Orb should still exist
            assert!(app.world().get_entity(orb_entity).is_ok());
        }
    }

    mod static_orb_zap_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;
        use crate::movement::components::to_xz;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_orb_zaps_enemy_in_range() {
            let mut app = setup_test_app();

            // Create orb at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time to trigger first zap
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Check that orb timer has triggered (zap occurred)
            let mut orb_query = app.world_mut().query::<&StaticOrb>();
            let orb = orb_query.single(app.world()).unwrap();
            assert!(orb.zap_timer.just_finished(), "Zap timer should have triggered");
        }

        #[test]
        fn test_orb_does_not_zap_enemy_outside_range() {
            let mut app = setup_test_app();

            // Create orb at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));

            // Create enemy outside range (distance = 100)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            )).id();

            // Advance time to trigger zap attempt
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Enemy should NOT have SlowedDebuff (no zap occurred)
            assert!(app.world().get::<SlowedDebuff>(enemy_entity).is_none(),
                "Enemy outside range should not be hit");
        }

        #[test]
        fn test_orb_zaps_nearest_enemy_when_multiple_in_range() {
            let mut app = setup_test_app();

            // Create orb at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));

            // Create far enemy first (distance = 5)
            let far_enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            )).id();

            // Create near enemy (distance = 2)
            let near_enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();

            // Advance time to trigger first zap
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Only the nearest enemy should have been hit
            assert!(app.world().get::<SlowedDebuff>(near_enemy).is_some(),
                "Nearest enemy should be hit");
            assert!(app.world().get::<SlowedDebuff>(far_enemy).is_none(),
                "Far enemy should not be hit (only nearest is targeted)");
        }

        #[test]
        fn test_orb_zaps_repeatedly_at_interval() {
            let mut app = setup_test_app();

            // Create orb at origin
            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            )).id();

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Track how many times zap triggered
            let mut zap_count = 0;

            // Run 3 zap cycles
            for _ in 0..3 {
                // Advance time to trigger zap
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
                }

                // Run the system
                let _ = app.world_mut().run_system_once(static_orb_zap_system);

                // Check if zap triggered
                let orb = app.world().get::<StaticOrb>(orb_entity).unwrap();
                if orb.zap_timer.just_finished() {
                    zap_count += 1;
                }
            }

            assert_eq!(zap_count, 3, "Should have zapped 3 times");
        }

        #[test]
        fn test_no_zap_when_no_enemies_in_range() {
            let mut app = setup_test_app();

            // Create orb at origin with no enemies
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));

            // Advance time to trigger zap attempt
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system - should not crash
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Verify orb still exists (no crash)
            let mut orb_query = app.world_mut().query::<&StaticOrb>();
            let count = orb_query.iter(app.world()).count();
            assert_eq!(count, 1, "Orb should still exist");
        }

        #[test]
        fn test_orb_position_remains_constant() {
            let mut app = setup_test_app();

            // Create orb at specific position
            let initial_position = Vec2::new(10.0, 20.0);
            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(to_xz(initial_position) + Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(initial_position, 15.0),
            )).id();

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(12.0, 0.375, 22.0)),
            ));

            // Run several zap cycles
            for _ in 0..5 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
                }
                let _ = app.world_mut().run_system_once(static_orb_zap_system);
            }

            // Orb position should not have changed
            let orb = app.world().get::<StaticOrb>(orb_entity).unwrap();
            assert_eq!(orb.position, initial_position, "Orb position should remain constant");
        }

        #[test]
        fn test_multiple_orbs_operate_independently() {
            let mut app = setup_test_app();

            // Create two orbs at different positions
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(20.0, 0.3, 20.0)),
                StaticOrb::new(Vec2::new(20.0, 20.0), 15.0),
            ));

            // Create enemies near each orb
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            )).id();
            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(22.0, 0.375, 20.0)),
            )).id();

            // Advance time to trigger zaps from both orbs
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Both enemies should have been zapped (both have SlowedDebuff)
            assert!(app.world().get::<SlowedDebuff>(enemy1).is_some(),
                "Enemy near first orb should be hit");
            assert!(app.world().get::<SlowedDebuff>(enemy2).is_some(),
                "Enemy near second orb should be hit");
        }

        #[test]
        fn test_orb_uses_xz_plane_ignores_y() {
            let mut app = setup_test_app();

            // Create orb at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));

            // Create enemy close on XZ plane but far on Y - should still be hit
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            )).id();

            // Advance time to trigger zap
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Enemy should be hit (Y distance is ignored)
            assert!(app.world().get::<SlowedDebuff>(enemy_entity).is_some(),
                "Y distance should be ignored");
        }

        #[test]
        fn test_orb_applies_slow_effect_on_zap() {
            let mut app = setup_test_app();

            // Create orb at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                StaticOrb::new(Vec2::ZERO, 15.0),
            ));

            // Create enemy within range
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // Advance time to trigger first zap
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(STATIC_ORB_ZAP_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(static_orb_zap_system);

            // Enemy should have SlowedDebuff
            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "Enemy should have SlowedDebuff after zap");
            assert_eq!(slowed.unwrap().speed_multiplier, STATIC_ORB_SLOW_MULTIPLIER);
        }
    }

    mod fire_static_orb_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_static_orb_spawns_orb() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StaticField);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_static_orb(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 static orb
            let mut query = app.world_mut().query::<&StaticOrb>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_static_orb_at_spawn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StaticField);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_static_orb(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&StaticOrb>();
            for orb in query.iter(app.world()) {
                // Orb position should match spawn XZ (10.0, 20.0)
                assert_eq!(orb.position.x, 10.0);
                assert_eq!(orb.position.y, 20.0); // Z maps to Y in Vec2
            }
        }

        #[test]
        fn test_fire_static_orb_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StaticField);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_static_orb(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&StaticOrb>();
            for orb in query.iter(app.world()) {
                assert_eq!(orb.damage_per_zap, expected_damage);
            }
        }

        #[test]
        fn test_fire_static_orb_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::StaticField);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_static_orb_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&StaticOrb>();
            for orb in query.iter(app.world()) {
                assert_eq!(orb.damage_per_zap, explicit_damage);
            }
        }
    }
}
