use std::collections::HashSet;
use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Poison Cloud spell
pub const POISON_CLOUD_PROJECTILE_SPEED: f32 = 12.0;
pub const POISON_CLOUD_ARC_HEIGHT: f32 = 4.0;
pub const POISON_CLOUD_MAX_RANGE: f32 = 15.0;
pub const POISON_CLOUD_ZONE_RADIUS: f32 = 3.0;
pub const POISON_CLOUD_ZONE_DURATION: f32 = 4.0;
pub const POISON_CLOUD_TICK_INTERVAL: f32 = 0.5;
pub const POISON_CLOUD_TICK_DAMAGE_RATIO: f32 = 0.15; // 15% of spell damage per tick

/// Get the poison element color for visual effects
pub fn poison_cloud_color() -> Color {
    Element::Poison.color()
}

/// Arcing projectile that creates a poison cloud on impact.
/// Uses parabolic trajectory to reach target position.
#[derive(Component, Debug, Clone)]
pub struct PoisonCloudProjectile {
    /// Starting position on XZ plane
    pub start_pos: Vec2,
    /// Target position on XZ plane
    pub target_pos: Vec2,
    /// Current progress along arc (0.0 to 1.0)
    pub progress: f32,
    /// Speed multiplier for arc traversal
    pub speed: f32,
    /// Maximum height of the arc
    pub arc_height: f32,
    /// Damage for the cloud zone
    pub damage: f32,
    /// Tick damage for cloud DOT
    pub tick_damage: f32,
}

impl PoisonCloudProjectile {
    pub fn new(start_pos: Vec2, target_pos: Vec2, damage: f32) -> Self {
        let tick_damage = damage * POISON_CLOUD_TICK_DAMAGE_RATIO;
        Self {
            start_pos,
            target_pos,
            progress: 0.0,
            speed: POISON_CLOUD_PROJECTILE_SPEED,
            arc_height: POISON_CLOUD_ARC_HEIGHT,
            damage,
            tick_damage,
        }
    }

    pub fn from_spell(start_pos: Vec2, target_pos: Vec2, spell: &Spell) -> Self {
        Self::new(start_pos, target_pos, spell.damage())
    }

    /// Calculate the total distance of the arc trajectory
    pub fn arc_distance(&self) -> f32 {
        self.start_pos.distance(self.target_pos)
    }

    /// Calculate current position based on progress (0.0 to 1.0)
    /// Returns (XZ position, Y height)
    pub fn position_at_progress(&self, progress: f32) -> (Vec2, f32) {
        // Linear interpolation for XZ position
        let xz = self.start_pos.lerp(self.target_pos, progress);

        // Parabolic arc for height: h = 4 * arc_height * t * (1 - t)
        // Maximum at t=0.5, zero at t=0 and t=1
        let height = 4.0 * self.arc_height * progress * (1.0 - progress);

        (xz, height)
    }

    /// Check if the projectile has reached its target
    pub fn is_finished(&self) -> bool {
        self.progress >= 1.0
    }

    /// Advance the projectile along its arc
    pub fn advance(&mut self, delta_secs: f32) {
        let distance = self.arc_distance();
        if distance > 0.0 {
            // Progress rate based on speed and distance
            let progress_rate = self.speed / distance;
            self.progress = (self.progress + progress_rate * delta_secs).min(1.0);
        } else {
            // Already at target
            self.progress = 1.0;
        }
    }
}

/// Lingering toxic cloud that damages enemies over time.
#[derive(Component, Debug, Clone)]
pub struct PoisonCloudZone {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Radius of the damage zone
    pub radius: f32,
    /// Duration timer (despawns when finished)
    pub duration: Timer,
    /// Damage per tick
    pub tick_damage: f32,
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Set of enemies damaged this tick (prevents double damage)
    pub hit_this_tick: HashSet<Entity>,
}

impl PoisonCloudZone {
    pub fn new(center: Vec2, _damage: f32, tick_damage: f32) -> Self {
        Self {
            center,
            radius: POISON_CLOUD_ZONE_RADIUS,
            duration: Timer::from_seconds(POISON_CLOUD_ZONE_DURATION, TimerMode::Once),
            tick_damage,
            tick_timer: Timer::from_seconds(POISON_CLOUD_TICK_INTERVAL, TimerMode::Repeating),
            hit_this_tick: HashSet::new(),
        }
    }

    pub fn from_projectile(projectile: &PoisonCloudProjectile) -> Self {
        Self::new(projectile.target_pos, projectile.damage, projectile.tick_damage)
    }

    /// Check if the cloud has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Tick both timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
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

/// System that moves poison cloud projectiles along their arc trajectory
pub fn poison_cloud_projectile_movement_system(
    mut projectile_query: Query<(&mut Transform, &mut PoisonCloudProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, mut projectile) in projectile_query.iter_mut() {
        projectile.advance(time.delta_secs());
        let (xz, height) = projectile.position_at_progress(projectile.progress);
        // Base height of projectile + arc height
        transform.translation = Vec3::new(xz.x, 0.5 + height, xz.y);
    }
}

/// System that spawns poison cloud zones when projectiles reach their target
pub fn poison_cloud_spawn_zone_system(
    mut commands: Commands,
    projectile_query: Query<(Entity, &PoisonCloudProjectile)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (entity, projectile) in projectile_query.iter() {
        if projectile.is_finished() {
            // Despawn the projectile
            commands.entity(entity).despawn();

            // Spawn the poison cloud zone at target location
            let zone = PoisonCloudZone::from_projectile(projectile);
            let zone_pos = Vec3::new(projectile.target_pos.x, 0.1, projectile.target_pos.y);

            if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), game_materials.as_ref()) {
                commands.spawn((
                    Mesh3d(meshes.explosion.clone()),
                    MeshMaterial3d(materials.poison_cloud.clone()),
                    Transform::from_translation(zone_pos).with_scale(Vec3::splat(zone.radius)),
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
    }
}

/// System that applies damage to enemies in poison cloud zones
pub fn poison_cloud_damage_system(
    mut zone_query: Query<&mut PoisonCloudZone>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    time: Res<Time>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut zone in zone_query.iter_mut() {
        zone.tick(time.delta());

        if zone.should_damage() {
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);

                if zone.can_damage(enemy_entity, enemy_pos) {
                    damage_events.write(DamageEvent::new(enemy_entity, zone.tick_damage));
                    zone.mark_hit(enemy_entity);
                }
            }
        }
    }
}

/// System that despawns expired poison cloud zones
pub fn poison_cloud_cleanup_system(
    mut commands: Commands,
    zone_query: Query<(Entity, &PoisonCloudZone)>,
) {
    for (entity, zone) in zone_query.iter() {
        if zone.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast poison cloud spell - spawns an arcing projectile toward target.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is target on XZ plane.
#[allow(clippy::too_many_arguments)]
pub fn fire_poison_cloud(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_poison_cloud_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast poison cloud spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is target on XZ plane.
/// `damage` is the pre-calculated final damage (including attunement multiplier).
#[allow(clippy::too_many_arguments)]
pub fn fire_poison_cloud_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let start_pos = from_xz(spawn_position);

    // Clamp target to max range
    let direction = (target_pos - start_pos).normalize_or_zero();
    let distance = start_pos.distance(target_pos).min(POISON_CLOUD_MAX_RANGE);
    let clamped_target = start_pos + direction * distance;

    let projectile = PoisonCloudProjectile::new(start_pos, clamped_target, damage);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.bullet.clone()),
            MeshMaterial3d(materials.poison_projectile.clone()),
            Transform::from_translation(spawn_position),
            projectile,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(spawn_position),
            projectile,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod poison_cloud_projectile_tests {
        use super::*;

        #[test]
        fn test_projectile_new() {
            let start = Vec2::new(0.0, 0.0);
            let target = Vec2::new(10.0, 0.0);
            let projectile = PoisonCloudProjectile::new(start, target, 30.0);

            assert_eq!(projectile.start_pos, start);
            assert_eq!(projectile.target_pos, target);
            assert_eq!(projectile.progress, 0.0);
            assert_eq!(projectile.damage, 30.0);
            assert_eq!(projectile.tick_damage, 30.0 * POISON_CLOUD_TICK_DAMAGE_RATIO);
        }

        #[test]
        fn test_projectile_from_spell() {
            let spell = Spell::new(SpellType::PlagueCloud);
            let start = Vec2::new(0.0, 0.0);
            let target = Vec2::new(10.0, 0.0);
            let projectile = PoisonCloudProjectile::from_spell(start, target, &spell);

            assert_eq!(projectile.damage, spell.damage());
        }

        #[test]
        fn test_arc_distance() {
            let projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            assert_eq!(projectile.arc_distance(), 10.0);
        }

        #[test]
        fn test_position_at_progress_start() {
            let projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            let (xz, height) = projectile.position_at_progress(0.0);
            assert_eq!(xz, Vec2::new(0.0, 0.0));
            assert_eq!(height, 0.0);
        }

        #[test]
        fn test_position_at_progress_middle() {
            let projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            let (xz, height) = projectile.position_at_progress(0.5);
            assert_eq!(xz, Vec2::new(5.0, 0.0));
            // Height at midpoint: 4 * arc_height * 0.5 * 0.5 = arc_height
            assert_eq!(height, POISON_CLOUD_ARC_HEIGHT);
        }

        #[test]
        fn test_position_at_progress_end() {
            let projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            let (xz, height) = projectile.position_at_progress(1.0);
            assert_eq!(xz, Vec2::new(10.0, 0.0));
            assert_eq!(height, 0.0);
        }

        #[test]
        fn test_is_finished() {
            let mut projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            assert!(!projectile.is_finished());

            projectile.progress = 1.0;
            assert!(projectile.is_finished());
        }

        #[test]
        fn test_advance_increases_progress() {
            let mut projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            let initial_progress = projectile.progress;
            projectile.advance(0.5);
            assert!(projectile.progress > initial_progress);
        }

        #[test]
        fn test_advance_caps_at_1() {
            let mut projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            projectile.advance(100.0); // Huge delta
            assert_eq!(projectile.progress, 1.0);
        }

        #[test]
        fn test_uses_poison_element_color() {
            let color = poison_cloud_color();
            assert_eq!(color, Element::Poison.color());
            assert_eq!(color, Color::srgb_u8(0, 255, 0));
        }
    }

    mod poison_cloud_zone_tests {
        use super::*;

        #[test]
        fn test_zone_new() {
            let center = Vec2::new(5.0, 5.0);
            let zone = PoisonCloudZone::new(center, 30.0, 4.5);

            assert_eq!(zone.center, center);
            assert_eq!(zone.radius, POISON_CLOUD_ZONE_RADIUS);
            assert_eq!(zone.tick_damage, 4.5);
            assert!(!zone.is_expired());
        }

        #[test]
        fn test_zone_from_projectile() {
            let projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 10.0),
                30.0,
            );
            let zone = PoisonCloudZone::from_projectile(&projectile);

            assert_eq!(zone.center, projectile.target_pos);
            assert_eq!(zone.tick_damage, projectile.tick_damage);
        }

        #[test]
        fn test_zone_is_expired() {
            let mut zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 4.5);
            assert!(!zone.is_expired());

            // Tick past duration
            zone.tick(Duration::from_secs_f32(POISON_CLOUD_ZONE_DURATION + 0.1));
            assert!(zone.is_expired());
        }

        #[test]
        fn test_zone_should_damage() {
            let mut zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 4.5);
            assert!(!zone.should_damage());

            // Tick to first damage
            zone.tick(Duration::from_secs_f32(POISON_CLOUD_TICK_INTERVAL + 0.01));
            assert!(zone.should_damage());
        }

        #[test]
        fn test_zone_can_damage_in_range() {
            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 4.5);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(1.0, 0.0);

            assert!(zone.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_zone_cannot_damage_out_of_range() {
            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 4.5);
            let entity = Entity::from_bits(1);
            let out_of_range_pos = Vec2::new(100.0, 0.0);

            assert!(!zone.can_damage(entity, out_of_range_pos));
        }

        #[test]
        fn test_zone_cannot_damage_already_hit() {
            let mut zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 4.5);
            let entity = Entity::from_bits(1);
            let in_range_pos = Vec2::new(1.0, 0.0);

            zone.mark_hit(entity);
            assert!(!zone.can_damage(entity, in_range_pos));
        }

        #[test]
        fn test_zone_resets_hit_tracking_on_tick() {
            let mut zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 4.5);
            let entity = Entity::from_bits(1);

            zone.mark_hit(entity);
            assert!(zone.hit_this_tick.contains(&entity));

            // Tick to next damage interval
            zone.tick(Duration::from_secs_f32(POISON_CLOUD_TICK_INTERVAL + 0.01));
            assert!(zone.hit_this_tick.is_empty());
        }
    }

    mod projectile_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_projectile_moves_along_arc() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                PoisonCloudProjectile::new(
                    Vec2::new(0.0, 0.0),
                    Vec2::new(10.0, 0.0),
                    30.0,
                ),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            let _ = app.world_mut().run_system_once(poison_cloud_projectile_movement_system);

            let projectile = app.world().get::<PoisonCloudProjectile>(entity).unwrap();
            assert!(projectile.progress > 0.0, "Progress should increase");

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!(transform.translation.x > 0.0, "X should move toward target");
        }

        #[test]
        fn test_projectile_arcs_upward_at_midpoint() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let mut projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );
            projectile.progress = 0.5; // Force to midpoint

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                projectile,
            )).id();

            // Just tick to update transform
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.001));
            }

            let _ = app.world_mut().run_system_once(poison_cloud_projectile_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            // Y should be elevated at midpoint (base 0.5 + arc height)
            assert!(
                transform.translation.y > 0.5 + POISON_CLOUD_ARC_HEIGHT * 0.9,
                "Y should be near max arc height at midpoint, got {}",
                transform.translation.y
            );
        }
    }

    mod spawn_zone_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_zone_spawns_when_projectile_finished() {
            let mut app = App::new();

            // Create finished projectile
            let mut projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 5.0),
                30.0,
            );
            projectile.progress = 1.0;

            let projectile_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.5, 5.0)),
                projectile,
            )).id();

            let _ = app.world_mut().run_system_once(poison_cloud_spawn_zone_system);

            // Projectile should be despawned
            assert!(!app.world().entities().contains(projectile_entity));

            // Zone should exist
            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 1, "One zone should spawn");
        }

        #[test]
        fn test_zone_spawns_at_target_position() {
            let mut app = App::new();

            let mut projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(15.0, 20.0),
                30.0,
            );
            projectile.progress = 1.0;

            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                projectile,
            ));

            let _ = app.world_mut().run_system_once(poison_cloud_spawn_zone_system);

            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            for zone in zone_query.iter(app.world()) {
                assert_eq!(zone.center, Vec2::new(15.0, 20.0));
            }
        }

        #[test]
        fn test_zone_not_spawned_if_projectile_not_finished() {
            let mut app = App::new();

            let projectile = PoisonCloudProjectile::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                30.0,
            );

            let projectile_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                projectile,
            )).id();

            let _ = app.world_mut().run_system_once(poison_cloud_spawn_zone_system);

            // Projectile should still exist
            assert!(app.world().entities().contains(projectile_entity));

            // No zone should exist
            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 0);
        }
    }

    mod damage_system_tests {
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
        fn test_damage_applied_to_enemies_in_zone() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 5.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick - this affects Time resource's delta
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(POISON_CLOUD_TICK_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(poison_cloud_damage_system);

            // Check that damage event was fired (enemy was marked as hit)
            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert!(!zone.hit_this_tick.is_empty(), "Enemy should have been marked as hit");
        }

        #[test]
        fn test_no_damage_outside_zone() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 5.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create enemy far outside range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(100.0, 0.375, 0.0)),
            ));

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(POISON_CLOUD_TICK_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(poison_cloud_damage_system);

            // Check that no damage event was fired (no entities hit)
            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert!(zone.hit_this_tick.is_empty(), "No enemy should have been marked as hit");
        }

        #[test]
        fn test_multiple_enemies_damaged() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 5.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create 3 enemies in range
            for i in 0..3 {
                app.world_mut().spawn((
                    Enemy { speed: 50.0, strength: 10.0 },
                    Transform::from_translation(Vec3::new(i as f32 * 0.5, 0.375, 0.0)),
                ));
            }

            // Advance time to trigger tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(POISON_CLOUD_TICK_INTERVAL + 0.01));
            }

            // Run the system
            let _ = app.world_mut().run_system_once(poison_cloud_damage_system);

            // Check that all 3 enemies were hit
            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            let zone = zone_query.single(app.world()).unwrap();
            assert_eq!(zone.hit_this_tick.len(), 3, "All 3 enemies should have been marked as hit");
        }

        #[test]
        fn test_damage_tick_rate_correct() {
            let mut app = setup_damage_test_app();

            // Create zone at origin
            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 5.0);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            ));

            // Create enemy in range
            let enemy_entity = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(1.0, 0.375, 0.0)),
            )).id();

            // Run 3 tick cycles
            let mut total_hits = 0;
            for _ in 0..3 {
                // Advance time to trigger tick
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(POISON_CLOUD_TICK_INTERVAL + 0.01));
                }

                // Run the system
                let _ = app.world_mut().run_system_once(poison_cloud_damage_system);

                // Count hits and clear hit tracking
                let mut zone_query = app.world_mut().query::<&mut PoisonCloudZone>();
                let mut zone = zone_query.single_mut(app.world_mut()).unwrap();
                if zone.hit_this_tick.contains(&enemy_entity) {
                    total_hits += 1;
                }
                zone.hit_this_tick.clear();
            }

            assert_eq!(total_hits, 3, "Enemy should have been hit 3 times over 3 tick cycles");
        }
    }

    mod cleanup_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_zone_despawns_when_expired() {
            let mut app = App::new();

            let mut zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 5.0);
            zone.duration = Timer::from_seconds(0.0, TimerMode::Once);
            zone.duration.tick(Duration::from_secs(1)); // Force expired

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(poison_cloud_cleanup_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_zone_survives_before_expiry() {
            let mut app = App::new();

            let zone = PoisonCloudZone::new(Vec2::ZERO, 30.0, 5.0);

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                zone,
            )).id();

            let _ = app.world_mut().run_system_once(poison_cloud_cleanup_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod fire_poison_cloud_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_poison_cloud_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PlagueCloud);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_poison_cloud(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PoisonCloudProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_poison_cloud_uses_spell_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PlagueCloud);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_poison_cloud(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PoisonCloudProjectile>();
            for projectile in query.iter(app.world()) {
                assert_eq!(projectile.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_poison_cloud_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PlagueCloud);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_poison_cloud_with_damage(
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

            let mut query = app.world_mut().query::<&PoisonCloudProjectile>();
            for projectile in query.iter(app.world()) {
                assert_eq!(projectile.damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_poison_cloud_clamps_to_max_range() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::PlagueCloud);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(100.0, 0.0); // Far beyond max range

            {
                let mut commands = app.world_mut().commands();
                fire_poison_cloud(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&PoisonCloudProjectile>();
            for projectile in query.iter(app.world()) {
                let distance = projectile.start_pos.distance(projectile.target_pos);
                assert!(
                    distance <= POISON_CLOUD_MAX_RANGE + 0.01,
                    "Distance {} should be clamped to max range {}",
                    distance,
                    POISON_CLOUD_MAX_RANGE
                );
            }
        }

        #[test]
        fn test_multiple_clouds_independent() {
            let mut app = App::new();
            app.init_resource::<Time>();
            app.add_message::<DamageEvent>();
            app.add_systems(Update, poison_cloud_damage_system);

            // Spawn 3 zones at different positions
            for i in 0..3 {
                let center = Vec2::new(i as f32 * 10.0, 0.0);
                app.world_mut().spawn((
                    Transform::from_translation(Vec3::new(center.x, 0.1, center.y)),
                    PoisonCloudZone::new(center, 30.0, 5.0),
                ));
            }

            // Verify 3 zones exist
            let mut zone_query = app.world_mut().query::<&PoisonCloudZone>();
            let count = zone_query.iter(app.world()).count();
            assert_eq!(count, 3);
        }
    }
}
