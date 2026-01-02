use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, Knockback};
use crate::player::components::Player;
use crate::spell::components::Spell;
use crate::spells::chaos::fear::FearedEnemy;
use crate::spells::chaos::chaos_bolt::StunnedEnemy;
use crate::spells::fire::fireball::BurnEffect;
use crate::spells::frost::ice_shard::SlowedDebuff;
use rand::Rng;

/// Default configuration for Disorder Pulse spell
pub const DISORDER_PULSE_INTERVAL: f32 = 1.2;
pub const DISORDER_PULSE_RANGE: f32 = 8.0;
pub const DISORDER_PULSE_DURATION: f32 = 8.0;

// Effect-specific constants
pub const DISORDER_SLOW_FACTOR: f32 = 0.5;
pub const DISORDER_SLOW_DURATION: f32 = 2.0;
pub const DISORDER_KNOCKBACK_FORCE: f32 = 400.0;
pub const DISORDER_KNOCKBACK_DURATION: f32 = 0.3;
pub const DISORDER_FEAR_DURATION: f32 = 2.0;
pub const DISORDER_STUN_DURATION: f32 = 1.0;
pub const DISORDER_BURN_DAMAGE: f32 = 5.0;

/// Get the chaos element color for visual effects (magenta)
pub fn disorder_pulse_color() -> Color {
    Element::Chaos.color()
}

/// The random effect that can be applied by a Disorder Pulse.
/// Each pulse randomly selects one effect to apply to all enemies hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PulseEffect {
    /// Extra damage (already applied via DamageEvent, this is a bonus)
    BonusDamage,
    /// Slow enemies
    Slow,
    /// Knockback enemies away from player
    Knockback,
    /// Fear enemies (make them flee)
    Fear,
    /// Stun enemies briefly
    Stun,
    /// Apply burn damage over time
    Burn,
}

impl PulseEffect {
    /// Get a random effect
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..6) {
            0 => PulseEffect::BonusDamage,
            1 => PulseEffect::Slow,
            2 => PulseEffect::Knockback,
            3 => PulseEffect::Fear,
            4 => PulseEffect::Stun,
            _ => PulseEffect::Burn,
        }
    }

    /// Get a specific effect by index (for testing)
    pub fn from_index(index: u8) -> Self {
        match index % 6 {
            0 => PulseEffect::BonusDamage,
            1 => PulseEffect::Slow,
            2 => PulseEffect::Knockback,
            3 => PulseEffect::Fear,
            4 => PulseEffect::Stun,
            _ => PulseEffect::Burn,
        }
    }

    /// Returns the number of possible effects
    pub fn count() -> u8 {
        6
    }
}

/// Disorder Pulse component - periodic chaotic bursts that hit all enemies in range
/// with a random effect each pulse.
#[derive(Component, Debug, Clone)]
pub struct DisorderPulse {
    /// Timer for periodic pulse attacks
    pub pulse_timer: Timer,
    /// Range within which enemies are affected
    pub pulse_range: f32,
    /// Base damage dealt per pulse
    pub base_damage: f32,
    /// Total duration the spell is active
    pub duration: Timer,
}

impl DisorderPulse {
    pub fn new(damage: f32) -> Self {
        Self {
            pulse_timer: Timer::from_seconds(DISORDER_PULSE_INTERVAL, TimerMode::Repeating),
            pulse_range: DISORDER_PULSE_RANGE,
            base_damage: damage,
            duration: Timer::from_seconds(DISORDER_PULSE_DURATION, TimerMode::Once),
        }
    }

    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(spell.damage())
    }

    /// Check if the spell has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }
}

/// System that updates Disorder Pulse duration timers and removes expired instances
pub fn disorder_pulse_duration_system(
    mut commands: Commands,
    time: Res<Time>,
    mut pulse_query: Query<(Entity, &mut DisorderPulse)>,
) {
    for (entity, mut pulse) in pulse_query.iter_mut() {
        pulse.duration.tick(time.delta());

        if pulse.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that handles Disorder Pulse periodic effects on enemies
pub fn disorder_pulse_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut pulse_query: Query<&mut DisorderPulse>,
    enemy_query: Query<(Entity, &Transform, &Enemy), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = from_xz(player_transform.translation);

    for mut pulse in pulse_query.iter_mut() {
        pulse.pulse_timer.tick(time.delta());

        if pulse.pulse_timer.just_finished() {
            // Roll a random effect for this pulse
            let effect = PulseEffect::random();

            // Find all enemies within range
            for (enemy_entity, enemy_transform, enemy) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);
                let distance = player_pos.distance(enemy_pos);

                if distance <= pulse.pulse_range {
                    // Always apply base damage
                    damage_events.write(DamageEvent::new(enemy_entity, pulse.base_damage));

                    // Apply the random effect
                    apply_pulse_effect(
                        &mut commands,
                        enemy_entity,
                        enemy,
                        player_pos,
                        enemy_pos,
                        effect,
                        pulse.base_damage,
                    );
                }
            }
        }
    }
}

/// Apply the selected effect to an enemy
fn apply_pulse_effect(
    commands: &mut Commands,
    enemy_entity: Entity,
    _enemy: &Enemy,
    player_pos: Vec2,
    enemy_pos: Vec2,
    effect: PulseEffect,
    base_damage: f32,
) {
    match effect {
        PulseEffect::BonusDamage => {
            // Bonus damage is handled via the damage event - no additional component needed
            // The damage event already sent handles base damage; bonus is implicit
        }
        PulseEffect::Slow => {
            commands
                .entity(enemy_entity)
                .insert(SlowedDebuff::new(DISORDER_SLOW_DURATION, DISORDER_SLOW_FACTOR));
        }
        PulseEffect::Knockback => {
            let direction = (enemy_pos - player_pos).normalize_or_zero();
            commands.entity(enemy_entity).insert(Knockback::new(
                direction,
                DISORDER_KNOCKBACK_FORCE,
                DISORDER_KNOCKBACK_DURATION,
            ));
        }
        PulseEffect::Fear => {
            let flee_direction = (enemy_pos - player_pos).normalize_or_zero();
            commands
                .entity(enemy_entity)
                .insert(FearedEnemy::new(DISORDER_FEAR_DURATION, flee_direction));
        }
        PulseEffect::Stun => {
            commands
                .entity(enemy_entity)
                .insert(StunnedEnemy::new(DISORDER_STUN_DURATION));
        }
        PulseEffect::Burn => {
            commands
                .entity(enemy_entity)
                .insert(BurnEffect::new(base_damage * 0.2 + DISORDER_BURN_DAMAGE));
        }
    }
}

/// Cast Disorder Pulse spell - creates a pulsing chaotic aura around the player.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_disorder_pulse(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_disorder_pulse_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Disorder Pulse spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_disorder_pulse_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let pulse = DisorderPulse::new(damage);
    let pulse_pos = spawn_position + Vec3::new(0.0, 0.3, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.chaos_aoe.clone()), // Transparent chaos AOE material
            Transform::from_translation(pulse_pos).with_scale(Vec3::splat(1.0)),
            pulse,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((Transform::from_translation(pulse_pos), pulse));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod pulse_effect_tests {
        use super::*;

        #[test]
        fn test_pulse_effect_from_index_covers_all() {
            assert_eq!(PulseEffect::from_index(0), PulseEffect::BonusDamage);
            assert_eq!(PulseEffect::from_index(1), PulseEffect::Slow);
            assert_eq!(PulseEffect::from_index(2), PulseEffect::Knockback);
            assert_eq!(PulseEffect::from_index(3), PulseEffect::Fear);
            assert_eq!(PulseEffect::from_index(4), PulseEffect::Stun);
            assert_eq!(PulseEffect::from_index(5), PulseEffect::Burn);
        }

        #[test]
        fn test_pulse_effect_from_index_wraps() {
            assert_eq!(PulseEffect::from_index(6), PulseEffect::BonusDamage);
            assert_eq!(PulseEffect::from_index(7), PulseEffect::Slow);
        }

        #[test]
        fn test_pulse_effect_count() {
            assert_eq!(PulseEffect::count(), 6);
        }

        #[test]
        fn test_pulse_effect_random_returns_valid() {
            // Run multiple times to increase confidence
            for _ in 0..20 {
                let effect = PulseEffect::random();
                // Just verify it matches one of the variants
                let is_valid = matches!(
                    effect,
                    PulseEffect::BonusDamage
                        | PulseEffect::Slow
                        | PulseEffect::Knockback
                        | PulseEffect::Fear
                        | PulseEffect::Stun
                        | PulseEffect::Burn
                );
                assert!(is_valid);
            }
        }
    }

    mod disorder_pulse_component_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_disorder_pulse_new() {
            let damage = 20.0;
            let pulse = DisorderPulse::new(damage);

            assert_eq!(pulse.base_damage, damage);
            assert_eq!(pulse.pulse_range, DISORDER_PULSE_RANGE);
            assert!(!pulse.is_expired());
        }

        #[test]
        fn test_disorder_pulse_from_spell() {
            let spell = Spell::new(SpellType::Mayhem);
            let pulse = DisorderPulse::from_spell(&spell);

            assert_eq!(pulse.base_damage, spell.damage());
        }

        #[test]
        fn test_disorder_pulse_timer_initial_state() {
            let pulse = DisorderPulse::new(10.0);
            assert!(!pulse.pulse_timer.just_finished());
            assert_eq!(
                pulse.pulse_timer.duration(),
                Duration::from_secs_f32(DISORDER_PULSE_INTERVAL)
            );
        }

        #[test]
        fn test_disorder_pulse_duration_initial_state() {
            let pulse = DisorderPulse::new(10.0);
            assert!(!pulse.is_expired());
            assert_eq!(
                pulse.duration.duration(),
                Duration::from_secs_f32(DISORDER_PULSE_DURATION)
            );
        }

        #[test]
        fn test_disorder_pulse_expires_after_duration() {
            let mut pulse = DisorderPulse::new(10.0);
            pulse
                .duration
                .tick(Duration::from_secs_f32(DISORDER_PULSE_DURATION + 0.1));
            assert!(pulse.is_expired());
        }

        #[test]
        fn test_disorder_pulse_does_not_expire_before_duration() {
            let mut pulse = DisorderPulse::new(10.0);
            pulse
                .duration
                .tick(Duration::from_secs_f32(DISORDER_PULSE_DURATION / 2.0));
            assert!(!pulse.is_expired());
        }

        #[test]
        fn test_disorder_pulse_uses_chaos_element_color() {
            let color = disorder_pulse_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod disorder_pulse_duration_system_tests {
        use super::*;

        #[test]
        fn test_pulse_despawns_after_duration() {
            let mut app = App::new();
            app.add_systems(Update, disorder_pulse_duration_system);
            app.init_resource::<Time>();

            let pulse_entity = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                    DisorderPulse::new(10.0),
                ))
                .id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(DISORDER_PULSE_DURATION + 0.1));
            }

            app.update();

            // Pulse should be despawned
            assert!(app.world().get_entity(pulse_entity).is_err());
        }

        #[test]
        fn test_pulse_survives_before_duration() {
            let mut app = App::new();
            app.add_systems(Update, disorder_pulse_duration_system);
            app.init_resource::<Time>();

            let pulse_entity = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                    DisorderPulse::new(10.0),
                ))
                .id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(DISORDER_PULSE_DURATION / 2.0));
            }

            app.update();

            // Pulse should still exist
            assert!(app.world().get_entity(pulse_entity).is_ok());
        }
    }

    mod disorder_pulse_effect_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_pulse_timer_triggers() {
            let mut app = setup_test_app();

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create disorder pulse
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                DisorderPulse::new(15.0),
            ));

            // Create enemy within range
            app.world_mut().spawn((
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time to trigger first pulse
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(DISORDER_PULSE_INTERVAL + 0.01));
            }

            // Run the system directly
            let _ = app.world_mut().run_system_once(disorder_pulse_effect_system);

            // Check that pulse timer has triggered
            let mut pulse_query = app.world_mut().query::<&DisorderPulse>();
            let pulse = pulse_query.single(app.world()).unwrap();
            assert!(
                pulse.pulse_timer.just_finished(),
                "Pulse timer should have triggered"
            );
        }

        #[test]
        fn test_pulse_radius_correct() {
            let pulse = DisorderPulse::new(10.0);
            assert_eq!(pulse.pulse_range, DISORDER_PULSE_RANGE);
        }

        #[test]
        fn test_pulse_affects_enemies_in_range() {
            let mut app = App::new();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_message::<DamageEvent>();
            app.init_resource::<Time>();
            app.add_systems(
                Update,
                (disorder_pulse_effect_system, count_damage_events).chain(),
            );

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create disorder pulse with timer almost at threshold
            let mut pulse = DisorderPulse::new(15.0);
            pulse
                .pulse_timer
                .tick(Duration::from_secs_f32(DISORDER_PULSE_INTERVAL - 0.001));
            app.world_mut()
                .spawn((Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)), pulse));

            // Create 3 enemies within range
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Transform::from_translation(Vec3::new(i as f32 + 1.0, 0.375, 0.0)),
                ));
            }

            // Manually advance time and update
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // Should hit all 3 enemies
            assert_eq!(
                counter.0.load(Ordering::SeqCst),
                3,
                "Should hit all enemies in range"
            );
        }

        #[test]
        fn test_pulse_does_not_affect_enemies_outside_range() {
            let mut app = App::new();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            #[derive(Resource, Clone)]
            struct DamageEventCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageEventCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageEventCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_message::<DamageEvent>();
            app.init_resource::<Time>();
            app.add_systems(
                Update,
                (disorder_pulse_effect_system, count_damage_events).chain(),
            );

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create disorder pulse
            let mut pulse = DisorderPulse::new(15.0);
            pulse
                .pulse_timer
                .tick(Duration::from_secs_f32(DISORDER_PULSE_INTERVAL - 0.001));
            app.world_mut()
                .spawn((Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)), pulse));

            // Create enemy outside range (distance = 100)
            app.world_mut().spawn((
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger pulse attempt
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            // No damage should have been dealt
            assert_eq!(
                counter.0.load(Ordering::SeqCst),
                0,
                "Enemy outside range should not be hit"
            );
        }

        #[test]
        fn test_pulse_timer_resets() {
            let mut app = setup_test_app();

            // Create player at origin
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Create disorder pulse
            let pulse_entity = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                    DisorderPulse::new(15.0),
                ))
                .id();

            // Create enemy within range
            app.world_mut().spawn((
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Track how many times pulse triggered
            let mut pulse_count = 0;

            // Run 3 pulse cycles
            for _ in 0..3 {
                // Advance time to trigger pulse
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(DISORDER_PULSE_INTERVAL + 0.01));
                }

                // Run the system
                let _ = app.world_mut().run_system_once(disorder_pulse_effect_system);

                // Check if pulse triggered
                let pulse = app.world().get::<DisorderPulse>(pulse_entity).unwrap();
                if pulse.pulse_timer.just_finished() {
                    pulse_count += 1;
                }
            }

            assert_eq!(pulse_count, 3, "Should have pulsed 3 times");
        }

        #[test]
        fn test_pulse_no_player_does_not_crash() {
            let mut app = setup_test_app();

            // Create disorder pulse without a player
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                DisorderPulse::new(15.0),
            ));

            // Create enemy
            app.world_mut().spawn((
                Enemy {
                    speed: 50.0,
                    strength: 10.0,
                },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time to trigger pulse
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(DISORDER_PULSE_INTERVAL + 0.01));
            }

            // Run the system - should not crash due to missing player
            let _ = app.world_mut().run_system_once(disorder_pulse_effect_system);

            // Verify pulse still exists
            let mut pulse_query = app.world_mut().query::<&DisorderPulse>();
            let count = pulse_query.iter(app.world()).count();
            assert_eq!(count, 1, "Pulse should still exist");
        }
    }

    mod apply_effect_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_pulse_applies_slow_effect() {
            let mut app = setup_test_app();

            let enemy_entity = app
                .world_mut()
                .spawn((
                    Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                ))
                .id();

            // Apply slow effect
            {
                let mut commands = app.world_mut().commands();
                apply_pulse_effect(
                    &mut commands,
                    enemy_entity,
                    &Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Vec2::ZERO,
                    Vec2::new(3.0, 0.0),
                    PulseEffect::Slow,
                    10.0,
                );
            }
            app.update();

            // Check slow debuff was applied
            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "SlowedDebuff should be applied");
            let slowed = slowed.unwrap();
            assert_eq!(slowed.speed_multiplier, DISORDER_SLOW_FACTOR);
        }

        #[test]
        fn test_pulse_applies_knockback_effect() {
            let mut app = setup_test_app();

            let enemy_entity = app
                .world_mut()
                .spawn((
                    Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                ))
                .id();

            // Apply knockback effect
            {
                let mut commands = app.world_mut().commands();
                apply_pulse_effect(
                    &mut commands,
                    enemy_entity,
                    &Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Vec2::ZERO,
                    Vec2::new(3.0, 0.0),
                    PulseEffect::Knockback,
                    10.0,
                );
            }
            app.update();

            // Check knockback was applied
            let knockback = app.world().get::<Knockback>(enemy_entity);
            assert!(knockback.is_some(), "Knockback should be applied");
        }

        #[test]
        fn test_pulse_applies_fear_effect() {
            let mut app = setup_test_app();

            let enemy_entity = app
                .world_mut()
                .spawn((
                    Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                ))
                .id();

            // Apply fear effect
            {
                let mut commands = app.world_mut().commands();
                apply_pulse_effect(
                    &mut commands,
                    enemy_entity,
                    &Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Vec2::ZERO,
                    Vec2::new(3.0, 0.0),
                    PulseEffect::Fear,
                    10.0,
                );
            }
            app.update();

            // Check fear was applied
            let feared = app.world().get::<FearedEnemy>(enemy_entity);
            assert!(feared.is_some(), "FearedEnemy should be applied");
        }

        #[test]
        fn test_pulse_applies_stun_effect() {
            let mut app = setup_test_app();

            let enemy_entity = app
                .world_mut()
                .spawn((
                    Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                ))
                .id();

            // Apply stun effect
            {
                let mut commands = app.world_mut().commands();
                apply_pulse_effect(
                    &mut commands,
                    enemy_entity,
                    &Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Vec2::ZERO,
                    Vec2::new(3.0, 0.0),
                    PulseEffect::Stun,
                    10.0,
                );
            }
            app.update();

            // Check stun was applied
            let stunned = app.world().get::<StunnedEnemy>(enemy_entity);
            assert!(stunned.is_some(), "StunnedEnemy should be applied");
        }

        #[test]
        fn test_pulse_applies_burn_effect() {
            let mut app = setup_test_app();

            let enemy_entity = app
                .world_mut()
                .spawn((
                    Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                ))
                .id();

            // Apply burn effect
            {
                let mut commands = app.world_mut().commands();
                apply_pulse_effect(
                    &mut commands,
                    enemy_entity,
                    &Enemy {
                        speed: 50.0,
                        strength: 10.0,
                    },
                    Vec2::ZERO,
                    Vec2::new(3.0, 0.0),
                    PulseEffect::Burn,
                    10.0,
                );
            }
            app.update();

            // Check burn was applied
            let burn = app.world().get::<BurnEffect>(enemy_entity);
            assert!(burn.is_some(), "BurnEffect should be applied");
        }
    }

    mod fire_disorder_pulse_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_disorder_pulse_spawns_pulse() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Mayhem);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_disorder_pulse(&mut commands, &spell, spawn_pos, None, None);
            }
            app.update();

            // Should spawn 1 disorder pulse
            let mut query = app.world_mut().query::<&DisorderPulse>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_disorder_pulse_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Mayhem);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_disorder_pulse(&mut commands, &spell, spawn_pos, None, None);
            }
            app.update();

            let mut query = app.world_mut().query::<&DisorderPulse>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.base_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_disorder_pulse_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Mayhem);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_disorder_pulse_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&DisorderPulse>();
            for pulse in query.iter(app.world()) {
                assert_eq!(pulse.base_damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_disorder_pulse_spawns_at_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Mayhem);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_disorder_pulse(&mut commands, &spell, spawn_pos, None, None);
            }
            app.update();

            let mut query = app.world_mut().query::<&Transform>();
            for transform in query.iter(app.world()) {
                // Should be at spawn position with slight Y offset
                assert_eq!(transform.translation.x, 10.0);
                assert_eq!(transform.translation.z, 20.0);
            }
        }
    }
}
