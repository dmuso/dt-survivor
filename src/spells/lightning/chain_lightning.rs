use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;

/// Default configuration for Chain Lightning spell
pub const CHAIN_LIGHTNING_JUMP_RANGE: f32 = 8.0;
pub const CHAIN_LIGHTNING_MAX_JUMPS: u8 = 4;
pub const CHAIN_LIGHTNING_DAMAGE_DECAY: f32 = 0.8;
pub const CHAIN_LIGHTNING_SPEED: f32 = 50.0;
pub const CHAIN_LIGHTNING_VISUAL_LIFETIME: f32 = 0.2;

/// Get the lightning element color for visual effects (yellow)
pub fn chain_lightning_color() -> Color {
    Element::Lightning.color()
}

/// Component for the chain lightning bolt that arcs between enemies.
/// Tracks which enemies have been hit and how many jumps remain.
#[derive(Component, Debug, Clone)]
pub struct ChainLightningBolt {
    /// Current damage (decays with each jump)
    pub current_damage: f32,
    /// Number of jumps remaining
    pub jumps_remaining: u8,
    /// Range to search for next target
    pub jump_range: f32,
    /// Damage multiplier for each jump (0.8 = 80% of previous)
    pub damage_decay: f32,
    /// Set of enemy entities already hit in this chain
    pub hit_enemies: HashSet<Entity>,
    /// Current target entity
    pub target: Entity,
    /// Starting position of current arc
    pub start_pos: Vec2,
    /// Whether this bolt has dealt damage to its current target
    pub damage_applied: bool,
}

impl ChainLightningBolt {
    pub fn new(damage: f32, target: Entity, start_pos: Vec2) -> Self {
        Self {
            current_damage: damage,
            jumps_remaining: CHAIN_LIGHTNING_MAX_JUMPS,
            jump_range: CHAIN_LIGHTNING_JUMP_RANGE,
            damage_decay: CHAIN_LIGHTNING_DAMAGE_DECAY,
            hit_enemies: HashSet::new(),
            target,
            start_pos,
            damage_applied: false,
        }
    }

    pub fn from_spell(spell: &Spell, target: Entity, start_pos: Vec2) -> Self {
        Self::new(spell.damage(), target, start_pos)
    }

    /// Check if the chain can continue jumping
    pub fn can_jump(&self) -> bool {
        self.jumps_remaining > 0
    }

    /// Prepare for next jump with reduced damage
    pub fn prepare_jump(&mut self, new_target: Entity, new_start_pos: Vec2) {
        self.current_damage *= self.damage_decay;
        self.jumps_remaining = self.jumps_remaining.saturating_sub(1);
        self.target = new_target;
        self.start_pos = new_start_pos;
        self.damage_applied = false;
    }

    /// Mark an enemy as hit
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }

    /// Check if an enemy has already been hit
    pub fn already_hit(&self, entity: Entity) -> bool {
        self.hit_enemies.contains(&entity)
    }
}

/// Visual arc component for displaying the lightning effect between positions
#[derive(Component, Debug, Clone)]
pub struct ChainLightningArc {
    /// Start and end positions for this arc segment
    pub start: Vec2,
    pub end: Vec2,
    /// Lifetime timer for the visual effect
    pub lifetime: Timer,
}

impl ChainLightningArc {
    pub fn new(start: Vec2, end: Vec2) -> Self {
        Self {
            start,
            end,
            lifetime: Timer::from_seconds(CHAIN_LIGHTNING_VISUAL_LIFETIME, TimerMode::Once),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// System that moves chain lightning toward its current target
pub fn chain_lightning_movement_system(
    time: Res<Time>,
    mut bolt_query: Query<(&mut ChainLightningBolt, &mut Transform)>,
    enemy_query: Query<&Transform, (With<Enemy>, Without<ChainLightningBolt>)>,
) {
    for (mut bolt, mut transform) in bolt_query.iter_mut() {
        // Get target position
        let Ok(target_transform) = enemy_query.get(bolt.target) else {
            continue;
        };

        let target_pos = from_xz(target_transform.translation);
        let current_pos = from_xz(transform.translation);

        // Calculate direction and move
        let direction = (target_pos - current_pos).normalize_or_zero();
        let movement = direction * CHAIN_LIGHTNING_SPEED * time.delta_secs();

        // Update bolt position
        let new_pos = current_pos + movement;
        transform.translation = to_xz(new_pos) + Vec3::new(0.0, 0.5, 0.0);

        // Update start position for visual arc
        bolt.start_pos = new_pos;
    }
}

/// System that detects when chain lightning reaches its target and applies damage
pub fn chain_lightning_hit_system(
    mut commands: Commands,
    mut bolt_query: Query<(Entity, &mut ChainLightningBolt, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (bolt_entity, mut bolt, bolt_transform) in bolt_query.iter_mut() {
        if bolt.damage_applied {
            continue;
        }

        // Check if we reached the target
        let Ok((_, target_transform)) = enemy_query.get(bolt.target) else {
            // Target despawned, try to find new target or despawn bolt
            if !try_find_next_target(&mut commands, bolt_entity, &mut bolt, &enemy_query, game_meshes.as_deref(), game_materials.as_deref()) {
                commands.entity(bolt_entity).despawn();
            }
            continue;
        };

        let bolt_pos = from_xz(bolt_transform.translation);
        let target_pos = from_xz(target_transform.translation);
        let distance = bolt_pos.distance(target_pos);

        // Check if close enough to hit (within 1 unit)
        if distance <= 1.0 {
            // Apply damage
            let target_entity = bolt.target;
            let damage = bolt.current_damage;
            damage_events.write(DamageEvent::new(target_entity, damage));
            bolt.mark_hit(target_entity);
            bolt.damage_applied = true;

            // Spawn visual arc
            spawn_lightning_arc(&mut commands, bolt.start_pos, target_pos, game_meshes.as_deref(), game_materials.as_deref());

            // Try to find next target
            if !try_find_next_target(&mut commands, bolt_entity, &mut bolt, &enemy_query, game_meshes.as_deref(), game_materials.as_deref()) {
                commands.entity(bolt_entity).despawn();
            }
        }
    }
}

/// Helper function to find the next target for chain lightning
fn try_find_next_target(
    commands: &mut Commands,
    bolt_entity: Entity,
    bolt: &mut ChainLightningBolt,
    enemy_query: &Query<(Entity, &Transform), With<Enemy>>,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) -> bool {
    if !bolt.can_jump() {
        return false;
    }

    // Get current target position for distance calculation
    let Ok((_, current_target_transform)) = enemy_query.get(bolt.target) else {
        return false;
    };
    let current_target_pos = from_xz(current_target_transform.translation);

    // Find nearest unvisited enemy within range
    let mut best_target: Option<(Entity, Vec2, f32)> = None;

    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        // Skip already-hit enemies
        if bolt.already_hit(enemy_entity) {
            continue;
        }

        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = current_target_pos.distance(enemy_pos);

        // Check if within jump range
        if distance > bolt.jump_range {
            continue;
        }

        // Track the closest valid target
        match &best_target {
            Some((_, _, best_distance)) if distance < *best_distance => {
                best_target = Some((enemy_entity, enemy_pos, distance));
            }
            None => {
                best_target = Some((enemy_entity, enemy_pos, distance));
            }
            _ => {}
        }
    }

    if let Some((new_target, _, _)) = best_target {
        bolt.prepare_jump(new_target, current_target_pos);

        // Update bolt transform to current target position
        if let Ok(mut entity_commands) = commands.get_entity(bolt_entity) {
            entity_commands.insert(Transform::from_translation(to_xz(current_target_pos) + Vec3::new(0.0, 0.5, 0.0)));
        }
        true
    } else {
        false
    }
}

/// Spawn a visual lightning arc between two positions
fn spawn_lightning_arc(
    commands: &mut Commands,
    start: Vec2,
    end: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let arc = ChainLightningArc::new(start, end);
    let mid_point = (start + end) / 2.0;
    let arc_pos = to_xz(mid_point) + Vec3::new(0.0, 0.5, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(arc_pos).with_scale(Vec3::splat(0.3)),
            arc,
        ));
    } else {
        commands.spawn((
            Transform::from_translation(arc_pos),
            arc,
        ));
    }
}

/// System that updates and cleans up lightning arc visuals
pub fn chain_lightning_arc_cleanup_system(
    mut commands: Commands,
    time: Res<Time>,
    mut arc_query: Query<(Entity, &mut ChainLightningArc)>,
) {
    for (entity, mut arc) in arc_query.iter_mut() {
        arc.lifetime.tick(time.delta());

        if arc.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast chain lightning spell - spawns a bolt that arcs between enemies.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is the target on XZ plane.
#[allow(clippy::too_many_arguments)]
pub fn fire_chain_lightning(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_entity: Entity,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_chain_lightning_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_entity,
        game_meshes,
        game_materials,
    );
}

/// Cast chain lightning spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_chain_lightning_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_entity: Entity,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let start_pos = from_xz(spawn_position);
    let bolt = ChainLightningBolt::new(damage, target_entity, start_pos);
    // Mark the initial target as "to be hit" by not adding to hit_enemies yet
    // It will be added when damage is applied

    let bolt_pos = spawn_position + Vec3::new(0.0, 0.5, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(bolt_pos).with_scale(Vec3::splat(0.5)),
            bolt,
        ));
    } else {
        commands.spawn((
            Transform::from_translation(bolt_pos),
            bolt,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    mod chain_lightning_bolt_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_bolt_creation() {
            let target = Entity::from_bits(1);
            let start_pos = Vec2::new(10.0, 20.0);
            let damage = 30.0;
            let bolt = ChainLightningBolt::new(damage, target, start_pos);

            assert_eq!(bolt.current_damage, damage);
            assert_eq!(bolt.target, target);
            assert_eq!(bolt.start_pos, start_pos);
            assert_eq!(bolt.jumps_remaining, CHAIN_LIGHTNING_MAX_JUMPS);
            assert_eq!(bolt.damage_decay, CHAIN_LIGHTNING_DAMAGE_DECAY);
            assert!(!bolt.damage_applied);
            assert!(bolt.hit_enemies.is_empty());
        }

        #[test]
        fn test_bolt_from_spell() {
            let spell = Spell::new(SpellType::ChainLightning);
            let target = Entity::from_bits(1);
            let start_pos = Vec2::new(5.0, 15.0);
            let bolt = ChainLightningBolt::from_spell(&spell, target, start_pos);

            assert_eq!(bolt.current_damage, spell.damage());
            assert_eq!(bolt.target, target);
            assert_eq!(bolt.start_pos, start_pos);
        }

        #[test]
        fn test_bolt_can_jump() {
            let mut bolt = ChainLightningBolt::new(30.0, Entity::from_bits(1), Vec2::ZERO);
            assert!(bolt.can_jump());

            bolt.jumps_remaining = 0;
            assert!(!bolt.can_jump());
        }

        #[test]
        fn test_bolt_prepare_jump_reduces_damage() {
            let mut bolt = ChainLightningBolt::new(100.0, Entity::from_bits(1), Vec2::ZERO);
            let new_target = Entity::from_bits(2);
            let new_pos = Vec2::new(5.0, 5.0);

            bolt.prepare_jump(new_target, new_pos);

            assert_eq!(bolt.current_damage, 80.0); // 100 * 0.8
            assert_eq!(bolt.target, new_target);
            assert_eq!(bolt.start_pos, new_pos);
            assert_eq!(bolt.jumps_remaining, CHAIN_LIGHTNING_MAX_JUMPS - 1);
            assert!(!bolt.damage_applied);
        }

        #[test]
        fn test_damage_decay_chain() {
            let mut bolt = ChainLightningBolt::new(100.0, Entity::from_bits(1), Vec2::ZERO);

            // First jump: 100 -> 80
            bolt.prepare_jump(Entity::from_bits(2), Vec2::ZERO);
            assert!((bolt.current_damage - 80.0).abs() < 0.01);

            // Second jump: 80 -> 64
            bolt.prepare_jump(Entity::from_bits(3), Vec2::ZERO);
            assert!((bolt.current_damage - 64.0).abs() < 0.01);

            // Third jump: 64 -> 51.2
            bolt.prepare_jump(Entity::from_bits(4), Vec2::ZERO);
            assert!((bolt.current_damage - 51.2).abs() < 0.01);

            // Fourth jump: 51.2 -> 40.96
            bolt.prepare_jump(Entity::from_bits(5), Vec2::ZERO);
            assert!((bolt.current_damage - 40.96).abs() < 0.01);
        }

        #[test]
        fn test_bolt_mark_hit() {
            let mut bolt = ChainLightningBolt::new(30.0, Entity::from_bits(1), Vec2::ZERO);
            let enemy1 = Entity::from_bits(10);
            let enemy2 = Entity::from_bits(20);

            assert!(!bolt.already_hit(enemy1));
            assert!(!bolt.already_hit(enemy2));

            bolt.mark_hit(enemy1);

            assert!(bolt.already_hit(enemy1));
            assert!(!bolt.already_hit(enemy2));
        }

        #[test]
        fn test_uses_lightning_element_color() {
            let color = chain_lightning_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0));
        }
    }

    mod chain_lightning_arc_tests {
        use super::*;

        #[test]
        fn test_arc_creation() {
            let start = Vec2::new(0.0, 0.0);
            let end = Vec2::new(10.0, 10.0);
            let arc = ChainLightningArc::new(start, end);

            assert_eq!(arc.start, start);
            assert_eq!(arc.end, end);
            assert!(!arc.is_expired());
        }

        #[test]
        fn test_arc_expires_after_lifetime() {
            let mut arc = ChainLightningArc::new(Vec2::ZERO, Vec2::ONE);
            assert!(!arc.is_expired());

            arc.lifetime.tick(Duration::from_secs_f32(CHAIN_LIGHTNING_VISUAL_LIFETIME + 0.1));
            assert!(arc.is_expired());
        }
    }

    mod chain_lightning_hit_system_tests {
        use super::*;

        #[test]
        fn test_damage_applied_on_hit() {
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
            app.add_systems(Update, (chain_lightning_hit_system, count_damage_events).chain());

            // Create enemy at origin
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            // Create bolt very close to enemy (within hit range)
            let bolt = ChainLightningBolt::new(30.0, enemy_entity, Vec2::new(-5.0, 0.0));
            app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_no_damage_when_far_from_target() {
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
            app.add_systems(Update, (chain_lightning_hit_system, count_damage_events).chain());

            // Create enemy at origin
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            // Create bolt far from enemy
            let bolt = ChainLightningBolt::new(30.0, enemy_entity, Vec2::new(-10.0, 0.0));
            app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(10.0, 0.5, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_damage_applied_only_once_per_target() {
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
            app.add_systems(Update, (chain_lightning_hit_system, count_damage_events).chain());

            // Create single enemy
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            // Create bolt at enemy position with no jumps remaining
            let mut bolt = ChainLightningBolt::new(30.0, enemy_entity, Vec2::new(-5.0, 0.0));
            bolt.jumps_remaining = 0;
            app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
            ));

            // Run multiple updates
            app.update();
            app.update();
            app.update();

            // Should only damage once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }
    }

    mod chain_lightning_jump_tests {
        use super::*;

        #[test]
        fn test_chain_finds_next_nearest_enemy() {
            let mut app = App::new();

            app.add_message::<DamageEvent>();
            app.add_systems(Update, chain_lightning_hit_system);

            // Create first enemy (target)
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            // Create second enemy nearby (should be next target)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            // Create bolt at enemy1 position
            let bolt = ChainLightningBolt::new(30.0, enemy1, Vec2::new(-5.0, 0.0));
            let bolt_entity = app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
            )).id();

            app.update();

            // Bolt should still exist (jumped to next target)
            assert!(app.world().get_entity(bolt_entity).is_ok());

            // Bolt should have new target and reduced damage
            let bolt = app.world().get::<ChainLightningBolt>(bolt_entity).unwrap();
            assert!((bolt.current_damage - 24.0).abs() < 0.01); // 30 * 0.8
        }

        #[test]
        fn test_chain_excludes_already_hit_enemies() {
            let mut app = App::new();

            app.add_message::<DamageEvent>();
            app.add_systems(Update, chain_lightning_hit_system);

            // Create first enemy (target)
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            // Create second enemy that was already hit
            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            // Create third enemy (not hit, should be next target)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(6.0, 0.375, 0.0)),
            ));

            // Create bolt with enemy2 already marked as hit
            let mut bolt = ChainLightningBolt::new(30.0, enemy1, Vec2::new(-5.0, 0.0));
            bolt.mark_hit(enemy2);
            let bolt_entity = app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
            )).id();

            app.update();

            // Bolt should have jumped to enemy3, not enemy2
            let bolt = app.world().get::<ChainLightningBolt>(bolt_entity).unwrap();
            assert!(bolt.already_hit(enemy1)); // Original target now hit
            assert!(bolt.already_hit(enemy2)); // Was already hit
            assert!(!bolt.already_hit(bolt.target)); // New target not yet hit
        }

        #[test]
        fn test_chain_stops_when_no_enemies_in_range() {
            let mut app = App::new();

            app.add_message::<DamageEvent>();
            app.add_systems(Update, chain_lightning_hit_system);

            // Create single enemy
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            // Create another enemy too far away
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Create bolt at enemy position
            let bolt = ChainLightningBolt::new(30.0, enemy1, Vec2::new(-5.0, 0.0));
            let bolt_entity = app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
            )).id();

            app.update();

            // Bolt should be despawned (no valid targets in range)
            assert!(app.world().get_entity(bolt_entity).is_err());
        }

        #[test]
        fn test_chain_stops_when_jumps_exhausted() {
            let mut app = App::new();

            app.add_message::<DamageEvent>();
            app.add_systems(Update, chain_lightning_hit_system);

            // Create two enemies
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            )).id();

            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            // Create bolt with no jumps remaining
            let mut bolt = ChainLightningBolt::new(30.0, enemy1, Vec2::new(-5.0, 0.0));
            bolt.jumps_remaining = 0;
            let bolt_entity = app.world_mut().spawn((
                bolt,
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
            )).id();

            app.update();

            // Bolt should be despawned (no jumps remaining)
            assert!(app.world().get_entity(bolt_entity).is_err());
        }
    }

    mod chain_lightning_arc_cleanup_tests {
        use super::*;

        #[test]
        fn test_arc_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, chain_lightning_arc_cleanup_system);
            app.init_resource::<Time>();

            let arc_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ChainLightningArc::new(Vec2::ZERO, Vec2::ONE),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(CHAIN_LIGHTNING_VISUAL_LIFETIME + 0.1));
            }

            app.update();

            assert!(app.world().get_entity(arc_entity).is_err());
        }

        #[test]
        fn test_arc_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, chain_lightning_arc_cleanup_system);
            app.init_resource::<Time>();

            let arc_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ChainLightningArc::new(Vec2::ZERO, Vec2::ONE),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(CHAIN_LIGHTNING_VISUAL_LIFETIME / 2.0));
            }

            app.update();

            assert!(app.world().get_entity(arc_entity).is_ok());
        }
    }

    mod fire_chain_lightning_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_chain_lightning_spawns_bolt() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChainLightning);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_entity = Entity::from_bits(1);

            {
                let mut commands = app.world_mut().commands();
                fire_chain_lightning(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_entity,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChainLightningBolt>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_chain_lightning_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChainLightning);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_entity = Entity::from_bits(1);

            {
                let mut commands = app.world_mut().commands();
                fire_chain_lightning(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_entity,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChainLightningBolt>();
            for bolt in query.iter(app.world()) {
                assert_eq!(bolt.current_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_chain_lightning_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChainLightning);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_entity = Entity::from_bits(1);

            {
                let mut commands = app.world_mut().commands();
                fire_chain_lightning_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    target_entity,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChainLightningBolt>();
            for bolt in query.iter(app.world()) {
                assert_eq!(bolt.current_damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_chain_lightning_sets_correct_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ChainLightning);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_entity = Entity::from_bits(42);

            {
                let mut commands = app.world_mut().commands();
                fire_chain_lightning(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_entity,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChainLightningBolt>();
            for bolt in query.iter(app.world()) {
                assert_eq!(bolt.target, target_entity);
            }
        }
    }
}
