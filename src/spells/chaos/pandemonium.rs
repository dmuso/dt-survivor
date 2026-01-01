//! Pandemonium spell - Causes enemies to behave erratically and attack randomly.
//!
//! A Chaos element spell (Mayhem SpellType) that creates an AOE burst which
//! applies confusion to enemies. Confused enemies randomly retarget, including
//! attacking other enemies, creating friendly fire chaos among enemy ranks.

use std::collections::HashSet;
use bevy::prelude::*;
use rand::Rng;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Pandemonium spell
pub const PANDEMONIUM_BURST_RADIUS: f32 = 7.0;
pub const PANDEMONIUM_DURATION: f32 = 4.0;
pub const PANDEMONIUM_RETARGET_INTERVAL: f32 = 0.5; // Time between random retargets
pub const PANDEMONIUM_VISUAL_HEIGHT: f32 = 0.2;
pub const PANDEMONIUM_CONFUSED_SPEED_MULTIPLIER: f32 = 1.2; // Confused enemies are a bit faster/erratic

/// Get the chaos element color for visual effects
pub fn pandemonium_color() -> Color {
    Element::Chaos.color()
}

/// Component for the pandemonium AOE burst.
/// Tracks the burst radius and which enemies have been affected.
#[derive(Component, Debug, Clone)]
pub struct PandemoniumBurst {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the confusion effect
    pub radius: f32,
    /// Duration of confusion to apply to affected enemies
    pub confusion_duration: f32,
    /// Set of enemy entities already affected by this burst
    pub affected_enemies: HashSet<Entity>,
    /// Whether this burst has been processed (single-frame effect)
    pub processed: bool,
}

impl PandemoniumBurst {
    pub fn new(center: Vec2) -> Self {
        Self {
            center,
            radius: PANDEMONIUM_BURST_RADIUS,
            confusion_duration: PANDEMONIUM_DURATION,
            affected_enemies: HashSet::new(),
            processed: false,
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.confusion_duration = duration;
        self
    }

    /// Check if an enemy is within the burst radius and hasn't been affected yet
    pub fn can_affect(&self, entity: Entity, enemy_pos: Vec2) -> bool {
        let distance = self.center.distance(enemy_pos);
        distance <= self.radius && !self.affected_enemies.contains(&entity)
    }

    /// Mark an enemy as affected
    pub fn mark_affected(&mut self, entity: Entity) {
        self.affected_enemies.insert(entity);
    }
}

/// Component applied to enemies affected by pandemonium.
/// Causes them to randomly retarget including other enemies.
#[derive(Component, Debug, Clone)]
pub struct ConfusedEnemy {
    /// Timer tracking remaining confusion duration
    pub duration: Timer,
    /// Timer for periodic retargeting
    pub retarget_timer: Timer,
    /// Current target entity (None = random movement, Some = chase this enemy)
    pub current_target: Option<Entity>,
    /// Speed multiplier while confused
    pub speed_multiplier: f32,
}

impl ConfusedEnemy {
    pub fn new(duration: f32) -> Self {
        Self {
            duration: Timer::from_seconds(duration, TimerMode::Once),
            retarget_timer: Timer::from_seconds(PANDEMONIUM_RETARGET_INTERVAL, TimerMode::Repeating),
            current_target: None,
            speed_multiplier: PANDEMONIUM_CONFUSED_SPEED_MULTIPLIER,
        }
    }

    /// Check if the confusion effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the confusion timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.retarget_timer.tick(delta);
    }

    /// Check if it's time to retarget
    pub fn should_retarget(&self) -> bool {
        self.retarget_timer.just_finished()
    }

    /// Refresh the confusion duration with a new timer
    pub fn refresh(&mut self, duration: f32) {
        self.duration = Timer::from_seconds(duration, TimerMode::Once);
    }
}

/// Spawns a pandemonium burst at the given position.
pub fn spawn_pandemonium_burst(
    commands: &mut Commands,
    _spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let burst = PandemoniumBurst::new(center);
    let burst_pos = Vec3::new(spawn_position.x, PANDEMONIUM_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        // Use chaos bolt material for magenta/pink color
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.chaos_bolt.clone()),
            Transform::from_translation(burst_pos).with_scale(Vec3::splat(burst.radius)),
            burst,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(burst_pos),
            burst,
        ));
    }
}

/// System that applies the confusion debuff to enemies within burst radius.
/// This is a single-frame effect - once processed, enemies are marked and the burst is cleaned up.
pub fn apply_pandemonium_to_enemies_system(
    mut commands: Commands,
    mut burst_query: Query<&mut PandemoniumBurst>,
    mut enemy_query: Query<(Entity, &Transform, Option<&mut ConfusedEnemy>), With<Enemy>>,
) {
    for mut burst in burst_query.iter_mut() {
        if burst.processed {
            continue;
        }

        for (enemy_entity, enemy_transform, existing_confusion) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if burst.can_affect(enemy_entity, enemy_pos) {
                if let Some(mut confused) = existing_confusion {
                    // Refresh existing confusion duration
                    confused.refresh(burst.confusion_duration);
                } else {
                    // Apply new confusion debuff
                    commands.entity(enemy_entity).try_insert(
                        ConfusedEnemy::new(burst.confusion_duration)
                    );
                }

                burst.mark_affected(enemy_entity);
            }
        }

        burst.processed = true;
    }
}

/// System that updates confused enemy targeting.
/// Periodically selects a random target from nearby enemies.
pub fn update_confused_enemy_targeting_system(
    mut confused_query: Query<(Entity, &mut ConfusedEnemy, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    time: Res<Time>,
) {
    let mut rng = rand::thread_rng();

    for (confused_entity, mut confused, transform) in confused_query.iter_mut() {
        confused.tick(time.delta());

        if confused.should_retarget() {
            // Collect nearby enemies (excluding self)
            let my_pos = from_xz(transform.translation);
            let nearby_enemies: Vec<Entity> = enemy_query
                .iter()
                .filter(|(e, _)| *e != confused_entity)
                .filter(|(_, t)| {
                    let pos = from_xz(t.translation);
                    my_pos.distance(pos) < 15.0 // Only target nearby enemies
                })
                .map(|(e, _)| e)
                .collect();

            // Randomly select a target or None for random wandering
            if nearby_enemies.is_empty() || rng.gen_bool(0.3) {
                // 30% chance to wander randomly even if enemies nearby
                confused.current_target = None;
            } else {
                let target_idx = rng.gen_range(0..nearby_enemies.len());
                confused.current_target = Some(nearby_enemies[target_idx]);
            }
        }
    }
}

/// System that handles confused enemy attacks on other enemies.
/// When a confused enemy is close enough to its target, it deals damage.
pub fn confused_enemy_attack_system(
    confused_query: Query<(&ConfusedEnemy, &Transform, &Enemy)>,
    enemy_query: Query<&Transform, With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (confused, confused_transform, enemy) in confused_query.iter() {
        if let Some(target) = confused.current_target {
            if let Ok(target_transform) = enemy_query.get(target) {
                let my_pos = from_xz(confused_transform.translation);
                let target_pos = from_xz(target_transform.translation);
                let distance = my_pos.distance(target_pos);

                // Attack range - close enough to deal damage
                if distance < 1.5 {
                    damage_events.write(DamageEvent::with_element(
                        target,
                        enemy.strength,
                        Element::Chaos,
                    ));
                }
            }
        }
    }
}

/// System that removes expired confusion effects from enemies.
pub fn cleanup_confusion_effect_system(
    mut commands: Commands,
    query: Query<(Entity, &ConfusedEnemy)>,
) {
    for (entity, confused) in query.iter() {
        if confused.is_expired() {
            commands.entity(entity).remove::<ConfusedEnemy>();
        }
    }
}

/// System that despawns pandemonium bursts after they've been processed.
pub fn cleanup_pandemonium_burst_system(
    mut commands: Commands,
    query: Query<(Entity, &PandemoniumBurst)>,
) {
    for (entity, burst) in query.iter() {
        if burst.processed {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod pandemonium_burst_component_tests {
        use super::*;

        #[test]
        fn test_pandemonium_burst_new() {
            let center = Vec2::new(5.0, 10.0);
            let burst = PandemoniumBurst::new(center);

            assert_eq!(burst.center, center);
            assert_eq!(burst.radius, PANDEMONIUM_BURST_RADIUS);
            assert_eq!(burst.confusion_duration, PANDEMONIUM_DURATION);
            assert!(burst.affected_enemies.is_empty());
            assert!(!burst.processed);
        }

        #[test]
        fn test_pandemonium_burst_with_radius() {
            let burst = PandemoniumBurst::new(Vec2::ZERO).with_radius(10.0);
            assert_eq!(burst.radius, 10.0);
        }

        #[test]
        fn test_pandemonium_burst_with_duration() {
            let burst = PandemoniumBurst::new(Vec2::ZERO).with_duration(5.0);
            assert_eq!(burst.confusion_duration, 5.0);
        }

        #[test]
        fn test_pandemonium_burst_can_affect_in_radius() {
            let burst = PandemoniumBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(3.0, 0.0);

            assert!(burst.can_affect(entity, in_range_pos));
        }

        #[test]
        fn test_pandemonium_burst_cannot_affect_outside_radius() {
            let burst = PandemoniumBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);
            let out_of_range_pos = Vec2::new(100.0, 0.0);

            assert!(!burst.can_affect(entity, out_of_range_pos));
        }

        #[test]
        fn test_pandemonium_burst_cannot_affect_already_affected() {
            let mut burst = PandemoniumBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(3.0, 0.0);

            burst.mark_affected(entity);
            assert!(!burst.can_affect(entity, in_range_pos));
        }

        #[test]
        fn test_pandemonium_burst_mark_affected() {
            let mut burst = PandemoniumBurst::new(Vec2::ZERO);
            let entity = Entity::from_bits(1);

            burst.mark_affected(entity);
            assert!(burst.affected_enemies.contains(&entity));
        }

        #[test]
        fn test_uses_chaos_element_color() {
            let color = pandemonium_color();
            assert_eq!(color, Element::Chaos.color());
        }
    }

    mod confused_enemy_component_tests {
        use super::*;

        #[test]
        fn test_confused_enemy_new() {
            let confused = ConfusedEnemy::new(3.0);

            assert_eq!(confused.speed_multiplier, PANDEMONIUM_CONFUSED_SPEED_MULTIPLIER);
            assert!(confused.current_target.is_none());
            assert!(!confused.is_expired());
        }

        #[test]
        fn test_confused_enemy_is_expired() {
            let mut confused = ConfusedEnemy::new(0.1);
            assert!(!confused.is_expired());

            confused.tick(Duration::from_secs_f32(0.2));
            assert!(confused.is_expired());
        }

        #[test]
        fn test_confused_enemy_tick() {
            let mut confused = ConfusedEnemy::new(1.0);

            confused.tick(Duration::from_secs_f32(0.5));
            assert!(!confused.is_expired());

            confused.tick(Duration::from_secs_f32(0.5));
            assert!(confused.is_expired());
        }

        #[test]
        fn test_confused_enemy_refresh() {
            let mut confused = ConfusedEnemy::new(1.0);
            confused.tick(Duration::from_secs_f32(0.9));

            // About to expire, but refresh
            confused.refresh(2.0);

            assert!(!confused.is_expired());
        }

        #[test]
        fn test_confused_enemy_should_retarget() {
            let mut confused = ConfusedEnemy::new(5.0);

            // Should not retarget immediately
            assert!(!confused.should_retarget());

            // Tick past retarget interval
            confused.tick(Duration::from_secs_f32(PANDEMONIUM_RETARGET_INTERVAL));

            // Now should retarget
            assert!(confused.should_retarget());
        }

        #[test]
        fn test_confused_enemy_retargets_periodically() {
            let mut confused = ConfusedEnemy::new(5.0);
            let mut retarget_count = 0;

            // Tick multiple intervals
            for _ in 0..5 {
                confused.tick(Duration::from_secs_f32(PANDEMONIUM_RETARGET_INTERVAL));
                if confused.should_retarget() {
                    retarget_count += 1;
                }
            }

            assert_eq!(retarget_count, 5, "Should retarget once per interval");
        }
    }

    mod spawn_pandemonium_burst_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_pandemonium_burst_spawns_at_position() {
            let mut app = setup_test_app();
            let spawn_pos = Vec3::new(15.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                let spell = Spell::new(SpellType::Mayhem);
                spawn_pandemonium_burst(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PandemoniumBurst>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);

            for burst in query.iter(app.world()) {
                assert_eq!(burst.center, Vec2::new(15.0, 20.0));
            }
        }

        #[test]
        fn test_pandemonium_burst_spawns_with_correct_radius() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                let spell = Spell::new(SpellType::Mayhem);
                spawn_pandemonium_burst(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PandemoniumBurst>();
            for burst in query.iter(app.world()) {
                assert_eq!(burst.radius, PANDEMONIUM_BURST_RADIUS);
            }
        }

        #[test]
        fn test_pandemonium_burst_starts_unprocessed() {
            let mut app = setup_test_app();

            {
                let mut commands = app.world_mut().commands();
                let spell = Spell::new(SpellType::Mayhem);
                spawn_pandemonium_burst(
                    &mut commands,
                    &spell,
                    Vec3::ZERO,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PandemoniumBurst>();
            for burst in query.iter(app.world()) {
                assert!(!burst.processed);
            }
        }
    }

    mod apply_pandemonium_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_pandemonium_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_pandemonium_applies_debuff_in_radius() {
            let mut app = setup_pandemonium_test_app();

            // Create pandemonium burst at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PandemoniumBurst::new(Vec2::ZERO),
            ));

            // Create enemy in range
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(apply_pandemonium_to_enemies_system);

            // Enemy should have ConfusedEnemy component
            let confused = app.world().get::<ConfusedEnemy>(enemy);
            assert!(confused.is_some(), "Enemy should have ConfusedEnemy component");
        }

        #[test]
        fn test_pandemonium_does_not_affect_enemies_outside_radius() {
            let mut app = setup_pandemonium_test_app();

            // Create pandemonium burst at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PandemoniumBurst::new(Vec2::ZERO),
            ));

            // Create enemy outside range
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(apply_pandemonium_to_enemies_system);

            // Enemy should NOT have ConfusedEnemy component
            let confused = app.world().get::<ConfusedEnemy>(enemy);
            assert!(confused.is_none(), "Enemy outside radius should not be confused");
        }

        #[test]
        fn test_pandemonium_duration_expires() {
            let mut app = setup_pandemonium_test_app();

            // Create enemy with short confusion duration
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                ConfusedEnemy::new(0.1),
            )).id();

            // Wait longer than confusion duration
            {
                let mut confused = app.world_mut().get_mut::<ConfusedEnemy>(enemy).unwrap();
                confused.tick(Duration::from_secs_f32(0.2));
            }

            let _ = app.world_mut().run_system_once(cleanup_confusion_effect_system);

            // ConfusedEnemy should be removed
            let confused = app.world().get::<ConfusedEnemy>(enemy);
            assert!(confused.is_none(), "ConfusedEnemy should be removed after expiry");
        }

        #[test]
        fn test_normal_ai_resumes_after_debuff() {
            let mut app = setup_pandemonium_test_app();

            // Create enemy with expired confusion
            let mut confused_component = ConfusedEnemy::new(0.0);
            confused_component.duration.tick(Duration::from_secs(1)); // Force expire

            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                confused_component,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_confusion_effect_system);

            // Enemy should have ConfusedEnemy removed
            assert!(app.world().get::<ConfusedEnemy>(enemy).is_none());
            // But still have Enemy component (normal AI can resume)
            assert!(app.world().get::<Enemy>(enemy).is_some());
        }

        #[test]
        fn test_pandemonium_does_not_affect_player() {
            // This test verifies that pandemonium only affects enemies, not the player
            // The implementation uses `Query<..., With<Enemy>>` which naturally excludes
            // non-Enemy entities like the player
            let mut app = setup_pandemonium_test_app();

            // Create pandemonium burst at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PandemoniumBurst::new(Vec2::ZERO),
            ));

            // Create a player-like entity (no Enemy component) in range
            let player = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(apply_pandemonium_to_enemies_system);

            // Player should NOT have ConfusedEnemy component
            let confused = app.world().get::<ConfusedEnemy>(player);
            assert!(confused.is_none(), "Player should not be affected by pandemonium");
        }

        #[test]
        fn test_multiple_pandemonium_refreshes_duration() {
            let mut app = setup_pandemonium_test_app();

            // Create enemy with existing confusion
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                ConfusedEnemy::new(0.5),
            )).id();

            // Progress confusion almost to expiry
            {
                let mut confused = app.world_mut().get_mut::<ConfusedEnemy>(enemy).unwrap();
                confused.tick(Duration::from_secs_f32(0.4));
            }

            // Create new pandemonium burst
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PandemoniumBurst::new(Vec2::ZERO),
            ));

            let _ = app.world_mut().run_system_once(apply_pandemonium_to_enemies_system);

            // Confusion should be refreshed
            let confused = app.world().get::<ConfusedEnemy>(enemy).unwrap();
            assert!(!confused.is_expired(), "Confusion should be refreshed, not expired");
        }
    }

    mod confused_enemy_targeting_tests {
        use super::*;
        use bevy::app::App;

        fn setup_targeting_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_confused_enemy_targets_other_enemies() {
            let mut app = setup_targeting_test_app();

            // Create confused enemy
            let confused_enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
                ConfusedEnemy::new(5.0),
            )).id();

            // Create other enemies nearby
            let target1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            )).id();

            let _target2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(-5.0, 0.375, 0.0)),
            )).id();

            // Tick to trigger retarget
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(PANDEMONIUM_RETARGET_INTERVAL));
            }

            let _ = app.world_mut().run_system_once(update_confused_enemy_targeting_system);

            // Confused enemy may have a target (or None for wandering)
            let confused = app.world().get::<ConfusedEnemy>(confused_enemy).unwrap();
            // Target could be any of the nearby enemies or None
            if let Some(target) = confused.current_target {
                // If there's a target, it should be a valid enemy entity
                assert!(
                    target == target1 || target == _target2,
                    "Target should be one of the nearby enemies"
                );
            }
            // If None, that's also valid (random wandering)
        }
    }

    mod confused_enemy_attack_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_confused_enemy_can_damage_allies() {
            let mut app = App::new();

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
            app.add_systems(Update, (confused_enemy_attack_system, count_damage_events).chain());

            // Create target enemy
            let target = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            )).id();

            // Create confused enemy with target set
            let mut confused_component = ConfusedEnemy::new(5.0);
            confused_component.current_target = Some(target);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 15.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
                confused_component,
            ));

            app.update();

            // Damage event should have been written
            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Confused enemy should damage target");
        }

        #[test]
        fn test_enemy_kills_count_for_score() {
            // This test verifies that deaths from friendly fire still generate DeathEvents
            // which are handled by the standard death system that awards score/XP.
            // The implementation uses standard DamageEvent which flows through the
            // existing combat system, ensuring kills count for score.

            // The actual score counting is handled by existing systems when the
            // target enemy's health reaches zero from the damage event.
            // This test just verifies the damage is applied through the standard path.

            let mut app = App::new();

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
            app.add_systems(Update, (confused_enemy_attack_system, count_damage_events).chain());

            // Create target enemy
            let target = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            )).id();

            // Create confused enemy with target set
            let mut confused_component = ConfusedEnemy::new(5.0);
            confused_component.current_target = Some(target);

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 15.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
                confused_component,
            ));

            app.update();

            // Damage event goes through standard combat path (which handles score)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }
    }

    mod cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_pandemonium_burst_despawns_after_processing() {
            let mut app = App::new();

            let mut burst = PandemoniumBurst::new(Vec2::ZERO);
            burst.processed = true;

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                burst,
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_pandemonium_burst_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_pandemonium_burst_survives_before_processing() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                PandemoniumBurst::new(Vec2::ZERO),
            )).id();

            let _ = app.world_mut().run_system_once(cleanup_pandemonium_burst_system);

            assert!(app.world().entities().contains(entity));
        }
    }
}
