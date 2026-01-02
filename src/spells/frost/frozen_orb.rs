//! Frozen Orb spell - Slow-moving orb that damages enemies in its aura.
//!
//! A Frost element spell (Blizzard SpellType) that creates a slow-moving orb
//! which radiates freezing energy. Enemies within the damage radius take
//! periodic damage as the orb passes through them. The orb pierces enemies
//! rather than despawning on collision. Ice shard particles fall from the sky
//! around the orb for visual effect.

use std::collections::HashMap;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Frozen Orb spell
pub const FROZEN_ORB_SPEED: f32 = 8.0;
pub const FROZEN_ORB_DAMAGE_RADIUS: f32 = 3.0;
pub const FROZEN_ORB_LIFETIME: f32 = 4.0;
pub const FROZEN_ORB_TICK_INTERVAL: f32 = 0.25;
pub const FROZEN_ORB_HIT_COOLDOWN: f32 = 0.5; // Per-enemy damage cooldown

/// Ice shard particle configuration
pub const ICE_SHARD_SPAWN_RATE: f32 = 0.08; // Seconds between ice shard spawns
pub const ICE_SHARD_SPAWN_HEIGHT: f32 = 6.0; // Height above orb where shards spawn
pub const ICE_SHARD_FALL_SPEED: f32 = 12.0; // Vertical fall speed
pub const ICE_SHARD_HORIZONTAL_SPEED: f32 = 4.0; // Horizontal drift speed (creates angle)
pub const ICE_SHARD_SPAWN_RADIUS: f32 = 3.0; // Radius around orb for spawn positions
pub const ICE_SHARD_VISUAL_SCALE: f32 = 0.15; // Scale of ice shard mesh
pub const ICE_SHARD_GROUND_LEVEL: f32 = 0.0; // Ground level for cleanup

/// Get the frost element color for visual effects
pub fn frozen_orb_color() -> Color {
    Element::Frost.color()
}

/// A slow-moving orb that damages enemies within its aura radius.
/// Pierces through enemies and applies damage on a tick timer.
/// Spawns falling ice shard particles for visual effect.
#[derive(Component, Debug, Clone)]
pub struct FrozenOrb {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer (despawns when finished)
    pub lifetime: Timer,
    /// Damage radius (enemies within this range take damage)
    pub damage_radius: f32,
    /// Damage per tick
    pub damage_per_tick: f32,
    /// Timer between damage applications
    pub tick_timer: Timer,
    /// Per-enemy cooldown tracking (prevents damage spam)
    pub hit_cooldowns: HashMap<Entity, Timer>,
    /// Timer for spawning ice shard particles
    pub shard_spawn_timer: Timer,
}

/// Individual falling ice shard particle (purely visual).
/// Falls from above at an angle toward the ground.
#[derive(Component, Debug, Clone)]
pub struct FallingIceShard {
    /// Horizontal drift direction (normalized XZ direction)
    pub drift_direction: Vec2,
    /// Horizontal speed in units per second
    pub horizontal_speed: f32,
    /// Vertical fall speed in units per second
    pub fall_speed: f32,
}

impl FallingIceShard {
    pub fn new(drift_direction: Vec2) -> Self {
        Self {
            drift_direction: drift_direction.normalize_or_zero(),
            horizontal_speed: ICE_SHARD_HORIZONTAL_SPEED,
            fall_speed: ICE_SHARD_FALL_SPEED,
        }
    }

    /// Create with a random drift direction
    pub fn random() -> Self {
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let direction = Vec2::new(angle.cos(), angle.sin());
        Self::new(direction)
    }
}

impl FrozenOrb {
    pub fn new(direction: Vec2, damage: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed: FROZEN_ORB_SPEED,
            lifetime: Timer::from_seconds(FROZEN_ORB_LIFETIME, TimerMode::Once),
            damage_radius: FROZEN_ORB_DAMAGE_RADIUS,
            damage_per_tick: damage,
            tick_timer: Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating),
            hit_cooldowns: HashMap::new(),
            shard_spawn_timer: Timer::from_seconds(ICE_SHARD_SPAWN_RATE, TimerMode::Repeating),
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(direction, spell.damage())
    }

    /// Check if the orb has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }

    /// Tick all timers and cooldowns
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.lifetime.tick(delta);
        self.tick_timer.tick(delta);
        self.shard_spawn_timer.tick(delta);

        // Tick all per-enemy cooldowns
        for timer in self.hit_cooldowns.values_mut() {
            timer.tick(delta);
        }

        // Remove expired cooldowns
        self.hit_cooldowns.retain(|_, timer| !timer.is_finished());
    }

    /// Check if ready to spawn an ice shard particle
    pub fn should_spawn_shard(&self) -> bool {
        self.shard_spawn_timer.just_finished()
    }

    /// Check if ready to apply damage (tick interval elapsed)
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished()
    }

    /// Check if an enemy can be damaged (not on cooldown)
    pub fn can_damage(&self, entity: Entity) -> bool {
        match self.hit_cooldowns.get(&entity) {
            Some(timer) => timer.is_finished(),
            None => true,
        }
    }

    /// Mark an enemy as hit, starting the cooldown
    pub fn mark_hit(&mut self, entity: Entity) {
        self.hit_cooldowns.insert(
            entity,
            Timer::from_seconds(FROZEN_ORB_HIT_COOLDOWN, TimerMode::Once),
        );
    }
}

/// System that moves frozen orbs in their travel direction.
pub fn frozen_orb_movement_system(
    mut orb_query: Query<(&mut Transform, &FrozenOrb)>,
    time: Res<Time>,
) {
    for (mut transform, orb) in orb_query.iter_mut() {
        let movement = orb.direction * orb.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that ticks frozen orb timers.
pub fn frozen_orb_tick_system(
    mut orb_query: Query<&mut FrozenOrb>,
    time: Res<Time>,
) {
    for mut orb in orb_query.iter_mut() {
        orb.tick(time.delta());
    }
}

/// System that applies damage to enemies within frozen orb aura.
pub fn frozen_orb_damage_system(
    mut orb_query: Query<(&Transform, &mut FrozenOrb)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (orb_transform, mut orb) in orb_query.iter_mut() {
        if !orb.should_damage() {
            continue;
        }

        let orb_pos = from_xz(orb_transform.translation);

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = orb_pos.distance(enemy_pos);

            if distance <= orb.damage_radius && orb.can_damage(enemy_entity) {
                damage_events.write(DamageEvent::new(enemy_entity, orb.damage_per_tick));
                orb.mark_hit(enemy_entity);
            }
        }
    }
}

/// System that despawns expired frozen orbs.
pub fn frozen_orb_cleanup_system(
    mut commands: Commands,
    orb_query: Query<(Entity, &FrozenOrb)>,
) {
    for (entity, orb) in orb_query.iter() {
        if orb.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that spawns falling ice shard particles around frozen orbs (Blizzard spell).
pub fn blizzard_shard_spawn_system(
    mut commands: Commands,
    orb_query: Query<(&Transform, &FrozenOrb)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (orb_transform, orb) in orb_query.iter() {
        if !orb.should_spawn_shard() {
            continue;
        }

        // Spawn ice shard at random position around the orb, above it
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let distance = rand::random::<f32>().sqrt() * ICE_SHARD_SPAWN_RADIUS;
        let offset_x = angle.cos() * distance;
        let offset_z = angle.sin() * distance;

        let shard_pos = Vec3::new(
            orb_transform.translation.x + offset_x,
            orb_transform.translation.y + ICE_SHARD_SPAWN_HEIGHT,
            orb_transform.translation.z + offset_z,
        );

        let shard = FallingIceShard::random();

        if let (Some(meshes), Some(materials)) = (game_meshes.as_deref(), game_materials.as_deref()) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.ice_shard.clone()),
                Transform::from_translation(shard_pos).with_scale(Vec3::splat(ICE_SHARD_VISUAL_SCALE)),
                shard,
            ));
        } else {
            commands.spawn((
                Transform::from_translation(shard_pos),
                shard,
            ));
        }
    }
}

/// System that moves Blizzard ice shards downward at an angle.
pub fn blizzard_shard_movement_system(
    time: Res<Time>,
    mut shard_query: Query<(&FallingIceShard, &mut Transform)>,
) {
    for (shard, mut transform) in shard_query.iter_mut() {
        // Move downward
        transform.translation.y -= shard.fall_speed * time.delta_secs();

        // Apply horizontal drift for angled falling
        let drift = shard.drift_direction * shard.horizontal_speed * time.delta_secs();
        transform.translation.x += drift.x;
        transform.translation.z += drift.y;
    }
}

/// System that despawns Blizzard ice shards that reach ground level.
pub fn blizzard_shard_cleanup_system(
    mut commands: Commands,
    shard_query: Query<(Entity, &Transform), With<FallingIceShard>>,
) {
    for (entity, transform) in shard_query.iter() {
        if transform.translation.y <= ICE_SHARD_GROUND_LEVEL {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawns a frozen orb traveling in the given direction.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane.
#[allow(clippy::too_many_arguments)]
pub fn fire_frozen_orb(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_frozen_orb_with_damage(
        commands,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Spawns a frozen orb with explicit damage value.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_frozen_orb_with_damage(
    commands: &mut Commands,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let spawn_xz = from_xz(spawn_position);
    let direction = (target_pos - spawn_xz).normalize_or_zero();
    let orb = FrozenOrb::new(direction, damage);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.ice_shard.clone()),
            Transform::from_translation(spawn_position).with_scale(Vec3::splat(2.0)),
            orb,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            orb,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use bevy::ecs::system::RunSystemOnce;
    use crate::spell::SpellType;

    mod frozen_orb_component_tests {
        use super::*;

        #[test]
        fn test_frozen_orb_new() {
            let direction = Vec2::new(1.0, 0.0);
            let orb = FrozenOrb::new(direction, 20.0);

            assert_eq!(orb.direction, direction.normalize());
            assert_eq!(orb.speed, FROZEN_ORB_SPEED);
            assert_eq!(orb.damage_radius, FROZEN_ORB_DAMAGE_RADIUS);
            assert_eq!(orb.damage_per_tick, 20.0);
            assert!(!orb.is_expired());
            assert!(orb.hit_cooldowns.is_empty());
        }

        #[test]
        fn test_frozen_orb_from_spell() {
            let spell = Spell::new(SpellType::Blizzard);
            let direction = Vec2::new(0.0, 1.0);
            let orb = FrozenOrb::from_spell(direction, &spell);

            assert_eq!(orb.direction, direction.normalize());
            assert_eq!(orb.damage_per_tick, spell.damage());
        }

        #[test]
        fn test_frozen_orb_normalizes_direction() {
            let direction = Vec2::new(3.0, 4.0); // Length 5
            let orb = FrozenOrb::new(direction, 20.0);

            let expected = direction.normalize();
            assert!((orb.direction - expected).length() < 0.001);
        }

        #[test]
        fn test_frozen_orb_handles_zero_direction() {
            let orb = FrozenOrb::new(Vec2::ZERO, 20.0);
            assert_eq!(orb.direction, Vec2::ZERO);
        }

        #[test]
        fn test_frozen_orb_is_expired() {
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            assert!(!orb.is_expired());

            orb.tick(Duration::from_secs_f32(FROZEN_ORB_LIFETIME + 0.1));
            assert!(orb.is_expired());
        }

        #[test]
        fn test_frozen_orb_should_damage_on_tick() {
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            assert!(!orb.should_damage());

            orb.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL + 0.01));
            assert!(orb.should_damage());
        }

        #[test]
        fn test_frozen_orb_can_damage_new_enemy() {
            let orb = FrozenOrb::new(Vec2::X, 20.0);
            let entity = Entity::from_bits(1);

            assert!(orb.can_damage(entity));
        }

        #[test]
        fn test_frozen_orb_cannot_damage_on_cooldown() {
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            let entity = Entity::from_bits(1);

            orb.mark_hit(entity);
            assert!(!orb.can_damage(entity));
        }

        #[test]
        fn test_frozen_orb_can_damage_after_cooldown() {
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            let entity = Entity::from_bits(1);

            orb.mark_hit(entity);
            assert!(!orb.can_damage(entity));

            orb.tick(Duration::from_secs_f32(FROZEN_ORB_HIT_COOLDOWN + 0.01));
            assert!(orb.can_damage(entity));
        }

        #[test]
        fn test_frozen_orb_removes_expired_cooldowns() {
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            let entity = Entity::from_bits(1);

            orb.mark_hit(entity);
            assert!(!orb.hit_cooldowns.is_empty());

            orb.tick(Duration::from_secs_f32(FROZEN_ORB_HIT_COOLDOWN + 0.01));
            assert!(orb.hit_cooldowns.is_empty());
        }

        #[test]
        fn test_uses_frost_element_color() {
            let color = frozen_orb_color();
            assert_eq!(color, Element::Frost.color());
        }
    }

    mod frozen_orb_movement_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_frozen_orb_moves_in_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FrozenOrb::new(Vec2::new(1.0, 0.0), 20.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(frozen_orb_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!((transform.translation.x - FROZEN_ORB_SPEED).abs() < 0.01);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_frozen_orb_moves_slowly() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FrozenOrb::new(Vec2::X, 20.0),
            )).id();

            // Move for 0.5 seconds
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(frozen_orb_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            let expected = FROZEN_ORB_SPEED * 0.5;
            assert!((transform.translation.x - expected).abs() < 0.01);
        }

        #[test]
        fn test_frozen_orb_moves_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Moving in Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FrozenOrb::new(Vec2::new(0.0, 1.0), 20.0),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(frozen_orb_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert!((transform.translation.z - FROZEN_ORB_SPEED).abs() < 0.01);
        }
    }

    mod frozen_orb_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        fn setup_damage_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_frozen_orb_damages_enemy_in_radius() {
            let mut app = setup_damage_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Only use damage system and counter - no tick system since we've pre-ticked
            app.add_systems(Update, (frozen_orb_damage_system, count_damage).chain());

            // Create orb at origin
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            // Pre-tick to trigger damage on first update (just_finished will be true)
            orb.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
            orb.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
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
        fn test_frozen_orb_no_damage_outside_radius() {
            let mut app = setup_damage_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Only use damage system and counter - no tick system since we've pre-ticked
            app.add_systems(Update, (frozen_orb_damage_system, count_damage).chain());

            // Create orb at origin with pre-ticked timer
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
            orb.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
            ));

            // Create enemy outside radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_frozen_orb_pierces_enemies() {
            let mut app = setup_damage_test_app();

            // Only use damage system - no tick system since we've pre-ticked
            app.add_systems(Update, frozen_orb_damage_system);

            // Create orb
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
            orb.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));

            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
            )).id();

            // Create enemy to collide with
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            app.update();

            // Orb should still exist (not despawned on collision)
            assert!(app.world().entities().contains(orb_entity));
        }

        #[test]
        fn test_frozen_orb_cooldown_prevents_spam() {
            let mut app = setup_damage_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Only use damage system and counter - no tick system since we've pre-ticked
            app.add_systems(Update, (frozen_orb_damage_system, count_damage).chain());

            // Create orb with pre-ticked timer (just_finished is true)
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
            orb.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));

            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
            )).id();

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            // First update should damage
            app.update();
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);

            // Reset tick_timer to just_finished again for next damage tick
            {
                let mut orb_comp = app.world_mut().get_mut::<FrozenOrb>(orb_entity).unwrap();
                orb_comp.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
                orb_comp.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));
            }
            app.update();

            // Should still be 1 (enemy is on cooldown)
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_frozen_orb_damages_after_cooldown_expires() {
            let mut app = setup_damage_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Only use damage system and counter - no tick system
            app.add_systems(Update, (frozen_orb_damage_system, count_damage).chain());

            // Create orb with pre-ticked timer (just_finished is true)
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
            orb.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));

            let orb_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
            )).id();

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            // First update should damage
            app.update();
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);

            // Manually expire the cooldown and reset tick timer
            {
                let mut orb_comp = app.world_mut().get_mut::<FrozenOrb>(orb_entity).unwrap();
                // Tick the cooldowns past expiry
                orb_comp.tick(Duration::from_secs_f32(FROZEN_ORB_HIT_COOLDOWN + 0.01));
                // Reset tick_timer to just_finished for next damage tick
                orb_comp.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
                orb_comp.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));
            }
            app.update();

            // Should have damaged again (cooldown expired)
            assert_eq!(counter.0.load(Ordering::SeqCst), 2);
        }

        #[test]
        fn test_frozen_orb_damages_multiple_enemies() {
            let mut app = setup_damage_test_app();

            #[derive(Resource, Clone)]
            struct DamageCounter(Arc<AtomicUsize>);

            fn count_damage(
                mut events: MessageReader<DamageEvent>,
                counter: Res<DamageCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = DamageCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            // Only use damage system and counter - no tick system since we've pre-ticked
            app.add_systems(Update, (frozen_orb_damage_system, count_damage).chain());

            // Create orb with pre-ticked timer (just_finished is true)
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.tick_timer = Timer::from_seconds(FROZEN_ORB_TICK_INTERVAL, TimerMode::Repeating);
            orb.tick_timer.tick(Duration::from_secs_f32(FROZEN_ORB_TICK_INTERVAL));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
            ));

            // Create 3 enemies in range
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 * 0.5, 0.375, 0.0)),
                ));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 3);
        }
    }

    mod frozen_orb_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_frozen_orb_despawns_after_lifetime() {
            let mut app = App::new();

            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.lifetime = Timer::from_seconds(0.0, TimerMode::Once);
            orb.lifetime.tick(Duration::from_secs(1));

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                orb,
            )).id();

            let _ = app.world_mut().run_system_once(frozen_orb_cleanup_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_frozen_orb_survives_before_expiry() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                FrozenOrb::new(Vec2::X, 20.0),
            )).id();

            let _ = app.world_mut().run_system_once(frozen_orb_cleanup_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod fire_frozen_orb_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_frozen_orb_spawns_orb() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Blizzard);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_frozen_orb(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FrozenOrb>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_frozen_orb_direction_toward_target() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Blizzard);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_frozen_orb(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FrozenOrb>();
            for orb in query.iter(app.world()) {
                assert!(orb.direction.x > 0.9, "Orb should move toward target");
            }
        }

        #[test]
        fn test_fire_frozen_orb_uses_spell_damage() {
            let mut app = setup_test_app();
            let spell = Spell::new(SpellType::Blizzard);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_frozen_orb(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FrozenOrb>();
            for orb in query.iter(app.world()) {
                assert_eq!(orb.damage_per_tick, expected_damage);
            }
        }

        #[test]
        fn test_fire_frozen_orb_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_frozen_orb_with_damage(
                    &mut commands,
                    explicit_damage,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FrozenOrb>();
            for orb in query.iter(app.world()) {
                assert_eq!(orb.damage_per_tick, explicit_damage);
            }
        }

        #[test]
        fn test_fire_frozen_orb_spawns_at_position() {
            let mut app = setup_test_app();
            let spawn_pos = Vec3::new(5.0, 0.5, 10.0);
            let target_pos = Vec2::new(15.0, 10.0);

            {
                let mut commands = app.world_mut().commands();
                fire_frozen_orb_with_damage(
                    &mut commands,
                    20.0,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&FrozenOrb, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation, spawn_pos);
            }
        }
    }

    mod falling_ice_shard_tests {
        use super::*;

        #[test]
        fn test_falling_ice_shard_new() {
            let direction = Vec2::new(1.0, 0.0);
            let shard = FallingIceShard::new(direction);

            assert_eq!(shard.drift_direction, direction.normalize());
            assert_eq!(shard.horizontal_speed, ICE_SHARD_HORIZONTAL_SPEED);
            assert_eq!(shard.fall_speed, ICE_SHARD_FALL_SPEED);
        }

        #[test]
        fn test_falling_ice_shard_normalizes_direction() {
            let direction = Vec2::new(3.0, 4.0); // Length 5
            let shard = FallingIceShard::new(direction);

            let expected = direction.normalize();
            assert!((shard.drift_direction - expected).length() < 0.001);
        }

        #[test]
        fn test_falling_ice_shard_handles_zero_direction() {
            let shard = FallingIceShard::new(Vec2::ZERO);
            assert_eq!(shard.drift_direction, Vec2::ZERO);
        }

        #[test]
        fn test_falling_ice_shard_random_has_normalized_direction() {
            let shard = FallingIceShard::random();
            // Either zero or normalized (length ~1)
            let len = shard.drift_direction.length();
            assert!(len < 0.001 || (len - 1.0).abs() < 0.001);
        }
    }

    mod blizzard_shard_movement_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_blizzard_shard_moves_downward() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let initial_y = ICE_SHARD_SPAWN_HEIGHT;
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, initial_y, 0.0)),
                FallingIceShard::new(Vec2::ZERO), // No horizontal drift
            )).id();

            // Advance time
            let delta_time = 0.5;
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(delta_time));
            }

            let _ = app.world_mut().run_system_once(blizzard_shard_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            let expected_y = initial_y - ICE_SHARD_FALL_SPEED * delta_time;
            assert!(
                (transform.translation.y - expected_y).abs() < 0.1,
                "Ice shard should move down. Expected y={}, got y={}",
                expected_y,
                transform.translation.y
            );
        }

        #[test]
        fn test_blizzard_shard_drifts_horizontally() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)),
                FallingIceShard::new(Vec2::new(1.0, 0.0)), // Drift in +X direction
            )).id();

            let delta_time = 0.5;
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(delta_time));
            }

            let _ = app.world_mut().run_system_once(blizzard_shard_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            let expected_x = ICE_SHARD_HORIZONTAL_SPEED * delta_time;
            assert!(
                (transform.translation.x - expected_x).abs() < 0.1,
                "Ice shard should drift in X. Expected x={}, got x={}",
                expected_x,
                transform.translation.x
            );
        }

        #[test]
        fn test_blizzard_shard_moves_at_angle() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)),
                FallingIceShard::new(Vec2::new(1.0, 0.0)),
            )).id();

            let delta_time = 1.0;
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(delta_time));
            }

            let _ = app.world_mut().run_system_once(blizzard_shard_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Should have moved both down and to the right
            assert!(transform.translation.y < 5.0, "Should have moved down");
            assert!(transform.translation.x > 0.0, "Should have moved right");
        }
    }

    mod blizzard_shard_cleanup_system_tests {
        use super::*;
        use bevy::app::App;

        #[test]
        fn test_blizzard_shard_despawns_at_ground() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, ICE_SHARD_GROUND_LEVEL, 0.0)),
                FallingIceShard::random(),
            )).id();

            let _ = app.world_mut().run_system_once(blizzard_shard_cleanup_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Ice shard should despawn at ground level"
            );
        }

        #[test]
        fn test_blizzard_shard_despawns_below_ground() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
                FallingIceShard::random(),
            )).id();

            let _ = app.world_mut().run_system_once(blizzard_shard_cleanup_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Ice shard should despawn below ground level"
            );
        }

        #[test]
        fn test_blizzard_shard_survives_above_ground() {
            let mut app = App::new();

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
                FallingIceShard::random(),
            )).id();

            let _ = app.world_mut().run_system_once(blizzard_shard_cleanup_system);

            assert!(
                app.world().get_entity(entity).is_ok(),
                "Ice shard should survive above ground level"
            );
        }
    }

    mod blizzard_shard_spawn_system_tests {
        use super::*;
        use bevy::app::App;

        fn setup_spawn_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_orb_has_shard_spawn_timer() {
            let orb = FrozenOrb::new(Vec2::X, 20.0);
            assert!(!orb.should_spawn_shard(), "Should not spawn immediately");
        }

        #[test]
        fn test_orb_should_spawn_shard_after_interval() {
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.tick(Duration::from_secs_f32(ICE_SHARD_SPAWN_RATE + 0.01));
            assert!(orb.should_spawn_shard(), "Should spawn after interval");
        }

        #[test]
        fn test_blizzard_shard_spawn_system_creates_shards() {
            let mut app = setup_spawn_test_app();

            // Create orb with pre-ticked spawn timer
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.shard_spawn_timer.tick(Duration::from_secs_f32(ICE_SHARD_SPAWN_RATE + 0.01));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                orb,
            ));

            let _ = app.world_mut().run_system_once(blizzard_shard_spawn_system);

            let mut shard_query = app.world_mut().query::<&FallingIceShard>();
            let count = shard_query.iter(app.world()).count();
            assert!(count >= 1, "Should spawn at least one ice shard");
        }

        #[test]
        fn test_blizzard_shard_spawns_above_orb() {
            let mut app = setup_spawn_test_app();

            let orb_y = 0.5;
            let mut orb = FrozenOrb::new(Vec2::X, 20.0);
            orb.shard_spawn_timer.tick(Duration::from_secs_f32(ICE_SHARD_SPAWN_RATE + 0.01));

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, orb_y, 0.0)),
                orb,
            ));

            let _ = app.world_mut().run_system_once(blizzard_shard_spawn_system);

            let mut shard_query = app.world_mut().query::<(&FallingIceShard, &Transform)>();
            for (_, transform) in shard_query.iter(app.world()) {
                assert!(
                    transform.translation.y > orb_y,
                    "Ice shard should spawn above orb. Orb y={}, shard y={}",
                    orb_y,
                    transform.translation.y
                );
            }
        }

        #[test]
        fn test_blizzard_shard_spawns_within_radius() {
            let mut app = setup_spawn_test_app();

            let orb_pos = Vec3::new(5.0, 0.5, 5.0);

            // Spawn multiple shards to test radius
            for _ in 0..10 {
                let mut orb = FrozenOrb::new(Vec2::X, 20.0);
                orb.shard_spawn_timer.tick(Duration::from_secs_f32(ICE_SHARD_SPAWN_RATE + 0.01));

                app.world_mut().spawn((
                    Transform::from_translation(orb_pos),
                    orb,
                ));

                let _ = app.world_mut().run_system_once(blizzard_shard_spawn_system);
            }

            let mut shard_query = app.world_mut().query::<&Transform>();
            for transform in shard_query.iter(app.world()) {
                let dx = transform.translation.x - orb_pos.x;
                let dz = transform.translation.z - orb_pos.z;
                let distance = (dx * dx + dz * dz).sqrt();

                assert!(
                    distance <= ICE_SHARD_SPAWN_RADIUS,
                    "Ice shard should spawn within radius. Distance={}, max={}",
                    distance,
                    ICE_SHARD_SPAWN_RADIUS
                );
            }
        }
    }
}
