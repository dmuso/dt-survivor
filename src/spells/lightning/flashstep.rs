use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::movement::components::{from_xz, to_xz};
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default teleport distance in world units
pub const FLASHSTEP_DISTANCE: f32 = 8.0;

/// Default lightning burst radius
pub const FLASHSTEP_BURST_RADIUS: f32 = 4.0;

/// Visual effect lifetime for bursts
pub const FLASHSTEP_BURST_LIFETIME: f32 = 0.3;

/// Get the lightning element color for visual effects (yellow)
pub fn flashstep_color() -> Color {
    Element::Lightning.color()
}

/// Lightning burst effect that spawns at origin or destination of Flashstep.
/// Deals AoE damage to enemies within radius.
#[derive(Component, Debug, Clone)]
pub struct LightningBurst {
    /// Center position on XZ plane where burst occurs
    pub position: Vec2,
    /// Radius of damage area
    pub radius: f32,
    /// Damage dealt to enemies in area
    pub damage: f32,
    /// Lifetime timer for visual effect
    pub lifetime: Timer,
    /// Whether damage has been applied (only apply once)
    pub damage_applied: bool,
}

impl LightningBurst {
    pub fn new(position: Vec2, radius: f32, damage: f32) -> Self {
        Self {
            position,
            radius,
            damage,
            lifetime: Timer::from_seconds(FLASHSTEP_BURST_LIFETIME, TimerMode::Once),
            damage_applied: false,
        }
    }

    /// Check if the visual effect has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// Marker component for entities performing a flashstep teleport.
/// Attach to player to trigger teleportation on next frame.
#[derive(Component, Debug, Clone)]
pub struct FlashstepTeleport {
    /// Target position for teleport on XZ plane
    pub destination: Vec2,
    /// Original position on XZ plane (for origin burst)
    pub origin: Vec2,
    /// Damage for lightning bursts
    pub burst_damage: f32,
}

impl FlashstepTeleport {
    pub fn new(origin: Vec2, destination: Vec2, burst_damage: f32) -> Self {
        Self {
            origin,
            destination,
            burst_damage,
        }
    }
}

/// System that applies lightning burst damage to nearby enemies.
pub fn lightning_burst_damage_system(
    mut burst_query: Query<&mut LightningBurst>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut burst in burst_query.iter_mut() {
        if burst.damage_applied {
            continue;
        }

        // Apply damage to all enemies in radius
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = burst.position.distance(enemy_pos);

            if distance <= burst.radius {
                damage_events.write(DamageEvent::new(enemy_entity, burst.damage));
            }
        }

        burst.damage_applied = true;
    }
}

/// System that updates lightning burst lifetime and despawns expired bursts.
pub fn update_lightning_bursts(
    mut commands: Commands,
    time: Res<Time>,
    mut burst_query: Query<(Entity, &mut LightningBurst, &mut Transform)>,
) {
    for (entity, mut burst, mut transform) in burst_query.iter_mut() {
        burst.lifetime.tick(time.delta());

        // Fade out effect by scaling down
        let progress = burst.lifetime.elapsed_secs() / FLASHSTEP_BURST_LIFETIME;
        let scale = burst.radius * (1.0 - progress * 0.5);
        transform.scale = Vec3::splat(scale);

        if burst.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that executes flashstep teleports for players with FlashstepTeleport component.
/// Teleports player to destination and spawns lightning bursts at origin and destination.
pub fn execute_flashstep_system(
    mut commands: Commands,
    mut player_query: Query<(Entity, &mut Transform, &FlashstepTeleport), With<Player>>,
) {
    for (entity, mut transform, flashstep) in player_query.iter_mut() {
        // Store current Y position (keep height)
        let player_y = transform.translation.y;

        // Teleport player to destination
        let new_pos = to_xz(flashstep.destination);
        transform.translation = Vec3::new(new_pos.x, player_y, new_pos.z);

        // Spawn lightning burst at origin
        let origin_pos = to_xz(flashstep.origin) + Vec3::new(0.0, 0.2, 0.0);
        commands.spawn((
            Transform::from_translation(origin_pos).with_scale(Vec3::splat(FLASHSTEP_BURST_RADIUS)),
            LightningBurst::new(
                flashstep.origin,
                FLASHSTEP_BURST_RADIUS,
                flashstep.burst_damage,
            ),
        ));

        // Spawn lightning burst at destination
        let dest_pos = to_xz(flashstep.destination) + Vec3::new(0.0, 0.2, 0.0);
        commands.spawn((
            Transform::from_translation(dest_pos).with_scale(Vec3::splat(FLASHSTEP_BURST_RADIUS)),
            LightningBurst::new(
                flashstep.destination,
                FLASHSTEP_BURST_RADIUS,
                flashstep.burst_damage,
            ),
        ));

        // Remove FlashstepTeleport component after execution
        commands.entity(entity).remove::<FlashstepTeleport>();
    }
}

/// Cast flashstep spell - queues a teleport for the player.
/// `player_entity` is the player to teleport.
/// `origin_pos` is the current player position in 3D.
/// `direction` is the normalized direction to teleport (on XZ plane).
/// `damage` is the pre-calculated final damage for lightning bursts.
#[allow(clippy::too_many_arguments)]
pub fn fire_flashstep_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    player_entity: Entity,
    origin_pos: Vec3,
    direction: Vec2,
) {
    let origin_xz = from_xz(origin_pos);

    // Calculate destination (clamp direction to unit vector for safety)
    let direction_normalized = if direction.length() > 0.0 {
        direction.normalize()
    } else {
        Vec2::X // Default to +X if no direction
    };

    let destination = origin_xz + direction_normalized * FLASHSTEP_DISTANCE;

    // Add FlashstepTeleport component to player
    commands.entity(player_entity).insert(FlashstepTeleport::new(
        origin_xz,
        destination,
        damage,
    ));
}

/// Cast flashstep spell using spell damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_flashstep(
    commands: &mut Commands,
    spell: &Spell,
    player_entity: Entity,
    origin_pos: Vec3,
    direction: Vec2,
) {
    fire_flashstep_with_damage(
        commands,
        spell,
        spell.damage(),
        player_entity,
        origin_pos,
        direction,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Health;
    use crate::spell::SpellType;
    use std::time::Duration;

    mod lightning_burst_tests {
        use super::*;

        #[test]
        fn test_burst_creation() {
            let position = Vec2::new(10.0, 20.0);
            let radius = 5.0;
            let damage = 30.0;
            let burst = LightningBurst::new(position, radius, damage);

            assert_eq!(burst.position, position);
            assert_eq!(burst.radius, radius);
            assert_eq!(burst.damage, damage);
            assert!(!burst.damage_applied);
            assert!(!burst.is_expired());
        }

        #[test]
        fn test_burst_expires_after_lifetime() {
            let mut burst = LightningBurst::new(Vec2::ZERO, 3.0, 30.0);

            assert!(!burst.is_expired());

            burst.lifetime.tick(Duration::from_secs_f32(FLASHSTEP_BURST_LIFETIME + 0.1));

            assert!(burst.is_expired());
        }

        #[test]
        fn test_uses_lightning_element_color() {
            let color = flashstep_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }
    }

    mod flashstep_teleport_tests {
        use super::*;

        #[test]
        fn test_teleport_creation() {
            let origin = Vec2::new(0.0, 0.0);
            let destination = Vec2::new(8.0, 0.0);
            let damage = 25.0;
            let teleport = FlashstepTeleport::new(origin, destination, damage);

            assert_eq!(teleport.origin, origin);
            assert_eq!(teleport.destination, destination);
            assert_eq!(teleport.burst_damage, damage);
        }
    }

    mod execute_flashstep_system_tests {
        use super::*;

        #[test]
        fn test_flashstep_teleports_player_to_destination() {
            let mut app = App::new();
            app.add_systems(Update, execute_flashstep_system);

            // Create player with flashstep teleport queued
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlashstepTeleport::new(Vec2::ZERO, Vec2::new(8.0, 0.0), 30.0),
            )).id();

            app.update();

            // Player should be at destination
            let transform = app.world().get::<Transform>(player_entity).unwrap();
            assert_eq!(transform.translation.x, 8.0, "Player X should be at destination");
            assert_eq!(transform.translation.y, 0.5, "Player Y should be preserved");
            assert_eq!(transform.translation.z, 0.0, "Player Z should be at destination");
        }

        #[test]
        fn test_flashstep_removes_teleport_component() {
            let mut app = App::new();
            app.add_systems(Update, execute_flashstep_system);

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlashstepTeleport::new(Vec2::ZERO, Vec2::new(8.0, 0.0), 30.0),
            )).id();

            app.update();

            // FlashstepTeleport should be removed
            assert!(
                app.world().get::<FlashstepTeleport>(player_entity).is_none(),
                "FlashstepTeleport should be removed after execution"
            );
        }

        #[test]
        fn test_flashstep_spawns_origin_burst() {
            let mut app = App::new();
            app.add_systems(Update, execute_flashstep_system);

            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(5.0, 0.5, 10.0)),
                FlashstepTeleport::new(Vec2::new(5.0, 10.0), Vec2::new(13.0, 10.0), 30.0),
            ));

            app.update();

            // Find bursts
            let mut burst_query = app.world_mut().query::<&LightningBurst>();
            let bursts: Vec<_> = burst_query.iter(app.world()).collect();
            assert_eq!(bursts.len(), 2, "Should spawn 2 bursts (origin and destination)");

            // Verify one burst is at origin
            let has_origin_burst = bursts.iter().any(|b| b.position == Vec2::new(5.0, 10.0));
            assert!(has_origin_burst, "Should have burst at origin position");
        }

        #[test]
        fn test_flashstep_spawns_destination_burst() {
            let mut app = App::new();
            app.add_systems(Update, execute_flashstep_system);

            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(5.0, 0.5, 10.0)),
                FlashstepTeleport::new(Vec2::new(5.0, 10.0), Vec2::new(13.0, 10.0), 30.0),
            ));

            app.update();

            // Find bursts
            let mut burst_query = app.world_mut().query::<&LightningBurst>();
            let bursts: Vec<_> = burst_query.iter(app.world()).collect();
            assert_eq!(bursts.len(), 2, "Should spawn 2 bursts (origin and destination)");

            // Verify one burst is at destination
            let has_dest_burst = bursts.iter().any(|b| b.position == Vec2::new(13.0, 10.0));
            assert!(has_dest_burst, "Should have burst at destination position");
        }

        #[test]
        fn test_flashstep_preserves_player_y_position() {
            let mut app = App::new();
            app.add_systems(Update, execute_flashstep_system);

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 2.5, 0.0)), // Y = 2.5
                FlashstepTeleport::new(Vec2::ZERO, Vec2::new(8.0, 4.0), 30.0),
            )).id();

            app.update();

            let transform = app.world().get::<Transform>(player_entity).unwrap();
            assert_eq!(transform.translation.y, 2.5, "Player Y should be preserved during teleport");
        }

        #[test]
        fn test_flashstep_burst_has_correct_damage() {
            let mut app = App::new();
            app.add_systems(Update, execute_flashstep_system);

            let expected_damage = 45.0;
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FlashstepTeleport::new(Vec2::ZERO, Vec2::new(8.0, 0.0), expected_damage),
            ));

            app.update();

            // All bursts should have correct damage
            let mut burst_query = app.world_mut().query::<&LightningBurst>();
            for burst in burst_query.iter(app.world()) {
                assert_eq!(burst.damage, expected_damage);
            }
        }
    }

    mod lightning_burst_damage_system_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_damage_applied_to_enemies_in_radius() {
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
            app.add_systems(Update, (lightning_burst_damage_system, count_damage_events).chain());

            // Create burst at origin with radius 5.0
            app.world_mut().spawn(LightningBurst::new(Vec2::ZERO, 5.0, 30.0));

            // Create enemy within radius (XZ distance = 3)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_no_damage_outside_radius() {
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
            app.add_systems(Update, (lightning_burst_damage_system, count_damage_events).chain());

            // Create burst at origin with radius 3.0
            app.world_mut().spawn(LightningBurst::new(Vec2::ZERO, 3.0, 30.0));

            // Create enemy outside radius (XZ distance = 5)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_damage_applied_only_once() {
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
            app.add_systems(Update, (lightning_burst_damage_system, count_damage_events).chain());

            // Create burst
            app.world_mut().spawn(LightningBurst::new(Vec2::ZERO, 5.0, 30.0));

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
        fn test_multiple_enemies_in_radius() {
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
            app.add_systems(Update, (lightning_burst_damage_system, count_damage_events).chain());

            // Create burst
            app.world_mut().spawn(LightningBurst::new(Vec2::ZERO, 5.0, 30.0));

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
    }

    mod update_lightning_bursts_system_tests {
        use super::*;

        #[test]
        fn test_burst_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_lightning_bursts);
            app.init_resource::<Time>();

            let burst_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                LightningBurst::new(Vec2::ZERO, 3.0, 30.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(FLASHSTEP_BURST_LIFETIME + 0.1));
            }

            app.update();

            // Burst should be despawned
            assert!(app.world().get_entity(burst_entity).is_err());
        }

        #[test]
        fn test_burst_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_lightning_bursts);
            app.init_resource::<Time>();

            let burst_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                LightningBurst::new(Vec2::ZERO, 3.0, 30.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(FLASHSTEP_BURST_LIFETIME / 2.0));
            }

            app.update();

            // Burst should still exist
            assert!(app.world().get_entity(burst_entity).is_ok());
        }
    }

    mod fire_flashstep_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_flashstep_adds_teleport_component() {
            let mut app = setup_test_app();

            // Create player
            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            let spell = Spell::new(SpellType::ThunderStrike); // Using existing spell type for test
            let origin_pos = Vec3::new(0.0, 0.5, 0.0);
            let direction = Vec2::new(1.0, 0.0);
            let damage = 25.0;

            {
                let mut commands = app.world_mut().commands();
                fire_flashstep_with_damage(
                    &mut commands,
                    &spell,
                    damage,
                    player_entity,
                    origin_pos,
                    direction,
                );
            }
            app.update();

            // Player should have FlashstepTeleport component
            let teleport = app.world().get::<FlashstepTeleport>(player_entity);
            assert!(teleport.is_some(), "Player should have FlashstepTeleport component");

            let teleport = teleport.unwrap();
            assert_eq!(teleport.origin, Vec2::ZERO);
            assert_eq!(teleport.burst_damage, damage);
        }

        #[test]
        fn test_fire_flashstep_calculates_destination() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(5.0, 0.5, 10.0)),
            )).id();

            let spell = Spell::new(SpellType::ThunderStrike);
            let origin_pos = Vec3::new(5.0, 0.5, 10.0);
            let direction = Vec2::new(1.0, 0.0); // +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_flashstep_with_damage(
                    &mut commands,
                    &spell,
                    25.0,
                    player_entity,
                    origin_pos,
                    direction,
                );
            }
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // Origin at (5.0, 10.0) + direction (1, 0) * 8.0 = (13.0, 10.0)
            assert_eq!(teleport.destination, Vec2::new(5.0 + FLASHSTEP_DISTANCE, 10.0));
        }

        #[test]
        fn test_fire_flashstep_normalizes_direction() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            let spell = Spell::new(SpellType::ThunderStrike);
            let origin_pos = Vec3::new(0.0, 0.5, 0.0);
            // Non-unit direction vector
            let direction = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_flashstep_with_damage(
                    &mut commands,
                    &spell,
                    25.0,
                    player_entity,
                    origin_pos,
                    direction,
                );
            }
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // Should normalize to (1, 0) * 8.0 = (8.0, 0.0)
            assert!((teleport.destination.x - FLASHSTEP_DISTANCE).abs() < 0.01);
            assert!((teleport.destination.y - 0.0).abs() < 0.01);
        }

        #[test]
        fn test_fire_flashstep_diagonal_direction() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            let spell = Spell::new(SpellType::ThunderStrike);
            let origin_pos = Vec3::new(0.0, 0.5, 0.0);
            // Diagonal direction
            let direction = Vec2::new(1.0, 1.0);

            {
                let mut commands = app.world_mut().commands();
                fire_flashstep_with_damage(
                    &mut commands,
                    &spell,
                    25.0,
                    player_entity,
                    origin_pos,
                    direction,
                );
            }
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // Direction normalized: (1/sqrt(2), 1/sqrt(2)) * 8.0
            let expected_offset = FLASHSTEP_DISTANCE / 2.0_f32.sqrt();
            assert!((teleport.destination.x - expected_offset).abs() < 0.01);
            assert!((teleport.destination.y - expected_offset).abs() < 0.01);
        }

        #[test]
        fn test_fire_flashstep_zero_direction_defaults_to_x() {
            let mut app = setup_test_app();

            let player_entity = app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Health::new(100.0),
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            )).id();

            let spell = Spell::new(SpellType::ThunderStrike);
            let origin_pos = Vec3::new(0.0, 0.5, 0.0);
            // Zero direction
            let direction = Vec2::ZERO;

            {
                let mut commands = app.world_mut().commands();
                fire_flashstep_with_damage(
                    &mut commands,
                    &spell,
                    25.0,
                    player_entity,
                    origin_pos,
                    direction,
                );
            }
            app.update();

            let teleport = app.world().get::<FlashstepTeleport>(player_entity).unwrap();
            // Should default to +X direction
            assert_eq!(teleport.destination, Vec2::new(FLASHSTEP_DISTANCE, 0.0));
        }
    }
}
