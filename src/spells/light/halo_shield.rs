//! Halo Shield spell (DivineLight) - A protective aura that damages enemies on contact.
//!
//! Creates a glowing ring around the player that acts as a barrier. Enemies that
//! touch the ring take damage on contact with a cooldown to prevent rapid multi-hits.

use bevy::prelude::*;
use std::collections::HashMap;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Halo Shield spell
pub const HALO_SHIELD_RADIUS: f32 = 3.0;
pub const HALO_SHIELD_RING_THICKNESS: f32 = 0.5;
pub const HALO_SHIELD_VISUAL_HEIGHT: f32 = 0.5;
pub const HALO_SHIELD_HIT_COOLDOWN: f32 = 0.5; // Seconds between hits on same enemy

/// Get the light element color for visual effects (white/gold)
pub fn halo_shield_color() -> Color {
    Element::Light.color()
}

/// HaloShield component - a protective ring centered on the player that damages enemies on contact.
#[derive(Component, Debug, Clone)]
pub struct HaloShield {
    /// Center position on XZ plane (follows player/Whisper)
    pub center: Vec2,
    /// Radius of the shield ring (inner edge)
    pub radius: f32,
    /// Thickness of the ring
    pub ring_thickness: f32,
    /// Damage dealt on contact
    pub damage: f32,
    /// Tracks hit cooldowns per enemy entity
    pub hit_cooldowns: HashMap<Entity, Timer>,
}

impl HaloShield {
    pub fn new(center: Vec2, damage: f32, radius: f32) -> Self {
        Self {
            center,
            radius,
            ring_thickness: HALO_SHIELD_RING_THICKNESS,
            damage,
            hit_cooldowns: HashMap::new(),
        }
    }

    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage(), HALO_SHIELD_RADIUS)
    }

    /// Check if a position (XZ plane) is touching the ring.
    /// Returns true if the position is between inner radius and outer radius.
    pub fn is_touching_ring(&self, position: Vec2) -> bool {
        let distance = self.center.distance(position);
        let inner_radius = self.radius;
        let outer_radius = self.radius + self.ring_thickness;
        distance >= inner_radius && distance <= outer_radius
    }

    /// Check if an enemy can be hit (not on cooldown)
    pub fn can_hit(&self, enemy: Entity) -> bool {
        match self.hit_cooldowns.get(&enemy) {
            Some(timer) => timer.is_finished(),
            None => true,
        }
    }

    /// Start cooldown for an enemy that was just hit
    pub fn start_cooldown(&mut self, enemy: Entity) {
        self.hit_cooldowns.insert(
            enemy,
            Timer::from_seconds(HALO_SHIELD_HIT_COOLDOWN, TimerMode::Once),
        );
    }

    /// Tick all cooldown timers
    pub fn tick_cooldowns(&mut self, delta: std::time::Duration) {
        for timer in self.hit_cooldowns.values_mut() {
            timer.tick(delta);
        }
        // Remove finished cooldowns to prevent memory growth
        self.hit_cooldowns.retain(|_, timer| !timer.is_finished());
    }
}

/// System that updates HaloShield position to follow the player and ticks cooldowns
pub fn halo_shield_update_system(
    time: Res<Time>,
    mut shield_query: Query<(&mut HaloShield, &Transform)>,
) {
    for (mut shield, transform) in shield_query.iter_mut() {
        // Update shield center to follow the attached entity
        shield.center = from_xz(transform.translation);
        // Tick cooldowns
        shield.tick_cooldowns(time.delta());
    }
}

/// System that detects enemies touching the shield ring and applies damage
pub fn halo_shield_contact_damage_system(
    mut shield_query: Query<&mut HaloShield>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut shield in shield_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if shield.is_touching_ring(enemy_pos) && shield.can_hit(enemy_entity) {
                // Apply damage and start cooldown
                damage_events.write(DamageEvent::new(enemy_entity, shield.damage));
                shield.start_cooldown(enemy_entity);
            }
        }
    }
}

/// Cast Halo Shield spell - spawns a protective ring centered on the spell origin.
/// `spawn_position` is Whisper's full 3D position (where the shield will be centered).
#[allow(clippy::too_many_arguments)]
pub fn fire_halo_shield(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_halo_shield_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Halo Shield spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage per hit (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_halo_shield_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let shield_center = from_xz(spawn_position);
    let shield = HaloShield::new(shield_center, damage, HALO_SHIELD_RADIUS);
    let shield_pos = Vec3::new(spawn_position.x, HALO_SHIELD_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.radiant_beam.clone()),
            Transform::from_translation(shield_pos).with_scale(Vec3::splat(HALO_SHIELD_RADIUS)),
            shield,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(shield_pos),
            shield,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod halo_shield_component_tests {
        use super::*;

        #[test]
        fn test_halo_shield_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 15.0;
            let shield = HaloShield::new(center, damage, 3.0);

            assert_eq!(shield.center, center);
            assert_eq!(shield.radius, 3.0);
            assert_eq!(shield.damage, damage);
            assert!(shield.hit_cooldowns.is_empty());
        }

        #[test]
        fn test_halo_shield_from_spell() {
            let spell = Spell::new(SpellType::DivineLight);
            let center = Vec2::new(5.0, 15.0);
            let shield = HaloShield::from_spell(center, &spell);

            assert_eq!(shield.center, center);
            assert_eq!(shield.radius, HALO_SHIELD_RADIUS);
            assert_eq!(shield.damage, spell.damage());
        }

        #[test]
        fn test_halo_shield_uses_light_element_color() {
            let color = halo_shield_color();
            assert_eq!(color, Element::Light.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 255)); // White
        }
    }

    mod ring_contact_tests {
        use super::*;

        #[test]
        fn test_position_inside_ring_inner_radius() {
            let shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            // Position closer than inner radius
            assert!(!shield.is_touching_ring(Vec2::new(1.0, 0.0)));
            assert!(!shield.is_touching_ring(Vec2::ZERO));
        }

        #[test]
        fn test_position_on_ring_inner_edge() {
            let shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            // Position exactly at inner radius
            assert!(shield.is_touching_ring(Vec2::new(3.0, 0.0)));
        }

        #[test]
        fn test_position_within_ring_thickness() {
            let shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            // Position between inner and outer radius
            assert!(shield.is_touching_ring(Vec2::new(3.25, 0.0)));
        }

        #[test]
        fn test_position_on_ring_outer_edge() {
            let shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            // Position exactly at outer radius (3.0 + 0.5 = 3.5)
            assert!(shield.is_touching_ring(Vec2::new(3.5, 0.0)));
        }

        #[test]
        fn test_position_outside_ring() {
            let shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            // Position beyond outer radius
            assert!(!shield.is_touching_ring(Vec2::new(4.0, 0.0)));
            assert!(!shield.is_touching_ring(Vec2::new(10.0, 0.0)));
        }
    }

    mod hit_cooldown_tests {
        use super::*;

        #[test]
        fn test_can_hit_new_enemy() {
            let shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            let enemy = Entity::from_bits(1);
            assert!(shield.can_hit(enemy));
        }

        #[test]
        fn test_cannot_hit_enemy_on_cooldown() {
            let mut shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            let enemy = Entity::from_bits(1);

            shield.start_cooldown(enemy);
            assert!(!shield.can_hit(enemy));
        }

        #[test]
        fn test_can_hit_after_cooldown_expires() {
            let mut shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            let enemy = Entity::from_bits(1);

            shield.start_cooldown(enemy);
            // Tick past cooldown duration
            shield.tick_cooldowns(Duration::from_secs_f32(HALO_SHIELD_HIT_COOLDOWN + 0.1));
            assert!(shield.can_hit(enemy));
        }

        #[test]
        fn test_cooldown_cleanup_removes_finished_timers() {
            let mut shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            let enemy = Entity::from_bits(1);

            shield.start_cooldown(enemy);
            assert_eq!(shield.hit_cooldowns.len(), 1);

            // Tick past cooldown
            shield.tick_cooldowns(Duration::from_secs_f32(HALO_SHIELD_HIT_COOLDOWN + 0.1));
            assert_eq!(shield.hit_cooldowns.len(), 0);
        }

        #[test]
        fn test_multiple_enemies_tracked_independently() {
            let mut shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            let enemy1 = Entity::from_bits(1);
            let enemy2 = Entity::from_bits(2);

            shield.start_cooldown(enemy1);

            assert!(!shield.can_hit(enemy1));
            assert!(shield.can_hit(enemy2));
        }
    }

    mod halo_shield_update_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_shield_follows_entity_position() {
            let mut app = setup_test_app();

            let shield_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(halo_shield_update_system);

            let shield = app.world().get::<HaloShield>(shield_entity).unwrap();
            assert_eq!(shield.center.x, 10.0);
            assert_eq!(shield.center.y, 20.0); // Z maps to Y in Vec2
        }

        #[test]
        fn test_cooldowns_tick_down() {
            let mut app = setup_test_app();

            let enemy = Entity::from_bits(123);
            let mut shield = HaloShield::new(Vec2::ZERO, 10.0, 3.0);
            shield.start_cooldown(enemy);

            let shield_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                shield,
            )).id();

            // Advance time past cooldown
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(HALO_SHIELD_HIT_COOLDOWN + 0.1));
            }

            let _ = app.world_mut().run_system_once(halo_shield_update_system);

            let shield = app.world().get::<HaloShield>(shield_entity).unwrap();
            assert!(shield.can_hit(enemy), "Cooldown should have expired");
        }
    }

    mod halo_shield_contact_damage_system_tests {
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
        fn test_enemy_on_ring_takes_damage() {
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

            // Create shield at origin with radius 3.0
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            ));

            // Create enemy on the ring (at radius 3.0)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(halo_shield_contact_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy on ring should take damage");
        }

        #[test]
        fn test_enemy_inside_ring_takes_no_damage() {
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

            // Create shield at origin with radius 3.0
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            ));

            // Create enemy inside ring (at radius 1.0)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(halo_shield_contact_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy inside ring should not take damage");
        }

        #[test]
        fn test_enemy_outside_ring_takes_no_damage() {
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

            // Create shield at origin with radius 3.0
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            ));

            // Create enemy outside ring (at radius 10.0)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));

            let _ = app.world_mut().run_system_once(halo_shield_contact_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside ring should not take damage");
        }

        #[test]
        fn test_hit_cooldown_prevents_rapid_damage() {
            let mut app = setup_test_app();

            // Create shield at origin
            let shield_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            )).id();

            // Create enemy on the ring
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // First hit - should succeed
            let _ = app.world_mut().run_system_once(halo_shield_contact_damage_system);

            // Verify cooldown was started
            let shield = app.world().get::<HaloShield>(shield_entity).unwrap();
            assert!(!shield.can_hit(enemy_entity), "Enemy should be on cooldown after first hit");
        }

        #[test]
        fn test_enemy_can_be_hit_after_cooldown_expires() {
            let mut app = setup_test_app();

            // Create shield at origin
            let shield_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            )).id();

            // Create enemy on the ring
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // First hit - starts cooldown
            let _ = app.world_mut().run_system_once(halo_shield_contact_damage_system);

            // Verify on cooldown
            let shield = app.world().get::<HaloShield>(shield_entity).unwrap();
            assert!(!shield.can_hit(enemy_entity), "Enemy should be on cooldown after hit");

            // Manually expire the cooldown
            {
                let mut shield = app.world_mut().get_mut::<HaloShield>(shield_entity).unwrap();
                shield.tick_cooldowns(Duration::from_secs_f32(HALO_SHIELD_HIT_COOLDOWN + 0.1));
            }

            // Verify cooldown expired
            let shield = app.world().get::<HaloShield>(shield_entity).unwrap();
            assert!(shield.can_hit(enemy_entity), "Enemy should be hittable after cooldown expires");
        }

        #[test]
        fn test_multiple_enemies_can_be_damaged_simultaneously() {
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

            // Create shield at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                HaloShield::new(Vec2::ZERO, 10.0, 3.0),
            ));

            // Create 3 enemies on the ring at different angles
            for i in 0..3 {
                let angle = (i as f32) * std::f32::consts::TAU / 3.0;
                let pos = Vec3::new(3.0 * angle.cos(), 0.375, 3.0 * angle.sin());
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(pos),
                ));
            }

            let _ = app.world_mut().run_system_once(halo_shield_contact_damage_system);
            let _ = app.world_mut().run_system_once(count_damage_events);

            assert_eq!(counter.0.load(Ordering::SeqCst), 3, "All 3 enemies on ring should take damage");
        }
    }

    mod fire_halo_shield_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_halo_shield_spawns_shield() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DivineLight);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_halo_shield(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&HaloShield>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_halo_shield_at_spawn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DivineLight);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_halo_shield(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&HaloShield>();
            for shield in query.iter(app.world()) {
                // Shield center should match spawn XZ (10.0, 20.0)
                assert_eq!(shield.center.x, 10.0);
                assert_eq!(shield.center.y, 20.0); // Z maps to Y in Vec2
            }
        }

        #[test]
        fn test_fire_halo_shield_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DivineLight);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_halo_shield(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&HaloShield>();
            for shield in query.iter(app.world()) {
                assert_eq!(shield.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_halo_shield_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DivineLight);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_halo_shield_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&HaloShield>();
            for shield in query.iter(app.world()) {
                assert_eq!(shield.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_halo_shield_has_correct_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::DivineLight);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_halo_shield(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&HaloShield>();
            for shield in query.iter(app.world()) {
                assert_eq!(shield.radius, HALO_SHIELD_RADIUS);
            }
        }
    }
}
