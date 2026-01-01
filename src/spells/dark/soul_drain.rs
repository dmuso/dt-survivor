use bevy::prelude::*;
use crate::combat::{DamageEvent, Health};
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Soul Drain spell
pub const SOUL_DRAIN_RADIUS: f32 = 6.0;
pub const SOUL_DRAIN_TICK_INTERVAL: f32 = 0.5;
pub const SOUL_DRAIN_HEAL_PERCENTAGE: f32 = 0.25; // 25% of damage dealt returned as healing
pub const SOUL_DRAIN_VISUAL_HEIGHT: f32 = 0.1;

/// Get the dark element color for visual effects (purple)
pub fn soul_drain_color() -> Color {
    Element::Dark.color()
}

/// SoulDrainAura component - a pulsing dark energy aura centered on the player
/// that damages all enemies within radius and heals the player for a percentage
/// of damage dealt.
#[derive(Component, Debug, Clone)]
pub struct SoulDrainAura {
    /// Center position on XZ plane (follows player/Whisper)
    pub center: Vec2,
    /// Radius of the drain effect
    pub radius: f32,
    /// Damage dealt per tick
    pub damage: f32,
    /// Percentage of damage dealt returned as healing (0.0 to 1.0)
    pub heal_percentage: f32,
    /// Timer for drain tick intervals
    pub tick_timer: Timer,
}

impl SoulDrainAura {
    pub fn new(center: Vec2, damage: f32, radius: f32, heal_percentage: f32) -> Self {
        Self {
            center,
            radius,
            damage,
            heal_percentage,
            tick_timer: Timer::from_seconds(SOUL_DRAIN_TICK_INTERVAL, TimerMode::Repeating),
        }
    }

    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage(), SOUL_DRAIN_RADIUS, SOUL_DRAIN_HEAL_PERCENTAGE)
    }

    /// Check if a position (XZ plane) is inside the aura radius
    pub fn contains(&self, position: Vec2) -> bool {
        self.center.distance(position) <= self.radius
    }
}

/// Marker component for the visual pulse effect that expands outward
#[derive(Component, Debug, Clone)]
pub struct SoulDrainPulseVisual {
    /// Current radius of the visual effect
    pub current_radius: f32,
    /// Maximum radius before despawn
    pub max_radius: f32,
    /// Expansion speed
    pub expansion_speed: f32,
    /// Center position on XZ plane
    pub center: Vec2,
}

impl SoulDrainPulseVisual {
    pub fn new(center: Vec2, max_radius: f32) -> Self {
        Self {
            current_radius: 0.0,
            max_radius,
            expansion_speed: max_radius * 2.0, // Complete expansion in 0.5 seconds
            center,
        }
    }
}

/// System that updates SoulDrainAura tick timers, damages enemies, and heals the player
#[allow(clippy::too_many_arguments)]
pub fn soul_drain_system(
    mut commands: Commands,
    time: Res<Time>,
    mut aura_query: Query<(Entity, &mut SoulDrainAura, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut player_query: Query<&mut Health, With<Player>>,
    mut damage_events: MessageWriter<DamageEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (_aura_entity, mut aura, aura_transform) in aura_query.iter_mut() {
        // Update aura center to follow the attached entity
        aura.center = from_xz(aura_transform.translation);

        aura.tick_timer.tick(time.delta());

        if aura.tick_timer.just_finished() {
            let mut total_damage_dealt = 0.0;

            // Damage all enemies within radius
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);
                if aura.contains(enemy_pos) {
                    damage_events.write(DamageEvent::with_element(
                        enemy_entity,
                        aura.damage,
                        Element::Dark,
                    ));
                    total_damage_dealt += aura.damage;
                }
            }

            // Heal the player for a percentage of total damage dealt
            if total_damage_dealt > 0.0 {
                let heal_amount = total_damage_dealt * aura.heal_percentage;
                if let Ok(mut player_health) = player_query.single_mut() {
                    player_health.heal(heal_amount);
                }
            }

            // Spawn visual pulse effect
            let pulse_visual = SoulDrainPulseVisual::new(aura.center, aura.radius);
            let visual_pos = Vec3::new(aura.center.x, SOUL_DRAIN_VISUAL_HEIGHT, aura.center.y);

            if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), game_materials.as_ref()) {
                commands.spawn((
                    Mesh3d(meshes.explosion.clone()),
                    MeshMaterial3d(materials.explosion.clone()),
                    Transform::from_translation(visual_pos).with_scale(Vec3::splat(0.1)),
                    pulse_visual,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(visual_pos),
                    pulse_visual,
                ));
            }
        }
    }
}

/// System that updates SoulDrainPulseVisual expansion and despawns when complete
pub fn soul_drain_pulse_visual_system(
    mut commands: Commands,
    time: Res<Time>,
    mut visual_query: Query<(Entity, &mut SoulDrainPulseVisual, &mut Transform)>,
) {
    for (entity, mut visual, mut transform) in visual_query.iter_mut() {
        visual.current_radius += visual.expansion_speed * time.delta_secs();

        // Update scale based on current radius
        let scale = visual.current_radius / visual.max_radius * visual.max_radius;
        transform.scale = Vec3::splat(scale.max(0.1));

        // Despawn when fully expanded
        if visual.current_radius >= visual.max_radius {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Soul Drain spell - spawns a pulsing dark aura centered on the spell origin.
/// `spawn_position` is Whisper's full 3D position (where the aura will be centered).
#[allow(clippy::too_many_arguments)]
pub fn fire_soul_drain(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_soul_drain_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Soul Drain spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage per tick (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_soul_drain_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let aura_center = from_xz(spawn_position);
    let aura = SoulDrainAura::new(aura_center, damage, SOUL_DRAIN_RADIUS, SOUL_DRAIN_HEAL_PERCENTAGE);
    let aura_pos = Vec3::new(spawn_position.x, SOUL_DRAIN_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.explosion.clone()),
            Transform::from_translation(aura_pos).with_scale(Vec3::splat(SOUL_DRAIN_RADIUS)),
            aura,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(aura_pos),
            aura,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod soul_drain_aura_component_tests {
        use super::*;

        #[test]
        fn test_soul_drain_aura_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 15.0;
            let heal_percentage = 0.25;
            let aura = SoulDrainAura::new(center, damage, 6.0, heal_percentage);

            assert_eq!(aura.center, center);
            assert_eq!(aura.radius, 6.0);
            assert_eq!(aura.damage, damage);
            assert_eq!(aura.heal_percentage, heal_percentage);
            assert!(!aura.tick_timer.just_finished());
        }

        #[test]
        fn test_soul_drain_aura_from_spell() {
            let spell = Spell::new(SpellType::SoulDrain);
            let center = Vec2::new(5.0, 15.0);
            let aura = SoulDrainAura::from_spell(center, &spell);

            assert_eq!(aura.center, center);
            assert_eq!(aura.radius, SOUL_DRAIN_RADIUS);
            assert_eq!(aura.damage, spell.damage());
            assert_eq!(aura.heal_percentage, SOUL_DRAIN_HEAL_PERCENTAGE);
        }

        #[test]
        fn test_soul_drain_aura_tick_timer_initial_state() {
            let aura = SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25);
            assert!(!aura.tick_timer.just_finished());
            assert_eq!(aura.tick_timer.duration(), Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL));
        }

        #[test]
        fn test_soul_drain_aura_contains_position_inside() {
            let aura = SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25);
            assert!(aura.contains(Vec2::new(3.0, 0.0)));
            assert!(aura.contains(Vec2::new(0.0, 5.0)));
            assert!(aura.contains(Vec2::ZERO));
        }

        #[test]
        fn test_soul_drain_aura_does_not_contain_position_outside() {
            let aura = SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25);
            assert!(!aura.contains(Vec2::new(8.0, 0.0)));
            assert!(!aura.contains(Vec2::new(0.0, 8.0)));
            assert!(!aura.contains(Vec2::new(5.0, 5.0)));
        }

        #[test]
        fn test_soul_drain_aura_contains_position_on_edge() {
            let aura = SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25);
            assert!(aura.contains(Vec2::new(6.0, 0.0)));
            assert!(aura.contains(Vec2::new(0.0, 6.0)));
        }

        #[test]
        fn test_soul_drain_uses_dark_element_color() {
            let color = soul_drain_color();
            assert_eq!(color, Element::Dark.color());
            assert_eq!(color, Color::srgb_u8(128, 0, 128)); // Purple
        }
    }

    mod soul_drain_pulse_visual_tests {
        use super::*;

        #[test]
        fn test_pulse_visual_new() {
            let center = Vec2::new(5.0, 10.0);
            let visual = SoulDrainPulseVisual::new(center, 6.0);

            assert_eq!(visual.center, center);
            assert_eq!(visual.current_radius, 0.0);
            assert_eq!(visual.max_radius, 6.0);
            assert!(visual.expansion_speed > 0.0);
        }

        #[test]
        fn test_pulse_visual_starts_at_zero_radius() {
            let visual = SoulDrainPulseVisual::new(Vec2::ZERO, 6.0);
            assert_eq!(visual.current_radius, 0.0);
        }
    }

    mod soul_drain_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_soul_drain_damages_enemies_in_radius() {
            let mut app = setup_test_app();

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

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create enemy inside aura radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Create player with health (needed for healing)
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
            ));

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system then count events
            let _ = app.world_mut().run_system_once(soul_drain_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy inside radius should take damage");
        }

        #[test]
        fn test_soul_drain_does_not_damage_enemies_outside_radius() {
            let mut app = setup_test_app();

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

            // Create aura at origin with radius 6
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create enemy outside aura radius (distance 10, radius 6)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            // Create player with health
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
            ));

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system then count events
            let _ = app.world_mut().run_system_once(soul_drain_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside radius should not take damage");
        }

        #[test]
        fn test_soul_drain_damages_multiple_enemies() {
            let mut app = setup_test_app();

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

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create 3 enemies inside radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            // Create player with health
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
            ));

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system then count events
            let _ = app.world_mut().run_system_once(soul_drain_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 3, "All 3 enemies inside radius should take damage");
        }

        #[test]
        fn test_soul_drain_heals_player_for_percentage_of_damage() {
            let mut app = setup_test_app();

            // Create aura at origin with 10 damage and 25% heal
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create 2 enemies inside radius (total 20 damage, should heal 5 HP)
            for i in 0..2 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            // Create player with 50 current health (out of 100)
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health { current: 50.0, max: 100.0 },
            )).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system
            let _ = app.world_mut().run_system_once(soul_drain_system);

            // Check player was healed
            let player_health = app.world().get::<Health>(player_entity).unwrap();
            // 2 enemies * 10 damage = 20 total damage * 0.25 = 5 HP healed
            // 50 + 5 = 55 HP
            assert_eq!(player_health.current, 55.0, "Player should be healed for 25% of damage dealt");
        }

        #[test]
        fn test_soul_drain_does_not_heal_when_no_enemies_in_range() {
            let mut app = setup_test_app();

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create enemy OUTSIDE radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Create player with 50 current health
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health { current: 50.0, max: 100.0 },
            )).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system
            let _ = app.world_mut().run_system_once(soul_drain_system);

            // Check player was NOT healed
            let player_health = app.world().get::<Health>(player_entity).unwrap();
            assert_eq!(player_health.current, 50.0, "Player should not be healed when no enemies in range");
        }

        #[test]
        fn test_soul_drain_heal_does_not_exceed_max_health() {
            let mut app = setup_test_app();

            // Create aura at origin with high damage (100) and high heal percentage (100%)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 100.0, 6.0, 1.0), // 100% heal for testing
            ));

            // Create enemy inside radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            // Create player with 90 current health (out of 100)
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health { current: 90.0, max: 100.0 },
            )).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system
            let _ = app.world_mut().run_system_once(soul_drain_system);

            // Check player health is capped at max
            let player_health = app.world().get::<Health>(player_entity).unwrap();
            assert_eq!(player_health.current, 100.0, "Healing should not exceed max health");
        }

        #[test]
        fn test_soul_drain_spawns_visual_on_trigger() {
            let mut app = setup_test_app();

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
            ));

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL + 0.01));
            }

            // Run the soul drain system
            let _ = app.world_mut().run_system_once(soul_drain_system);

            // Should spawn a pulse visual
            let mut visual_query = app.world_mut().query::<&SoulDrainPulseVisual>();
            let visual_count = visual_query.iter(app.world()).count();
            assert_eq!(visual_count, 1, "Soul drain should spawn a visual effect on tick");
        }

        #[test]
        fn test_no_damage_before_tick_interval() {
            let mut app = setup_test_app();

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

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainAura::new(Vec2::ZERO, 10.0, 6.0, 0.25),
            ));

            // Create enemy inside radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Create player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
            ));

            // Advance time but NOT past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SOUL_DRAIN_TICK_INTERVAL / 2.0));
            }

            // Run the soul drain system then count events
            let _ = app.world_mut().run_system_once(soul_drain_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "No damage before tick interval");
        }
    }

    mod soul_drain_pulse_visual_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_systems(Update, soul_drain_pulse_visual_system);
            app.init_resource::<Time>();
            app
        }

        #[test]
        fn test_visual_expands_over_time() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainPulseVisual::new(Vec2::ZERO, 6.0),
            )).id();

            // Initial radius should be 0
            {
                let visual = app.world().get::<SoulDrainPulseVisual>(visual_entity).unwrap();
                assert_eq!(visual.current_radius, 0.0);
            }

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            // Radius should have increased
            {
                let visual = app.world().get::<SoulDrainPulseVisual>(visual_entity).unwrap();
                assert!(visual.current_radius > 0.0, "Visual radius should increase over time");
            }
        }

        #[test]
        fn test_visual_despawns_at_max_radius() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainPulseVisual::new(Vec2::ZERO, 6.0),
            )).id();

            // Advance time past full expansion (0.5 seconds for max_radius * 2.0 speed)
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }
            app.update();

            // Visual should be despawned
            assert!(app.world().get_entity(visual_entity).is_err(), "Visual should despawn at max radius");
        }

        #[test]
        fn test_visual_survives_before_max_radius() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                SoulDrainPulseVisual::new(Vec2::ZERO, 6.0),
            )).id();

            // Advance time but not to max radius
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            // Visual should still exist
            assert!(app.world().get_entity(visual_entity).is_ok(), "Visual should exist before max radius");
        }

        #[test]
        fn test_visual_scale_increases_with_radius() {
            let mut app = setup_test_app();

            let visual_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)).with_scale(Vec3::splat(0.1)),
                SoulDrainPulseVisual::new(Vec2::ZERO, 6.0),
            )).id();

            let initial_scale = app.world().get::<Transform>(visual_entity).unwrap().scale;

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            let new_scale = app.world().get::<Transform>(visual_entity).unwrap().scale;
            assert!(new_scale.x > initial_scale.x, "Scale should increase over time");
        }
    }

    mod fire_soul_drain_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_soul_drain_spawns_aura() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::SoulDrain);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_drain(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 soul drain aura
            let mut query = app.world_mut().query::<&SoulDrainAura>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_soul_drain_at_spawn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::SoulDrain);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_drain(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulDrainAura>();
            for aura in query.iter(app.world()) {
                // Aura center should match spawn XZ (10.0, 20.0)
                assert_eq!(aura.center.x, 10.0);
                assert_eq!(aura.center.y, 20.0); // Z maps to Y in Vec2
            }
        }

        #[test]
        fn test_fire_soul_drain_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::SoulDrain);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_drain(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulDrainAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_soul_drain_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::SoulDrain);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_drain_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulDrainAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_soul_drain_has_correct_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::SoulDrain);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_drain(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulDrainAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.radius, SOUL_DRAIN_RADIUS);
            }
        }

        #[test]
        fn test_fire_soul_drain_has_correct_heal_percentage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::SoulDrain);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_soul_drain(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&SoulDrainAura>();
            for aura in query.iter(app.world()) {
                assert_eq!(aura.heal_percentage, SOUL_DRAIN_HEAL_PERCENTAGE);
            }
        }
    }
}
