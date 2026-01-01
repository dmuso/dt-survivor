//! Venom Spray spell - Short-range cone attack that applies stacking poison.
//!
//! A Poison element spell (ToxicSpray SpellType) that fires an instant cone
//! attack in front of the player. Each hit adds a poison stack to enemies,
//! and stacks increase the DOT damage.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Venom Spray spell
pub const VENOM_SPRAY_CONE_ANGLE: f32 = 60.0; // degrees
pub const VENOM_SPRAY_RANGE: f32 = 6.0;
pub const VENOM_SPRAY_DURATION: f32 = 0.3; // brief cone hitbox duration
pub const POISON_STACK_MAX: u32 = 5;
pub const POISON_STACK_DAMAGE_PER_STACK: f32 = 3.0;
pub const POISON_STACK_TICK_INTERVAL: f32 = 0.5;
pub const POISON_STACK_DURATION: f32 = 4.0;

/// Get the poison element color for visual effects
pub fn venom_spray_color() -> Color {
    Element::Poison.color()
}

/// Cone hitbox for the venom spray attack.
/// Spawned on spell cast and despawns after brief duration.
#[derive(Component, Debug, Clone)]
pub struct VenomSprayCone {
    /// Center position of the cone origin on XZ plane
    pub origin: Vec2,
    /// Direction the cone is facing (normalized)
    pub direction: Vec2,
    /// Half-angle of the cone in radians
    pub half_angle: f32,
    /// Range/length of the cone
    pub range: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Base damage from the spell (before stack scaling)
    pub base_damage: f32,
    /// Set of enemies already hit by this cone instance
    pub hit_enemies: HashSet<Entity>,
}

impl VenomSprayCone {
    pub fn new(origin: Vec2, direction: Vec2, damage: f32) -> Self {
        let half_angle = (VENOM_SPRAY_CONE_ANGLE / 2.0).to_radians();
        Self {
            origin,
            direction: direction.normalize_or_zero(),
            half_angle,
            range: VENOM_SPRAY_RANGE,
            duration: Timer::from_seconds(VENOM_SPRAY_DURATION, TimerMode::Once),
            base_damage: damage,
            hit_enemies: HashSet::new(),
        }
    }

    /// Check if the cone has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Check if a position is within the cone
    pub fn contains(&self, position: Vec2) -> bool {
        let to_target = position - self.origin;
        let distance = to_target.length();

        // Check distance first
        if distance > self.range || distance < 0.001 {
            return false;
        }

        // Check angle
        let to_target_normalized = to_target / distance;
        let dot = self.direction.dot(to_target_normalized);
        let angle = dot.clamp(-1.0, 1.0).acos();

        angle <= self.half_angle
    }

    /// Check if an enemy can be damaged (in cone and not already hit)
    pub fn can_damage(&self, entity: Entity, position: Vec2) -> bool {
        self.contains(position) && !self.hit_enemies.contains(&entity)
    }

    /// Mark an enemy as hit
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_enemies.insert(entity);
    }
}

/// Poison stack debuff applied to enemies hit by venom spray.
/// Stacks increase DOT damage linearly.
#[derive(Component, Debug, Clone)]
pub struct PoisonStack {
    /// Current number of stacks
    pub stacks: u32,
    /// Maximum stacks allowed
    pub max_stacks: u32,
    /// Damage per tick per stack
    pub damage_per_stack: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Duration before stacks expire
    pub duration: Timer,
}

impl PoisonStack {
    pub fn new() -> Self {
        Self {
            stacks: 1,
            max_stacks: POISON_STACK_MAX,
            damage_per_stack: POISON_STACK_DAMAGE_PER_STACK,
            tick_timer: Timer::from_seconds(POISON_STACK_TICK_INTERVAL, TimerMode::Repeating),
            duration: Timer::from_seconds(POISON_STACK_DURATION, TimerMode::Once),
        }
    }

    /// Add a stack, capped at max_stacks
    pub fn add_stack(&mut self) {
        if self.stacks < self.max_stacks {
            self.stacks += 1;
        }
        // Refresh duration when adding stacks
        self.duration.reset();
    }

    /// Calculate damage for this tick based on stack count
    pub fn tick_damage(&self) -> f32 {
        self.stacks as f32 * self.damage_per_stack
    }

    /// Check if the stack has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick both timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);
    }

    /// Check if ready to apply damage
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }
}

impl Default for PoisonStack {
    fn default() -> Self {
        Self::new()
    }
}

/// System that detects enemies in venom spray cone and applies/adds poison stacks
pub fn venom_spray_hit_detection(
    mut commands: Commands,
    mut cone_query: Query<&mut VenomSprayCone>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut poison_query: Query<&mut PoisonStack>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut cone in cone_query.iter_mut() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);

            if cone.can_damage(enemy_entity, enemy_pos) {
                // Mark as hit
                cone.mark_hit(enemy_entity);

                // Apply initial damage
                damage_events.write(DamageEvent::with_element(
                    enemy_entity,
                    cone.base_damage,
                    Element::Poison,
                ));

                // Apply or add poison stack
                if let Ok(mut existing_stack) = poison_query.get_mut(enemy_entity) {
                    existing_stack.add_stack();
                } else {
                    commands.entity(enemy_entity).insert(PoisonStack::new());
                }
            }
        }
    }
}

/// System that applies DOT damage based on poison stack count
pub fn poison_stack_damage_tick(
    mut poison_query: Query<(Entity, &mut PoisonStack)>,
    time: Res<Time>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (entity, mut stack) in poison_query.iter_mut() {
        stack.tick(time.delta());

        if stack.should_damage() {
            let damage = stack.tick_damage();
            damage_events.write(DamageEvent::with_element(entity, damage, Element::Poison));
        }
    }
}

/// System that removes expired poison stacks
pub fn poison_stack_decay(
    mut commands: Commands,
    poison_query: Query<(Entity, &PoisonStack)>,
) {
    for (entity, stack) in poison_query.iter() {
        if stack.is_expired() {
            commands.entity(entity).remove::<PoisonStack>();
        }
    }
}

/// System that despawns expired venom spray cones
pub fn cleanup_venom_spray(
    mut commands: Commands,
    mut cone_query: Query<(Entity, &mut VenomSprayCone)>,
    time: Res<Time>,
) {
    for (entity, mut cone) in cone_query.iter_mut() {
        cone.tick(time.delta());

        if cone.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast venom spray spell - spawns a cone hitbox in front of player
#[allow(clippy::too_many_arguments)]
pub fn fire_venom_spray(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_venom_spray_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast venom spray spell with explicit damage
#[allow(clippy::too_many_arguments)]
pub fn fire_venom_spray_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let origin = from_xz(spawn_position);
    let direction = (target_pos - origin).normalize_or_zero();

    let cone = VenomSprayCone::new(origin, direction, damage);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.poison_cloud.clone()),
            Transform::from_translation(spawn_position)
                .with_scale(Vec3::new(VENOM_SPRAY_RANGE, 0.5, VENOM_SPRAY_RANGE)),
            cone,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            cone,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod venom_spray_cone_tests {
        use super::*;

        #[test]
        fn test_cone_new() {
            let origin = Vec2::new(0.0, 0.0);
            let direction = Vec2::new(1.0, 0.0);
            let cone = VenomSprayCone::new(origin, direction, 20.0);

            assert_eq!(cone.origin, origin);
            assert_eq!(cone.direction, direction);
            assert_eq!(cone.range, VENOM_SPRAY_RANGE);
            assert_eq!(cone.base_damage, 20.0);
            assert!(cone.hit_enemies.is_empty());
        }

        #[test]
        fn test_cone_normalizes_direction() {
            let origin = Vec2::new(0.0, 0.0);
            let unnormalized = Vec2::new(3.0, 4.0);
            let cone = VenomSprayCone::new(origin, unnormalized, 20.0);

            assert!((cone.direction.length() - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_cone_is_expired() {
            let mut cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);
            assert!(!cone.is_expired());

            cone.tick(Duration::from_secs_f32(VENOM_SPRAY_DURATION + 0.1));
            assert!(cone.is_expired());
        }

        #[test]
        fn test_cone_contains_position_in_front() {
            let cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);

            // Position directly in front within range
            assert!(cone.contains(Vec2::new(3.0, 0.0)));
        }

        #[test]
        fn test_cone_contains_position_at_edge_of_angle() {
            let cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);
            let half_angle = (VENOM_SPRAY_CONE_ANGLE / 2.0).to_radians();

            // Position at edge of cone angle
            let edge_pos = Vec2::new(3.0 * half_angle.cos(), 3.0 * half_angle.sin() * 0.9);
            assert!(cone.contains(edge_pos));
        }

        #[test]
        fn test_cone_does_not_contain_position_behind() {
            let cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);

            // Position behind the cone
            assert!(!cone.contains(Vec2::new(-3.0, 0.0)));
        }

        #[test]
        fn test_cone_does_not_contain_position_outside_angle() {
            let cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);

            // Position outside cone angle (perpendicular)
            assert!(!cone.contains(Vec2::new(0.0, 3.0)));
        }

        #[test]
        fn test_cone_does_not_contain_position_beyond_range() {
            let cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);

            // Position beyond range
            assert!(!cone.contains(Vec2::new(VENOM_SPRAY_RANGE + 1.0, 0.0)));
        }

        #[test]
        fn test_cone_can_damage_in_cone() {
            let cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);
            let entity = Entity::from_bits(1);

            assert!(cone.can_damage(entity, Vec2::new(3.0, 0.0)));
        }

        #[test]
        fn test_cone_cannot_damage_already_hit() {
            let mut cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);
            let entity = Entity::from_bits(1);

            cone.mark_hit(entity);
            assert!(!cone.can_damage(entity, Vec2::new(3.0, 0.0)));
        }

        #[test]
        fn test_uses_poison_element_color() {
            let color = venom_spray_color();
            assert_eq!(color, Element::Poison.color());
        }
    }

    mod poison_stack_tests {
        use super::*;

        #[test]
        fn test_poison_stack_new() {
            let stack = PoisonStack::new();

            assert_eq!(stack.stacks, 1);
            assert_eq!(stack.max_stacks, POISON_STACK_MAX);
            assert_eq!(stack.damage_per_stack, POISON_STACK_DAMAGE_PER_STACK);
        }

        #[test]
        fn test_poison_stack_add_stack() {
            let mut stack = PoisonStack::new();
            assert_eq!(stack.stacks, 1);

            stack.add_stack();
            assert_eq!(stack.stacks, 2);

            stack.add_stack();
            assert_eq!(stack.stacks, 3);
        }

        #[test]
        fn test_poison_stack_max_cap_enforced() {
            let mut stack = PoisonStack::new();

            // Add stacks up to max
            for _ in 0..(POISON_STACK_MAX + 5) {
                stack.add_stack();
            }

            assert_eq!(stack.stacks, POISON_STACK_MAX);
        }

        #[test]
        fn test_poison_stack_damage_scales_with_count() {
            let mut stack = PoisonStack::new();
            assert_eq!(stack.tick_damage(), 1.0 * POISON_STACK_DAMAGE_PER_STACK);

            stack.add_stack();
            assert_eq!(stack.tick_damage(), 2.0 * POISON_STACK_DAMAGE_PER_STACK);

            stack.add_stack();
            assert_eq!(stack.tick_damage(), 3.0 * POISON_STACK_DAMAGE_PER_STACK);
        }

        #[test]
        fn test_poison_stack_expires_after_duration() {
            let mut stack = PoisonStack::new();
            assert!(!stack.is_expired());

            stack.tick(Duration::from_secs_f32(POISON_STACK_DURATION + 0.1));
            assert!(stack.is_expired());
        }

        #[test]
        fn test_poison_stack_should_damage_on_tick() {
            let mut stack = PoisonStack::new();
            assert!(!stack.should_damage());

            stack.tick(Duration::from_secs_f32(POISON_STACK_TICK_INTERVAL + 0.01));
            assert!(stack.should_damage());
        }

        #[test]
        fn test_poison_stack_add_refreshes_duration() {
            let mut stack = PoisonStack::new();

            // Tick partway through duration
            stack.tick(Duration::from_secs_f32(POISON_STACK_DURATION * 0.8));
            assert!(!stack.is_expired());

            // Add stack should refresh
            stack.add_stack();

            // Should need full duration again to expire
            stack.tick(Duration::from_secs_f32(POISON_STACK_DURATION * 0.8));
            assert!(!stack.is_expired());
        }
    }

    mod venom_spray_hit_detection_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_venom_spray_cone_spawns_on_cast() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ToxicSpray);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(5.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_venom_spray(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VenomSprayCone>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_venom_spray_hits_enemies_in_cone() {
            let mut app = setup_test_app();

            // Create cone facing +X
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0),
            ));

            // Spawn enemy in cone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(venom_spray_hit_detection);

            // Enemy should have PoisonStack
            let stack = app.world().get::<PoisonStack>(enemy_entity);
            assert!(stack.is_some(), "Enemy in cone should have PoisonStack");
        }

        #[test]
        fn test_venom_spray_misses_enemies_outside_cone() {
            let mut app = setup_test_app();

            // Create cone facing +X
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0),
            ));

            // Spawn enemy behind cone (negative X)
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(-3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(venom_spray_hit_detection);

            // Enemy should NOT have PoisonStack
            let stack = app.world().get::<PoisonStack>(enemy_entity);
            assert!(stack.is_none(), "Enemy outside cone should not have PoisonStack");
        }

        #[test]
        fn test_poison_stack_applied_on_hit() {
            let mut app = setup_test_app();

            // Create cone
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0),
            ));

            // Spawn enemy in cone
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            )).id();

            let _ = app.world_mut().run_system_once(venom_spray_hit_detection);

            let stack = app.world().get::<PoisonStack>(enemy_entity).unwrap();
            assert_eq!(stack.stacks, 1);
        }

        #[test]
        fn test_poison_stacks_accumulate() {
            let mut app = setup_test_app();

            // Spawn enemy with existing poison stack
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
                PoisonStack::new(),
            )).id();

            // Create cone hitting that enemy
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0),
            ));

            let _ = app.world_mut().run_system_once(venom_spray_hit_detection);

            let stack = app.world().get::<PoisonStack>(enemy_entity).unwrap();
            assert_eq!(stack.stacks, 2, "Stack should accumulate from 1 to 2");
        }
    }

    mod poison_stack_damage_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_poison_stack_should_damage_after_tick_interval() {
            // Test that the poison stack correctly identifies when damage should be applied
            let mut stack = PoisonStack::new();

            // Initially should not damage
            assert!(!stack.should_damage());

            // Tick past interval
            stack.tick(Duration::from_secs_f32(POISON_STACK_TICK_INTERVAL + 0.01));

            // Now should damage
            assert!(stack.should_damage());
        }

        #[test]
        fn test_poison_stack_damage_scales_with_stack_count() {
            // Test that tick_damage method returns correct values
            let mut stack = PoisonStack::new();
            assert_eq!(stack.tick_damage(), POISON_STACK_DAMAGE_PER_STACK);

            stack.stacks = 3;
            let expected = 3.0 * POISON_STACK_DAMAGE_PER_STACK;
            assert!(
                (stack.tick_damage() - expected).abs() < 0.01,
                "Damage should be {} for 3 stacks, got {}",
                expected,
                stack.tick_damage()
            );
        }

        #[test]
        fn test_poison_stack_damage_at_max_stacks() {
            let mut stack = PoisonStack::new();
            stack.stacks = POISON_STACK_MAX;
            let expected = POISON_STACK_MAX as f32 * POISON_STACK_DAMAGE_PER_STACK;
            assert!(
                (stack.tick_damage() - expected).abs() < 0.01,
                "Max stack damage should be {}",
                expected
            );
        }
    }

    mod poison_stack_decay_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_poison_stack_expires_after_duration() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();

            // Create entity with expired poison stack
            let mut stack = PoisonStack::new();
            stack.duration = Timer::from_seconds(0.0, TimerMode::Once);
            stack.duration.tick(Duration::from_secs(1));

            let entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                stack,
            )).id();

            let _ = app.world_mut().run_system_once(poison_stack_decay);

            // PoisonStack should be removed
            let stack = app.world().get::<PoisonStack>(entity);
            assert!(stack.is_none(), "Expired poison stack should be removed");
        }

        #[test]
        fn test_poison_stack_survives_before_expiry() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create entity with fresh poison stack
            let entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                PoisonStack::new(),
            )).id();

            let _ = app.world_mut().run_system_once(poison_stack_decay);

            // PoisonStack should still exist
            let stack = app.world().get::<PoisonStack>(entity);
            assert!(stack.is_some(), "Non-expired poison stack should remain");
        }
    }

    mod cleanup_venom_spray_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_venom_spray_cone_despawns_when_expired() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create expired cone
            let mut cone = VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0);
            cone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            cone.duration.tick(Duration::from_secs(1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                cone,
            )).id();

            // Need to advance time for the system
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(cleanup_venom_spray);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_venom_spray_cone_survives_before_expiry() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                VenomSprayCone::new(Vec2::ZERO, Vec2::X, 20.0),
            )).id();

            // Small time advance that doesn't expire the cone
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(cleanup_venom_spray);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod fire_venom_spray_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_venom_spray_spawns_cone() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ToxicSpray);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_venom_spray(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VenomSprayCone>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_venom_spray_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ToxicSpray);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_venom_spray(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VenomSprayCone>();
            for cone in query.iter(app.world()) {
                assert_eq!(cone.base_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_venom_spray_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ToxicSpray);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_venom_spray_with_damage(
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

            let mut query = app.world_mut().query::<&VenomSprayCone>();
            for cone in query.iter(app.world()) {
                assert_eq!(cone.base_damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_venom_spray_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ToxicSpray);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_venom_spray(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&VenomSprayCone>();
            for cone in query.iter(app.world()) {
                assert!(
                    cone.direction.x > 0.9,
                    "Cone should face toward target (+X), got direction {:?}",
                    cone.direction
                );
            }
        }
    }
}
