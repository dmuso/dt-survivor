//! Chaos Bolt spell - Unstable projectile with random effects.
//!
//! A Chaos element spell (ChaosBolt SpellType) that fires a projectile which
//! applies a random effect on hit from a pool including: extra damage, slow,
//! burn, poison, stun, knockback, and fear.

use std::collections::HashSet;
use bevy::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, Knockback};
use crate::spell::components::Spell;
use crate::spells::chaos::fear::FearedEnemy;
use crate::spells::fire::fireball::BurnEffect;
use crate::spells::frost::ice_shard::SlowedDebuff;

/// Default configuration for Chaos Bolt spell
pub const CHAOS_BOLT_SPEED: f32 = 20.0;
pub const CHAOS_BOLT_LIFETIME: f32 = 5.0;
pub const CHAOS_BOLT_COLLISION_RADIUS: f32 = 1.0;
pub const CHAOS_BOLT_SPREAD_ANGLE: f32 = 15.0;

/// Effect configuration
pub const EXTRA_DAMAGE_MULTIPLIER: f32 = 1.5; // 50% extra damage
pub const SLOW_DURATION: f32 = 3.0;
pub const SLOW_SPEED_MULTIPLIER: f32 = 0.4;
pub const BURN_DURATION: f32 = 3.0;
pub const BURN_DAMAGE_PER_TICK: f32 = 5.0;
pub const POISON_DURATION: f32 = 4.0;
pub const POISON_DAMAGE_PER_TICK: f32 = 3.0;
pub const STUN_DURATION: f32 = 1.5;
pub const KNOCKBACK_FORCE: f32 = 400.0;
pub const CHAOS_FEAR_DURATION: f32 = 2.0;

/// Get the chaos element color for visual effects
pub fn chaos_bolt_color() -> Color {
    Element::Chaos.color()
}

/// Random effects that a Chaos Bolt can apply on hit.
#[derive(Component, Debug, Clone, PartialEq)]
pub enum RandomEffect {
    /// Deal extra damage (multiplier applied to base damage)
    ExtraDamage(f32),
    /// Slow the enemy (factor is speed multiplier, duration in seconds)
    Slow { factor: f32, duration: f32 },
    /// Apply burning DOT (damage per tick, duration in seconds)
    Burn { damage_per_tick: f32, duration: f32 },
    /// Apply poison DOT (damage per tick, duration in seconds)
    Poison { damage_per_tick: f32, duration: f32 },
    /// Stun the enemy (prevents movement, duration in seconds)
    Stun { duration: f32 },
    /// Knock the enemy back (force magnitude)
    Knockback { force: f32 },
    /// Cause the enemy to flee (duration in seconds)
    Fear { duration: f32 },
}

impl RandomEffect {
    /// Select a random effect with balanced distribution.
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..7) {
            0 => RandomEffect::ExtraDamage(EXTRA_DAMAGE_MULTIPLIER),
            1 => RandomEffect::Slow {
                factor: SLOW_SPEED_MULTIPLIER,
                duration: SLOW_DURATION,
            },
            2 => RandomEffect::Burn {
                damage_per_tick: BURN_DAMAGE_PER_TICK,
                duration: BURN_DURATION,
            },
            3 => RandomEffect::Poison {
                damage_per_tick: POISON_DAMAGE_PER_TICK,
                duration: POISON_DURATION,
            },
            4 => RandomEffect::Stun { duration: STUN_DURATION },
            5 => RandomEffect::Knockback { force: KNOCKBACK_FORCE },
            _ => RandomEffect::Fear { duration: CHAOS_FEAR_DURATION },
        }
    }

    /// Create a specific effect (for testing).
    pub fn extra_damage() -> Self {
        RandomEffect::ExtraDamage(EXTRA_DAMAGE_MULTIPLIER)
    }

    pub fn slow() -> Self {
        RandomEffect::Slow {
            factor: SLOW_SPEED_MULTIPLIER,
            duration: SLOW_DURATION,
        }
    }

    pub fn burn() -> Self {
        RandomEffect::Burn {
            damage_per_tick: BURN_DAMAGE_PER_TICK,
            duration: BURN_DURATION,
        }
    }

    pub fn poison() -> Self {
        RandomEffect::Poison {
            damage_per_tick: POISON_DAMAGE_PER_TICK,
            duration: POISON_DURATION,
        }
    }

    pub fn stun() -> Self {
        RandomEffect::Stun { duration: STUN_DURATION }
    }

    pub fn knockback() -> Self {
        RandomEffect::Knockback { force: KNOCKBACK_FORCE }
    }

    pub fn fear() -> Self {
        RandomEffect::Fear { duration: CHAOS_FEAR_DURATION }
    }
}

/// Stun effect applied to enemies - prevents movement.
#[derive(Component, Debug, Clone)]
pub struct StunnedEnemy {
    /// Remaining stun duration
    pub duration: Timer,
}

impl StunnedEnemy {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }
}

/// Poison DOT effect applied to enemies.
#[derive(Component, Debug, Clone)]
pub struct ChaosPoisonDebuff {
    /// Remaining poison duration
    pub duration: Timer,
    /// Damage per tick
    pub damage_per_tick: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
}

impl ChaosPoisonDebuff {
    pub fn new(duration_secs: f32, damage_per_tick: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            damage_per_tick,
            tick_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);
    }

    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }
}

/// Chaos Bolt projectile component.
#[derive(Component, Debug, Clone)]
pub struct ChaosBoltProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Base damage dealt on hit
    pub damage: f32,
    /// Pre-determined random effect to apply on hit
    pub effect: RandomEffect,
}

impl ChaosBoltProjectile {
    pub fn new(direction: Vec2, damage: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed: CHAOS_BOLT_SPEED,
            lifetime: Timer::from_seconds(CHAOS_BOLT_LIFETIME, TimerMode::Once),
            damage,
            effect: RandomEffect::random(),
        }
    }

    pub fn with_effect(mut self, effect: RandomEffect) -> Self {
        self.effect = effect;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.lifetime.tick(delta);
    }
}

/// Event fired when a Chaos Bolt collides with an enemy.
#[derive(Message)]
pub struct ChaosBoltEnemyCollisionEvent {
    pub chaos_bolt_entity: Entity,
    pub enemy_entity: Entity,
}

/// System that moves Chaos Bolt projectiles.
pub fn chaos_bolt_movement_system(
    mut projectile_query: Query<(&mut Transform, &ChaosBoltProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, bolt) in projectile_query.iter_mut() {
        let movement = Vec3::new(
            bolt.direction.x * bolt.speed * time.delta_secs(),
            0.0,
            bolt.direction.y * bolt.speed * time.delta_secs(),
        );
        transform.translation += movement;
    }
}

/// System that handles Chaos Bolt lifetime.
pub fn chaos_bolt_lifetime_system(
    mut commands: Commands,
    mut projectile_query: Query<(Entity, &mut ChaosBoltProjectile)>,
    time: Res<Time>,
) {
    for (entity, mut bolt) in projectile_query.iter_mut() {
        bolt.tick(time.delta());

        if bolt.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that detects Chaos Bolt-enemy collisions and fires events.
pub fn chaos_bolt_collision_detection(
    chaos_bolt_query: Query<(Entity, &Transform), With<ChaosBoltProjectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<ChaosBoltEnemyCollisionEvent>,
) {
    for (bolt_entity, bolt_transform) in chaos_bolt_query.iter() {
        let bolt_xz = Vec2::new(
            bolt_transform.translation.x,
            bolt_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = bolt_xz.distance(enemy_xz);

            if distance < CHAOS_BOLT_COLLISION_RADIUS {
                collision_events.write(ChaosBoltEnemyCollisionEvent {
                    chaos_bolt_entity: bolt_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per bolt
            }
        }
    }
}

/// System that applies effects when Chaos Bolts collide with enemies.
#[allow(clippy::too_many_arguments)]
pub fn chaos_bolt_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<ChaosBoltEnemyCollisionEvent>,
    chaos_bolt_query: Query<(&ChaosBoltProjectile, &Transform)>,
    enemy_query: Query<&Transform, With<Enemy>>,
    player_query: Query<&Transform, With<crate::player::components::Player>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut bolts_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32, RandomEffect, Vec3, Vec3)> = Vec::new();

    for event in collision_events.read() {
        bolts_to_despawn.insert(event.chaos_bolt_entity);

        if let (Ok((bolt, bolt_transform)), Ok(enemy_transform)) = (
            chaos_bolt_query.get(event.chaos_bolt_entity),
            enemy_query.get(event.enemy_entity),
        ) {
            effects_to_apply.push((
                event.enemy_entity,
                bolt.damage,
                bolt.effect.clone(),
                bolt_transform.translation,
                enemy_transform.translation,
            ));
        }
    }

    // Despawn bolts
    for bolt_entity in bolts_to_despawn {
        commands.entity(bolt_entity).try_despawn();
    }

    // Get player position for fear direction
    let player_pos = player_query.single().map(|t| from_xz(t.translation)).ok();

    // Apply damage and effects
    for (enemy_entity, base_damage, effect, _bolt_pos, enemy_pos) in effects_to_apply {
        let enemy_xz = from_xz(enemy_pos);

        // Calculate final damage based on effect
        let final_damage = match &effect {
            RandomEffect::ExtraDamage(multiplier) => base_damage * multiplier,
            _ => base_damage,
        };

        // Send damage event with Chaos element
        damage_events.write(DamageEvent::with_element(enemy_entity, final_damage, Element::Chaos));

        // Apply the random effect
        match effect {
            RandomEffect::ExtraDamage(_) => {
                // Extra damage already applied above
            }
            RandomEffect::Slow { factor, duration } => {
                commands.entity(enemy_entity).try_insert(SlowedDebuff::new(duration, factor));
            }
            RandomEffect::Burn { damage_per_tick, duration: _ } => {
                commands.entity(enemy_entity).try_insert(BurnEffect::new(damage_per_tick));
            }
            RandomEffect::Poison { damage_per_tick, duration } => {
                commands.entity(enemy_entity).try_insert(ChaosPoisonDebuff::new(duration, damage_per_tick));
            }
            RandomEffect::Stun { duration } => {
                commands.entity(enemy_entity).try_insert(StunnedEnemy::new(duration));
            }
            RandomEffect::Knockback { force } => {
                // Knockback away from bolt direction
                if let Some(player_pos) = player_pos {
                    let direction = (enemy_xz - player_pos).normalize_or_zero();
                    commands.entity(enemy_entity).try_insert(Knockback::new(direction, force, 0.2));
                }
            }
            RandomEffect::Fear { duration } => {
                // Fear away from player
                if let Some(player_pos) = player_pos {
                    let flee_direction = (enemy_xz - player_pos).normalize_or_zero();
                    commands.entity(enemy_entity).try_insert(FearedEnemy::new(duration, flee_direction));
                }
            }
        }
    }
}

/// System that ticks stunned enemies and removes expired stuns.
pub fn stunned_enemy_system(
    mut commands: Commands,
    mut stunned_query: Query<(Entity, &mut StunnedEnemy)>,
    time: Res<Time>,
) {
    for (entity, mut stunned) in stunned_query.iter_mut() {
        stunned.tick(time.delta());

        if stunned.is_expired() {
            commands.entity(entity).remove::<StunnedEnemy>();
        }
    }
}

/// System that applies chaos poison damage over time.
pub fn chaos_poison_damage_system(
    mut commands: Commands,
    mut poison_query: Query<(Entity, &mut ChaosPoisonDebuff)>,
    mut damage_events: MessageWriter<DamageEvent>,
    time: Res<Time>,
) {
    for (entity, mut poison) in poison_query.iter_mut() {
        poison.tick(time.delta());

        if poison.should_damage() {
            damage_events.write(DamageEvent::with_element(entity, poison.damage_per_tick, Element::Poison));
        }

        if poison.is_expired() {
            commands.entity(entity).remove::<ChaosPoisonDebuff>();
        }
    }
}

/// Cast Chaos Bolt spell - spawns projectiles with chaos element visuals.
#[allow(clippy::too_many_arguments)]
pub fn fire_chaos_bolt(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_chaos_bolt_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast Chaos Bolt spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_chaos_bolt_with_damage(
    commands: &mut Commands,
    spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let spawn_xz = from_xz(spawn_position);
    let base_direction = (target_pos - spawn_xz).normalize();

    let projectile_count = spell.projectile_count();
    let spread_angle_rad = CHAOS_BOLT_SPREAD_ANGLE.to_radians();

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

        let bolt = ChaosBoltProjectile::new(direction, damage);

        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.chaos_bolt.clone()),
                Transform::from_translation(spawn_position),
                bolt,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                bolt,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;
    use crate::player::components::Player;

    mod random_effect_tests {
        use super::*;

        #[test]
        fn test_random_effect_all_variants_can_be_created() {
            let extra_damage = RandomEffect::extra_damage();
            assert!(matches!(extra_damage, RandomEffect::ExtraDamage(_)));

            let slow = RandomEffect::slow();
            assert!(matches!(slow, RandomEffect::Slow { .. }));

            let burn = RandomEffect::burn();
            assert!(matches!(burn, RandomEffect::Burn { .. }));

            let poison = RandomEffect::poison();
            assert!(matches!(poison, RandomEffect::Poison { .. }));

            let stun = RandomEffect::stun();
            assert!(matches!(stun, RandomEffect::Stun { .. }));

            let knockback = RandomEffect::knockback();
            assert!(matches!(knockback, RandomEffect::Knockback { .. }));

            let fear = RandomEffect::fear();
            assert!(matches!(fear, RandomEffect::Fear { .. }));
        }

        #[test]
        fn test_random_effect_has_correct_values() {
            match RandomEffect::extra_damage() {
                RandomEffect::ExtraDamage(mult) => assert_eq!(mult, EXTRA_DAMAGE_MULTIPLIER),
                _ => panic!("Expected ExtraDamage"),
            }

            match RandomEffect::slow() {
                RandomEffect::Slow { factor, duration } => {
                    assert_eq!(factor, SLOW_SPEED_MULTIPLIER);
                    assert_eq!(duration, SLOW_DURATION);
                }
                _ => panic!("Expected Slow"),
            }

            match RandomEffect::burn() {
                RandomEffect::Burn { damage_per_tick, duration } => {
                    assert_eq!(damage_per_tick, BURN_DAMAGE_PER_TICK);
                    assert_eq!(duration, BURN_DURATION);
                }
                _ => panic!("Expected Burn"),
            }
        }

        #[test]
        fn test_random_effect_random_returns_valid_effect() {
            // Call random multiple times to ensure it doesn't panic
            for _ in 0..20 {
                let effect = RandomEffect::random();
                // All variants should be valid
                match effect {
                    RandomEffect::ExtraDamage(_)
                    | RandomEffect::Slow { .. }
                    | RandomEffect::Burn { .. }
                    | RandomEffect::Poison { .. }
                    | RandomEffect::Stun { .. }
                    | RandomEffect::Knockback { .. }
                    | RandomEffect::Fear { .. } => {}
                }
            }
        }
    }

    mod chaos_bolt_projectile_tests {
        use super::*;

        #[test]
        fn test_chaos_bolt_spawns_with_random_effect() {
            let bolt = ChaosBoltProjectile::new(Vec2::X, 20.0);

            // Effect should be assigned (not checking specific value since it's random)
            assert!(!bolt.is_expired());
            assert_eq!(bolt.damage, 20.0);
            assert_eq!(bolt.speed, CHAOS_BOLT_SPEED);
        }

        #[test]
        fn test_chaos_bolt_with_specific_effect() {
            let bolt = ChaosBoltProjectile::new(Vec2::X, 20.0)
                .with_effect(RandomEffect::stun());

            assert!(matches!(bolt.effect, RandomEffect::Stun { .. }));
        }

        #[test]
        fn test_chaos_bolt_normalizes_direction() {
            let bolt = ChaosBoltProjectile::new(Vec2::new(3.0, 4.0), 20.0);

            assert!((bolt.direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_chaos_bolt_lifetime_expires() {
            let mut bolt = ChaosBoltProjectile::new(Vec2::X, 20.0);
            assert!(!bolt.is_expired());

            bolt.tick(Duration::from_secs_f32(CHAOS_BOLT_LIFETIME + 0.1));
            assert!(bolt.is_expired());
        }

        #[test]
        fn test_chaos_bolt_uses_chaos_element_color() {
            let color = chaos_bolt_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod stunned_enemy_tests {
        use super::*;

        #[test]
        fn test_stunned_enemy_new() {
            let stunned = StunnedEnemy::new(2.0);
            assert!(!stunned.is_expired());
        }

        #[test]
        fn test_stunned_enemy_expires_after_duration() {
            let mut stunned = StunnedEnemy::new(1.0);
            assert!(!stunned.is_expired());

            stunned.tick(Duration::from_secs_f32(1.1));
            assert!(stunned.is_expired());
        }
    }

    mod chaos_poison_debuff_tests {
        use super::*;

        #[test]
        fn test_chaos_poison_new() {
            let poison = ChaosPoisonDebuff::new(4.0, 3.0);
            assert!(!poison.is_expired());
            assert_eq!(poison.damage_per_tick, 3.0);
        }

        #[test]
        fn test_chaos_poison_ticks_and_damages() {
            let mut poison = ChaosPoisonDebuff::new(4.0, 3.0);

            // Tick to first damage interval
            poison.tick(Duration::from_secs_f32(0.5));
            assert!(poison.should_damage());

            // Tick again but not to next interval
            poison.tick(Duration::from_secs_f32(0.1));
            assert!(!poison.should_damage());
        }

        #[test]
        fn test_chaos_poison_expires() {
            let mut poison = ChaosPoisonDebuff::new(1.0, 3.0);
            poison.tick(Duration::from_secs_f32(1.1));
            assert!(poison.is_expired());
        }
    }

    mod movement_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_chaos_bolt_moves_toward_target() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(chaos_bolt_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!(
                (transform.translation.x - CHAOS_BOLT_SPEED).abs() < 0.1,
                "Expected X ~{}, got {}",
                CHAOS_BOLT_SPEED,
                transform.translation.x
            );
        }

        #[test]
        fn test_chaos_bolt_moves_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::new(0.0, 1.0), 20.0),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(chaos_bolt_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.y, 0.5, "Y should not change");
            assert!(
                (transform.translation.z - CHAOS_BOLT_SPEED).abs() < 0.1,
                "Expected Z ~{}, got {}",
                CHAOS_BOLT_SPEED,
                transform.translation.z
            );
        }
    }

    mod lifetime_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_chaos_bolt_despawns_on_hit_or_timeout() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let mut bolt = ChaosBoltProjectile::new(Vec2::X, 20.0);
            bolt.lifetime = Timer::from_seconds(0.0, TimerMode::Once);
            bolt.lifetime.tick(Duration::from_secs(1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                bolt,
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(chaos_bolt_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_chaos_bolt_survives_before_timeout() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ChaosBoltProjectile::new(Vec2::X, 20.0),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(chaos_bolt_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod collision_detection_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<ChaosBoltEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_chaos_bolt_deals_base_damage() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<ChaosBoltEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (chaos_bolt_collision_detection, count_collisions).chain());

            // Spawn chaos bolt at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0),
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
        fn test_chaos_bolt_no_collision_when_far() {
            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<ChaosBoltEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (chaos_bolt_collision_detection, count_collisions).chain());

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }
    }

    mod collision_effects_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<ChaosBoltEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        fn test_player() -> Player {
            Player {
                speed: 200.0,
                regen_rate: 1.0,
                pickup_radius: 50.0,
                last_movement_direction: Vec3::ZERO,
            }
        }

        #[test]
        fn test_chaos_bolt_applies_slow_effect() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (chaos_bolt_collision_detection, chaos_bolt_collision_effects).chain(),
            );

            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0).with_effect(RandomEffect::slow()),
            ));

            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            assert!(
                app.world().get::<SlowedDebuff>(enemy).is_some(),
                "Enemy should have SlowedDebuff"
            );
        }

        #[test]
        fn test_chaos_bolt_applies_burn_effect() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (chaos_bolt_collision_detection, chaos_bolt_collision_effects).chain(),
            );

            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0).with_effect(RandomEffect::burn()),
            ));

            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            assert!(
                app.world().get::<BurnEffect>(enemy).is_some(),
                "Enemy should have BurnEffect"
            );
        }

        #[test]
        fn test_chaos_bolt_applies_poison_effect() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (chaos_bolt_collision_detection, chaos_bolt_collision_effects).chain(),
            );

            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0).with_effect(RandomEffect::poison()),
            ));

            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            assert!(
                app.world().get::<ChaosPoisonDebuff>(enemy).is_some(),
                "Enemy should have ChaosPoisonDebuff"
            );
        }

        #[test]
        fn test_chaos_bolt_applies_stun_effect() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (chaos_bolt_collision_detection, chaos_bolt_collision_effects).chain(),
            );

            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0).with_effect(RandomEffect::stun()),
            ));

            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            assert!(
                app.world().get::<StunnedEnemy>(enemy).is_some(),
                "Enemy should have StunnedEnemy"
            );
        }

        #[test]
        fn test_chaos_bolt_despawns_on_hit() {
            let mut app = setup_test_app();

            app.add_systems(
                Update,
                (chaos_bolt_collision_detection, chaos_bolt_collision_effects).chain(),
            );

            app.world_mut().spawn((
                test_player(),
                Transform::from_translation(Vec3::ZERO),
            ));

            let bolt = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ChaosBoltProjectile::new(Vec2::X, 20.0),
            )).id();

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert!(!app.world().entities().contains(bolt), "Bolt should despawn on hit");
        }
    }

    mod fire_chaos_bolt_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_chaos_bolt_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChaosBolt);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_chaos_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChaosBoltProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_chaos_bolt_spawns_multiple_at_higher_levels() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::ChaosBolt);
            spell.level = 5;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_chaos_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChaosBoltProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2);
        }

        #[test]
        fn test_fire_chaos_bolt_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChaosBolt);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_chaos_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChaosBoltProjectile>();
            for bolt in query.iter(app.world()) {
                assert!(
                    bolt.direction.x > 0.9,
                    "Bolt should face toward target (+X), got direction {:?}",
                    bolt.direction
                );
            }
        }

        #[test]
        fn test_fire_chaos_bolt_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChaosBolt);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_chaos_bolt(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChaosBoltProjectile>();
            for bolt in query.iter(app.world()) {
                assert_eq!(bolt.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_chaos_bolt_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChaosBolt);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_chaos_bolt_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChaosBoltProjectile>();
            for bolt in query.iter(app.world()) {
                assert_eq!(bolt.damage, explicit_damage);
            }
        }
    }
}
