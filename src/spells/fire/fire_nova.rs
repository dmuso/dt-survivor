use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Fire Nova (Inferno) spell
pub const FIRE_NOVA_MAX_RADIUS: f32 = 8.0;
pub const FIRE_NOVA_EXPANSION_DURATION: f32 = 0.4;
pub const FIRE_NOVA_VISUAL_HEIGHT: f32 = 0.2;

/// Get the fire element color for visual effects
pub fn fire_nova_color() -> Color {
    Element::Fire.color()
}

/// Component for the expanding fire nova ring.
/// Tracks expansion state and which enemies have been hit.
#[derive(Component, Debug, Clone)]
pub struct FireNovaRing {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Current radius of the ring
    pub current_radius: f32,
    /// Maximum radius the ring will expand to
    pub max_radius: f32,
    /// Expansion rate in units per second
    pub expansion_rate: f32,
    /// Damage to deal to enemies as the ring passes through them
    pub damage: f32,
    /// Set of enemy entities already hit by this nova (prevents double damage)
    pub hit_enemies: HashSet<Entity>,
}

impl FireNovaRing {
    pub fn new(center: Vec2, damage: f32) -> Self {
        Self {
            center,
            current_radius: 0.0,
            max_radius: FIRE_NOVA_MAX_RADIUS,
            expansion_rate: FIRE_NOVA_MAX_RADIUS / FIRE_NOVA_EXPANSION_DURATION,
            damage,
            hit_enemies: HashSet::new(),
        }
    }

    pub fn from_spell(center: Vec2, spell: &Spell) -> Self {
        Self::new(center, spell.damage())
    }

    /// Check if the nova has finished expanding
    pub fn is_finished(&self) -> bool {
        self.current_radius >= self.max_radius
    }

    /// Expand the ring by the given delta time
    pub fn expand(&mut self, delta_secs: f32) {
        self.current_radius = (self.current_radius + self.expansion_rate * delta_secs)
            .min(self.max_radius);
    }

    /// Check if an enemy at the given distance should be hit.
    /// Returns true if enemy is within the current ring radius and hasn't been hit yet.
    pub fn should_hit(&self, entity: Entity, distance: f32) -> bool {
        distance <= self.current_radius && !self.hit_enemies.contains(&entity)
    }

    /// Mark an enemy as hit
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }
}

/// System that expands fire nova rings over time
pub fn fire_nova_expansion_system(
    mut nova_query: Query<&mut FireNovaRing>,
    time: Res<Time>,
) {
    for mut nova in nova_query.iter_mut() {
        nova.expand(time.delta_secs());
    }
}

/// System that checks for enemy collisions with the expanding ring
/// and applies damage to enemies as the ring passes through them
pub fn fire_nova_collision_system(
    mut nova_query: Query<&mut FireNovaRing>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut nova in nova_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = nova.center.distance(enemy_pos);

            if nova.should_hit(enemy_entity, distance) {
                damage_events.write(DamageEvent::new(enemy_entity, nova.damage));
                nova.mark_hit(enemy_entity);
            }
        }
    }
}

/// System that despawns fire novas when they finish expanding
pub fn fire_nova_cleanup_system(
    mut commands: Commands,
    nova_query: Query<(Entity, &FireNovaRing)>,
) {
    for (entity, nova) in nova_query.iter() {
        if nova.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast fire nova (Inferno) spell - spawns an expanding ring of fire.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_fire_nova(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_fire_nova_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast fire nova (Inferno) spell with explicit damage - spawns an expanding ring of fire.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_fire_nova_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);
    let nova = FireNovaRing::new(center, damage);
    let nova_pos = Vec3::new(spawn_position.x, FIRE_NOVA_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.fire_nova.clone()),
            Transform::from_translation(nova_pos).with_scale(Vec3::splat(0.1)),
            nova,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(nova_pos),
            nova,
        ));
    }
}

/// System that updates the visual scale of fire novas based on their current radius
pub fn fire_nova_visual_system(
    mut nova_query: Query<(&FireNovaRing, &mut Transform)>,
) {
    for (nova, mut transform) in nova_query.iter_mut() {
        // Scale the visual to match current radius
        transform.scale = Vec3::splat(nova.current_radius.max(0.1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod fire_nova_ring_tests {
        use super::*;

        #[test]
        fn test_fire_nova_ring_new() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 30.0;
            let nova = FireNovaRing::new(center, damage);

            assert_eq!(nova.center, center);
            assert_eq!(nova.damage, damage);
            assert_eq!(nova.current_radius, 0.0);
            assert_eq!(nova.max_radius, FIRE_NOVA_MAX_RADIUS);
            assert!(!nova.is_finished());
            assert!(nova.hit_enemies.is_empty());
        }

        #[test]
        fn test_fire_nova_ring_from_spell() {
            let spell = Spell::new(SpellType::Inferno);
            let center = Vec2::new(5.0, 15.0);
            let nova = FireNovaRing::from_spell(center, &spell);

            assert_eq!(nova.center, center);
            assert_eq!(nova.damage, spell.damage());
        }

        #[test]
        fn test_fire_nova_ring_expand() {
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);

            nova.expand(FIRE_NOVA_EXPANSION_DURATION / 2.0);
            assert!(
                (nova.current_radius - FIRE_NOVA_MAX_RADIUS / 2.0).abs() < 0.01,
                "Radius should be half of max after half duration"
            );

            nova.expand(FIRE_NOVA_EXPANSION_DURATION / 2.0);
            assert!(
                (nova.current_radius - FIRE_NOVA_MAX_RADIUS).abs() < 0.01,
                "Radius should be max after full duration"
            );
        }

        #[test]
        fn test_fire_nova_ring_expand_caps_at_max() {
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);

            // Expand way past max
            nova.expand(FIRE_NOVA_EXPANSION_DURATION * 10.0);

            assert_eq!(nova.current_radius, FIRE_NOVA_MAX_RADIUS);
        }

        #[test]
        fn test_fire_nova_ring_is_finished() {
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            assert!(!nova.is_finished());

            nova.current_radius = FIRE_NOVA_MAX_RADIUS;
            assert!(nova.is_finished());
        }

        #[test]
        fn test_fire_nova_ring_should_hit() {
            let nova = FireNovaRing {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                damage: 30.0,
                hit_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(nova.should_hit(entity, 3.0), "Should hit enemy within radius");
            assert!(nova.should_hit(entity, 5.0), "Should hit enemy at radius edge");
            assert!(!nova.should_hit(entity, 6.0), "Should not hit enemy outside radius");
        }

        #[test]
        fn test_fire_nova_ring_should_hit_excludes_already_hit() {
            let mut nova = FireNovaRing {
                center: Vec2::ZERO,
                current_radius: 5.0,
                max_radius: 10.0,
                expansion_rate: 20.0,
                damage: 30.0,
                hit_enemies: HashSet::new(),
            };

            let entity = Entity::from_bits(1);
            assert!(nova.should_hit(entity, 3.0));

            nova.mark_hit(entity);
            assert!(!nova.should_hit(entity, 3.0), "Should not hit already-hit enemy");
        }

        #[test]
        fn test_fire_nova_ring_mark_hit() {
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);

            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);

            nova.mark_hit(entity1);
            assert!(nova.hit_enemies.contains(&entity1));
            assert!(!nova.hit_enemies.contains(&entity2));

            nova.mark_hit(entity2);
            assert!(nova.hit_enemies.contains(&entity1));
            assert!(nova.hit_enemies.contains(&entity2));
        }

        #[test]
        fn test_fire_nova_uses_fire_element_color() {
            let color = fire_nova_color();
            assert_eq!(color, Element::Fire.color());
            assert_eq!(color, Color::srgb_u8(255, 128, 0));
        }
    }

    mod fire_nova_expansion_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_fire_nova_expands_over_time() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FireNovaRing::new(Vec2::ZERO, 30.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(FIRE_NOVA_EXPANSION_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(fire_nova_expansion_system);

            let nova = app.world().get::<FireNovaRing>(entity).unwrap();
            assert!(
                (nova.current_radius - FIRE_NOVA_MAX_RADIUS / 2.0).abs() < 0.1,
                "Radius should be approximately half after half duration: got {}",
                nova.current_radius
            );
        }

        #[test]
        fn test_fire_nova_multiple_rings_expand_independently() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create two novas with different starting radii
            let entity1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FireNovaRing::new(Vec2::ZERO, 30.0),
            )).id();

            let mut nova2 = FireNovaRing::new(Vec2::new(10.0, 10.0), 20.0);
            nova2.current_radius = 3.0; // Pre-expanded
            let entity2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                nova2,
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            let _ = app.world_mut().run_system_once(fire_nova_expansion_system);

            let nova1 = app.world().get::<FireNovaRing>(entity1).unwrap();
            let nova2 = app.world().get::<FireNovaRing>(entity2).unwrap();

            // Both should have expanded but from different starting points
            assert!(nova1.current_radius > 0.0);
            assert!(nova2.current_radius > 3.0);
        }
    }

    mod fire_nova_collision_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_fire_nova_damages_enemy_in_radius() {
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
            app.add_systems(Update, (fire_nova_collision_system, count_damage_events).chain());

            // Create nova at origin with radius 5.0
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            ));

            // Create enemy within radius (XZ distance = 3)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_fire_nova_no_damage_outside_radius() {
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
            app.add_systems(Update, (fire_nova_collision_system, count_damage_events).chain());

            // Create nova at origin with radius 3.0
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = 3.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            ));

            // Create enemy outside radius (XZ distance = 5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_fire_nova_damages_enemy_only_once() {
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
            app.add_systems(Update, (fire_nova_collision_system, count_damage_events).chain());

            // Create nova
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            ));

            // Create enemy in radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(2.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Should only damage once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_fire_nova_damages_multiple_enemies() {
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
            app.add_systems(Update, (fire_nova_collision_system, count_damage_events).chain());

            // Create nova
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            ));

            // Create 3 enemies in radius
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn test_fire_nova_uses_xz_plane_ignores_y() {
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
            app.add_systems(Update, (fire_nova_collision_system, count_damage_events).chain());

            // Create nova at origin
            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = 5.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            ));

            // Create enemy close on XZ plane but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }
    }

    mod fire_nova_cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_fire_nova_despawns_when_finished() {
            let mut app = App::new();

            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = FIRE_NOVA_MAX_RADIUS; // Already at max
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            )).id();

            let _ = app.world_mut().run_system_once(fire_nova_cleanup_system);

            // Nova should be despawned
            assert!(app.world().get_entity(entity).is_err());
        }

        #[test]
        fn test_fire_nova_survives_before_finished() {
            let mut app = App::new();

            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = FIRE_NOVA_MAX_RADIUS / 2.0; // Only halfway
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            )).id();

            let _ = app.world_mut().run_system_once(fire_nova_cleanup_system);

            // Nova should still exist
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    mod fire_fire_nova_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_fire_nova_spawns_ring() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Inferno);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fire_nova(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 nova
            let mut query = app.world_mut().query::<&FireNovaRing>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_fire_nova_at_player_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Inferno);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fire_nova(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireNovaRing>();
            for nova in query.iter(app.world()) {
                assert_eq!(nova.center, Vec2::new(15.0, 25.0));
            }
        }

        #[test]
        fn test_fire_fire_nova_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Inferno);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fire_nova(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireNovaRing>();
            for nova in query.iter(app.world()) {
                assert_eq!(nova.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_fire_nova_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Inferno);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fire_nova_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireNovaRing>();
            for nova in query.iter(app.world()) {
                assert_eq!(nova.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_fire_nova_starts_at_zero_radius() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Inferno);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fire_nova(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireNovaRing>();
            for nova in query.iter(app.world()) {
                assert_eq!(nova.current_radius, 0.0);
            }
        }
    }

    mod fire_nova_visual_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_fire_nova_visual_scale_matches_radius() {
            let mut app = App::new();

            let mut nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            nova.current_radius = 5.0;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            )).id();

            let _ = app.world_mut().run_system_once(fire_nova_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(5.0));
        }

        #[test]
        fn test_fire_nova_visual_minimum_scale() {
            let mut app = App::new();

            let nova = FireNovaRing::new(Vec2::ZERO, 30.0);
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                nova,
            )).id();

            let _ = app.world_mut().run_system_once(fire_nova_visual_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.scale, Vec3::splat(0.1), "Should have minimum scale of 0.1");
        }
    }
}
