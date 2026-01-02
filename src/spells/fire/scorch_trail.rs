//! Scorch Trail spell - Leaves burning ground behind the player while moving.
//!
//! A Fire element spell (Immolate SpellType) that creates fire patches at the player's
//! previous positions as they move. Patches persist for a duration and damage enemies
//! standing in them with damage-over-time ticks.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Scorch Trail spell
pub const SCORCH_TRAIL_PATCH_LIFETIME: f32 = 4.0;
pub const SCORCH_TRAIL_TICK_INTERVAL: f32 = 0.5;
pub const SCORCH_TRAIL_TICK_DAMAGE_RATIO: f32 = 0.1; // 10% of spell damage per tick
pub const SCORCH_TRAIL_SPAWN_DISTANCE: f32 = 1.5; // Minimum distance between patches
pub const SCORCH_TRAIL_PATCH_RADIUS: f32 = 1.2;
pub const SCORCH_TRAIL_DURATION: f32 = 8.0; // How long the trail is active

/// Get the fire element color for visual effects
pub fn scorch_trail_color() -> Color {
    Element::Fire.color()
}

/// Ground fire patch that damages enemies over time.
/// Spawned at player's previous positions as they move.
#[derive(Component, Debug, Clone)]
pub struct ScorchPatch {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the damage zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub lifetime: Timer,
    /// Damage per tick
    pub damage_per_tick: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Set of enemies damaged this tick (prevents double damage)
    pub hit_this_tick: HashSet<Entity>,
}

impl ScorchPatch {
    pub fn new(center: Vec2, damage: f32) -> Self {
        let damage_per_tick = damage * SCORCH_TRAIL_TICK_DAMAGE_RATIO;
        Self {
            center,
            radius: SCORCH_TRAIL_PATCH_RADIUS,
            lifetime: Timer::from_seconds(SCORCH_TRAIL_PATCH_LIFETIME, TimerMode::Once),
            damage_per_tick,
            tick_timer: Timer::from_seconds(SCORCH_TRAIL_TICK_INTERVAL, TimerMode::Repeating),
            hit_this_tick: HashSet::new(),
        }
    }

    /// Check if the patch has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }

    /// Tick both timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.lifetime.tick(delta);
        self.tick_timer.tick(delta);

        // Reset hit tracking each tick
        if self.tick_timer.just_finished() {
            self.hit_this_tick.clear();
        }
    }

    /// Check if ready to apply damage
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }

    /// Check if an enemy is in range and hasn't been damaged this tick
    pub fn can_damage(&self, entity: Entity, enemy_pos: Vec2) -> bool {
        let distance = self.center.distance(enemy_pos);
        distance <= self.radius && !self.hit_this_tick.contains(&entity)
    }

    /// Mark an enemy as damaged this tick
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_this_tick.insert(entity);
    }
}

/// Marker component on player indicating the scorch trail is active.
/// Tracks when to spawn new patches based on movement.
#[derive(Component, Debug, Clone)]
pub struct ScorchTrailActive {
    /// Minimum distance player must move before spawning a new patch
    pub spawn_distance: f32,
    /// Last position where a patch was spawned
    pub last_spawn_position: Vec2,
    /// Duration timer (trail ends when finished)
    pub duration: Timer,
    /// Base damage for spawned patches
    pub damage: f32,
}

impl ScorchTrailActive {
    pub fn new(start_position: Vec2, damage: f32) -> Self {
        Self {
            spawn_distance: SCORCH_TRAIL_SPAWN_DISTANCE,
            last_spawn_position: start_position,
            duration: Timer::from_seconds(SCORCH_TRAIL_DURATION, TimerMode::Once),
            damage,
        }
    }

    /// Check if trail duration has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick the duration timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
    }

    /// Check if player has moved far enough to spawn a new patch
    pub fn should_spawn_patch(&self, current_position: Vec2) -> bool {
        let distance = self.last_spawn_position.distance(current_position);
        distance >= self.spawn_distance
    }

    /// Update the last spawn position
    pub fn update_spawn_position(&mut self, position: Vec2) {
        self.last_spawn_position = position;
    }
}

/// Component to track which patches have already damaged an enemy this tick.
/// Prevents stacking damage from multiple overlapping patches.
#[derive(Component, Debug, Clone, Default)]
pub struct ScorchedBy(pub HashSet<Entity>);

impl ScorchedBy {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// Clear the scorched tracking (called each damage tick)
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Check if this entity was already damaged by a specific patch
    pub fn was_scorched_by(&self, patch_entity: Entity) -> bool {
        self.0.contains(&patch_entity)
    }

    /// Mark this entity as scorched by a patch
    pub fn mark_scorched(&mut self, patch_entity: Entity) {
        self.0.insert(patch_entity);
    }
}

/// Activates scorch trail on player when spell is cast.
pub fn activate_scorch_trail(
    commands: &mut Commands,
    player_entity: Entity,
    player_transform: &Transform,
    spell: &Spell,
) {
    let player_pos = from_xz(player_transform.translation);
    let trail = ScorchTrailActive::new(player_pos, spell.damage());
    commands.entity(player_entity).insert(trail);
}

/// Activates scorch trail with explicit damage value.
pub fn activate_scorch_trail_with_damage(
    commands: &mut Commands,
    player_entity: Entity,
    player_transform: &Transform,
    damage: f32,
) {
    let player_pos = from_xz(player_transform.translation);
    let trail = ScorchTrailActive::new(player_pos, damage);
    commands.entity(player_entity).insert(trail);
}

/// System that spawns fire patches as player moves.
pub fn spawn_scorch_patches_system(
    mut commands: Commands,
    mut player_query: Query<(&Transform, &mut ScorchTrailActive), With<Player>>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (transform, mut trail) in player_query.iter_mut() {
        let current_pos = from_xz(transform.translation);

        if trail.should_spawn_patch(current_pos) {
            // Spawn patch at current position
            spawn_scorch_patch(
                &mut commands,
                current_pos,
                trail.damage,
                game_meshes.as_deref(),
                game_materials.as_deref(),
            );
            trail.update_spawn_position(current_pos);
        }
    }
}

/// Spawns a single scorch patch at the given position.
pub fn spawn_scorch_patch(
    commands: &mut Commands,
    position: Vec2,
    damage: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let patch = ScorchPatch::new(position, damage);
    let patch_pos = Vec3::new(position.x, 0.05, position.y); // Slightly above ground

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.fireball.clone()),
            Transform::from_translation(patch_pos).with_scale(Vec3::splat(patch.radius)),
            patch,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(patch_pos),
            patch,
        ));
    }
}

/// System that ticks scorch trail duration and removes expired trails.
pub fn tick_scorch_trail_system(
    mut commands: Commands,
    time: Res<Time>,
    mut trail_query: Query<(Entity, &mut ScorchTrailActive), With<Player>>,
) {
    for (entity, mut trail) in trail_query.iter_mut() {
        trail.tick(time.delta());

        if trail.is_expired() {
            commands.entity(entity).remove::<ScorchTrailActive>();
        }
    }
}

/// System that applies damage to enemies standing in scorch patches.
/// Ensures an enemy only takes damage once per tick regardless of overlapping patches.
pub fn scorch_patch_damage_system(
    mut patch_query: Query<&mut ScorchPatch>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    time: Res<Time>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    // Track which enemies have been damaged this tick cycle (across all patches)
    let mut enemies_damaged_this_tick: HashSet<Entity> = HashSet::new();

    // First, tick all patches and collect info about which patches are ready to damage
    // We need to collect this info before we start iterating because we'll need to
    // mark hits on patches afterwards
    let mut patches_ready_to_damage: Vec<(Vec2, f32, f32)> = Vec::new();
    let mut patch_indices_ready: Vec<usize> = Vec::new();

    for (idx, mut patch) in patch_query.iter_mut().enumerate() {
        patch.tick(time.delta());

        if patch.should_damage() {
            patches_ready_to_damage.push((patch.center, patch.radius, patch.damage_per_tick));
            patch_indices_ready.push(idx);
        }
    }

    // If no patches are ready to damage, we're done
    if patches_ready_to_damage.is_empty() {
        return;
    }

    // For each enemy, check if they're in range of any damage-ready patch
    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        // Skip if already damaged this tick
        if enemies_damaged_this_tick.contains(&enemy_entity) {
            continue;
        }

        let enemy_pos = from_xz(enemy_transform.translation);

        // Check each patch that's ready to damage
        for (center, radius, damage_per_tick) in &patches_ready_to_damage {
            let distance = center.distance(enemy_pos);
            if distance <= *radius {
                // Enemy is in this patch - apply damage and mark as damaged
                damage_events.write(DamageEvent::new(enemy_entity, *damage_per_tick));
                enemies_damaged_this_tick.insert(enemy_entity);
                break; // Only damage once per tick
            }
        }
    }

    // Update hit tracking in patches (for tests that check hit_this_tick)
    // We iterate again since we can't hold mutable references while doing other work
    for (patch_idx, mut patch) in patch_query.iter_mut().enumerate() {
        if patch_indices_ready.contains(&patch_idx) {
            for &enemy_entity in &enemies_damaged_this_tick {
                let enemy_pos = enemy_query.get(enemy_entity)
                    .map(|(_, t)| from_xz(t.translation))
                    .unwrap_or(Vec2::ZERO);
                if patch.center.distance(enemy_pos) <= patch.radius {
                    patch.mark_hit(enemy_entity);
                }
            }
        }
    }
}

/// System that despawns expired scorch patches.
pub fn scorch_patch_cleanup_system(
    mut commands: Commands,
    patch_query: Query<(Entity, &ScorchPatch)>,
) {
    for (entity, patch) in patch_query.iter() {
        if patch.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod scorch_patch_component_tests {
        use super::*;

        #[test]
        fn test_scorch_patch_new() {
            let center = Vec2::new(5.0, 10.0);
            let patch = ScorchPatch::new(center, 30.0);

            assert_eq!(patch.center, center);
            assert_eq!(patch.radius, SCORCH_TRAIL_PATCH_RADIUS);
            assert_eq!(patch.damage_per_tick, 30.0 * SCORCH_TRAIL_TICK_DAMAGE_RATIO);
            assert!(!patch.is_expired());
        }

        #[test]
        fn test_scorch_patch_is_expired() {
            let mut patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            assert!(!patch.is_expired());

            patch.tick(Duration::from_secs_f32(SCORCH_TRAIL_PATCH_LIFETIME + 0.1));
            assert!(patch.is_expired());
        }

        #[test]
        fn test_scorch_patch_should_damage() {
            let mut patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            assert!(!patch.should_damage());

            patch.tick(Duration::from_secs_f32(SCORCH_TRAIL_TICK_INTERVAL + 0.01));
            assert!(patch.should_damage());
        }

        #[test]
        fn test_scorch_patch_can_damage_in_range() {
            let patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(0.5, 0.0); // Within radius

            assert!(patch.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_scorch_patch_no_damage_outside() {
            let patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let out_of_range_pos = Vec2::new(100.0, 0.0);

            assert!(!patch.can_damage(entity, out_of_range_pos));
        }

        #[test]
        fn test_scorch_patch_cannot_damage_already_hit() {
            let mut patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(0.5, 0.0);

            patch.mark_hit(entity);
            assert!(!patch.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_scorch_patch_resets_hit_tracking_on_tick() {
            let mut patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            let entity = Entity::from_bits(1);

            patch.mark_hit(entity);
            assert!(patch.hit_this_tick.contains(&entity));

            patch.tick(Duration::from_secs_f32(SCORCH_TRAIL_TICK_INTERVAL + 0.01));
            assert!(patch.hit_this_tick.is_empty());
        }

        #[test]
        fn test_uses_fire_element_color() {
            let color = scorch_trail_color();
            assert_eq!(color, Element::Fire.color());
        }
    }

    mod scorch_trail_active_tests {
        use super::*;

        #[test]
        fn test_scorch_trail_active_new() {
            let start_pos = Vec2::new(10.0, 20.0);
            let trail = ScorchTrailActive::new(start_pos, 50.0);

            assert_eq!(trail.spawn_distance, SCORCH_TRAIL_SPAWN_DISTANCE);
            assert_eq!(trail.last_spawn_position, start_pos);
            assert_eq!(trail.damage, 50.0);
            assert!(!trail.is_expired());
        }

        #[test]
        fn test_scorch_trail_active_is_expired() {
            let mut trail = ScorchTrailActive::new(Vec2::ZERO, 50.0);
            assert!(!trail.is_expired());

            trail.tick(Duration::from_secs_f32(SCORCH_TRAIL_DURATION + 0.1));
            assert!(trail.is_expired());
        }

        #[test]
        fn test_scorch_trail_should_spawn_patch() {
            let trail = ScorchTrailActive::new(Vec2::ZERO, 50.0);

            // Near position - should not spawn
            let near_pos = Vec2::new(0.5, 0.0);
            assert!(!trail.should_spawn_patch(near_pos));

            // Far position - should spawn
            let far_pos = Vec2::new(SCORCH_TRAIL_SPAWN_DISTANCE + 0.1, 0.0);
            assert!(trail.should_spawn_patch(far_pos));
        }

        #[test]
        fn test_scorch_trail_update_spawn_position() {
            let mut trail = ScorchTrailActive::new(Vec2::ZERO, 50.0);
            let new_pos = Vec2::new(10.0, 15.0);

            trail.update_spawn_position(new_pos);

            assert_eq!(trail.last_spawn_position, new_pos);
        }
    }

    mod scorched_by_tests {
        use super::*;

        #[test]
        fn test_scorched_by_new() {
            let scorched = ScorchedBy::new();
            assert!(scorched.0.is_empty());
        }

        #[test]
        fn test_scorched_by_mark_and_check() {
            let mut scorched = ScorchedBy::new();
            let patch_entity = Entity::from_bits(42);

            assert!(!scorched.was_scorched_by(patch_entity));

            scorched.mark_scorched(patch_entity);
            assert!(scorched.was_scorched_by(patch_entity));
        }

        #[test]
        fn test_scorched_by_clear() {
            let mut scorched = ScorchedBy::new();
            let patch_entity = Entity::from_bits(42);

            scorched.mark_scorched(patch_entity);
            assert!(scorched.was_scorched_by(patch_entity));

            scorched.clear();
            assert!(!scorched.was_scorched_by(patch_entity));
        }
    }

    mod activate_scorch_trail_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_activate_scorch_trail_adds_component() {
            let mut app = setup_test_app();

            // Create player
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
            )).id();

            let spell = Spell::new(SpellType::Immolate);

            {
                let transform = app.world().get::<Transform>(player_entity).unwrap().clone();
                let mut commands = app.world_mut().commands();
                activate_scorch_trail(&mut commands, player_entity, &transform, &spell);
            }
            app.update();

            // Player should have ScorchTrailActive component
            assert!(app.world().get::<ScorchTrailActive>(player_entity).is_some());
        }

        #[test]
        fn test_activate_scorch_trail_uses_spell_damage() {
            let mut app = setup_test_app();

            // Create player
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(5.0, 0.5, 5.0)),
            )).id();

            let spell = Spell::new(SpellType::Immolate);
            let expected_damage = spell.damage();

            {
                let transform = app.world().get::<Transform>(player_entity).unwrap().clone();
                let mut commands = app.world_mut().commands();
                activate_scorch_trail(&mut commands, player_entity, &transform, &spell);
            }
            app.update();

            let trail = app.world().get::<ScorchTrailActive>(player_entity).unwrap();
            assert_eq!(trail.damage, expected_damage);
        }

        #[test]
        fn test_activate_scorch_trail_sets_initial_position() {
            let mut app = setup_test_app();

            let start_pos = Vec3::new(15.0, 0.5, 25.0);
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(start_pos),
            )).id();

            let spell = Spell::new(SpellType::Immolate);

            {
                let transform = app.world().get::<Transform>(player_entity).unwrap().clone();
                let mut commands = app.world_mut().commands();
                activate_scorch_trail(&mut commands, player_entity, &transform, &spell);
            }
            app.update();

            let trail = app.world().get::<ScorchTrailActive>(player_entity).unwrap();
            // from_xz converts 3D to 2D: x stays x, z becomes y
            assert_eq!(trail.last_spawn_position, Vec2::new(15.0, 25.0));
        }
    }

    mod spawn_scorch_patches_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_scorch_patches_spawn_on_movement() {
            let mut app = setup_test_app();

            // Create player with active trail at origin
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ScorchTrailActive::new(Vec2::ZERO, 30.0),
            )).id();

            // Move player far enough to trigger patch spawn
            {
                let mut transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
                transform.translation = Vec3::new(SCORCH_TRAIL_SPAWN_DISTANCE + 0.5, 0.5, 0.0);
            }

            let _ = app.world_mut().run_system_once(spawn_scorch_patches_system);
            app.update();

            // Should have spawned a patch
            let mut patch_query = app.world_mut().query::<&ScorchPatch>();
            let count = patch_query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn one patch on movement");
        }

        #[test]
        fn test_scorch_patches_respect_spawn_distance() {
            let mut app = setup_test_app();

            // Create player with active trail at origin
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ScorchTrailActive::new(Vec2::ZERO, 30.0),
            )).id();

            // Move player less than spawn distance
            {
                let mut transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
                transform.translation = Vec3::new(0.5, 0.5, 0.0); // Less than SCORCH_TRAIL_SPAWN_DISTANCE
            }

            let _ = app.world_mut().run_system_once(spawn_scorch_patches_system);
            app.update();

            // Should NOT have spawned a patch
            let mut patch_query = app.world_mut().query::<&ScorchPatch>();
            let count = patch_query.iter(app.world()).count();
            assert_eq!(count, 0, "Should not spawn patch when movement is too small");
        }

        #[test]
        fn test_scorch_patches_update_last_spawn_position() {
            let mut app = setup_test_app();

            // Create player with active trail at origin
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                ScorchTrailActive::new(Vec2::ZERO, 30.0),
            )).id();

            let new_pos = Vec3::new(SCORCH_TRAIL_SPAWN_DISTANCE + 0.5, 0.5, 2.0);
            // Move player far enough to trigger patch spawn
            {
                let mut transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
                transform.translation = new_pos;
            }

            let _ = app.world_mut().run_system_once(spawn_scorch_patches_system);

            let trail = app.world().get::<ScorchTrailActive>(player_entity).unwrap();
            // from_xz: x stays x, z becomes y
            assert_eq!(trail.last_spawn_position, Vec2::new(new_pos.x, new_pos.z));
        }
    }

    mod tick_scorch_trail_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_scorch_trail_deactivates_after_duration() {
            let mut app = setup_test_app();

            // Create player with active trail
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
                ScorchTrailActive::new(Vec2::ZERO, 30.0),
            )).id();

            // Advance time past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SCORCH_TRAIL_DURATION + 0.1));
            }

            let _ = app.world_mut().run_system_once(tick_scorch_trail_system);

            // Trail should be removed
            assert!(app.world().get::<ScorchTrailActive>(player_entity).is_none());
        }

        #[test]
        fn test_scorch_trail_survives_before_duration() {
            let mut app = setup_test_app();

            // Create player with active trail
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::ZERO),
                ScorchTrailActive::new(Vec2::ZERO, 30.0),
            )).id();

            // Advance time but not past duration
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(1.0));
            }

            let _ = app.world_mut().run_system_once(tick_scorch_trail_system);

            // Trail should still exist
            assert!(app.world().get::<ScorchTrailActive>(player_entity).is_some());
        }
    }

    mod scorch_patch_damage_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_damage_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_scorch_patch_damages_enemy_in_patch() {
            let mut app = setup_damage_test_app();

            // Create patch at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ScorchPatch::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SCORCH_TRAIL_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(scorch_patch_damage_system);

            // Check that enemy was marked as hit
            let mut patch_query = app.world_mut().query::<&ScorchPatch>();
            let patch = patch_query.single(app.world()).unwrap();
            assert!(!patch.hit_this_tick.is_empty(), "Enemy should have been marked as hit");
        }

        #[test]
        fn test_scorch_patch_no_damage_outside() {
            let mut app = setup_damage_test_app();

            // Create patch at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ScorchPatch::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy far outside range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(SCORCH_TRAIL_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(scorch_patch_damage_system);

            // Check that no enemy was marked as hit
            let mut patch_query = app.world_mut().query::<&ScorchPatch>();
            let patch = patch_query.single(app.world()).unwrap();
            assert!(patch.hit_this_tick.is_empty(), "No enemy should have been hit");
        }

        #[test]
        fn test_scorch_patch_damage_ticks_at_correct_interval() {
            let mut app = setup_damage_test_app();

            // Create patch
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ScorchPatch::new(Vec2::ZERO, 30.0),
            ));

            // Create enemy in range
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
            )).id();

            // Run 3 tick cycles
            let mut total_hits = 0;
            for _ in 0..3 {
                // Advance time to trigger tick
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(SCORCH_TRAIL_TICK_INTERVAL + 0.01));
                }

                // Run the system
                let _ = app.world_mut().run_system_once(scorch_patch_damage_system);

                // Count hits and clear hit tracking
                let mut patch_query = app.world_mut().query::<&mut ScorchPatch>();
                let mut patch = patch_query.single_mut(app.world_mut()).unwrap();
                if patch.hit_this_tick.contains(&enemy_entity) {
                    total_hits += 1;
                }
                patch.hit_this_tick.clear();
            }

            assert_eq!(total_hits, 3, "Enemy should have been hit 3 times over 3 tick cycles");
        }

        #[test]
        fn test_overlapping_patches_no_stack() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = App::new();
            app.add_message::<DamageEvent>();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage_events(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());
            app.add_systems(Update, count_damage_events);

            // Create 3 overlapping patches at origin, pre-ticked to be ready to damage
            for _ in 0..3 {
                let mut patch = ScorchPatch::new(Vec2::ZERO, 30.0);
                // Pre-tick the patch to make it ready to damage
                patch.tick(Duration::from_secs_f32(SCORCH_TRAIL_TICK_INTERVAL + 0.01));
                app.world_mut().spawn((
                    Transform::from_translation(Vec3::ZERO),
                    patch,
                ));
            }

            // Create enemy in range of all patches
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 0.0)),
            ));

            // Run damage system (patches are already ticked, so it will apply damage)
            // We use a custom system that doesn't tick but just applies damage
            fn apply_damage_no_tick(
                patch_query: Query<&ScorchPatch>,
                enemy_query: Query<(Entity, &Transform), With<Enemy>>,
                mut damage_events: MessageWriter<DamageEvent>,
            ) {
                let mut enemies_damaged: HashSet<Entity> = HashSet::new();

                // Collect patches that are ready to damage
                let mut patches_ready: Vec<(Vec2, f32, f32)> = Vec::new();
                for patch in patch_query.iter() {
                    // Check if patch's tick timer just finished (it was pre-ticked)
                    if patch.tick_timer.just_finished() {
                        patches_ready.push((patch.center, patch.radius, patch.damage_per_tick));
                    }
                }

                // Apply damage to enemies (only once per enemy)
                for (enemy_entity, enemy_transform) in enemy_query.iter() {
                    if enemies_damaged.contains(&enemy_entity) {
                        continue;
                    }

                    let enemy_pos = from_xz(enemy_transform.translation);
                    for (center, radius, damage) in &patches_ready {
                        if center.distance(enemy_pos) <= *radius {
                            damage_events.write(DamageEvent::new(enemy_entity, *damage));
                            enemies_damaged.insert(enemy_entity);
                            break;
                        }
                    }
                }
            }

            let _ = app.world_mut().run_system_once(apply_damage_no_tick);
            app.update(); // Process the damage events

            // Verify only one damage event was generated despite 3 overlapping patches
            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy should only be damaged once per tick regardless of overlapping patches");
        }
    }

    mod scorch_patch_cleanup_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_scorch_patch_despawns_after_lifetime() {
            let mut app = App::new();

            let mut patch = ScorchPatch::new(Vec2::ZERO, 30.0);
            patch.lifetime = Timer::from_seconds(0.0, TimerMode::Once);
            patch.lifetime.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                patch,
            )).id();

            let _ = app.world_mut().run_system_once(scorch_patch_cleanup_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_scorch_patch_survives_before_expiry() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ScorchPatch::new(Vec2::ZERO, 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(scorch_patch_cleanup_system);

            assert!(app.world().entities().contains(entity));
        }

        #[test]
        fn test_multiple_patches_independent_cleanup() {
            let mut app = App::new();

            // Expired patch
            let mut expired_patch = ScorchPatch::new(Vec2::new(0.0, 0.0), 30.0);
            expired_patch.lifetime = Timer::from_seconds(0.0, TimerMode::Once);
            expired_patch.lifetime.tick(Duration::from_secs(1));

            let expired_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                expired_patch,
            )).id();

            // Active patch
            let active_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.0, 10.0)),
                ScorchPatch::new(Vec2::new(10.0, 10.0), 30.0),
            )).id();

            let _ = app.world_mut().run_system_once(scorch_patch_cleanup_system);

            assert!(!app.world().entities().contains(expired_entity), "Expired patch should despawn");
            assert!(app.world().entities().contains(active_entity), "Active patch should survive");
        }
    }
}
