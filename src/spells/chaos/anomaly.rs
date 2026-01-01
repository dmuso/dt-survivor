//! Anomaly spell - Zone with constantly shifting elemental effects.
//!
//! A Chaos element spell (Paradox SpellType) that creates a zone at a target
//! location that cycles through different damage types - fire, frost, poison,
//! lightning - rotating the effect applied to enemies within.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;
use crate::spells::fire::fireball::BurnEffect;
use crate::spells::frost::ice_shard::SlowedDebuff;
use crate::spells::poison::corrode::CorrodedDebuff;
use crate::spells::chaos::chaos_bolt::StunnedEnemy;

/// Default configuration for Anomaly spell
pub const ANOMALY_ZONE_RADIUS: f32 = 6.0;
pub const ANOMALY_ZONE_DURATION: f32 = 8.0;
pub const ANOMALY_EFFECT_CYCLE_INTERVAL: f32 = 2.0;
pub const ANOMALY_DAMAGE_TICK_INTERVAL: f32 = 0.5;
pub const ANOMALY_VISUAL_HEIGHT: f32 = 0.2;

// Effect-specific constants
pub const ANOMALY_BURN_TICK_DAMAGE: f32 = 5.0;
pub const ANOMALY_SLOW_FACTOR: f32 = 0.4;
pub const ANOMALY_SLOW_DURATION: f32 = 1.5;
pub const ANOMALY_CORRODED_DURATION: f32 = 3.0;
pub const ANOMALY_CORRODED_MULTIPLIER: f32 = 1.15; // 15% more damage taken
pub const ANOMALY_STUN_DURATION: f32 = 0.5;
pub const ANOMALY_STUN_CHANCE: f32 = 0.3;

/// Get the chaos element color for visual effects (magenta)
pub fn anomaly_zone_color() -> Color {
    Element::Chaos.color()
}

/// The current effect type cycling through the Anomaly zone.
/// Each effect applies different damage/debuff to enemies within.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnomalyEffect {
    #[default]
    Fire,    // Burn damage over time
    Frost,   // Slow + cold damage
    Poison,  // Poison DOT
    Lightning, // Burst damage + stun chance
}

impl AnomalyEffect {
    /// Get the next effect in the cycle
    pub fn next(&self) -> Self {
        match self {
            AnomalyEffect::Fire => AnomalyEffect::Frost,
            AnomalyEffect::Frost => AnomalyEffect::Poison,
            AnomalyEffect::Poison => AnomalyEffect::Lightning,
            AnomalyEffect::Lightning => AnomalyEffect::Fire,
        }
    }

    /// Get the element for this effect (for damage type)
    pub fn element(&self) -> Element {
        match self {
            AnomalyEffect::Fire => Element::Fire,
            AnomalyEffect::Frost => Element::Frost,
            AnomalyEffect::Poison => Element::Poison,
            AnomalyEffect::Lightning => Element::Lightning,
        }
    }

    /// Get the effect by index (0=Fire, 1=Frost, 2=Poison, 3=Lightning)
    pub fn from_index(index: u8) -> Self {
        match index % 4 {
            0 => AnomalyEffect::Fire,
            1 => AnomalyEffect::Frost,
            2 => AnomalyEffect::Poison,
            _ => AnomalyEffect::Lightning,
        }
    }

    /// Returns the number of possible effects
    pub fn count() -> u8 {
        4
    }
}

/// Component for the Anomaly zone.
/// Creates a field at a location that cycles through different elemental effects.
#[derive(Component, Debug, Clone)]
pub struct AnomalyZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the effect zone
    pub radius: f32,
    /// Remaining duration of the zone
    pub duration: Timer,
    /// Current active effect type
    pub current_effect: AnomalyEffect,
    /// Timer for cycling between effects
    pub effect_timer: Timer,
    /// Base damage dealt per tick
    pub damage_per_tick: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
}

impl AnomalyZone {
    /// Create a new anomaly zone at the given center position.
    pub fn new(center: Vec2, base_damage: f32) -> Self {
        Self {
            center,
            radius: ANOMALY_ZONE_RADIUS,
            duration: Timer::from_seconds(ANOMALY_ZONE_DURATION, TimerMode::Once),
            current_effect: AnomalyEffect::default(),
            effect_timer: Timer::from_seconds(ANOMALY_EFFECT_CYCLE_INTERVAL, TimerMode::Repeating),
            damage_per_tick: base_damage,
            tick_timer: Timer::from_seconds(ANOMALY_DAMAGE_TICK_INTERVAL, TimerMode::Repeating),
        }
    }

    /// Create an anomaly zone from a Spell component.
    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the zone has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick all timers. Returns (should_cycle_effect, should_damage).
    pub fn tick(&mut self, delta: std::time::Duration) -> (bool, bool) {
        self.duration.tick(delta);
        self.effect_timer.tick(delta);
        self.tick_timer.tick(delta);

        let should_cycle = self.effect_timer.just_finished();
        let should_damage = self.tick_timer.just_finished();

        if should_cycle {
            self.current_effect = self.current_effect.next();
        }

        (should_cycle, should_damage)
    }

    /// Check if an entity at the given position is within the zone.
    pub fn is_in_zone(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }
}

/// System that ticks anomaly zone timers and handles effect cycling.
pub fn anomaly_zone_tick_system(
    mut zone_query: Query<&mut AnomalyZone>,
    time: Res<Time>,
) {
    for mut zone in zone_query.iter_mut() {
        zone.tick(time.delta());
    }
}

/// System that damages enemies within anomaly zones and applies the current effect.
pub fn anomaly_zone_damage_system(
    mut commands: Commands,
    zone_query: Query<&AnomalyZone>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for zone in zone_query.iter() {
        if !zone.tick_timer.just_finished() {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if zone.is_in_zone(enemy_pos) {
                // Apply base damage with the current effect's element
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    zone.damage_per_tick,
                    zone.current_effect.element(),
                ));

                // Apply the current effect's debuff
                apply_anomaly_effect(
                    &mut commands,
                    enemy_entity,
                    zone.current_effect,
                    zone.damage_per_tick,
                );
            }
        }
    }
}

/// Apply the current anomaly effect's debuff to an enemy
fn apply_anomaly_effect(
    commands: &mut Commands,
    enemy_entity: Entity,
    effect: AnomalyEffect,
    base_damage: f32,
) {
    match effect {
        AnomalyEffect::Fire => {
            commands
                .entity(enemy_entity)
                .insert(BurnEffect::new(base_damage * 0.2 + ANOMALY_BURN_TICK_DAMAGE));
        }
        AnomalyEffect::Frost => {
            commands
                .entity(enemy_entity)
                .insert(SlowedDebuff::new(ANOMALY_SLOW_DURATION, ANOMALY_SLOW_FACTOR));
        }
        AnomalyEffect::Poison => {
            commands
                .entity(enemy_entity)
                .insert(CorrodedDebuff::new(ANOMALY_CORRODED_DURATION, ANOMALY_CORRODED_MULTIPLIER));
        }
        AnomalyEffect::Lightning => {
            // Lightning has a chance to stun
            if rand::random::<f32>() < ANOMALY_STUN_CHANCE {
                commands
                    .entity(enemy_entity)
                    .insert(StunnedEnemy::new(ANOMALY_STUN_DURATION));
            }
        }
    }
}

/// System that despawns anomaly zones when their duration expires.
pub fn anomaly_zone_cleanup_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &AnomalyZone)>,
) {
    for (entity, zone) in zone_query.iter() {
        if zone.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast anomaly spell - spawns a zone at target location.
#[allow(clippy::too_many_arguments)]
pub fn fire_anomaly(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_anomaly_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast anomaly spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_anomaly_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    _spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let zone = AnomalyZone::new(target_pos, damage);
    let zone_pos = Vec3::new(target_pos.x, ANOMALY_VISUAL_HEIGHT, target_pos.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.chaos_bolt.clone()),
            Transform::from_translation(zone_pos).with_scale(Vec3::splat(0.1)),
            zone,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(zone_pos),
            zone,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod anomaly_effect_tests {
        use super::*;

        #[test]
        fn test_anomaly_effect_from_index_covers_all() {
            assert_eq!(AnomalyEffect::from_index(0), AnomalyEffect::Fire);
            assert_eq!(AnomalyEffect::from_index(1), AnomalyEffect::Frost);
            assert_eq!(AnomalyEffect::from_index(2), AnomalyEffect::Poison);
            assert_eq!(AnomalyEffect::from_index(3), AnomalyEffect::Lightning);
        }

        #[test]
        fn test_anomaly_effect_from_index_wraps() {
            assert_eq!(AnomalyEffect::from_index(4), AnomalyEffect::Fire);
            assert_eq!(AnomalyEffect::from_index(5), AnomalyEffect::Frost);
        }

        #[test]
        fn test_anomaly_effect_count() {
            assert_eq!(AnomalyEffect::count(), 4);
        }

        #[test]
        fn test_anomaly_effect_next_cycles() {
            let fire = AnomalyEffect::Fire;
            let frost = fire.next();
            let poison = frost.next();
            let lightning = poison.next();
            let back_to_fire = lightning.next();

            assert_eq!(frost, AnomalyEffect::Frost);
            assert_eq!(poison, AnomalyEffect::Poison);
            assert_eq!(lightning, AnomalyEffect::Lightning);
            assert_eq!(back_to_fire, AnomalyEffect::Fire);
        }

        #[test]
        fn test_anomaly_effect_element_mapping() {
            assert_eq!(AnomalyEffect::Fire.element(), Element::Fire);
            assert_eq!(AnomalyEffect::Frost.element(), Element::Frost);
            assert_eq!(AnomalyEffect::Poison.element(), Element::Poison);
            assert_eq!(AnomalyEffect::Lightning.element(), Element::Lightning);
        }

        #[test]
        fn test_anomaly_effect_default_is_fire() {
            assert_eq!(AnomalyEffect::default(), AnomalyEffect::Fire);
        }
    }

    mod anomaly_zone_component_tests {
        use super::*;

        #[test]
        fn test_anomaly_zone_spawns_at_location() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 25.0;
            let zone = AnomalyZone::new(center, damage);

            assert_eq!(zone.center, center);
            assert_eq!(zone.damage_per_tick, damage);
            assert_eq!(zone.radius, ANOMALY_ZONE_RADIUS);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_anomaly_zone_has_correct_radius() {
            let zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            assert_eq!(zone.radius, ANOMALY_ZONE_RADIUS);
        }

        #[test]
        fn test_anomaly_zone_starts_with_fire_effect() {
            let zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            assert_eq!(zone.current_effect, AnomalyEffect::Fire);
        }

        #[test]
        fn test_anomaly_cycles_through_effects() {
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);

            // Track effects we see
            let mut effects_seen = vec![zone.current_effect];

            // Cycle through all effects
            for _ in 0..4 {
                zone.tick(Duration::from_secs_f32(ANOMALY_EFFECT_CYCLE_INTERVAL));
                effects_seen.push(zone.current_effect);
            }

            // Should have cycled through Fire -> Frost -> Poison -> Lightning -> Fire
            assert_eq!(effects_seen[0], AnomalyEffect::Fire);
            assert_eq!(effects_seen[1], AnomalyEffect::Frost);
            assert_eq!(effects_seen[2], AnomalyEffect::Poison);
            assert_eq!(effects_seen[3], AnomalyEffect::Lightning);
            assert_eq!(effects_seen[4], AnomalyEffect::Fire);
        }

        #[test]
        fn test_anomaly_zone_is_in_zone() {
            let zone = AnomalyZone::new(Vec2::new(10.0, 10.0), 20.0);

            // Inside zone
            assert!(zone.is_in_zone(Vec2::new(10.0, 10.0))); // Center
            assert!(zone.is_in_zone(Vec2::new(12.0, 10.0))); // Within radius

            // On edge
            assert!(zone.is_in_zone(Vec2::new(10.0 + ANOMALY_ZONE_RADIUS, 10.0)));

            // Outside zone
            assert!(!zone.is_in_zone(Vec2::new(10.0 + ANOMALY_ZONE_RADIUS + 0.1, 10.0)));
            assert!(!zone.is_in_zone(Vec2::new(100.0, 100.0)));
        }

        #[test]
        fn test_anomaly_zone_duration_expires() {
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            assert!(!zone.is_expired());

            // Tick past duration
            zone.tick(Duration::from_secs_f32(ANOMALY_ZONE_DURATION + 0.1));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_anomaly_zone_from_spell() {
            let spell = Spell::new(SpellType::Paradox);
            let center = Vec2::new(5.0, 15.0);
            let zone = AnomalyZone::from_spell(center, &spell);

            assert_eq!(zone.center, center);
            assert_eq!(zone.damage_per_tick, spell.damage());
        }

        #[test]
        fn test_anomaly_zone_uses_chaos_element_color() {
            let color = anomaly_zone_color();
            assert_eq!(color, Element::Chaos.color());
        }

        #[test]
        fn test_anomaly_zone_tick_returns_correct_flags() {
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);

            // Small tick - no cycle, no damage
            let (cycle, damage) = zone.tick(Duration::from_secs_f32(0.1));
            assert!(!cycle);
            assert!(!damage);

            // Tick to damage threshold
            let mut zone2 = AnomalyZone::new(Vec2::ZERO, 20.0);
            let (cycle, damage) = zone2.tick(Duration::from_secs_f32(ANOMALY_DAMAGE_TICK_INTERVAL));
            assert!(!cycle);
            assert!(damage);

            // Tick to effect cycle threshold
            let mut zone3 = AnomalyZone::new(Vec2::ZERO, 20.0);
            let (cycle, _damage) = zone3.tick(Duration::from_secs_f32(ANOMALY_EFFECT_CYCLE_INTERVAL));
            assert!(cycle);
        }
    }

    mod anomaly_zone_tick_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_anomaly_zone_tick_updates_timer() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AnomalyZone::new(Vec2::ZERO, 20.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(anomaly_zone_tick_system);

            let zone = app.world().get::<AnomalyZone>(entity).unwrap();
            assert!(
                zone.duration.elapsed_secs() > 0.9,
                "Duration timer should have ticked"
            );
        }

        #[test]
        fn test_anomaly_zone_effect_cycles_on_timer() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                AnomalyZone::new(Vec2::ZERO, 20.0),
            )).id();

            // Verify initial effect is Fire
            {
                let zone = app.world().get::<AnomalyZone>(entity).unwrap();
                assert_eq!(zone.current_effect, AnomalyEffect::Fire);
            }

            // Advance time to trigger effect cycle
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ANOMALY_EFFECT_CYCLE_INTERVAL));
            }

            let _ = app.world_mut().run_system_once(anomaly_zone_tick_system);

            // Effect should have cycled to Frost
            let zone = app.world().get::<AnomalyZone>(entity).unwrap();
            assert_eq!(zone.current_effect, AnomalyEffect::Frost);
        }
    }

    mod anomaly_zone_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_anomaly_zone_damages_enemies_in_zone() {
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
            app.add_systems(Update, (anomaly_zone_damage_system, count_damage_events).chain());

            // Create zone at origin that should damage (tick timer just finished)
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            zone.tick_timer.tick(Duration::from_secs_f32(ANOMALY_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_anomaly_zone_ignores_enemies_outside() {
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
            app.add_systems(Update, (anomaly_zone_damage_system, count_damage_events).chain());

            // Create zone with tick timer ready
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            zone.tick_timer.tick(Duration::from_secs_f32(ANOMALY_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create enemy outside radius (100 units away)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_anomaly_zone_no_damage_before_tick() {
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
            app.add_systems(Update, (anomaly_zone_damage_system, count_damage_events).chain());

            // Create zone that hasn't ticked yet
            let zone = AnomalyZone::new(Vec2::ZERO, 20.0);

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create enemy within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            app.update();

            // No damage yet, tick timer hasn't fired
            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_anomaly_zone_damages_multiple_enemies() {
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
            app.add_systems(Update, (anomaly_zone_damage_system, count_damage_events).chain());

            // Create zone
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            zone.tick_timer.tick(Duration::from_secs_f32(ANOMALY_DAMAGE_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create 3 enemies within radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }
    }

    mod anomaly_zone_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_anomaly_zone_despawns_after_duration() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create a zone that's already expired
            let mut zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            zone.duration.tick(Duration::from_secs_f32(ANOMALY_ZONE_DURATION + 0.1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(anomaly_zone_cleanup_system);

            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_anomaly_zone_survives_before_expiry() {
            let mut app = App::new();

            let zone = AnomalyZone::new(Vec2::ZERO, 20.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(anomaly_zone_cleanup_system);

            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod anomaly_effect_application_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_anomaly_fire_effect_burns() {
            let mut app = setup_test_app();

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                apply_anomaly_effect(&mut commands, enemy_entity, AnomalyEffect::Fire, 20.0);
            }
            app.update();

            let burn = app.world().get::<BurnEffect>(enemy_entity);
            assert!(burn.is_some(), "BurnEffect should be applied by Fire effect");
        }

        #[test]
        fn test_anomaly_frost_effect_slows() {
            let mut app = setup_test_app();

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                apply_anomaly_effect(&mut commands, enemy_entity, AnomalyEffect::Frost, 20.0);
            }
            app.update();

            let slowed = app.world().get::<SlowedDebuff>(enemy_entity);
            assert!(slowed.is_some(), "SlowedDebuff should be applied by Frost effect");
            let slowed = slowed.unwrap();
            assert_eq!(slowed.speed_multiplier, ANOMALY_SLOW_FACTOR);
        }

        #[test]
        fn test_anomaly_poison_effect_corrodes() {
            let mut app = setup_test_app();

            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            {
                let mut commands = app.world_mut().commands();
                apply_anomaly_effect(&mut commands, enemy_entity, AnomalyEffect::Poison, 20.0);
            }
            app.update();

            let corroded = app.world().get::<CorrodedDebuff>(enemy_entity);
            assert!(corroded.is_some(), "CorrodedDebuff should be applied by Poison effect");
            let corroded = corroded.unwrap();
            assert_eq!(corroded.damage_multiplier, ANOMALY_CORRODED_MULTIPLIER);
        }

        #[test]
        fn test_anomaly_lightning_effect_can_stun() {
            // Lightning has a chance to stun - we test many times to verify it can happen
            let mut stun_applied = false;

            for _ in 0..100 {
                let mut app = setup_test_app();

                let enemy_entity = app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
                )).id();

                {
                    let mut commands = app.world_mut().commands();
                    apply_anomaly_effect(&mut commands, enemy_entity, AnomalyEffect::Lightning, 20.0);
                }
                app.update();

                if app.world().get::<StunnedEnemy>(enemy_entity).is_some() {
                    stun_applied = true;
                    break;
                }
            }

            assert!(stun_applied, "Lightning effect should be able to stun with 30% chance");
        }
    }

    mod fire_anomaly_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_anomaly_spawns_zone() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_anomaly(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&AnomalyZone>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_anomaly_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(15.0, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_anomaly(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&AnomalyZone>();
            for zone in query.iter(app.world()) {
                assert_eq!(zone.center, target_pos);
            }
        }

        #[test]
        fn test_fire_anomaly_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_anomaly(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&AnomalyZone>();
            for zone in query.iter(app.world()) {
                assert_eq!(zone.damage_per_tick, expected_damage);
            }
        }

        #[test]
        fn test_fire_anomaly_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Paradox);
            let explicit_damage = 150.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_anomaly_with_damage(
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

            let mut query = app.world_mut().query::<&AnomalyZone>();
            for zone in query.iter(app.world()) {
                assert_eq!(zone.damage_per_tick, explicit_damage);
            }
        }
    }
}
