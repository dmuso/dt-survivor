//! Mind Lash spell - A psychic whip that damages enemies in a line.
//!
//! A Psychic element spell (MentalSpike SpellType) that projects a line attack
//! in the direction of the nearest target, damaging all enemies caught in the
//! line's path with a single damage application per cast.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default height of mind lash above ground
pub const MIND_LASH_DEFAULT_Y_HEIGHT: f32 = 0.5;

/// Default lash length in world units
pub const MIND_LASH_LENGTH: f32 = 15.0;

/// Collision width on either side of the lash line
pub const MIND_LASH_WIDTH: f32 = 1.5;

/// Duration of the visual effect in seconds
pub const MIND_LASH_LIFETIME: f32 = 0.3;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn mind_lash_color() -> Color {
    Element::Psychic.color()
}

/// Marker component for mind lash spell entities.
/// A psychic whip that damages enemies in a line toward the nearest target.
#[derive(Component, Debug, Clone)]
pub struct MindLash {
    /// Start position on XZ plane
    pub start_pos: Vec2,
    /// End position on XZ plane
    pub end_pos: Vec2,
    /// Direction of the lash on XZ plane
    pub direction: Vec2,
    /// Width of the lash for collision detection
    pub width: f32,
    /// Lifetime timer for visual effect
    pub lifetime: Timer,
    /// Maximum lifetime in seconds
    pub max_lifetime: f32,
    /// Damage dealt to enemies hit by the lash
    pub damage: f32,
    /// Y height in 3D world
    pub y_height: f32,
    /// Set of enemy entities already hit by this lash
    pub hit_enemies: HashSet<Entity>,
    /// Whether damage has been applied (single application per cast)
    pub damage_applied: bool,
}

impl MindLash {
    /// Creates a new MindLash at the default height.
    pub fn new(start_pos: Vec2, direction: Vec2, damage: f32) -> Self {
        Self::with_height(start_pos, direction, damage, MIND_LASH_DEFAULT_Y_HEIGHT)
    }

    /// Creates a new MindLash at the specified Y height.
    pub fn with_height(start_pos: Vec2, direction: Vec2, damage: f32, y_height: f32) -> Self {
        let normalized_dir = direction.normalize_or_zero();
        let end_pos = start_pos + normalized_dir * MIND_LASH_LENGTH;
        Self {
            start_pos,
            end_pos,
            direction: normalized_dir,
            width: MIND_LASH_WIDTH,
            lifetime: Timer::from_seconds(MIND_LASH_LIFETIME, TimerMode::Once),
            max_lifetime: MIND_LASH_LIFETIME,
            damage,
            y_height,
            hit_enemies: HashSet::new(),
            damage_applied: false,
        }
    }

    /// Creates a MindLash from a Spell component.
    pub fn from_spell(start_pos: Vec2, direction: Vec2, spell: &Spell, y_height: f32) -> Self {
        Self::with_height(start_pos, direction, spell.damage(), y_height)
    }

    /// Calculate the fade progress for visual effect.
    /// Returns 0.0 at start, 1.0 when fully faded.
    pub fn get_fade_progress(&self) -> f32 {
        self.lifetime.elapsed_secs() / self.max_lifetime
    }

    /// Check if the lash is still active (not expired)
    pub fn is_active(&self) -> bool {
        !self.lifetime.is_finished()
    }

    /// Check if an enemy position is within the lash collision area
    pub fn is_in_lash(&self, enemy_pos: Vec2) -> bool {
        // Vector from start to enemy
        let to_enemy = enemy_pos - self.start_pos;

        // Project onto lash direction
        let projection_length = to_enemy.dot(self.direction);

        // Check if within lash segment
        let lash_length = MIND_LASH_LENGTH;
        if projection_length < 0.0 || projection_length > lash_length {
            return false;
        }

        // Calculate perpendicular distance
        let projection_point = self.start_pos + self.direction * projection_length;
        let distance_to_line = (enemy_pos - projection_point).length();

        distance_to_line <= self.width
    }

    /// Mark an enemy as hit by this lash
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }

    /// Check if enemy has already been hit
    pub fn was_hit(&self, entity: Entity) -> bool {
        self.hit_enemies.contains(&entity)
    }
}

/// System to update mind lash lifetime and despawn expired lashes.
pub fn update_mind_lash_system(
    mut commands: Commands,
    time: Res<Time>,
    mut lash_query: Query<(Entity, &mut MindLash)>,
) {
    for (entity, mut lash) in lash_query.iter_mut() {
        lash.lifetime.tick(time.delta());

        if !lash.is_active() {
            commands.entity(entity).despawn();
        }
    }
}

/// Mind lash collision system that sends DamageEvent to enemies in the lash path.
/// Damage is applied once per cast (not continuous like beams).
pub fn mind_lash_collision_system(
    mut lash_query: Query<&mut MindLash>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut lash in lash_query.iter_mut() {
        // Only apply damage once per lash
        if lash.damage_applied {
            continue;
        }

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            // Skip already hit enemies
            if lash.was_hit(enemy_entity) {
                continue;
            }

            // Extract XZ coordinates from enemy 3D position
            let enemy_pos = from_xz(enemy_transform.translation);

            // Check if enemy is within the lash bounds
            if lash.is_in_lash(enemy_pos) {
                damage_events.write(DamageEvent::new(enemy_entity, lash.damage));
                lash.mark_hit(enemy_entity);
            }
        }

        // Mark damage as applied after first frame
        lash.damage_applied = true;
    }
}

/// Renders mind lash as a 3D elongated cube with pink/magenta coloring.
pub fn render_mind_lash_system(
    mut commands: Commands,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
    lash_query: Query<(Entity, &MindLash), Changed<MindLash>>,
) {
    // Skip if resources not available (e.g., in tests)
    let Some(game_meshes) = game_meshes else { return; };
    let Some(game_materials) = game_materials else { return; };

    for (entity, lash) in lash_query.iter() {
        // Calculate fade for visual effect
        let fade = 1.0 - lash.get_fade_progress();
        let thickness = lash.width * fade;
        let length = (lash.end_pos - lash.start_pos).length();

        // Center position on XZ plane
        let center_xz = (lash.start_pos + lash.end_pos) / 2.0;

        // Rotation around Y axis to point toward target on XZ plane
        let angle = lash.direction.y.atan2(lash.direction.x);
        let rotation = Quat::from_rotation_y(-angle + std::f32::consts::FRAC_PI_2);

        // Scale: base mesh is 0.1 x 0.1 x 1.0
        let scale = Vec3::new(thickness / 5.0, thickness / 5.0, length);

        // Use powerup material (magenta/pink) for psychic element
        commands.entity(entity).insert((
            Mesh3d(game_meshes.laser.clone()),
            MeshMaterial3d(game_materials.powerup.clone()),
            Transform {
                translation: Vec3::new(center_xz.x, lash.y_height, center_xz.y),
                rotation,
                scale,
            },
        ));
    }
}

/// Cast mind lash spell - spawns a lash toward the nearest enemy.
#[allow(clippy::too_many_arguments)]
pub fn fire_mind_lash(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_mind_lash_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast mind lash spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_mind_lash_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let direction = (target_pos - spawn_xz).normalize_or_zero();

    let lash = MindLash::with_height(spawn_xz, direction, damage, spawn_position.y);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.laser.clone()),
            MeshMaterial3d(materials.powerup.clone()),
            Transform::from_translation(spawn_position),
            lash,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            lash,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod mind_lash_component_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_mind_lash_creation() {
            let start_pos = Vec2::new(0.0, 0.0);
            let direction = Vec2::new(1.0, 0.0);
            let damage = 25.0;

            let lash = MindLash::new(start_pos, direction, damage);

            assert_eq!(lash.start_pos, start_pos);
            assert_eq!(lash.end_pos, Vec2::new(MIND_LASH_LENGTH, 0.0));
            assert_eq!(lash.direction, direction);
            assert_eq!(lash.damage, damage);
            assert_eq!(lash.width, MIND_LASH_WIDTH);
            assert!(lash.is_active());
            assert!(!lash.damage_applied);
            assert!(lash.hit_enemies.is_empty());
        }

        #[test]
        fn test_mind_lash_with_height() {
            let lash = MindLash::with_height(Vec2::ZERO, Vec2::X, 25.0, 2.5);
            assert_eq!(lash.y_height, 2.5);
        }

        #[test]
        fn test_mind_lash_new_uses_default_height() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            assert_eq!(lash.y_height, MIND_LASH_DEFAULT_Y_HEIGHT);
        }

        #[test]
        fn test_mind_lash_from_spell() {
            let spell = Spell::new(SpellType::MentalSpike);
            let direction = Vec2::new(0.0, 1.0);
            let lash = MindLash::from_spell(Vec2::ZERO, direction, &spell, 1.0);

            assert_eq!(lash.direction, direction);
            assert_eq!(lash.damage, spell.damage());
            assert_eq!(lash.y_height, 1.0);
        }

        #[test]
        fn test_mind_lash_normalizes_direction() {
            let direction = Vec2::new(3.0, 4.0);
            let lash = MindLash::new(Vec2::ZERO, direction, 25.0);

            let expected = direction.normalize();
            assert!((lash.direction - expected).length() < 0.001);
        }

        #[test]
        fn test_mind_lash_handles_zero_direction() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::ZERO, 25.0);
            assert_eq!(lash.direction, Vec2::ZERO);
        }

        #[test]
        fn test_mind_lash_is_active() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            assert!(lash.is_active());
        }

        #[test]
        fn test_mind_lash_uses_psychic_element_color() {
            let color = mind_lash_color();
            assert_eq!(color, Element::Psychic.color());
        }

        #[test]
        fn test_mind_lash_fade_progress() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            // At start, fade progress should be 0
            assert_eq!(lash.get_fade_progress(), 0.0);
        }
    }

    mod mind_lash_collision_tests {
        use super::*;

        #[test]
        fn test_is_in_lash_enemy_on_line() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let enemy_pos = Vec2::new(5.0, 0.0);

            assert!(lash.is_in_lash(enemy_pos));
        }

        #[test]
        fn test_is_in_lash_enemy_within_width() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let enemy_pos = Vec2::new(5.0, 1.0); // 1.0 units off-center, within width

            assert!(lash.is_in_lash(enemy_pos));
        }

        #[test]
        fn test_is_in_lash_enemy_outside_width() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let enemy_pos = Vec2::new(5.0, 3.0); // 3.0 units off-center, outside width

            assert!(!lash.is_in_lash(enemy_pos));
        }

        #[test]
        fn test_is_in_lash_enemy_behind_start() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let enemy_pos = Vec2::new(-5.0, 0.0); // Behind the lash

            assert!(!lash.is_in_lash(enemy_pos));
        }

        #[test]
        fn test_is_in_lash_enemy_beyond_end() {
            let lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let enemy_pos = Vec2::new(20.0, 0.0); // Beyond lash length

            assert!(!lash.is_in_lash(enemy_pos));
        }

        #[test]
        fn test_mark_hit_and_was_hit() {
            let mut lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let entity = Entity::from_bits(1);

            assert!(!lash.was_hit(entity));
            lash.mark_hit(entity);
            assert!(lash.was_hit(entity));
        }

        #[test]
        fn test_multiple_enemies_hit() {
            let mut lash = MindLash::new(Vec2::ZERO, Vec2::X, 25.0);
            let entity1 = Entity::from_bits(1);
            let entity2 = Entity::from_bits(2);

            lash.mark_hit(entity1);
            lash.mark_hit(entity2);

            assert!(lash.was_hit(entity1));
            assert!(lash.was_hit(entity2));
            assert_eq!(lash.hit_enemies.len(), 2);
        }
    }

    mod update_system_tests {
        use super::*;

        #[test]
        fn test_mind_lash_update() {
            let mut app = App::new();
            app.add_systems(Update, update_mind_lash_system);
            app.init_resource::<Time>();

            let lash_entity = app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0)).id();

            // Initially active
            {
                let lash = app.world().get::<MindLash>(lash_entity).unwrap();
                assert!(lash.is_active());
            }

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }
            app.update();

            // Lash should be despawned
            assert!(app.world().get_entity(lash_entity).is_err());
        }

        #[test]
        fn test_mind_lash_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_mind_lash_system);
            app.init_resource::<Time>();

            let lash_entity = app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0)).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            app.update();

            // Lash should still exist
            assert!(app.world().entities().contains(lash_entity));
        }
    }

    mod collision_system_tests {
        use super::*;
        use crate::combat::{CheckDeath, Health};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_mind_lash_collision_sends_damage_event() {
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
            app.add_systems(Update, (mind_lash_collision_system, count_damage_events).chain());

            // Create lash along X axis
            app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0));

            // Create enemy on the lash line
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                Health::new(50.0),
                CheckDeath,
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_mind_lash_damages_enemies_in_line() {
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
            app.add_systems(Update, (mind_lash_collision_system, count_damage_events).chain());

            // Create lash along X axis
            app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0));

            // Create multiple enemies on the lash line
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                Health::new(50.0),
                CheckDeath,
            ));
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(7.0, 0.375, 0.0)),
                Health::new(50.0),
                CheckDeath,
            ));
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                Health::new(50.0),
                CheckDeath,
            ));

            app.update();

            // All 3 enemies should be hit
            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn test_mind_lash_respects_width() {
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
            app.add_systems(Update, (mind_lash_collision_system, count_damage_events).chain());

            // Create lash along X axis
            app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0));

            // Enemy within width (1.0 units from center)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 1.0)),
                Health::new(50.0),
                CheckDeath,
            ));

            // Enemy outside width (5.0 units from center)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 5.0)),
                Health::new(50.0),
                CheckDeath,
            ));

            app.update();

            // Only 1 enemy should be hit (within width)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_mind_lash_single_damage_application() {
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
            app.add_systems(Update, (mind_lash_collision_system, count_damage_events).chain());

            // Create lash
            app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0));

            // Create enemy on lash line
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
                Health::new(50.0),
                CheckDeath,
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Damage should only be applied once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_mind_lash_no_target_behavior() {
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
            app.add_systems(Update, (mind_lash_collision_system, count_damage_events).chain());

            // Create lash with no enemies
            app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0));

            app.update();

            // No damage events should be sent
            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_mind_lash_cleanup_after_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_mind_lash_system);
            app.init_resource::<Time>();

            let lash_entity = app.world_mut().spawn(MindLash::new(Vec2::ZERO, Vec2::X, 25.0)).id();

            // Advance past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(MIND_LASH_LIFETIME + 0.1));
            }
            app.update();

            // Entity should be despawned
            assert!(app.world().get_entity(lash_entity).is_err());
        }
    }

    mod fire_mind_lash_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_mind_lash_spawns_lash() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MentalSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_lash(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindLash>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_mind_lash_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MentalSpike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_lash(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindLash>();
            for lash in query.iter(app.world()) {
                assert!(lash.direction.x > 0.9, "Lash should point toward target");
            }
        }

        #[test]
        fn test_fire_mind_lash_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MentalSpike);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_lash(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindLash>();
            for lash in query.iter(app.world()) {
                assert_eq!(lash.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_mind_lash_uses_spawn_y_height() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::MentalSpike);
            let spawn_pos = Vec3::new(0.0, 2.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_mind_lash(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&MindLash>();
            for lash in query.iter(app.world()) {
                assert_eq!(lash.y_height, 2.5);
            }
        }
    }
}
