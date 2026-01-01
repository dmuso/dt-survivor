//! Judgment spell (Light) - Periodic divine energy strikes that target enemies from above.
//!
//! Creates a caster that periodically selects enemies within range and calls down
//! vertical beams of holy light that strike after a short delay.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::{from_xz, to_xz};
use crate::spell::components::Spell;

/// Default targeting radius in world units
pub const JUDGMENT_TARGET_RANGE: f32 = 15.0;

/// Default delay between targeting and beam hitting
pub const JUDGMENT_STRIKE_DELAY: f32 = 0.4;

/// Visual beam lifetime after strike lands
pub const JUDGMENT_BEAM_LIFETIME: f32 = 0.3;

/// Beam visual width
pub const JUDGMENT_BEAM_WIDTH: f32 = 0.5;

/// Beam visual height (how tall the beam appears)
pub const JUDGMENT_BEAM_HEIGHT: f32 = 10.0;

/// Get the light element color for visual effects (white/gold)
pub fn judgment_color() -> Color {
    Element::Light.color()
}

/// JudgmentCaster component - attached to player/Whisper to periodically trigger strikes.
/// The caster selects enemies in range and spawns JudgmentStrike markers.
#[derive(Component, Debug, Clone)]
pub struct JudgmentCaster {
    /// Timer controlling strike frequency
    pub strike_timer: Timer,
    /// Range within which enemies can be targeted
    pub target_range: f32,
    /// Damage per strike
    pub damage: f32,
    /// Delay between targeting and beam hitting
    pub strike_delay: f32,
}

impl JudgmentCaster {
    pub fn new(damage: f32, fire_rate: f32) -> Self {
        Self {
            strike_timer: Timer::from_seconds(fire_rate, TimerMode::Repeating),
            target_range: JUDGMENT_TARGET_RANGE,
            damage,
            strike_delay: JUDGMENT_STRIKE_DELAY,
        }
    }

    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(spell.damage(), spell.effective_fire_rate())
    }

    /// Check if the caster should select a new target
    pub fn is_ready(&self) -> bool {
        self.strike_timer.just_finished()
    }
}

/// JudgmentStrike component - marks an enemy for an incoming beam strike.
/// After the delay expires, a JudgmentBeam spawns at the target position.
#[derive(Component, Debug, Clone)]
pub struct JudgmentStrike {
    /// Position where the beam will strike (XZ plane)
    pub target_position: Vec2,
    /// Timer counting down to the beam
    pub delay: Timer,
    /// Damage when beam hits
    pub damage: f32,
}

impl JudgmentStrike {
    pub fn new(target_position: Vec2, damage: f32, delay: f32) -> Self {
        Self {
            target_position,
            delay: Timer::from_seconds(delay, TimerMode::Once),
            damage,
        }
    }

    /// Check if the strike should spawn a beam (delay finished)
    pub fn is_ready(&self) -> bool {
        self.delay.is_finished()
    }
}

/// JudgmentBeam component - the actual beam of light that descends on enemies.
#[derive(Component, Debug, Clone)]
pub struct JudgmentBeam {
    /// Position on XZ plane
    pub position: Vec2,
    /// Damage dealt
    pub damage: f32,
    /// Lifetime timer for visual effect
    pub lifetime: Timer,
    /// Whether damage has been applied
    pub damage_applied: bool,
}

impl JudgmentBeam {
    pub fn new(position: Vec2, damage: f32) -> Self {
        Self {
            position,
            damage,
            lifetime: Timer::from_seconds(JUDGMENT_BEAM_LIFETIME, TimerMode::Once),
            damage_applied: false,
        }
    }

    pub fn from_strike(strike: &JudgmentStrike) -> Self {
        Self::new(strike.target_position, strike.damage)
    }

    /// Check if the visual effect has expired
    pub fn is_expired(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// System that updates JudgmentCaster and selects targets for strikes.
pub fn judgment_caster_system(
    mut commands: Commands,
    time: Res<Time>,
    mut caster_query: Query<(&mut JudgmentCaster, &Transform)>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (mut caster, caster_transform) in caster_query.iter_mut() {
        caster.strike_timer.tick(time.delta());

        if !caster.is_ready() {
            continue;
        }

        let caster_pos = from_xz(caster_transform.translation);

        // Find nearest enemy within range
        let mut nearest_enemy: Option<(Entity, Vec2, f32)> = None;
        for (entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = caster_pos.distance(enemy_pos);

            if distance <= caster.target_range {
                match &nearest_enemy {
                    Some((_, _, nearest_dist)) if distance < *nearest_dist => {
                        nearest_enemy = Some((entity, enemy_pos, distance));
                    }
                    None => {
                        nearest_enemy = Some((entity, enemy_pos, distance));
                    }
                    _ => {}
                }
            }
        }

        // If we found a target, spawn a strike marker
        if let Some((_, target_pos, _)) = nearest_enemy {
            let strike = JudgmentStrike::new(target_pos, caster.damage, caster.strike_delay);
            let marker_pos = to_xz(target_pos) + Vec3::new(0.0, 0.1, 0.0);

            if let (Some(ref meshes), Some(ref materials)) = (&game_meshes, &game_materials) {
                commands.spawn((
                    Mesh3d(meshes.target_marker.clone()),
                    MeshMaterial3d(materials.radiant_beam.clone()),
                    Transform::from_translation(marker_pos).with_scale(Vec3::splat(1.0)),
                    strike,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(marker_pos),
                    strike,
                ));
            }
        }
    }
}

/// System that updates JudgmentStrike delays and spawns beams when ready.
pub fn update_judgment_strikes(
    mut commands: Commands,
    time: Res<Time>,
    mut strike_query: Query<(Entity, &mut JudgmentStrike)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (entity, mut strike) in strike_query.iter_mut() {
        strike.delay.tick(time.delta());

        if strike.is_ready() {
            // Spawn the beam at strike position
            let beam = JudgmentBeam::from_strike(&strike);
            let beam_pos = to_xz(strike.target_position) + Vec3::new(0.0, JUDGMENT_BEAM_HEIGHT / 2.0, 0.0);

            if let (Some(ref meshes), Some(ref materials)) = (&game_meshes, &game_materials) {
                commands.spawn((
                    Mesh3d(meshes.laser.clone()),
                    MeshMaterial3d(materials.radiant_beam.clone()),
                    Transform::from_translation(beam_pos)
                        .with_scale(Vec3::new(JUDGMENT_BEAM_WIDTH, JUDGMENT_BEAM_HEIGHT, JUDGMENT_BEAM_WIDTH)),
                    beam,
                ));
            } else {
                commands.spawn((
                    Transform::from_translation(beam_pos),
                    beam,
                ));
            }

            // Despawn the strike marker
            commands.entity(entity).despawn();
        }
    }
}

/// System that applies damage when JudgmentBeam spawns.
pub fn judgment_beam_damage_system(
    mut beam_query: Query<&mut JudgmentBeam>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut beam in beam_query.iter_mut() {
        if beam.damage_applied {
            continue;
        }

        // Apply damage to enemies at the beam position (small radius for precision)
        let hit_radius = JUDGMENT_BEAM_WIDTH;
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = beam.position.distance(enemy_pos);

            if distance <= hit_radius {
                damage_events.write(DamageEvent::new(enemy_entity, beam.damage));
            }
        }

        beam.damage_applied = true;
    }
}

/// System that updates JudgmentBeam lifetime and despawns expired beams.
pub fn update_judgment_beams(
    mut commands: Commands,
    time: Res<Time>,
    mut beam_query: Query<(Entity, &mut JudgmentBeam, &mut Transform)>,
) {
    for (entity, mut beam, mut transform) in beam_query.iter_mut() {
        beam.lifetime.tick(time.delta());

        // Fade out effect by scaling down width
        let progress = beam.lifetime.elapsed_secs() / JUDGMENT_BEAM_LIFETIME;
        let width_scale = JUDGMENT_BEAM_WIDTH * (1.0 - progress * 0.7);
        transform.scale = Vec3::new(width_scale, JUDGMENT_BEAM_HEIGHT, width_scale);

        if beam.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Judgment spell - spawns a JudgmentCaster that periodically strikes enemies.
/// `spawn_position` is Whisper's full 3D position.
#[allow(clippy::too_many_arguments)]
pub fn fire_judgment(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_judgment_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Judgment spell with explicit damage.
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_judgment_with_damage(
    commands: &mut Commands,
    spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) {
    let mut caster = JudgmentCaster::from_spell(spell);
    caster.damage = damage;

    commands.spawn((
        Transform::from_translation(spawn_position),
        caster,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod judgment_caster_tests {
        use super::*;

        #[test]
        fn test_caster_creation() {
            let damage = 40.0;
            let fire_rate = 5.0;
            let caster = JudgmentCaster::new(damage, fire_rate);

            assert_eq!(caster.damage, damage);
            assert_eq!(caster.target_range, JUDGMENT_TARGET_RANGE);
            assert_eq!(caster.strike_delay, JUDGMENT_STRIKE_DELAY);
            assert!(!caster.is_ready());
        }

        #[test]
        fn test_caster_from_spell() {
            let spell = Spell::new(SpellType::Judgment);
            let caster = JudgmentCaster::from_spell(&spell);

            assert_eq!(caster.damage, spell.damage());
            assert_eq!(caster.target_range, JUDGMENT_TARGET_RANGE);
        }

        #[test]
        fn test_caster_is_ready_after_timer() {
            let mut caster = JudgmentCaster::new(40.0, 0.1);

            // Not ready initially
            assert!(!caster.is_ready());

            // Tick past timer
            caster.strike_timer.tick(Duration::from_secs_f32(0.2));

            assert!(caster.is_ready());
        }

        #[test]
        fn test_caster_timer_repeats() {
            let mut caster = JudgmentCaster::new(40.0, 0.1);

            caster.strike_timer.tick(Duration::from_secs_f32(0.15));
            assert!(caster.is_ready());

            // After just_finished, is_ready returns false until next tick
            caster.strike_timer.tick(Duration::from_secs_f32(0.15));
            assert!(caster.is_ready());
        }

        #[test]
        fn test_uses_light_element_color() {
            let color = judgment_color();
            assert_eq!(color, Element::Light.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 255)); // White
        }
    }

    mod judgment_strike_tests {
        use super::*;

        #[test]
        fn test_strike_creation() {
            let position = Vec2::new(10.0, 20.0);
            let damage = 40.0;
            let delay = 0.5;
            let strike = JudgmentStrike::new(position, damage, delay);

            assert_eq!(strike.target_position, position);
            assert_eq!(strike.damage, damage);
            assert!(!strike.is_ready());
        }

        #[test]
        fn test_strike_is_ready_after_delay() {
            let mut strike = JudgmentStrike::new(Vec2::ZERO, 40.0, 0.1);

            assert!(!strike.is_ready());

            strike.delay.tick(Duration::from_secs_f32(0.2));

            assert!(strike.is_ready());
        }
    }

    mod judgment_beam_tests {
        use super::*;

        #[test]
        fn test_beam_creation() {
            let position = Vec2::new(10.0, 20.0);
            let damage = 40.0;
            let beam = JudgmentBeam::new(position, damage);

            assert_eq!(beam.position, position);
            assert_eq!(beam.damage, damage);
            assert!(!beam.damage_applied);
            assert!(!beam.is_expired());
        }

        #[test]
        fn test_beam_from_strike() {
            let strike = JudgmentStrike::new(Vec2::new(5.0, 10.0), 50.0, 0.5);
            let beam = JudgmentBeam::from_strike(&strike);

            assert_eq!(beam.position, strike.target_position);
            assert_eq!(beam.damage, strike.damage);
        }

        #[test]
        fn test_beam_expires_after_lifetime() {
            let mut beam = JudgmentBeam::new(Vec2::ZERO, 40.0);

            assert!(!beam.is_expired());

            beam.lifetime.tick(Duration::from_secs_f32(JUDGMENT_BEAM_LIFETIME + 0.1));

            assert!(beam.is_expired());
        }
    }

    mod judgment_caster_system_tests {
        use super::*;

        #[test]
        fn test_caster_spawns_strike_on_enemy_in_range() {
            let mut app = App::new();
            app.add_systems(Update, judgment_caster_system);
            app.init_resource::<Time>();

            // Create caster with short timer
            let mut caster = JudgmentCaster::new(40.0, 0.01);
            caster.target_range = 10.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create enemy in range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            app.update();

            // Should spawn strike
            let mut query = app.world_mut().query::<&JudgmentStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_caster_no_strike_when_no_enemies_in_range() {
            let mut app = App::new();
            app.add_systems(Update, judgment_caster_system);
            app.init_resource::<Time>();

            // Create caster with short timer
            let mut caster = JudgmentCaster::new(40.0, 0.01);
            caster.target_range = 10.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create enemy outside range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(20.0, 0.375, 0.0)),
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            app.update();

            // Should not spawn strike
            let mut query = app.world_mut().query::<&JudgmentStrike>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 0);
        }

        #[test]
        fn test_caster_selects_nearest_enemy() {
            let mut app = App::new();
            app.add_systems(Update, judgment_caster_system);
            app.init_resource::<Time>();

            // Create caster with short timer
            let mut caster = JudgmentCaster::new(40.0, 0.01);
            caster.target_range = 20.0;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Create two enemies - one closer than the other
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
            ));
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(3.0, 0.375, 0.0)),
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            app.update();

            // Should spawn strike at nearest enemy (3.0, 0.0)
            let mut query = app.world_mut().query::<&JudgmentStrike>();
            let strikes: Vec<&JudgmentStrike> = query.iter(app.world()).collect();
            assert_eq!(strikes.len(), 1);
            assert!((strikes[0].target_position.x - 3.0).abs() < 0.01);
        }

        #[test]
        fn test_no_strike_when_no_enemies() {
            let mut app = App::new();
            app.add_systems(Update, judgment_caster_system);
            app.init_resource::<Time>();

            // Create caster only, no enemies
            let caster = JudgmentCaster::new(40.0, 0.01);
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                caster,
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            app.update();

            // No strikes spawned
            let mut query = app.world_mut().query::<&JudgmentStrike>();
            assert_eq!(query.iter(app.world()).count(), 0);
        }
    }

    mod update_judgment_strikes_tests {
        use super::*;

        #[test]
        fn test_strike_spawns_beam_when_ready() {
            let mut app = App::new();
            app.add_systems(Update, update_judgment_strikes);
            app.init_resource::<Time>();

            // Create strike with short delay
            let strike = JudgmentStrike::new(Vec2::new(10.0, 20.0), 40.0, 0.01);
            let strike_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.1, 20.0)),
                strike,
            )).id();

            // Advance time past delay
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            app.update();

            // Strike should be despawned
            assert!(app.world().get_entity(strike_entity).is_err());

            // Beam should exist
            let mut query = app.world_mut().query::<&JudgmentBeam>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_strike_survives_before_delay() {
            let mut app = App::new();
            app.add_systems(Update, update_judgment_strikes);
            app.init_resource::<Time>();

            let strike_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                JudgmentStrike::new(Vec2::ZERO, 40.0, 1.0),
            )).id();

            // Advance time but not past delay
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            app.update();

            // Strike should still exist
            assert!(app.world().get_entity(strike_entity).is_ok());

            // No beam yet
            let mut query = app.world_mut().query::<&JudgmentBeam>();
            assert_eq!(query.iter(app.world()).count(), 0);
        }
    }

    mod judgment_beam_damage_system_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_beam_damages_enemy_at_position() {
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
            app.add_systems(Update, (judgment_beam_damage_system, count_damage_events).chain());

            // Create beam at position
            app.world_mut().spawn(JudgmentBeam::new(Vec2::new(5.0, 10.0), 40.0));

            // Create enemy at same position
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 10.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_beam_no_damage_to_distant_enemy() {
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
            app.add_systems(Update, (judgment_beam_damage_system, count_damage_events).chain());

            // Create beam at position
            app.world_mut().spawn(JudgmentBeam::new(Vec2::ZERO, 40.0));

            // Create enemy far away
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(20.0, 0.375, 0.0)),
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_beam_damage_applied_only_once() {
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
            app.add_systems(Update, (judgment_beam_damage_system, count_damage_events).chain());

            // Create beam
            app.world_mut().spawn(JudgmentBeam::new(Vec2::ZERO, 40.0));

            // Create enemy at position
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::ZERO),
            ));

            // Multiple updates
            app.update();
            app.update();
            app.update();

            // Only damaged once
            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }
    }

    mod update_judgment_beams_tests {
        use super::*;

        #[test]
        fn test_beam_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_judgment_beams);
            app.init_resource::<Time>();

            let beam_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                JudgmentBeam::new(Vec2::ZERO, 40.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(JUDGMENT_BEAM_LIFETIME + 0.1));
            }

            app.update();

            assert!(app.world().get_entity(beam_entity).is_err());
        }

        #[test]
        fn test_beam_survives_before_lifetime() {
            let mut app = App::new();
            app.add_systems(Update, update_judgment_beams);
            app.init_resource::<Time>();

            let beam_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                JudgmentBeam::new(Vec2::ZERO, 40.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(JUDGMENT_BEAM_LIFETIME / 2.0));
            }

            app.update();

            assert!(app.world().get_entity(beam_entity).is_ok());
        }
    }

    mod fire_judgment_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_judgment_spawns_caster() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Judgment);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_judgment(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&JudgmentCaster>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_judgment_caster_at_spawn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Judgment);
            let spawn_pos = Vec3::new(10.0, 2.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_judgment(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&JudgmentCaster, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation, spawn_pos);
            }
        }

        #[test]
        fn test_fire_judgment_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Judgment);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::ZERO;

            {
                let mut commands = app.world_mut().commands();
                fire_judgment(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&JudgmentCaster>();
            for caster in query.iter(app.world()) {
                assert_eq!(caster.damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_judgment_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Judgment);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::ZERO;

            {
                let mut commands = app.world_mut().commands();
                fire_judgment_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&JudgmentCaster>();
            for caster in query.iter(app.world()) {
                assert_eq!(caster.damage, explicit_damage);
            }
        }
    }
}
