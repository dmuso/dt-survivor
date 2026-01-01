//! Brainburn spell - Damage over time that increases as enemies stay close.
//!
//! A Psychic element spell (Confusion SpellType) that creates an aura around
//! the player. Enemies within the aura take damage over time that ramps up the
//! longer they remain inside, accumulating stacks that increase damage.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default radius of the Brainburn aura
pub const BRAINBURN_DEFAULT_RADIUS: f32 = 5.0;

/// Default time between stack applications (seconds)
pub const BRAINBURN_STACK_INTERVAL: f32 = 0.5;

/// Default base damage per stack per tick
pub const BRAINBURN_DAMAGE_PER_STACK: f32 = 3.0;

/// Default maximum stacks an enemy can accumulate
pub const BRAINBURN_MAX_STACKS: u32 = 10;

/// Default time for stacks to decay after leaving aura (seconds)
pub const BRAINBURN_STACK_DECAY_INTERVAL: f32 = 1.0;

/// Duration the aura persists (seconds)
pub const BRAINBURN_AURA_DURATION: f32 = 8.0;

/// Height of the visual effect above ground
pub const BRAINBURN_VISUAL_HEIGHT: f32 = 0.2;

/// Get the psychic element color for visual effects (pink/magenta)
pub fn brainburn_color() -> Color {
    Element::Psychic.color()
}

/// Marker component for the Brainburn aura entity.
/// The aura follows the player and affects enemies within its radius.
#[derive(Component, Debug, Clone)]
pub struct BrainburnAura {
    /// Radius of the aura effect
    pub radius: f32,
    /// Damage dealt per stack per damage tick
    pub damage_per_stack: f32,
    /// Timer for applying new stacks to enemies in range
    pub stack_timer: Timer,
    /// Timer for dealing damage to enemies with stacks
    pub damage_timer: Timer,
    /// Maximum stacks an enemy can have
    pub max_stacks: u32,
    /// Duration the aura lasts
    pub duration: Timer,
}

impl Default for BrainburnAura {
    fn default() -> Self {
        Self {
            radius: BRAINBURN_DEFAULT_RADIUS,
            damage_per_stack: BRAINBURN_DAMAGE_PER_STACK,
            stack_timer: Timer::from_seconds(BRAINBURN_STACK_INTERVAL, TimerMode::Repeating),
            damage_timer: Timer::from_seconds(BRAINBURN_STACK_INTERVAL, TimerMode::Repeating),
            max_stacks: BRAINBURN_MAX_STACKS,
            duration: Timer::from_seconds(BRAINBURN_AURA_DURATION, TimerMode::Once),
        }
    }
}

impl BrainburnAura {
    /// Create a new BrainburnAura with custom damage value.
    pub fn with_damage(damage_per_stack: f32) -> Self {
        Self {
            damage_per_stack,
            ..Default::default()
        }
    }

    /// Create a BrainburnAura from a Spell component.
    pub fn from_spell(spell: &Spell) -> Self {
        // Scale damage with spell's calculated damage
        Self::with_damage(spell.damage() * 0.1) // 10% of spell damage per stack
    }

    /// Check if the aura has expired.
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }
}

/// Component attached to enemies affected by Brainburn.
/// Tracks the number of stacks and handles stack decay.
#[derive(Component, Debug, Clone)]
pub struct BrainburnStack {
    /// Current number of stacks on this enemy
    pub stacks: u32,
    /// Maximum stacks this enemy can have
    pub max_stacks: u32,
    /// Timer for stack decay when outside aura
    pub decay_timer: Timer,
    /// Whether the enemy is currently inside an aura
    pub in_aura: bool,
}

impl Default for BrainburnStack {
    fn default() -> Self {
        Self {
            stacks: 0,
            max_stacks: BRAINBURN_MAX_STACKS,
            decay_timer: Timer::from_seconds(BRAINBURN_STACK_DECAY_INTERVAL, TimerMode::Repeating),
            in_aura: false,
        }
    }
}

impl BrainburnStack {
    /// Create a new BrainburnStack with custom max stacks.
    pub fn with_max_stacks(max_stacks: u32) -> Self {
        Self {
            max_stacks,
            ..Default::default()
        }
    }

    /// Add a stack, respecting the maximum.
    pub fn add_stack(&mut self) {
        if self.stacks < self.max_stacks {
            self.stacks += 1;
        }
    }

    /// Remove a stack (decay).
    pub fn decay_stack(&mut self) {
        if self.stacks > 0 {
            self.stacks -= 1;
        }
    }

    /// Check if all stacks have decayed.
    pub fn is_empty(&self) -> bool {
        self.stacks == 0
    }

    /// Calculate damage multiplier based on current stacks.
    pub fn damage_multiplier(&self) -> f32 {
        self.stacks as f32
    }
}

/// System that spawns the Brainburn aura and makes it follow the player.
pub fn spawn_brainburn_aura_system(
    mut commands: Commands,
    query: Query<(Entity, &BrainburnAura), Added<BrainburnAura>>,
) {
    for (entity, _aura) in query.iter() {
        // The aura entity already exists, just ensure it has required components
        commands.entity(entity).insert(Transform::default());
    }
}

/// System that updates the Brainburn aura position to follow the player.
pub fn update_brainburn_aura_position_system(
    player_query: Query<&Transform, (With<crate::player::components::Player>, Without<BrainburnAura>)>,
    mut aura_query: Query<&mut Transform, With<BrainburnAura>>,
) {
    if let Ok(player_transform) = player_query.single() {
        for mut aura_transform in aura_query.iter_mut() {
            aura_transform.translation = Vec3::new(
                player_transform.translation.x,
                BRAINBURN_VISUAL_HEIGHT,
                player_transform.translation.z,
            );
        }
    }
}

/// System that applies stacks to enemies within the aura radius.
pub fn apply_brainburn_stacks_system(
    mut commands: Commands,
    time: Res<Time>,
    mut aura_query: Query<(&Transform, &mut BrainburnAura)>,
    mut enemy_query: Query<(Entity, &Transform, Option<&mut BrainburnStack>), With<Enemy>>,
) {
    for (aura_transform, mut aura) in aura_query.iter_mut() {
        aura.stack_timer.tick(time.delta());

        let aura_pos = from_xz(aura_transform.translation);

        for (enemy_entity, enemy_transform, stack_opt) in enemy_query.iter_mut() {
            let enemy_pos = from_xz(enemy_transform.translation);
            let distance = aura_pos.distance(enemy_pos);
            let in_aura = distance <= aura.radius;

            match stack_opt {
                Some(mut stack) => {
                    stack.in_aura = in_aura;
                    if in_aura && aura.stack_timer.just_finished() {
                        stack.add_stack();
                    }
                }
                None => {
                    if in_aura {
                        // First time entering aura - add BrainburnStack component
                        let mut stack = BrainburnStack::with_max_stacks(aura.max_stacks);
                        stack.in_aura = true;
                        if aura.stack_timer.just_finished() {
                            stack.add_stack();
                        }
                        commands.entity(enemy_entity).insert(stack);
                    }
                }
            }
        }
    }
}

/// System that applies damage to enemies based on their stack count.
pub fn tick_brainburn_damage_system(
    time: Res<Time>,
    mut aura_query: Query<&mut BrainburnAura>,
    stack_query: Query<(Entity, &BrainburnStack), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for mut aura in aura_query.iter_mut() {
        aura.damage_timer.tick(time.delta());

        if aura.damage_timer.just_finished() {
            for (enemy_entity, stack) in stack_query.iter() {
                if stack.stacks > 0 {
                    let damage = aura.damage_per_stack * stack.damage_multiplier();
                    damage_events.write(DamageEvent::new(enemy_entity, damage));
                }
            }
        }
    }
}

/// System that decays stacks when enemies are outside the aura.
pub fn decay_brainburn_stacks_system(
    mut commands: Commands,
    time: Res<Time>,
    mut stack_query: Query<(Entity, &mut BrainburnStack), With<Enemy>>,
) {
    for (entity, mut stack) in stack_query.iter_mut() {
        if !stack.in_aura {
            stack.decay_timer.tick(time.delta());
            if stack.decay_timer.just_finished() {
                stack.decay_stack();
                if stack.is_empty() {
                    commands.entity(entity).remove::<BrainburnStack>();
                }
            }
        } else {
            // Reset decay timer when in aura
            stack.decay_timer.reset();
        }
    }
}

/// System that ticks the aura duration and cleans up expired auras.
pub fn update_brainburn_aura_duration_system(
    mut commands: Commands,
    time: Res<Time>,
    mut aura_query: Query<(Entity, &mut BrainburnAura)>,
) {
    for (entity, mut aura) in aura_query.iter_mut() {
        aura.duration.tick(time.delta());
        if aura.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that cleans up BrainburnStack components when enemies die.
pub fn cleanup_brainburn_on_death_system(
    mut commands: Commands,
    mut removed_enemies: RemovedComponents<Enemy>,
    stack_query: Query<Entity, With<BrainburnStack>>,
) {
    for entity in removed_enemies.read() {
        if stack_query.get(entity).is_ok() {
            commands.entity(entity).remove::<BrainburnStack>();
        }
    }
}

/// Cast Brainburn spell - spawns an aura around the player that applies stacking DOT.
#[allow(clippy::too_many_arguments)]
pub fn fire_brainburn(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) {
    fire_brainburn_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        _game_meshes,
        _game_materials,
    );
}

/// Cast Brainburn spell with explicit damage value.
#[allow(clippy::too_many_arguments)]
pub fn fire_brainburn_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let aura = BrainburnAura::with_damage(damage * 0.1);
    let aura_pos = Vec3::new(spawn_position.x, BRAINBURN_VISUAL_HEIGHT, spawn_position.z);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.powerup.clone()),
            Transform::from_translation(aura_pos).with_scale(Vec3::splat(aura.radius)),
            aura,
        ));
    } else {
        commands.spawn((
            Transform::from_translation(aura_pos),
            aura,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::spell::SpellType;

    mod brainburn_aura_tests {
        use super::*;

        #[test]
        fn test_brainburn_aura_default() {
            let aura = BrainburnAura::default();
            assert_eq!(aura.radius, BRAINBURN_DEFAULT_RADIUS);
            assert_eq!(aura.damage_per_stack, BRAINBURN_DAMAGE_PER_STACK);
            assert_eq!(aura.max_stacks, BRAINBURN_MAX_STACKS);
            assert!(!aura.is_expired());
        }

        #[test]
        fn test_brainburn_aura_with_damage() {
            let aura = BrainburnAura::with_damage(10.0);
            assert_eq!(aura.damage_per_stack, 10.0);
            assert_eq!(aura.radius, BRAINBURN_DEFAULT_RADIUS);
        }

        #[test]
        fn test_brainburn_aura_from_spell() {
            let spell = Spell::new(SpellType::Confusion);
            let aura = BrainburnAura::from_spell(&spell);
            // Damage per stack should be 10% of spell damage
            let expected_damage = spell.damage() * 0.1;
            assert!((aura.damage_per_stack - expected_damage).abs() < 0.01);
        }

        #[test]
        fn test_brainburn_aura_expires() {
            let mut aura = BrainburnAura::default();
            assert!(!aura.is_expired());

            aura.duration.tick(Duration::from_secs_f32(BRAINBURN_AURA_DURATION));
            assert!(aura.is_expired());
        }

        #[test]
        fn test_brainburn_uses_psychic_element_color() {
            let color = brainburn_color();
            assert_eq!(color, Element::Psychic.color());
        }
    }

    mod brainburn_stack_tests {
        use super::*;

        #[test]
        fn test_brainburn_stack_default() {
            let stack = BrainburnStack::default();
            assert_eq!(stack.stacks, 0);
            assert_eq!(stack.max_stacks, BRAINBURN_MAX_STACKS);
            assert!(!stack.in_aura);
            assert!(stack.is_empty());
        }

        #[test]
        fn test_brainburn_stack_with_max_stacks() {
            let stack = BrainburnStack::with_max_stacks(5);
            assert_eq!(stack.max_stacks, 5);
            assert_eq!(stack.stacks, 0);
        }

        #[test]
        fn test_brainburn_stack_add_stack() {
            let mut stack = BrainburnStack::default();
            stack.add_stack();
            assert_eq!(stack.stacks, 1);
            assert!(!stack.is_empty());
        }

        #[test]
        fn test_brainburn_stack_respects_max_stacks() {
            let mut stack = BrainburnStack::with_max_stacks(3);
            stack.add_stack();
            stack.add_stack();
            stack.add_stack();
            stack.add_stack(); // Should not exceed max
            assert_eq!(stack.stacks, 3);
        }

        #[test]
        fn test_brainburn_stack_decay_stack() {
            let mut stack = BrainburnStack::default();
            stack.stacks = 5;
            stack.decay_stack();
            assert_eq!(stack.stacks, 4);
        }

        #[test]
        fn test_brainburn_stack_decay_does_not_go_negative() {
            let mut stack = BrainburnStack::default();
            stack.stacks = 0;
            stack.decay_stack();
            assert_eq!(stack.stacks, 0);
        }

        #[test]
        fn test_brainburn_stack_damage_multiplier() {
            let mut stack = BrainburnStack::default();
            assert_eq!(stack.damage_multiplier(), 0.0);

            stack.stacks = 5;
            assert_eq!(stack.damage_multiplier(), 5.0);

            stack.stacks = 10;
            assert_eq!(stack.damage_multiplier(), 10.0);
        }

        #[test]
        fn test_brainburn_stack_is_empty() {
            let mut stack = BrainburnStack::default();
            assert!(stack.is_empty());

            stack.add_stack();
            assert!(!stack.is_empty());

            stack.decay_stack();
            assert!(stack.is_empty());
        }
    }

    mod apply_brainburn_stacks_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_brainburn_applies_stacks_to_enemies_in_range() {
            let mut app = setup_test_app();

            // Create aura at origin with stack timer ready to fire
            let mut aura = BrainburnAura::default();
            aura.stack_timer.tick(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL - 0.01));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                aura,
            ));

            // Create enemy within radius
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
            )).id();

            // Advance time to trigger stack application
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(apply_brainburn_stacks_system);

            // Enemy should have BrainburnStack component with 1 stack
            let stack = app.world().get::<BrainburnStack>(enemy);
            assert!(stack.is_some(), "Enemy should have BrainburnStack component");
            assert_eq!(stack.unwrap().stacks, 1);
        }

        #[test]
        fn test_brainburn_does_not_affect_enemies_outside_radius() {
            let mut app = setup_test_app();

            // Create aura at origin
            let mut aura = BrainburnAura::default();
            aura.stack_timer.tick(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL - 0.01));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                aura,
            ));

            // Create enemy outside radius
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(100.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(apply_brainburn_stacks_system);

            // Enemy should NOT have BrainburnStack component
            assert!(
                app.world().get::<BrainburnStack>(enemy).is_none(),
                "Enemy outside radius should not have BrainburnStack"
            );
        }

        #[test]
        fn test_brainburn_stacks_accumulate_over_time() {
            let mut app = setup_test_app();

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BrainburnAura::default(),
            ));

            // Create enemy within radius with existing stacks
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                BrainburnStack::default(),
            )).id();

            // Tick multiple times
            for _ in 0..3 {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL + 0.01));
                drop(time);
                let _ = app.world_mut().run_system_once(apply_brainburn_stacks_system);
            }

            let stack = app.world().get::<BrainburnStack>(enemy).unwrap();
            assert!(stack.stacks >= 3, "Stacks should accumulate: got {}", stack.stacks);
        }

        #[test]
        fn test_brainburn_max_stacks_limit() {
            let mut app = setup_test_app();

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BrainburnAura::default(),
            ));

            // Create enemy within radius with stacks near max
            let mut stack = BrainburnStack::with_max_stacks(3);
            stack.stacks = 3;
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack,
            )).id();

            // Tick to try adding more stacks
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(apply_brainburn_stacks_system);

            let stack = app.world().get::<BrainburnStack>(enemy).unwrap();
            assert_eq!(stack.stacks, 3, "Stacks should not exceed max");
        }

        #[test]
        fn test_brainburn_multiple_enemies_tracked_independently() {
            let mut app = setup_test_app();

            // Create aura at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BrainburnAura::default(),
            ));

            // Create two enemies in range with different initial stack counts
            let mut stack1 = BrainburnStack::default();
            stack1.stacks = 2;
            let enemy1 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack1,
            )).id();

            let mut stack2 = BrainburnStack::default();
            stack2.stacks = 5;
            let enemy2 = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(-2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack2,
            )).id();

            // Tick once
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(apply_brainburn_stacks_system);

            // Each enemy should have their own stack count incremented
            let stack1 = app.world().get::<BrainburnStack>(enemy1).unwrap();
            let stack2 = app.world().get::<BrainburnStack>(enemy2).unwrap();

            assert_eq!(stack1.stacks, 3, "Enemy 1 should have 3 stacks");
            assert_eq!(stack2.stacks, 6, "Enemy 2 should have 6 stacks");
        }
    }

    mod tick_brainburn_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_brainburn_damage_scales_with_stacks() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();

            // Create aura with damage timer that will fire on next tick
            let mut aura = BrainburnAura::with_damage(5.0);
            // Pre-tick to near completion, then final tick will trigger
            aura.damage_timer.tick(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL - 0.01));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                aura,
            ));

            // Create enemy with 3 stacks
            let mut stack = BrainburnStack::default();
            stack.stacks = 3;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack,
            ));

            // Advance time for the system to use
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(tick_brainburn_damage_system);

            // Read damage events
            let mut damages: Vec<f32> = Vec::new();
            let events = app.world_mut().resource_mut::<Messages<DamageEvent>>();
            let mut reader = events.get_cursor();
            for event in reader.read(&events) {
                damages.push(event.amount);
            }

            assert_eq!(damages.len(), 1, "Should fire one damage event");
            assert_eq!(damages[0], 15.0, "Damage should be 5.0 * 3 stacks = 15.0");
        }

        #[test]
        fn test_brainburn_no_damage_at_zero_stacks() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();

            // Create aura with damage timer that will fire on next tick
            let mut aura = BrainburnAura::default();
            aura.damage_timer.tick(Duration::from_secs_f32(BRAINBURN_STACK_INTERVAL - 0.01));
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                aura,
            ));

            // Create enemy with 0 stacks
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                BrainburnStack::default(),
            ));

            // Advance time for the system to use
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(tick_brainburn_damage_system);

            // Count damage events
            let events = app.world_mut().resource_mut::<Messages<DamageEvent>>();
            let mut reader = events.get_cursor();
            let count = reader.read(&events).count();

            assert_eq!(count, 0, "No damage at zero stacks");
        }
    }

    mod decay_brainburn_stacks_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_brainburn_stacks_decay_outside_aura() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create enemy with stacks but NOT in aura
            let mut stack = BrainburnStack::default();
            stack.stacks = 5;
            stack.in_aura = false;
            stack.decay_timer.tick(Duration::from_secs_f32(BRAINBURN_STACK_DECAY_INTERVAL - 0.01));
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack,
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(decay_brainburn_stacks_system);

            let stack = app.world().get::<BrainburnStack>(enemy).unwrap();
            assert_eq!(stack.stacks, 4, "Stacks should decay by 1");
        }

        #[test]
        fn test_brainburn_stacks_do_not_decay_inside_aura() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create enemy with stacks IN aura
            let mut stack = BrainburnStack::default();
            stack.stacks = 5;
            stack.in_aura = true;
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack,
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BRAINBURN_STACK_DECAY_INTERVAL * 2.0));
            }

            let _ = app.world_mut().run_system_once(decay_brainburn_stacks_system);

            let stack = app.world().get::<BrainburnStack>(enemy).unwrap();
            assert_eq!(stack.stacks, 5, "Stacks should not decay while in aura");
        }

        #[test]
        fn test_brainburn_stack_component_removed_when_empty() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create enemy with 1 stack outside aura
            let mut stack = BrainburnStack::default();
            stack.stacks = 1;
            stack.in_aura = false;
            stack.decay_timer.tick(Duration::from_secs_f32(BRAINBURN_STACK_DECAY_INTERVAL - 0.01));
            let enemy = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(2.0, 0.5, 0.0)),
                Enemy { speed: 2.0, strength: 10.0 },
                stack,
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(decay_brainburn_stacks_system);

            // Component should be removed
            assert!(
                app.world().get::<BrainburnStack>(enemy).is_none(),
                "BrainburnStack should be removed when stacks reach 0"
            );
        }
    }

    mod update_brainburn_aura_duration_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_brainburn_aura_despawns_after_duration() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let mut aura = BrainburnAura::default();
            aura.duration.tick(Duration::from_secs_f32(BRAINBURN_AURA_DURATION - 0.01));
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                aura,
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.02));
            }

            let _ = app.world_mut().run_system_once(update_brainburn_aura_duration_system);

            assert!(
                app.world().get_entity(entity).is_err(),
                "Aura should be despawned after duration"
            );
        }

        #[test]
        fn test_brainburn_aura_survives_before_duration() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BrainburnAura::default(),
            )).id();

            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BRAINBURN_AURA_DURATION / 2.0));
            }

            let _ = app.world_mut().run_system_once(update_brainburn_aura_duration_system);

            assert!(
                app.world().get_entity(entity).is_ok(),
                "Aura should survive before duration expires"
            );
        }
    }

    mod fire_brainburn_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_brainburn_spawns_aura() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Confusion);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_brainburn(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&BrainburnAura>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn 1 aura");
        }

        #[test]
        fn test_fire_brainburn_position() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Confusion);
            let spawn_pos = Vec3::new(15.0, 0.5, 25.0);

            {
                let mut commands = app.world_mut().commands();
                fire_brainburn(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&BrainburnAura, &Transform)>();
            for (_, transform) in query.iter(app.world()) {
                assert_eq!(transform.translation.x, 15.0);
                assert_eq!(transform.translation.y, BRAINBURN_VISUAL_HEIGHT);
                assert_eq!(transform.translation.z, 25.0);
            }
        }

        #[test]
        fn test_fire_brainburn_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Confusion);
            let explicit_damage = 100.0;
            let spawn_pos = Vec3::ZERO;

            {
                let mut commands = app.world_mut().commands();
                fire_brainburn_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&BrainburnAura>();
            for aura in query.iter(app.world()) {
                // 10% of explicit damage
                assert_eq!(aura.damage_per_stack, 10.0);
            }
        }
    }

    mod update_brainburn_aura_position_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;
        use crate::player::components::Player;

        #[test]
        fn test_brainburn_aura_follows_player() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create player at position
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
                Player {
                    speed: 150.0,
                    regen_rate: 1.0,
                    pickup_radius: 2.0,
                    last_movement_direction: Vec3::ZERO,
                },
            ));

            // Create aura at different position
            let aura_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                BrainburnAura::default(),
            )).id();

            let _ = app.world_mut().run_system_once(update_brainburn_aura_position_system);

            let aura_transform = app.world().get::<Transform>(aura_entity).unwrap();
            assert_eq!(aura_transform.translation.x, 10.0);
            assert_eq!(aura_transform.translation.y, BRAINBURN_VISUAL_HEIGHT);
            assert_eq!(aura_transform.translation.z, 20.0);
        }
    }
}
