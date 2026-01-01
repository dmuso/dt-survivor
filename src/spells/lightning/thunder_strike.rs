use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;

/// Default strike delay in seconds (time from target marker to strike)
pub const THUNDER_STRIKE_DELAY: f32 = 0.5;

/// Default area of effect radius in world units
pub const THUNDER_STRIKE_RADIUS: f32 = 3.0;

/// Visual effect lifetime after strike lands
pub const THUNDER_STRIKE_VISUAL_LIFETIME: f32 = 0.3;

/// Get the lightning element color for visual effects (yellow)
pub fn thunder_strike_color() -> Color {
    Element::Lightning.color()
}

/// Target marker component that appears before a thunder strike lands.
/// Shows the player where the strike will occur.
#[derive(Component, Debug, Clone)]
pub struct ThunderStrikeMarker {
    /// Center position on XZ plane where strike will land
    pub position: Vec2,
    /// Radius of the strike area
    pub radius: f32,
    /// Timer counting down to the strike
    pub delay_timer: Timer,
    /// Damage to deal when strike lands
    pub damage: f32,
}

impl ThunderStrikeMarker {
    pub fn new(position: Vec2, damage: f32) -> Self {
        Self {
            position,
            radius: THUNDER_STRIKE_RADIUS,
            delay_timer: Timer::from_seconds(THUNDER_STRIKE_DELAY, TimerMode::Once),
            damage,
        }
    }

    pub fn from_spell(position: Vec2, spell: &Spell) -> Self {
        Self::new(position, spell.damage())
    }

    /// Check if the strike should land (delay timer finished)
    pub fn is_ready(&self) -> bool {
        self.delay_timer.is_finished()
    }
}

/// Thunder strike effect component for the actual lightning bolt.
/// Spawned when ThunderStrikeMarker timer finishes.
#[derive(Component, Debug, Clone)]
pub struct ThunderStrike {
    /// Center position on XZ plane
    pub center: Vec2,
    /// Damage dealt to enemies in area
    pub damage: f32,
    /// Radius of damage area
    pub radius: f32,
    /// Lifetime timer for visual effect
    pub lifetime: Timer,
    /// Whether damage has been applied (only apply once)
    pub damage_applied: bool,
}

impl ThunderStrike {
    pub fn new(center: Vec2, damage: f32, radius: f32) -> Self {
        Self {
            center,
            damage,
            radius,
            lifetime: Timer::from_seconds(THUNDER_STRIKE_VISUAL_LIFETIME, TimerMode::Once),
            damage_applied: false,
        }
    }

    pub fn from_marker(marker: &ThunderStrikeMarker) -> Self {
        Self::new(marker.position, marker.damage, marker.radius)
    }

    /// Check if the visual effect has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// System that updates thunder strike markers and spawns strikes when ready.
pub fn update_thunder_strike_markers(
    mut commands: Commands,
    time: Res<Time>,
    mut marker_query: Query<(Entity, &mut ThunderStrikeMarker)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (entity, mut marker) in marker_query.iter_mut() {
        marker.delay_timer.tick(time.delta());

        if marker.is_ready() {
            // Spawn the thunder strike at marker position
            let strike = ThunderStrike::from_marker(&marker);
            let strike_pos = to_xz(marker.position) + Vec3::new(0.0, 0.2, 0.0);

            if let (Some(ref meshes), Some(ref materials)) = (&game_meshes, &game_materials) {
                commands.spawn((
                    Mesh3d(meshes.explosion.clone()),
                    MeshMaterial3d(materials.thunder_strike.clone()),
                    Transform::from_translation(strike_pos).with_scale(Vec3::splat(strike.radius)),
                    strike,
                ));
            } else {
                // Fallback for tests without mesh resources
                commands.spawn((
                    Transform::from_translation(strike_pos),
                    strike,
                ));
            }

            // Despawn the marker
            commands.entity(entity).despawn();
        }
    }
}

/// System that applies area damage when thunder strike lands.
pub fn thunder_strike_damage_system(
    mut strike_query: Query<&mut ThunderStrike>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut strike in strike_query.iter_mut() {
        if strike.damage_applied {
            continue;
        }

        // Apply damage to all enemies in radius
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = strike.center.distance(enemy_pos);

            if distance <= strike.radius {
                damage_events.write(DamageEvent::new(enemy_entity, strike.damage));
            }
        }

        strike.damage_applied = true;
    }
}

/// System that updates thunder strike lifetime and despawns expired strikes.
pub fn update_thunder_strikes(
    mut commands: Commands,
    time: Res<Time>,
    mut strike_query: Query<(Entity, &mut ThunderStrike, &mut Transform)>,
) {
    for (entity, mut strike, mut transform) in strike_query.iter_mut() {
        strike.lifetime.tick(time.delta());

        // Fade out effect by scaling down
        let progress = strike.lifetime.elapsed_secs() / THUNDER_STRIKE_VISUAL_LIFETIME;
        let scale = strike.radius * (1.0 - progress * 0.5); // Scale from full to half
        transform.scale = Vec3::splat(scale);

        if strike.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast thunder strike spell - spawns a target marker that becomes a strike after delay.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is the target on XZ plane.
#[allow(clippy::too_many_arguments)]
pub fn fire_thunder_strike(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_thunder_strike_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        game_meshes,
        game_materials,
    );
}

/// Cast thunder strike spell with explicit damage - spawns a target marker that becomes a strike after delay.
/// `spawn_position` is Whisper's full 3D position, `target_pos` is the target on XZ plane.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_thunder_strike_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    _spawn_position: Vec3,
    target_pos: Vec2,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let marker = ThunderStrikeMarker::new(target_pos, damage);
    let marker_pos = to_xz(target_pos) + Vec3::new(0.0, 0.1, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.target_marker.clone()),
            MeshMaterial3d(materials.thunder_strike_marker.clone()),
            Transform::from_translation(marker_pos).with_scale(Vec3::splat(THUNDER_STRIKE_RADIUS)),
            marker,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(marker_pos),
            marker,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod thunder_strike_marker_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_marker_creation() {
            let position = Vec2::new(10.0, 20.0);
            let damage = 30.0;
            let marker = ThunderStrikeMarker::new(position, damage);

            assert_eq!(marker.position, position);
            assert_eq!(marker.damage, damage);
            assert_eq!(marker.radius, THUNDER_STRIKE_RADIUS);
            assert!(!marker.is_ready());
        }

        #[test]
        fn test_marker_from_spell() {
            let spell = Spell::new(SpellType::ThunderStrike);
            let position = Vec2::new(5.0, 15.0);
            let marker = ThunderStrikeMarker::from_spell(position, &spell);

            assert_eq!(marker.position, position);
            assert_eq!(marker.damage, spell.damage());
        }

        #[test]
        fn test_marker_is_ready_after_delay() {
            let mut marker = ThunderStrikeMarker::new(Vec2::ZERO, 30.0);

            // Not ready initially
            assert!(!marker.is_ready());

            // Tick past delay
            marker.delay_timer.tick(Duration::from_secs_f32(THUNDER_STRIKE_DELAY + 0.1));

            assert!(marker.is_ready());
        }

        #[test]
        fn test_marker_not_ready_before_delay() {
            let mut marker = ThunderStrikeMarker::new(Vec2::ZERO, 30.0);
            marker.delay_timer.tick(Duration::from_secs_f32(THUNDER_STRIKE_DELAY / 2.0));

            assert!(!marker.is_ready());
        }
    }

    mod thunder_strike_tests {
        use super::*;

        #[test]
        fn test_strike_creation() {
            let center = Vec2::new(10.0, 20.0);
            let damage = 30.0;
            let radius = 5.0;
            let strike = ThunderStrike::new(center, damage, radius);

            assert_eq!(strike.center, center);
            assert_eq!(strike.damage, damage);
            assert_eq!(strike.radius, radius);
            assert!(!strike.damage_applied);
            assert!(!strike.is_expired());
        }

        #[test]
        fn test_strike_from_marker() {
            let marker = ThunderStrikeMarker::new(Vec2::new(5.0, 10.0), 25.0);
            let strike = ThunderStrike::from_marker(&marker);

            assert_eq!(strike.center, marker.position);
            assert_eq!(strike.damage, marker.damage);
            assert_eq!(strike.radius, marker.radius);
        }

        #[test]
        fn test_strike_expires_after_lifetime() {
            let mut strike = ThunderStrike::new(Vec2::ZERO, 30.0, 3.0);

            // Not expired initially
            assert!(!strike.is_expired());

            // Tick past lifetime
            strike.lifetime.tick(Duration::from_secs_f32(THUNDER_STRIKE_VISUAL_LIFETIME + 0.1));

            assert!(strike.is_expired());
        }

        #[test]
        fn test_uses_lightning_element_color() {
            let color = thunder_strike_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }
    }

    mod update_marker_system_tests {
        use super::*;

        #[test]
        fn test_marker_spawns_strike_when_ready() {
            let mut app = App::new();
            app.add_systems(Update, update_thunder_strike_markers);
            app.init_resource::<Time>();

            // Create marker with very short delay
            let mut marker = ThunderStrikeMarker::new(Vec2::new(10.0, 20.0), 30.0);
            marker.delay_timer = Timer::from_seconds(0.01, TimerMode::Once);
            let marker_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.1, 20.0)),
                marker,
            )).id();

            // Advance time past delay
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            app.update();

            // Marker should be despawned (using get_entity pattern for deferred despawn)
            assert!(app.world().get_entity(marker_entity).is_err());

            // Strike should exist
            let mut query = app.world_mut().query::<&ThunderStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_marker_survives_before_delay() {
            let mut app = App::new();
            app.add_systems(Update, update_thunder_strike_markers);
            app.init_resource::<Time>();

            let marker_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ThunderStrikeMarker::new(Vec2::ZERO, 30.0),
            )).id();

            // Advance time but not past delay
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(THUNDER_STRIKE_DELAY / 2.0));
            }

            app.update();

            // Marker should still exist
            assert!(app.world().get_entity(marker_entity).is_ok());

            // No strike yet
            let mut query = app.world_mut().query::<&ThunderStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 0);
        }
    }

    mod thunder_strike_damage_system_tests {
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
            app.add_systems(Update, (thunder_strike_damage_system, count_damage_events).chain());

            // Create strike at origin with radius 5.0
            app.world_mut().spawn(ThunderStrike::new(Vec2::ZERO, 30.0, 5.0));

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
            app.add_systems(Update, (thunder_strike_damage_system, count_damage_events).chain());

            // Create strike at origin with radius 3.0
            app.world_mut().spawn(ThunderStrike::new(Vec2::ZERO, 30.0, 3.0));

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
            app.add_systems(Update, (thunder_strike_damage_system, count_damage_events).chain());

            // Create strike
            app.world_mut().spawn(ThunderStrike::new(Vec2::ZERO, 30.0, 5.0));

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
        fn test_damage_uses_xz_plane_ignores_y() {
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
            app.add_systems(Update, (thunder_strike_damage_system, count_damage_events).chain());

            // Create strike at origin with radius 5.0
            app.world_mut().spawn(ThunderStrike::new(Vec2::ZERO, 30.0, 5.0));

            // Create enemy close on XZ plane but far on Y - should still be hit
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 100.0, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Y distance should be ignored");
        }

        #[test]
        fn test_damage_on_z_axis() {
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
            app.add_systems(Update, (thunder_strike_damage_system, count_damage_events).chain());

            // Create strike at origin
            app.world_mut().spawn(ThunderStrike::new(Vec2::ZERO, 30.0, 5.0));

            // Create enemy on Z axis within radius
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(0.0, 0.375, 4.0)),
            ));

            app.update();

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
            app.add_systems(Update, (thunder_strike_damage_system, count_damage_events).chain());

            // Create strike
            app.world_mut().spawn(ThunderStrike::new(Vec2::ZERO, 30.0, 5.0));

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

    mod update_thunder_strikes_system_tests {
        use super::*;

        #[test]
        fn test_strike_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_thunder_strikes);
            app.init_resource::<Time>();

            let strike_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ThunderStrike::new(Vec2::ZERO, 30.0, 3.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(THUNDER_STRIKE_VISUAL_LIFETIME + 0.1));
            }

            app.update();

            // Strike should be despawned (using get_entity pattern for deferred despawn)
            assert!(app.world().get_entity(strike_entity).is_err());
        }

        #[test]
        fn test_strike_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_thunder_strikes);
            app.init_resource::<Time>();

            let strike_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                ThunderStrike::new(Vec2::ZERO, 30.0, 3.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(THUNDER_STRIKE_VISUAL_LIFETIME / 2.0));
            }

            app.update();

            // Strike should still exist
            assert!(app.world().get_entity(strike_entity).is_ok());
        }
    }

    mod fire_thunder_strike_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_thunder_strike_spawns_marker() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ThunderStrike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 5.0);

            {
                let mut commands = app.world_mut().commands();
                fire_thunder_strike(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            // Should spawn 1 marker
            let mut query = app.world_mut().query::<&ThunderStrikeMarker>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_thunder_strike_marker_at_target_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ThunderStrike);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_thunder_strike(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ThunderStrikeMarker>();
            for marker in query.iter(app.world()) {
                assert_eq!(marker.position, target_pos);
            }
        }

        #[test]
        fn test_fire_thunder_strike_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::ThunderStrike);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_thunder_strike(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ThunderStrikeMarker>();
            for marker in query.iter(app.world()) {
                assert_eq!(marker.damage, expected_damage);
            }
        }
    }
}
