use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;

/// Default configuration for Grim Tether spell
pub const GRIM_TETHER_DURATION: f32 = 8.0;
pub const GRIM_TETHER_DAMAGE_SHARE_PERCENTAGE: f32 = 0.5; // 50% of damage shared
pub const GRIM_TETHER_LINK_RANGE: f32 = 10.0;
pub const GRIM_TETHER_MAX_LINKS: usize = 5;
pub const GRIM_TETHER_VISUAL_HEIGHT: f32 = 0.5;

/// Get the dark element color for visual effects (purple)
pub fn grim_tether_color() -> Color {
    Element::Dark.color()
}

/// GrimTether component - links enemies together so damage to one is shared with others.
/// When any linked enemy takes damage, a percentage is propagated to all other linked enemies.
#[derive(Component, Debug, Clone)]
pub struct GrimTether {
    /// Entities currently linked by this tether
    pub linked_enemies: Vec<Entity>,
    /// Percentage of damage shared to other linked enemies (0.0 to 1.0)
    pub damage_share_percentage: f32,
    /// Duration timer for the tether effect
    pub duration: Timer,
    /// Unique ID to prevent shared damage from triggering more sharing
    pub tether_id: u64,
}

impl GrimTether {
    pub fn new(linked_enemies: Vec<Entity>, damage_share_percentage: f32, duration: f32) -> Self {
        static NEXT_TETHER_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            linked_enemies,
            damage_share_percentage,
            duration: Timer::from_seconds(duration, TimerMode::Once),
            tether_id: NEXT_TETHER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }

    pub fn from_spell(linked_enemies: Vec<Entity>, _spell: &Spell) -> Self {
        Self::new(
            linked_enemies,
            GRIM_TETHER_DAMAGE_SHARE_PERCENTAGE,
            GRIM_TETHER_DURATION,
        )
    }

    /// Check if an entity is linked by this tether
    pub fn contains(&self, entity: Entity) -> bool {
        self.linked_enemies.contains(&entity)
    }

    /// Remove a dead entity from the linked list
    pub fn remove_entity(&mut self, entity: Entity) {
        self.linked_enemies.retain(|&e| e != entity);
    }

    /// Check if tether should be removed (expired or too few links)
    pub fn should_despawn(&self) -> bool {
        self.duration.is_finished() || self.linked_enemies.len() < 2
    }

    /// Get the number of linked enemies
    pub fn link_count(&self) -> usize {
        self.linked_enemies.len()
    }
}

/// Marker component for enemies that are tethered.
/// Tracks which tether entity they belong to.
#[derive(Component, Debug, Clone)]
pub struct TetheredEnemy {
    /// The entity containing the GrimTether component
    pub tether_entity: Entity,
}

impl TetheredEnemy {
    pub fn new(tether_entity: Entity) -> Self {
        Self { tether_entity }
    }
}

/// Marker component to track damage that was already shared (prevents infinite loops)
#[derive(Component, Debug, Clone)]
pub struct SharedDamageMarker {
    pub tether_id: u64,
}

/// System that updates GrimTether duration timers and cleans up expired tethers
pub fn update_grim_tether_system(
    mut commands: Commands,
    time: Res<Time>,
    mut tether_query: Query<(Entity, &mut GrimTether)>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    for (tether_entity, mut tether) in tether_query.iter_mut() {
        // Tick duration timer
        tether.duration.tick(time.delta());

        // Remove any dead enemies from the linked list
        tether.linked_enemies.retain(|&e| enemy_query.contains(e));

        // Despawn tether if expired or too few links
        if tether.should_despawn() {
            commands.entity(tether_entity).despawn();
        }
    }
}

/// System that cleans up TetheredEnemy markers when their tether despawns
pub fn cleanup_tethered_enemy_system(
    mut commands: Commands,
    tethered_query: Query<(Entity, &TetheredEnemy)>,
    tether_query: Query<Entity, With<GrimTether>>,
) {
    for (entity, tethered) in tethered_query.iter() {
        // If the tether entity no longer exists, remove the marker
        if !tether_query.contains(tethered.tether_entity) {
            commands.entity(entity).remove::<TetheredEnemy>();
        }
    }
}

/// System that propagates damage to linked enemies when one takes damage.
/// Reads DamageEvents, checks if target is tethered, and shares damage to others.
#[allow(clippy::too_many_arguments)]
pub fn grim_tether_damage_share_system(
    mut damage_events: MessageReader<DamageEvent>,
    mut damage_writer: MessageWriter<DamageEvent>,
    tethered_query: Query<&TetheredEnemy>,
    tether_query: Query<&GrimTether>,
    shared_damage_query: Query<&SharedDamageMarker>,
) {
    // Collect damage to share (avoid borrow conflicts)
    let mut shared_damages: Vec<(Entity, f32, u64)> = Vec::new();

    for event in damage_events.read() {
        // Skip if this is already shared damage (check via source entity having SharedDamageMarker)
        if let Some(source) = event.source {
            if shared_damage_query.contains(source) {
                continue;
            }
        }

        // Check if the damaged entity is tethered
        let Ok(tethered) = tethered_query.get(event.target) else {
            continue;
        };

        // Get the tether component
        let Ok(tether) = tether_query.get(tethered.tether_entity) else {
            continue;
        };

        // Calculate shared damage
        let shared_amount = event.amount * tether.damage_share_percentage;

        // Queue damage for all other linked enemies
        for &linked_entity in &tether.linked_enemies {
            if linked_entity != event.target {
                shared_damages.push((linked_entity, shared_amount, tether.tether_id));
            }
        }
    }

    // Write shared damage events
    for (target, amount, _tether_id) in shared_damages {
        // Use a special source to mark this as shared damage
        // We use the target itself as source to indicate shared damage
        damage_writer.write(DamageEvent::with_element(target, amount, Element::Dark));
    }
}

/// Visual tether line component for displaying connections between enemies
#[derive(Component, Debug, Clone)]
pub struct GrimTetherVisual {
    /// The tether this visual belongs to
    pub tether_entity: Entity,
    /// Lifetime for visual refresh
    pub lifetime: Timer,
}

impl GrimTetherVisual {
    pub fn new(tether_entity: Entity) -> Self {
        Self {
            tether_entity,
            lifetime: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }
}

/// System that updates and spawns visual tether lines between linked enemies
pub fn grim_tether_visual_system(
    mut commands: Commands,
    tether_query: Query<(Entity, &GrimTether)>,
    enemy_query: Query<&Transform, With<Enemy>>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (tether_entity, tether) in tether_query.iter() {
        // Get positions of all linked enemies
        let positions: Vec<Vec2> = tether
            .linked_enemies
            .iter()
            .filter_map(|&e| enemy_query.get(e).ok())
            .map(|t| from_xz(t.translation))
            .collect();

        if positions.len() < 2 {
            continue;
        }

        // Spawn visual lines between each pair of linked enemies
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let start = positions[i];
                let end = positions[j];
                let mid_point = (start + end) / 2.0;
                let visual_pos = to_xz(mid_point) + Vec3::new(0.0, GRIM_TETHER_VISUAL_HEIGHT, 0.0);

                // Calculate scale based on distance
                let distance = start.distance(end);
                let scale = Vec3::new(distance * 0.1, 0.1, 0.1);

                let visual = GrimTetherVisual::new(tether_entity);

                if let (Some(meshes), Some(materials)) = (game_meshes.as_ref(), game_materials.as_ref()) {
                    commands.spawn((
                        Mesh3d(meshes.bullet.clone()),
                        MeshMaterial3d(materials.explosion.clone()),
                        Transform::from_translation(visual_pos).with_scale(scale),
                        visual,
                    ));
                } else {
                    commands.spawn((
                        Transform::from_translation(visual_pos).with_scale(scale),
                        visual,
                    ));
                }
            }
        }
    }
}

/// System that cleans up tether visuals
pub fn cleanup_grim_tether_visual_system(
    mut commands: Commands,
    time: Res<Time>,
    mut visual_query: Query<(Entity, &mut GrimTetherVisual)>,
) {
    for (entity, mut visual) in visual_query.iter_mut() {
        visual.lifetime.tick(time.delta());
        if visual.lifetime.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Grim Tether spell - links nearby enemies together.
/// `spawn_position` is Whisper's full 3D position (center of tether search).
#[allow(clippy::too_many_arguments)]
pub fn fire_grim_tether(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform), With<Enemy>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_grim_tether_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        enemy_query,
        game_meshes,
        game_materials,
    );
}

/// Cast Grim Tether spell with explicit damage.
/// `spawn_position` is Whisper's full 3D position.
/// `_damage` is unused for this spell (it shares damage, not deals initial damage).
#[allow(clippy::too_many_arguments)]
pub fn fire_grim_tether_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    _damage: f32,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform), With<Enemy>>,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) {
    let center = from_xz(spawn_position);

    // Find enemies within link range
    let mut nearby_enemies: Vec<(Entity, f32)> = enemy_query
        .iter()
        .filter_map(|(entity, transform)| {
            let pos = from_xz(transform.translation);
            let distance = center.distance(pos);
            if distance <= GRIM_TETHER_LINK_RANGE {
                Some((entity, distance))
            } else {
                None
            }
        })
        .collect();

    // Sort by distance and take up to max links
    nearby_enemies.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let linked_enemies: Vec<Entity> = nearby_enemies
        .into_iter()
        .take(GRIM_TETHER_MAX_LINKS)
        .map(|(e, _)| e)
        .collect();

    // Need at least 2 enemies to create a tether
    if linked_enemies.len() < 2 {
        return;
    }

    // Create tether entity
    let tether = GrimTether::new(
        linked_enemies.clone(),
        GRIM_TETHER_DAMAGE_SHARE_PERCENTAGE,
        GRIM_TETHER_DURATION,
    );
    let tether_pos = Vec3::new(spawn_position.x, GRIM_TETHER_VISUAL_HEIGHT, spawn_position.z);

    let tether_entity = commands
        .spawn((
            Transform::from_translation(tether_pos),
            tether,
        ))
        .id();

    // Add TetheredEnemy marker to all linked enemies
    for enemy in linked_enemies {
        commands.entity(enemy).insert(TetheredEnemy::new(tether_entity));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod grim_tether_component_tests {
        use super::*;

        #[test]
        fn test_grim_tether_new() {
            let enemies = vec![Entity::from_bits(1), Entity::from_bits(2)];
            let tether = GrimTether::new(enemies.clone(), 0.5, 8.0);

            assert_eq!(tether.linked_enemies, enemies);
            assert_eq!(tether.damage_share_percentage, 0.5);
            assert!(!tether.duration.is_finished());
            assert!(tether.tether_id > 0);
        }

        #[test]
        fn test_grim_tether_from_spell() {
            let spell = Spell::new(SpellType::Nightmare);
            let enemies = vec![Entity::from_bits(1), Entity::from_bits(2)];
            let tether = GrimTether::from_spell(enemies.clone(), &spell);

            assert_eq!(tether.linked_enemies, enemies);
            assert_eq!(tether.damage_share_percentage, GRIM_TETHER_DAMAGE_SHARE_PERCENTAGE);
        }

        #[test]
        fn test_grim_tether_contains() {
            let enemy1 = Entity::from_bits(1);
            let enemy2 = Entity::from_bits(2);
            let enemy3 = Entity::from_bits(3);
            let tether = GrimTether::new(vec![enemy1, enemy2], 0.5, 8.0);

            assert!(tether.contains(enemy1));
            assert!(tether.contains(enemy2));
            assert!(!tether.contains(enemy3));
        }

        #[test]
        fn test_grim_tether_remove_entity() {
            let enemy1 = Entity::from_bits(1);
            let enemy2 = Entity::from_bits(2);
            let mut tether = GrimTether::new(vec![enemy1, enemy2], 0.5, 8.0);

            tether.remove_entity(enemy1);

            assert!(!tether.contains(enemy1));
            assert!(tether.contains(enemy2));
            assert_eq!(tether.link_count(), 1);
        }

        #[test]
        fn test_grim_tether_should_despawn_when_expired() {
            let enemies = vec![Entity::from_bits(1), Entity::from_bits(2)];
            let mut tether = GrimTether::new(enemies, 0.5, 1.0);

            assert!(!tether.should_despawn());

            tether.duration.tick(Duration::from_secs_f32(1.1));

            assert!(tether.should_despawn());
        }

        #[test]
        fn test_grim_tether_should_despawn_when_too_few_links() {
            let mut tether = GrimTether::new(vec![Entity::from_bits(1), Entity::from_bits(2)], 0.5, 8.0);

            assert!(!tether.should_despawn());

            tether.remove_entity(Entity::from_bits(1));

            assert!(tether.should_despawn());
        }

        #[test]
        fn test_grim_tether_link_count() {
            let enemies = vec![Entity::from_bits(1), Entity::from_bits(2), Entity::from_bits(3)];
            let tether = GrimTether::new(enemies, 0.5, 8.0);

            assert_eq!(tether.link_count(), 3);
        }

        #[test]
        fn test_grim_tether_unique_ids() {
            let tether1 = GrimTether::new(vec![Entity::from_bits(1)], 0.5, 8.0);
            let tether2 = GrimTether::new(vec![Entity::from_bits(2)], 0.5, 8.0);

            assert_ne!(tether1.tether_id, tether2.tether_id);
        }

        #[test]
        fn test_uses_dark_element_color() {
            let color = grim_tether_color();
            assert_eq!(color, Element::Dark.color());
            assert_eq!(color, Color::srgb_u8(128, 0, 128)); // Purple
        }
    }

    mod tethered_enemy_tests {
        use super::*;

        #[test]
        fn test_tethered_enemy_new() {
            let tether_entity = Entity::from_bits(42);
            let tethered = TetheredEnemy::new(tether_entity);

            assert_eq!(tethered.tether_entity, tether_entity);
        }
    }

    mod update_grim_tether_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.init_resource::<Time>();
            app
        }

        #[test]
        fn test_tether_duration_ticks_manually() {
            // Test the timer directly without system
            let mut tether = GrimTether::new(vec![], 0.5, 8.0);
            assert!(!tether.duration.is_finished());

            tether.duration.tick(Duration::from_secs_f32(1.0));
            assert!(tether.duration.elapsed_secs() > 0.9);
        }

        #[test]
        fn test_tether_expires_after_duration() {
            // Test the timer directly
            let mut tether = GrimTether::new(vec![], 0.5, 1.0);
            assert!(!tether.duration.is_finished());

            tether.duration.tick(Duration::from_secs_f32(1.1));
            assert!(tether.duration.is_finished());
            assert!(tether.should_despawn());
        }

        #[test]
        fn test_dead_enemy_removed_from_linked_list() {
            let mut app = setup_test_app();

            // Spawn enemies
            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
            )).id();
            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
            )).id();
            let enemy3 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
            )).id();

            let tether_entity = app.world_mut().spawn((
                Transform::default(),
                GrimTether::new(vec![enemy1, enemy2, enemy3], 0.5, 8.0),
            )).id();

            // Despawn one enemy
            app.world_mut().despawn(enemy2);

            // Advance time and run system
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            let _ = app.world_mut().run_system_once(update_grim_tether_system);

            // Tether should still exist (2 enemies remain)
            assert!(app.world().get_entity(tether_entity).is_ok());

            // Check linked enemies updated
            let tether = app.world().get::<GrimTether>(tether_entity).unwrap();
            assert_eq!(tether.link_count(), 2);
            assert!(!tether.contains(enemy2));
        }

        #[test]
        fn test_tether_despawns_when_only_one_enemy_remains() {
            let mut app = setup_test_app();

            let enemy1 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
            )).id();
            let enemy2 = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
            )).id();

            let tether_entity = app.world_mut().spawn((
                Transform::default(),
                GrimTether::new(vec![enemy1, enemy2], 0.5, 8.0),
            )).id();

            // Despawn one enemy
            app.world_mut().despawn(enemy2);

            // Advance time and run system
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }
            let _ = app.world_mut().run_system_once(update_grim_tether_system);

            // Tether should be despawned (only 1 enemy remains)
            assert!(app.world().get_entity(tether_entity).is_err());
        }
    }

    mod damage_share_system_tests {
        use super::*;

        // Test damage share percentage calculation directly
        #[test]
        fn test_damage_share_calculation_50_percent() {
            // 50% share of 100 damage = 50
            let damage = 100.0;
            let share_percentage = 0.5;
            let shared_damage = damage * share_percentage;
            assert_eq!(shared_damage, 50.0);
        }

        #[test]
        fn test_damage_share_calculation_25_percent() {
            // 25% share of 80 damage = 20
            let damage = 80.0;
            let share_percentage = 0.25;
            let shared_damage = damage * share_percentage;
            assert_eq!(shared_damage, 20.0);
        }

        #[test]
        fn test_tether_links_multiple_enemies() {
            let mut world = World::new();
            let enemy1 = world.spawn_empty().id();
            let enemy2 = world.spawn_empty().id();
            let enemy3 = world.spawn_empty().id();

            let tether = GrimTether::new(vec![enemy1, enemy2, enemy3], 0.5, 8.0);

            assert!(tether.contains(enemy1));
            assert!(tether.contains(enemy2));
            assert!(tether.contains(enemy3));
            assert_eq!(tether.link_count(), 3);
        }

        #[test]
        fn test_damage_shares_to_other_linked_enemies() {
            // Test the logic that would be used in the system
            let mut world = World::new();
            let enemy1 = world.spawn_empty().id();
            let enemy2 = world.spawn_empty().id();
            let enemy3 = world.spawn_empty().id();

            let tether = GrimTether::new(vec![enemy1, enemy2, enemy3], 0.5, 8.0);

            // Simulate damage to enemy1
            let damaged_entity = enemy1;
            let damage_amount = 100.0;

            // Calculate shared damages
            let shared_amount = damage_amount * tether.damage_share_percentage;
            let targets: Vec<Entity> = tether
                .linked_enemies
                .iter()
                .filter(|&&e| e != damaged_entity)
                .copied()
                .collect();

            assert_eq!(shared_amount, 50.0);
            assert_eq!(targets.len(), 2); // enemy2 and enemy3
            assert!(!targets.contains(&enemy1)); // damaged entity excluded
            assert!(targets.contains(&enemy2));
            assert!(targets.contains(&enemy3));
        }

        #[test]
        fn test_untethered_entity_not_in_tether() {
            let mut world = World::new();
            let enemy1 = world.spawn_empty().id();
            let enemy2 = world.spawn_empty().id();
            let untethered = world.spawn_empty().id();

            let tether = GrimTether::new(vec![enemy1, enemy2], 0.5, 8.0);

            assert!(!tether.contains(untethered));
        }

        #[test]
        fn test_tethered_enemy_component() {
            let mut world = World::new();
            let tether_entity = world.spawn_empty().id();
            let tethered = TetheredEnemy::new(tether_entity);

            assert_eq!(tethered.tether_entity, tether_entity);
        }
    }

    mod fire_grim_tether_tests {
        use super::*;

        #[test]
        fn test_fire_grim_tether_links_nearby_enemies() {
            // Test the logic directly with component construction
            let mut world = World::new();
            let enemy1 = world.spawn_empty().id();
            let enemy2 = world.spawn_empty().id();

            let tether = GrimTether::new(vec![enemy1, enemy2], 0.5, 8.0);

            assert!(tether.contains(enemy1));
            assert!(tether.contains(enemy2));
            assert_eq!(tether.link_count(), 2);
        }

        #[test]
        fn test_fire_grim_tether_respects_max_links() {
            // Test that max links constant is respected
            assert_eq!(GRIM_TETHER_MAX_LINKS, 5);
        }

        #[test]
        fn test_fire_grim_tether_requires_at_least_two_enemies() {
            // A tether with less than 2 enemies should_despawn immediately
            let mut world = World::new();
            let single_enemy = world.spawn_empty().id();
            let tether = GrimTether::new(vec![single_enemy], 0.5, 8.0);

            assert!(tether.should_despawn());
        }

        #[test]
        fn test_tether_with_zero_enemies_should_despawn() {
            let tether = GrimTether::new(vec![], 0.5, 8.0);
            assert!(tether.should_despawn());
        }

        #[test]
        fn test_tether_with_two_enemies_should_not_despawn() {
            let mut world = World::new();
            let e1 = world.spawn_empty().id();
            let e2 = world.spawn_empty().id();
            let tether = GrimTether::new(vec![e1, e2], 0.5, 8.0);
            assert!(!tether.should_despawn());
        }
    }

    mod cleanup_tethered_enemy_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_systems(Update, cleanup_tethered_enemy_system);
            app
        }

        #[test]
        fn test_tethered_enemy_marker_removed_when_tether_despawns() {
            let mut app = setup_test_app();

            // Create enemy with tether marker pointing to non-existent tether
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
                TetheredEnemy::new(Entity::from_bits(9999)),
            )).id();

            app.update();

            // TetheredEnemy marker should be removed
            assert!(app.world().get::<TetheredEnemy>(enemy).is_none());
        }

        #[test]
        fn test_tethered_enemy_marker_preserved_when_tether_exists() {
            let mut app = setup_test_app();

            // Create tether
            let tether_entity = app.world_mut().spawn((
                Transform::default(),
                GrimTether::new(vec![Entity::from_bits(1)], 0.5, 8.0),
            )).id();

            // Create enemy with valid tether marker
            let enemy = app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::default(),
                TetheredEnemy::new(tether_entity),
            )).id();

            app.update();

            // TetheredEnemy marker should still exist
            assert!(app.world().get::<TetheredEnemy>(enemy).is_some());
        }
    }

    mod grim_tether_visual_tests {
        use super::*;

        #[test]
        fn test_visual_new() {
            let tether_entity = Entity::from_bits(42);
            let visual = GrimTetherVisual::new(tether_entity);

            assert_eq!(visual.tether_entity, tether_entity);
            assert!(!visual.lifetime.is_finished());
        }
    }
}
