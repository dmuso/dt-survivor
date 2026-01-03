//! Echo Thought spell - Psychic afterimages that repeat the last attack.
//!
//! A Psychic element spell (Hallucination SpellType) that creates ghost copies
//! which replay recent spell casts at reduced damage. Allows for combo potential
//! with other spells.

use bevy::prelude::*;
use crate::element::Element;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::spell::components::Spell;
use crate::spell::SpellType;

/// Default delay before echo spawns (seconds)
pub const ECHO_THOUGHT_DELAY: f32 = 0.5;

/// Default damage multiplier for echoed spells
pub const ECHO_THOUGHT_DAMAGE_MULTIPLIER: f32 = 0.4;

/// Default number of echoes to spawn
pub const ECHO_THOUGHT_DEFAULT_ECHOES: u32 = 2;

/// Duration the echo caster effect lasts (seconds)
pub const ECHO_THOUGHT_DURATION: f32 = 8.0;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn echo_thought_color() -> Color {
    Element::Psychic.color()
}

/// Tracks the last spell cast for echo replay.
#[derive(Resource, Default, Debug, Clone)]
pub struct LastSpellCast {
    /// The type of spell that was last cast
    pub spell_type: Option<SpellType>,
    /// The position from which it was cast
    pub source_position: Option<Vec2>,
    /// The direction it was cast in
    pub target_direction: Option<Vec2>,
    /// The damage it dealt
    pub damage: f32,
}

impl LastSpellCast {
    /// Record a new spell cast.
    pub fn record(&mut self, spell_type: SpellType, source_pos: Vec2, target_dir: Vec2, damage: f32) {
        self.spell_type = Some(spell_type);
        self.source_position = Some(source_pos);
        self.target_direction = Some(target_dir);
        self.damage = damage;
    }

    /// Clear the recorded spell.
    pub fn clear(&mut self) {
        self.spell_type = None;
        self.source_position = None;
        self.target_direction = None;
        self.damage = 0.0;
    }

    /// Check if there is a spell to echo.
    pub fn has_spell(&self) -> bool {
        self.spell_type.is_some()
    }
}

/// Component for the Echo Thought caster entity.
/// Manages echo spawning and timing.
#[derive(Component, Debug, Clone)]
pub struct EchoThoughtCaster {
    /// Number of echoes remaining to spawn
    pub echoes_remaining: u32,
    /// Timer between echo spawns
    pub echo_delay: Timer,
    /// Damage multiplier for echoed spells
    pub damage_multiplier: f32,
    /// Duration the caster effect lasts
    pub duration: Timer,
    /// Unique ID for this activation to prevent double-echo
    pub activation_id: u64,
}

impl Default for EchoThoughtCaster {
    fn default() -> Self {
        Self {
            echoes_remaining: ECHO_THOUGHT_DEFAULT_ECHOES,
            echo_delay: Timer::from_seconds(ECHO_THOUGHT_DELAY, TimerMode::Repeating),
            damage_multiplier: ECHO_THOUGHT_DAMAGE_MULTIPLIER,
            duration: Timer::from_seconds(ECHO_THOUGHT_DURATION, TimerMode::Once),
            activation_id: 0,
        }
    }
}

impl EchoThoughtCaster {
    /// Create a new EchoThoughtCaster with specified number of echoes.
    pub fn with_echoes(echoes: u32) -> Self {
        Self {
            echoes_remaining: echoes,
            ..Default::default()
        }
    }

    /// Create an EchoThoughtCaster from a Spell component.
    pub fn from_spell(spell: &Spell) -> Self {
        // Scale echoes with spell level: 2 base + 1 per 3 levels
        let echoes = 2 + (spell.level / 3);
        Self {
            echoes_remaining: echoes,
            ..Default::default()
        }
    }

    /// Check if the caster has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Check if there are echoes remaining.
    pub fn has_echoes(&self) -> bool {
        self.echoes_remaining > 0
    }
}

/// Component for a spawned echo entity that will replay a spell.
#[derive(Component, Debug, Clone)]
pub struct SpellEcho {
    /// The spell type to replay
    pub spell_to_repeat: SpellType,
    /// Position to cast from
    pub source_position: Vec2,
    /// Direction to cast toward
    pub target_direction: Vec2,
    /// Damage multiplier (usually 0.3-0.5)
    pub damage_multiplier: f32,
    /// Original damage before multiplier
    pub original_damage: f32,
    /// Timer until the echo executes
    pub execute_timer: Timer,
    /// Whether the echo has executed
    pub executed: bool,
}

impl SpellEcho {
    /// Create a new SpellEcho with the given parameters.
    pub fn new(
        spell_type: SpellType,
        source_pos: Vec2,
        target_dir: Vec2,
        damage_multiplier: f32,
        original_damage: f32,
    ) -> Self {
        Self {
            spell_to_repeat: spell_type,
            source_position: source_pos,
            target_direction: target_dir,
            damage_multiplier,
            original_damage,
            execute_timer: Timer::from_seconds(0.1, TimerMode::Once), // Quick delay for visual effect
            executed: false,
        }
    }

    /// Calculate the damage this echo will deal.
    pub fn echo_damage(&self) -> f32 {
        self.original_damage * self.damage_multiplier
    }

    /// Check if the echo is ready to execute.
    pub fn is_ready(&self) -> bool {
        self.execute_timer.is_finished() && !self.executed
    }

    /// Mark the echo as executed.
    pub fn mark_executed(&mut self) {
        self.executed = true;
    }
}

/// Visual marker component for echo entities.
#[derive(Component, Debug, Clone)]
pub struct EchoVisual {
    /// Current opacity (0.0-1.0)
    pub opacity: f32,
    /// Fade speed
    pub fade_rate: f32,
}

impl Default for EchoVisual {
    fn default() -> Self {
        Self {
            opacity: 0.6,
            fade_rate: 0.5,
        }
    }
}

impl EchoVisual {
    /// Update the opacity based on time.
    pub fn update(&mut self, delta: f32) {
        self.opacity = (self.opacity - self.fade_rate * delta).max(0.0);
    }

    /// Check if the visual has faded completely.
    pub fn is_faded(&self) -> bool {
        self.opacity <= 0.0
    }
}

/// System that tracks spell casts for echo replay.
/// This should run after spell_casting_system.
/// Note: The actual spell recording is done in spell_casting_system via LastSpellCast resource.
/// This system is a placeholder for future enhancements.
pub fn track_last_spell_cast_system(
    _last_spell: Res<LastSpellCast>,
    _caster_query: Query<&EchoThoughtCaster>,
) {
    // The actual recording is done in spell_casting_system.
    // This system can be used for future spell tracking enhancements.
}

/// System that spawns echo entities from the caster.
pub fn spawn_echo_thought_system(
    mut commands: Commands,
    time: Res<Time>,
    last_spell: Res<LastSpellCast>,
    mut caster_query: Query<(Entity, &mut EchoThoughtCaster)>,
) {
    for (_entity, mut caster) in caster_query.iter_mut() {
        caster.echo_delay.tick(time.delta());

        if caster.echo_delay.just_finished() && caster.has_echoes() {
            // Only spawn echo if we have a spell to repeat
            if let (Some(spell_type), Some(source_pos), Some(target_dir)) = (
                last_spell.spell_type,
                last_spell.source_position,
                last_spell.target_direction,
            ) {
                // Spawn the echo with slight position offset
                let offset = Vec2::new(
                    (caster.echoes_remaining as f32 * 0.5) - 1.0,
                    0.0,
                );
                let echo_pos = source_pos + offset;

                commands.spawn((
                    SpellEcho::new(
                        spell_type,
                        echo_pos,
                        target_dir,
                        caster.damage_multiplier,
                        last_spell.damage,
                    ),
                    EchoVisual::default(),
                    Transform::from_translation(Vec3::new(echo_pos.x, 0.5, echo_pos.y)),
                ));

                caster.echoes_remaining -= 1;
            }
        }
    }
}

/// System that updates echo timers and marks them for execution.
pub fn update_echo_timers_system(
    time: Res<Time>,
    mut echo_query: Query<&mut SpellEcho>,
) {
    for mut echo in echo_query.iter_mut() {
        echo.execute_timer.tick(time.delta());
    }
}

/// System that executes ready echoes by triggering their spell effects.
pub fn execute_echo_spells_system(
    mut commands: Commands,
    mut echo_query: Query<(Entity, &mut SpellEcho)>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    for (_entity, mut echo) in echo_query.iter_mut() {
        if echo.is_ready() {
            // Execute the echoed spell at reduced damage
            let echo_damage = echo.echo_damage();
            let origin_pos = Vec3::new(echo.source_position.x, 0.5, echo.source_position.y);
            let target_pos = echo.source_position + echo.target_direction * 10.0; // Project direction

            // Fire the echoed spell based on type
            match echo.spell_to_repeat {
                SpellType::Fireball => {
                    crate::spells::fire::fireball::fire_fireball_with_damage(
                        &mut commands,
                        &Spell::new(SpellType::Fireball),
                        echo_damage,
                        origin_pos,
                        target_pos,
                        None,
                        None,
                        None,
                        game_meshes.as_deref(),
                        game_materials.as_deref(),
                        None, // No particle effects for echoed spell
                    );
                }
                SpellType::IceShard => {
                    crate::spells::frost::ice_shard::fire_ice_shard_with_damage(
                        &mut commands,
                        &Spell::new(SpellType::IceShard),
                        echo_damage,
                        origin_pos,
                        target_pos,
                        game_meshes.as_deref(),
                        game_materials.as_deref(),
                    );
                }
                // Add more spell types as needed
                _ => {
                    // For unimplemented spell types, just deal direct damage
                    // (placeholder until all spells are echoed)
                }
            }

            echo.mark_executed();
        }
    }
}

/// System that updates echo visual effects.
pub fn update_echo_visual_system(
    time: Res<Time>,
    mut echo_query: Query<&mut EchoVisual>,
) {
    for mut visual in echo_query.iter_mut() {
        visual.update(time.delta_secs());
    }
}

/// System that cleans up expired casters and executed echoes.
pub fn cleanup_echo_thought_system(
    mut commands: Commands,
    time: Res<Time>,
    mut caster_query: Query<(Entity, &mut EchoThoughtCaster)>,
    echo_query: Query<(Entity, &SpellEcho, &EchoVisual)>,
) {
    // Clean up expired casters
    for (entity, mut caster) in caster_query.iter_mut() {
        caster.duration.tick(time.delta());
        if caster.is_expired() && !caster.has_echoes() {
            commands.entity(entity).despawn();
        }
    }

    // Clean up executed echoes that have faded
    for (entity, echo, visual) in echo_query.iter() {
        if echo.executed && visual.is_faded() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Echo Thought spell - spawns a caster that creates echoes of recent spells.
#[allow(clippy::too_many_arguments)]
pub fn fire_echo_thought(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) {
    fire_echo_thought_with_echoes(
        commands,
        spell,
        ECHO_THOUGHT_DEFAULT_ECHOES,
        spawn_position,
        _game_meshes,
        _game_materials,
    );
}

/// Cast Echo Thought spell with explicit echo count.
#[allow(clippy::too_many_arguments)]
pub fn fire_echo_thought_with_echoes(
    commands: &mut Commands,
    spell: &Spell,
    echoes: u32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let mut caster = EchoThoughtCaster::from_spell(spell);
    caster.echoes_remaining = echoes;
    caster.activation_id = rand::random();

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.psychic_aoe.clone()), // Transparent magenta AOE material
            Transform::from_translation(spawn_position).with_scale(Vec3::splat(0.5)),
            caster,
        ));
    } else {
        commands.spawn((
            Transform::from_translation(spawn_position),
            caster,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod last_spell_cast_tests {
        use super::*;

        #[test]
        fn test_last_spell_cast_default_is_empty() {
            let last = LastSpellCast::default();
            assert!(!last.has_spell());
            assert!(last.spell_type.is_none());
            assert!(last.source_position.is_none());
            assert!(last.target_direction.is_none());
            assert_eq!(last.damage, 0.0);
        }

        #[test]
        fn test_last_spell_cast_record() {
            let mut last = LastSpellCast::default();
            last.record(
                SpellType::Fireball,
                Vec2::new(10.0, 20.0),
                Vec2::new(1.0, 0.0),
                25.0,
            );

            assert!(last.has_spell());
            assert_eq!(last.spell_type, Some(SpellType::Fireball));
            assert_eq!(last.source_position, Some(Vec2::new(10.0, 20.0)));
            assert_eq!(last.target_direction, Some(Vec2::new(1.0, 0.0)));
            assert_eq!(last.damage, 25.0);
        }

        #[test]
        fn test_last_spell_cast_clear() {
            let mut last = LastSpellCast::default();
            last.record(SpellType::Fireball, Vec2::ZERO, Vec2::X, 10.0);
            assert!(last.has_spell());

            last.clear();
            assert!(!last.has_spell());
            assert!(last.spell_type.is_none());
            assert_eq!(last.damage, 0.0);
        }

        #[test]
        fn test_last_spell_cast_overwrites_previous() {
            let mut last = LastSpellCast::default();
            last.record(SpellType::Fireball, Vec2::ZERO, Vec2::X, 10.0);
            last.record(SpellType::IceShard, Vec2::ONE, Vec2::Y, 20.0);

            assert_eq!(last.spell_type, Some(SpellType::IceShard));
            assert_eq!(last.source_position, Some(Vec2::ONE));
            assert_eq!(last.damage, 20.0);
        }
    }

    mod echo_thought_caster_tests {
        use super::*;

        #[test]
        fn test_echo_thought_caster_default() {
            let caster = EchoThoughtCaster::default();
            assert_eq!(caster.echoes_remaining, ECHO_THOUGHT_DEFAULT_ECHOES);
            assert_eq!(caster.damage_multiplier, ECHO_THOUGHT_DAMAGE_MULTIPLIER);
            assert!(!caster.is_expired());
            assert!(caster.has_echoes());
        }

        #[test]
        fn test_echo_thought_caster_with_echoes() {
            let caster = EchoThoughtCaster::with_echoes(5);
            assert_eq!(caster.echoes_remaining, 5);
            assert!(caster.has_echoes());
        }

        #[test]
        fn test_echo_thought_caster_from_spell() {
            let spell = Spell::new(SpellType::Hallucination);
            let caster = EchoThoughtCaster::from_spell(&spell);
            // Level 1: 2 + (1/3) = 2
            assert_eq!(caster.echoes_remaining, 2);
        }

        #[test]
        fn test_echo_thought_caster_from_spell_scales_with_level() {
            let mut spell = Spell::new(SpellType::Hallucination);
            spell.level = 6;
            let caster = EchoThoughtCaster::from_spell(&spell);
            // Level 6: 2 + (6/3) = 4
            assert_eq!(caster.echoes_remaining, 4);
        }

        #[test]
        fn test_echo_thought_caster_expires() {
            let mut caster = EchoThoughtCaster::default();
            assert!(!caster.is_expired());

            caster.duration.tick(Duration::from_secs_f32(ECHO_THOUGHT_DURATION));
            assert!(caster.is_expired());
        }

        #[test]
        fn test_echo_thought_caster_has_no_echoes_when_zero() {
            let mut caster = EchoThoughtCaster::with_echoes(1);
            assert!(caster.has_echoes());

            caster.echoes_remaining = 0;
            assert!(!caster.has_echoes());
        }

        #[test]
        fn test_echo_thought_uses_psychic_color() {
            let color = echo_thought_color();
            assert_eq!(color, Element::Psychic.color());
        }
    }

    mod spell_echo_tests {
        use super::*;

        #[test]
        fn test_spell_echo_new() {
            let echo = SpellEcho::new(
                SpellType::Fireball,
                Vec2::new(5.0, 10.0),
                Vec2::new(1.0, 0.0),
                0.4,
                50.0,
            );

            assert_eq!(echo.spell_to_repeat, SpellType::Fireball);
            assert_eq!(echo.source_position, Vec2::new(5.0, 10.0));
            assert_eq!(echo.target_direction, Vec2::new(1.0, 0.0));
            assert_eq!(echo.damage_multiplier, 0.4);
            assert_eq!(echo.original_damage, 50.0);
            assert!(!echo.executed);
        }

        #[test]
        fn test_spell_echo_damage_calculation() {
            let echo = SpellEcho::new(
                SpellType::Fireball,
                Vec2::ZERO,
                Vec2::X,
                0.4,
                100.0,
            );

            assert_eq!(echo.echo_damage(), 40.0);
        }

        #[test]
        fn test_spell_echo_damage_with_different_multiplier() {
            let echo = SpellEcho::new(
                SpellType::IceShard,
                Vec2::ZERO,
                Vec2::Y,
                0.3,
                80.0,
            );

            assert_eq!(echo.echo_damage(), 24.0);
        }

        #[test]
        fn test_spell_echo_is_ready_after_timer() {
            let mut echo = SpellEcho::new(
                SpellType::Fireball,
                Vec2::ZERO,
                Vec2::X,
                0.4,
                50.0,
            );

            assert!(!echo.is_ready());

            echo.execute_timer.tick(Duration::from_secs_f32(0.2));
            assert!(echo.is_ready());
        }

        #[test]
        fn test_spell_echo_not_ready_after_executed() {
            let mut echo = SpellEcho::new(
                SpellType::Fireball,
                Vec2::ZERO,
                Vec2::X,
                0.4,
                50.0,
            );

            echo.execute_timer.tick(Duration::from_secs_f32(0.2));
            echo.mark_executed();

            assert!(!echo.is_ready());
        }

        #[test]
        fn test_spell_echo_mark_executed() {
            let mut echo = SpellEcho::new(
                SpellType::Fireball,
                Vec2::ZERO,
                Vec2::X,
                0.4,
                50.0,
            );

            assert!(!echo.executed);
            echo.mark_executed();
            assert!(echo.executed);
        }
    }

    mod echo_visual_tests {
        use super::*;

        #[test]
        fn test_echo_visual_default() {
            let visual = EchoVisual::default();
            assert_eq!(visual.opacity, 0.6);
            assert_eq!(visual.fade_rate, 0.5);
            assert!(!visual.is_faded());
        }

        #[test]
        fn test_echo_visual_fades_over_time() {
            let mut visual = EchoVisual::default();
            visual.update(0.5);
            // 0.6 - (0.5 * 0.5) = 0.35
            assert!((visual.opacity - 0.35).abs() < 0.01);
        }

        #[test]
        fn test_echo_visual_is_faded_when_zero() {
            let mut visual = EchoVisual::default();
            visual.opacity = 0.0;
            assert!(visual.is_faded());
        }

        #[test]
        fn test_echo_visual_does_not_go_negative() {
            let mut visual = EchoVisual::default();
            visual.update(10.0); // Very large delta
            assert_eq!(visual.opacity, 0.0);
            assert!(visual.is_faded());
        }
    }

    mod spawn_echo_thought_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.init_resource::<LastSpellCast>();
            app
        }

        #[test]
        fn test_spawn_echo_when_spell_recorded() {
            let mut app = setup_test_app();

            // Record a spell
            let mut last_spell = LastSpellCast::default();
            last_spell.record(SpellType::Fireball, Vec2::new(5.0, 5.0), Vec2::X, 30.0);
            app.insert_resource(last_spell);

            // Create caster with echo delay almost ready
            let mut caster = EchoThoughtCaster::with_echoes(2);
            caster.echo_delay.tick(Duration::from_secs_f32(ECHO_THOUGHT_DELAY - 0.01));
            app.world_mut().spawn((Transform::default(), caster));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);

            // Should have spawned an echo
            let echo_count = app.world_mut().query::<&SpellEcho>().iter(app.world()).count();
            assert_eq!(echo_count, 1, "Should spawn 1 echo");
        }

        #[test]
        fn test_no_echo_when_no_spell_recorded() {
            let mut app = setup_test_app();

            // No spell recorded (default LastSpellCast)
            app.init_resource::<LastSpellCast>();

            // Create caster with echo delay ready
            let mut caster = EchoThoughtCaster::with_echoes(2);
            caster.echo_delay.tick(Duration::from_secs_f32(ECHO_THOUGHT_DELAY - 0.01));
            app.world_mut().spawn((Transform::default(), caster));

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);

            // Should not spawn any echoes
            let echo_count = app.world_mut().query::<&SpellEcho>().iter(app.world()).count();
            assert_eq!(echo_count, 0, "Should not spawn echo when no spell recorded");
        }

        #[test]
        fn test_echoes_remaining_decrements() {
            let mut app = setup_test_app();

            let mut last_spell = LastSpellCast::default();
            last_spell.record(SpellType::Fireball, Vec2::ZERO, Vec2::X, 30.0);
            app.insert_resource(last_spell);

            let mut caster = EchoThoughtCaster::with_echoes(3);
            caster.echo_delay.tick(Duration::from_secs_f32(ECHO_THOUGHT_DELAY - 0.01));
            let caster_entity = app.world_mut().spawn((Transform::default(), caster)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);

            let caster = app.world().get::<EchoThoughtCaster>(caster_entity).unwrap();
            assert_eq!(caster.echoes_remaining, 2, "Echoes remaining should decrement");
        }

        #[test]
        fn test_echo_has_correct_damage_multiplier() {
            let mut app = setup_test_app();

            let mut last_spell = LastSpellCast::default();
            last_spell.record(SpellType::Fireball, Vec2::ZERO, Vec2::X, 100.0);
            app.insert_resource(last_spell);

            let mut caster = EchoThoughtCaster::default();
            caster.echo_delay.tick(Duration::from_secs_f32(ECHO_THOUGHT_DELAY - 0.01));
            app.world_mut().spawn((Transform::default(), caster));

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);

            let mut echo_query = app.world_mut().query::<&SpellEcho>();
            let echo = echo_query.iter(app.world()).next().unwrap();

            assert_eq!(echo.damage_multiplier, ECHO_THOUGHT_DAMAGE_MULTIPLIER);
            assert_eq!(echo.echo_damage(), 100.0 * ECHO_THOUGHT_DAMAGE_MULTIPLIER);
        }

        #[test]
        fn test_echo_records_correct_spell_type() {
            let mut app = setup_test_app();

            let mut last_spell = LastSpellCast::default();
            last_spell.record(SpellType::IceShard, Vec2::ZERO, Vec2::Y, 50.0);
            app.insert_resource(last_spell);

            let mut caster = EchoThoughtCaster::default();
            caster.echo_delay.tick(Duration::from_secs_f32(ECHO_THOUGHT_DELAY - 0.01));
            app.world_mut().spawn((Transform::default(), caster));

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);

            let mut echo_query = app.world_mut().query::<&SpellEcho>();
            let echo = echo_query.iter(app.world()).next().unwrap();

            assert_eq!(echo.spell_to_repeat, SpellType::IceShard);
        }

        #[test]
        fn test_multiple_echoes_spawn_sequentially() {
            let mut app = setup_test_app();

            let mut last_spell = LastSpellCast::default();
            last_spell.record(SpellType::Fireball, Vec2::ZERO, Vec2::X, 30.0);
            app.insert_resource(last_spell);

            // Short delay for fast spawning
            let caster = EchoThoughtCaster::with_echoes(3);
            app.world_mut().spawn((Transform::default(), caster));

            // First tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ECHO_THOUGHT_DELAY + 0.01));
            }
            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);
            let count1 = app.world_mut().query::<&SpellEcho>().iter(app.world()).count();
            assert_eq!(count1, 1);

            // Second tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(ECHO_THOUGHT_DELAY + 0.01));
            }
            let _ = app.world_mut().run_system_once(spawn_echo_thought_system);
            let count2 = app.world_mut().query::<&SpellEcho>().iter(app.world()).count();
            assert_eq!(count2, 2);
        }
    }

    mod cleanup_echo_thought_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_cleanup_expired_caster_with_no_echoes() {
            let mut app = setup_test_app();

            let mut caster = EchoThoughtCaster::with_echoes(0);
            caster.duration.tick(Duration::from_secs_f32(ECHO_THOUGHT_DURATION));
            let entity = app.world_mut().spawn((Transform::default(), caster)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(cleanup_echo_thought_system);

            assert!(app.world().get_entity(entity).is_err(), "Caster should be despawned");
        }

        #[test]
        fn test_caster_not_cleaned_up_while_echoes_remaining() {
            let mut app = setup_test_app();

            let mut caster = EchoThoughtCaster::with_echoes(1);
            caster.duration.tick(Duration::from_secs_f32(ECHO_THOUGHT_DURATION));
            let entity = app.world_mut().spawn((Transform::default(), caster)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(cleanup_echo_thought_system);

            assert!(app.world().get_entity(entity).is_ok(), "Caster should remain while echoes pending");
        }

        #[test]
        fn test_cleanup_executed_faded_echo() {
            let mut app = setup_test_app();

            let mut echo = SpellEcho::new(SpellType::Fireball, Vec2::ZERO, Vec2::X, 0.4, 50.0);
            echo.executed = true;
            let mut visual = EchoVisual::default();
            visual.opacity = 0.0;

            let entity = app.world_mut().spawn((Transform::default(), echo, visual)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(cleanup_echo_thought_system);

            assert!(app.world().get_entity(entity).is_err(), "Executed faded echo should be despawned");
        }

        #[test]
        fn test_echo_not_cleaned_up_while_visible() {
            let mut app = setup_test_app();

            let mut echo = SpellEcho::new(SpellType::Fireball, Vec2::ZERO, Vec2::X, 0.4, 50.0);
            echo.executed = true;
            let visual = EchoVisual::default(); // opacity 0.6

            let entity = app.world_mut().spawn((Transform::default(), echo, visual)).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.01));
            }

            let _ = app.world_mut().run_system_once(cleanup_echo_thought_system);

            assert!(app.world().get_entity(entity).is_ok(), "Echo should remain while visible");
        }
    }

    mod fire_echo_thought_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_echo_thought_spawns_caster() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Hallucination);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_echo_thought(&mut commands, &spell, spawn_pos, None, None);
            }
            app.update();

            let count = app.world_mut().query::<&EchoThoughtCaster>().iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn 1 caster");
        }

        #[test]
        fn test_fire_echo_thought_caster_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Hallucination);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_echo_thought(&mut commands, &spell, spawn_pos, None, None);
            }
            app.update();

            let mut query = app.world_mut().query::<(&EchoThoughtCaster, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation.x, 15.0);
                assert_eq!(transform.translation.z, 25.0);
            }
        }

        #[test]
        fn test_fire_echo_thought_with_explicit_echoes() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Hallucination);

            {
                let mut commands = app.world_mut().commands();
                fire_echo_thought_with_echoes(&mut commands, &spell, 5, Vec3::ZERO, None, None);
            }
            app.update();

            let mut query = app.world_mut().query::<&EchoThoughtCaster>();
            let caster = query.iter(app.world()).next().unwrap();
            assert_eq!(caster.echoes_remaining, 5);
        }
    }
}
