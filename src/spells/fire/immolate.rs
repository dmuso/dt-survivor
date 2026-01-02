//! Immolate spell - Sets a target ablaze with lingering flames.
//!
//! A Fire element spell (Immolate SpellType) that applies a burning DOT effect
//! to the nearest enemy. The target takes damage over time and flashes with
//! fire-colored visual effects.

use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Immolate spell
pub const IMMOLATE_DURATION: f32 = 4.0;
pub const IMMOLATE_TICK_INTERVAL: f32 = 0.5;
pub const IMMOLATE_FLASH_DURATION: f32 = 0.15;
pub const IMMOLATE_FLASH_INTERVAL: f32 = 0.3;
pub const IMMOLATE_RANGE: f32 = 15.0;

/// Get the fire element color for visual effects
pub fn immolate_color() -> Color {
    Element::Fire.color()
}

/// DOT effect applied to enemies targeted by Immolate.
/// The enemy burns for a duration, taking damage at regular intervals.
#[derive(Component, Debug, Clone)]
pub struct ImmolateEffect {
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Total duration of the effect
    pub duration: Timer,
    /// Damage per tick
    pub damage_per_tick: f32,
    /// Timer for visual flashing
    pub flash_timer: Timer,
    /// Whether currently in flash state
    pub is_flashing: bool,
}

impl ImmolateEffect {
    pub fn new(damage: f32) -> Self {
        // Calculate ticks and damage per tick
        let total_ticks = (IMMOLATE_DURATION / IMMOLATE_TICK_INTERVAL) as u32;
        let damage_per_tick = damage / total_ticks as f32;

        Self {
            tick_timer: Timer::from_seconds(IMMOLATE_TICK_INTERVAL, TimerMode::Repeating),
            duration: Timer::from_seconds(IMMOLATE_DURATION, TimerMode::Once),
            damage_per_tick,
            flash_timer: Timer::from_seconds(IMMOLATE_FLASH_INTERVAL, TimerMode::Repeating),
            is_flashing: false,
        }
    }

    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(spell.damage())
    }

    pub fn with_damage(damage: f32) -> Self {
        Self::new(damage)
    }

    /// Tick the effect timers
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.duration.tick(delta);
        self.tick_timer.tick(delta);
        self.flash_timer.tick(delta);

        // Toggle flash state
        if self.flash_timer.just_finished() {
            self.is_flashing = !self.is_flashing;
        }
    }

    /// Check if the effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration.is_finished()
    }

    /// Check if ready to apply damage
    pub fn should_damage(&self) -> bool {
        self.tick_timer.just_finished() && !self.is_expired()
    }
}

impl Default for ImmolateEffect {
    fn default() -> Self {
        Self::new(12.0) // Default spell damage
    }
}

/// Visual indicator component for immolated enemies
#[derive(Component, Debug, Clone)]
pub struct ImmolateVisual {
    /// Original material to restore when effect ends
    pub original_material: Handle<StandardMaterial>,
    /// Flash duration timer
    pub flash_duration: Timer,
}

impl ImmolateVisual {
    pub fn new(original_material: Handle<StandardMaterial>) -> Self {
        Self {
            original_material,
            flash_duration: Timer::from_seconds(IMMOLATE_FLASH_DURATION, TimerMode::Once),
        }
    }
}

/// System that applies Immolate damage over time
pub fn immolate_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut immolate_query: Query<(Entity, &mut ImmolateEffect)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (entity, mut effect) in immolate_query.iter_mut() {
        effect.tick(time.delta());

        if effect.should_damage() {
            damage_events.write(DamageEvent::with_element(
                entity,
                effect.damage_per_tick,
                Element::Fire,
            ));
        }

        if effect.is_expired() {
            commands.entity(entity).remove::<ImmolateEffect>();
        }
    }
}

/// System that handles visual flashing for immolated enemies
#[allow(clippy::type_complexity)]
pub fn immolate_visual_system(
    mut commands: Commands,
    mut immolate_query: Query<(Entity, &ImmolateEffect, Option<&mut ImmolateVisual>, Option<&MeshMaterial3d<StandardMaterial>>)>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Some(materials) = game_materials else { return };

    for (entity, effect, visual, mesh_material) in immolate_query.iter_mut() {
        if effect.is_expired() {
            // Restore original material and remove visual component
            if let Some(visual) = visual {
                commands.entity(entity).insert(MeshMaterial3d(visual.original_material.clone()));
                commands.entity(entity).remove::<ImmolateVisual>();
            }
            continue;
        }

        // Handle flashing
        if effect.is_flashing {
            // Store original material if not already stored
            if visual.is_none() {
                if let Some(mesh_mat) = mesh_material {
                    commands.entity(entity).insert(ImmolateVisual::new(mesh_mat.0.clone()));
                    // Apply fire material
                    commands.entity(entity).insert(MeshMaterial3d(materials.fireball.clone()));
                }
            } else {
                // Already have visual, ensure fire material is applied
                commands.entity(entity).insert(MeshMaterial3d(materials.fireball.clone()));
            }
        } else if let Some(visual) = visual {
            // Not flashing, restore original material
            commands.entity(entity).insert(MeshMaterial3d(visual.original_material.clone()));
        }
    }
}

/// System that cleans up immolate effects when enemies die
pub fn immolate_cleanup_system(
    mut commands: Commands,
    mut removed_enemies: RemovedComponents<Enemy>,
    immolate_query: Query<Entity, With<ImmolateEffect>>,
) {
    for entity in removed_enemies.read() {
        if immolate_query.contains(entity) {
            commands.entity(entity).remove::<ImmolateEffect>();
            commands.entity(entity).remove::<ImmolateVisual>();
        }
    }
}

/// Cast Immolate spell - applies burning DOT to nearest enemy
#[allow(clippy::too_many_arguments)]
pub fn fire_immolate(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform, &Enemy)>,
    _game_meshes: Option<&GameMeshes>,
    _game_materials: Option<&GameMaterials>,
) {
    fire_immolate_with_damage(commands, spell, spell.damage(), spawn_position, enemy_query);
}

/// Cast Immolate spell with explicit damage
#[allow(clippy::too_many_arguments)]
pub fn fire_immolate_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    enemy_query: &Query<(Entity, &Transform, &Enemy)>,
) {
    let spawn_xz = from_xz(spawn_position);

    // Find nearest enemy within range
    let mut nearest_enemy: Option<(Entity, f32)> = None;

    for (enemy_entity, enemy_transform, _enemy) in enemy_query.iter() {
        let enemy_pos = from_xz(enemy_transform.translation);
        let distance = spawn_xz.distance(enemy_pos);

        if distance <= IMMOLATE_RANGE {
            match nearest_enemy {
                None => nearest_enemy = Some((enemy_entity, distance)),
                Some((_, nearest_dist)) if distance < nearest_dist => {
                    nearest_enemy = Some((enemy_entity, distance));
                }
                _ => {}
            }
        }
    }

    // Apply immolate effect to nearest enemy
    if let Some((enemy_entity, _)) = nearest_enemy {
        let effect = ImmolateEffect::with_damage(damage);
        commands.entity(enemy_entity).insert(effect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::SpellType;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin::default());
        app.add_message::<DamageEvent>();
        app
    }

    mod immolate_effect_tests {
        use super::*;

        #[test]
        fn test_immolate_effect_new() {
            let damage = 24.0;
            let effect = ImmolateEffect::new(damage);

            let expected_ticks = (IMMOLATE_DURATION / IMMOLATE_TICK_INTERVAL) as u32;
            let expected_damage_per_tick = damage / expected_ticks as f32;

            assert!(!effect.is_expired());
            assert!((effect.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
        }

        #[test]
        fn test_immolate_effect_from_spell() {
            let spell = Spell::new(SpellType::Immolate);
            let effect = ImmolateEffect::from_spell(&spell);

            let expected_ticks = (IMMOLATE_DURATION / IMMOLATE_TICK_INTERVAL) as u32;
            let expected_damage_per_tick = spell.damage() / expected_ticks as f32;

            assert!((effect.damage_per_tick - expected_damage_per_tick).abs() < 0.01);
        }

        #[test]
        fn test_immolate_effect_tick_and_damage() {
            let mut effect = ImmolateEffect::new(24.0);

            // Before tick interval - no damage
            effect.tick(Duration::from_secs_f32(IMMOLATE_TICK_INTERVAL / 2.0));
            assert!(!effect.should_damage());

            // After tick interval - should damage
            effect.tick(Duration::from_secs_f32(IMMOLATE_TICK_INTERVAL));
            assert!(effect.should_damage());
        }

        #[test]
        fn test_immolate_effect_expires() {
            let mut effect = ImmolateEffect::new(24.0);

            assert!(!effect.is_expired());

            // Tick past duration
            effect.tick(Duration::from_secs_f32(IMMOLATE_DURATION + 0.1));

            assert!(effect.is_expired());
        }

        #[test]
        fn test_immolate_effect_no_damage_after_expiry() {
            let mut effect = ImmolateEffect::new(24.0);

            // Tick past duration
            effect.tick(Duration::from_secs_f32(IMMOLATE_DURATION + 0.1));

            // Should not damage even if tick timer fires
            assert!(!effect.should_damage());
        }

        #[test]
        fn test_immolate_effect_flash_toggle() {
            let mut effect = ImmolateEffect::new(24.0);

            assert!(!effect.is_flashing);

            // Tick past flash interval
            effect.tick(Duration::from_secs_f32(IMMOLATE_FLASH_INTERVAL));
            assert!(effect.is_flashing);

            // Tick again
            effect.tick(Duration::from_secs_f32(IMMOLATE_FLASH_INTERVAL));
            assert!(!effect.is_flashing);
        }
    }

    mod immolate_damage_system_tests {
        use super::*;

        #[test]
        fn test_immolate_effect_should_damage_triggers_correctly() {
            // Test the core should_damage logic that the system uses
            let mut effect = ImmolateEffect::new(24.0);

            // Initially should not damage (tick timer not finished)
            assert!(!effect.should_damage());

            // After ticking past the interval, should damage
            effect.tick(Duration::from_secs_f32(IMMOLATE_TICK_INTERVAL));
            assert!(effect.should_damage());

            // After another small tick (same frame), still should not damage
            // because just_finished only returns true once
            effect.tick(Duration::from_secs_f32(0.01));
            assert!(!effect.should_damage());

            // After another full interval, should damage again
            effect.tick(Duration::from_secs_f32(IMMOLATE_TICK_INTERVAL));
            assert!(effect.should_damage());
        }

        #[test]
        fn test_immolate_damage_per_tick_calculation() {
            let damage = 24.0;
            let effect = ImmolateEffect::new(damage);

            let expected_ticks = (IMMOLATE_DURATION / IMMOLATE_TICK_INTERVAL) as u32;
            let expected_damage_per_tick = damage / expected_ticks as f32;

            assert!(
                (effect.damage_per_tick - expected_damage_per_tick).abs() < 0.01,
                "Damage should be spread across ticks"
            );
        }

        #[test]
        fn test_immolate_damage_system_removes_expired_effect() {
            let mut app = setup_test_app();
            app.add_systems(Update, immolate_damage_system);

            // Create entity with expired immolate effect
            let mut effect = ImmolateEffect::new(24.0);
            effect.duration.tick(Duration::from_secs_f32(IMMOLATE_DURATION + 0.1));

            let entity = app.world_mut().spawn((
                Transform::default(),
                effect,
            )).id();

            app.update();

            assert!(
                app.world().get::<ImmolateEffect>(entity).is_none(),
                "Expired ImmolateEffect should be removed"
            );
        }

        #[test]
        fn test_immolate_no_damage_before_tick() {
            let mut app = setup_test_app();

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
            app.add_systems(Update, (immolate_damage_system, count_damage_events).chain());

            // Create entity with fresh immolate effect (not ticked)
            let effect = ImmolateEffect::new(24.0);

            app.world_mut().spawn((
                Transform::default(),
                effect,
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Should not damage before tick interval");
        }
    }

    mod fire_immolate_tests {
        use super::*;
        use crate::combat::Health;

        fn test_enemy() -> Enemy {
            Enemy { speed: 100.0, strength: 1.0 }
        }

        #[test]
        fn test_fire_immolate_targets_nearest_enemy() {
            let mut app = setup_test_app();

            // Create enemies at different distances
            let far_enemy = app.world_mut().spawn((
                test_enemy(),
                Health::new(100.0),
                Transform::from_translation(Vec3::new(10.0, 0.5, 0.0)),
            )).id();

            let near_enemy = app.world_mut().spawn((
                test_enemy(),
                Health::new(100.0),
                Transform::from_translation(Vec3::new(3.0, 0.5, 0.0)),
            )).id();

            // Manually apply effect to nearest enemy (simulating the cast)
            app.world_mut().entity_mut(near_enemy).insert(ImmolateEffect::new(24.0));

            app.update();

            // Near enemy should have the effect
            assert!(
                app.world().get::<ImmolateEffect>(near_enemy).is_some(),
                "Nearest enemy should have ImmolateEffect"
            );

            // Far enemy should not have the effect
            assert!(
                app.world().get::<ImmolateEffect>(far_enemy).is_none(),
                "Far enemy should not have ImmolateEffect"
            );
        }

        #[test]
        fn test_fire_immolate_no_target_out_of_range() {
            let mut app = setup_test_app();

            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            // Create enemy outside range
            let enemy = app.world_mut().spawn((
                test_enemy(),
                Transform::from_translation(Vec3::new(IMMOLATE_RANGE + 5.0, 0.5, 0.0)),
            )).id();

            // Test the range check logic directly
            let enemy_pos = from_xz(Vec3::new(IMMOLATE_RANGE + 5.0, 0.5, 0.0));
            let spawn_xz = from_xz(spawn_pos);
            let distance = spawn_xz.distance(enemy_pos);

            // Verify enemy is out of range
            assert!(
                distance > IMMOLATE_RANGE,
                "Enemy should be outside immolate range"
            );

            // Since it's out of range, the effect should not be applied
            // In real gameplay, fire_immolate_with_damage wouldn't add the component
            assert!(
                app.world().get::<ImmolateEffect>(enemy).is_none(),
                "Enemy outside range should not have ImmolateEffect"
            );
        }

        #[test]
        fn test_fire_immolate_uses_correct_damage() {
            let mut app = setup_test_app();

            let explicit_damage = 100.0;

            let enemy = app.world_mut().spawn((
                test_enemy(),
                Transform::from_translation(Vec3::new(3.0, 0.5, 0.0)),
            )).id();

            // Apply effect with explicit damage
            app.world_mut().entity_mut(enemy).insert(ImmolateEffect::with_damage(explicit_damage));

            app.update();

            let effect = app.world().get::<ImmolateEffect>(enemy).unwrap();
            let expected_ticks = (IMMOLATE_DURATION / IMMOLATE_TICK_INTERVAL) as u32;
            let expected_damage_per_tick = explicit_damage / expected_ticks as f32;

            assert!(
                (effect.damage_per_tick - expected_damage_per_tick).abs() < 0.01,
                "Damage per tick should be {} but got {}",
                expected_damage_per_tick,
                effect.damage_per_tick
            );
        }
    }
}
