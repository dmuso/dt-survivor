use bevy::prelude::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::player::components::Player;
use crate::spell::components::Spell;

/// Default configuration for Overload spell
pub const OVERLOAD_MAX_CHARGE: f32 = 100.0;
pub const OVERLOAD_CHARGE_PER_HIT: f32 = 10.0;
pub const OVERLOAD_BLAST_RADIUS: f32 = 15.0;
pub const OVERLOAD_BLAST_DAMAGE_MULTIPLIER: f32 = 2.0;

/// Get the lightning element color for visual effects (yellow)
pub fn overload_color() -> Color {
    Element::Lightning.color()
}

/// OverloadCharge component - tracks charge buildup from lightning damage.
/// Attached to the player entity when Overload spell is equipped.
/// Charge accumulates passively when any lightning damage is dealt.
/// At max charge, automatically releases a powerful blast.
#[derive(Component, Debug, Clone)]
pub struct OverloadCharge {
    /// Current charge level (0 to max_charge)
    pub current_charge: f32,
    /// Maximum charge before blast triggers
    pub max_charge: f32,
    /// Amount of charge gained per lightning damage hit
    pub charge_per_hit: f32,
    /// Base damage for the blast (multiplied by spell damage)
    pub blast_damage: f32,
    /// Radius of the blast effect
    pub blast_radius: f32,
}

impl OverloadCharge {
    pub fn new(blast_damage: f32) -> Self {
        Self {
            current_charge: 0.0,
            max_charge: OVERLOAD_MAX_CHARGE,
            charge_per_hit: OVERLOAD_CHARGE_PER_HIT,
            blast_damage,
            blast_radius: OVERLOAD_BLAST_RADIUS,
        }
    }

    pub fn from_spell(spell: &Spell) -> Self {
        Self::new(spell.damage() * OVERLOAD_BLAST_DAMAGE_MULTIPLIER)
    }

    /// Add charge and return true if max charge is reached
    pub fn add_charge(&mut self, amount: f32) -> bool {
        self.current_charge = (self.current_charge + amount).min(self.max_charge);
        self.is_full()
    }

    /// Check if charge is at maximum
    pub fn is_full(&self) -> bool {
        self.current_charge >= self.max_charge
    }

    /// Reset charge to zero after blast
    pub fn reset(&mut self) {
        self.current_charge = 0.0;
    }

    /// Get charge percentage (0.0 to 1.0)
    pub fn charge_percent(&self) -> f32 {
        self.current_charge / self.max_charge
    }
}

/// OverloadBlast component - expanding nova effect that damages enemies.
/// Spawned when OverloadCharge reaches max and triggers.
#[derive(Component, Debug, Clone)]
pub struct OverloadBlast {
    /// Center position of the blast
    pub origin: Vec3,
    /// Maximum radius of the blast
    pub radius: f32,
    /// Damage to apply to enemies in radius
    pub damage: f32,
    /// Current expansion progress (0.0 to 1.0)
    pub expansion: f32,
    /// Speed of expansion
    pub expansion_speed: f32,
    /// Whether damage has been applied
    pub damage_applied: bool,
}

impl OverloadBlast {
    pub fn new(origin: Vec3, radius: f32, damage: f32) -> Self {
        Self {
            origin,
            radius,
            damage,
            expansion: 0.0,
            expansion_speed: 3.0,
            damage_applied: false,
        }
    }

    /// Check if blast is complete
    pub fn is_complete(&self) -> bool {
        self.expansion >= 1.0
    }

    /// Current visual radius based on expansion
    pub fn current_radius(&self) -> f32 {
        self.radius * self.expansion
    }
}

/// Marker component indicating Overload is actively equipped
#[derive(Component, Debug, Clone)]
pub struct OverloadActive;

/// Event fired when lightning damage is dealt (used to accumulate charge)
#[derive(Message, Debug, Clone)]
pub struct LightningDamageEvent {
    pub damage: f32,
}

/// System that adds charge when lightning damage is dealt
pub fn overload_charge_accumulate_system(
    mut damage_events: MessageReader<LightningDamageEvent>,
    mut charge_query: Query<&mut OverloadCharge>,
) {
    // Sum up all lightning damage dealt this frame
    let total_damage: f32 = damage_events.read().map(|e| e.damage).sum();

    if total_damage > 0.0 {
        for mut charge in charge_query.iter_mut() {
            // Add charge proportional to damage dealt
            let charge_amount = (total_damage / 10.0) * charge.charge_per_hit;
            charge.add_charge(charge_amount);
        }
    }
}

/// System that checks if overload should release blast
pub fn overload_check_release_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut charge_query: Query<&mut OverloadCharge>,
    game_meshes: Option<Res<GameMeshes>>,
    game_materials: Option<Res<GameMaterials>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for mut charge in charge_query.iter_mut() {
        if charge.is_full() {
            // Spawn blast at player position
            spawn_overload_blast(
                &mut commands,
                player_transform.translation,
                charge.blast_radius,
                charge.blast_damage,
                game_meshes.as_deref(),
                game_materials.as_deref(),
            );

            // Reset charge
            charge.reset();
        }
    }
}

/// Spawn an overload blast entity
fn spawn_overload_blast(
    commands: &mut Commands,
    origin: Vec3,
    radius: f32,
    damage: f32,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let blast = OverloadBlast::new(origin, radius, damage);
    let blast_pos = origin + Vec3::new(0.0, 0.3, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(blast_pos).with_scale(Vec3::splat(0.1)),
            blast,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(blast_pos),
            blast,
        ));
    }
}

/// System that expands blast and applies damage
pub fn overload_blast_system(
    mut commands: Commands,
    time: Res<Time>,
    mut blast_query: Query<(Entity, &mut OverloadBlast, &mut Transform), Without<Enemy>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (entity, mut blast, mut transform) in blast_query.iter_mut() {
        // Expand the blast
        blast.expansion += blast.expansion_speed * time.delta_secs();
        blast.expansion = blast.expansion.min(1.0);

        // Update visual scale
        let scale = blast.current_radius() * 0.5;
        transform.scale = Vec3::splat(scale.max(0.1));

        // Apply damage once at peak expansion (at 50% expansion)
        if !blast.damage_applied && blast.expansion >= 0.5 {
            blast.damage_applied = true;

            // Damage all enemies within blast radius
            let blast_origin = from_xz(blast.origin);
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_pos = from_xz(enemy_transform.translation);
                let distance = blast_origin.distance(enemy_pos);

                if distance <= blast.radius {
                    damage_events.write(DamageEvent::new(enemy_entity, blast.damage));
                }
            }
        }

        // Despawn when complete
        if blast.is_complete() {
            commands.entity(entity).despawn();
        }
    }
}

/// Cast Overload spell - adds OverloadCharge to an entity (typically spawned alongside player).
/// The Overload spell is passive - it accumulates charge from lightning damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_overload(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_overload_with_damage(
        commands,
        spell,
        spell.damage() * OVERLOAD_BLAST_DAMAGE_MULTIPLIER,
        spawn_position,
        game_meshes,
        game_materials,
    );
}

/// Cast Overload spell with explicit damage.
#[allow(clippy::too_many_arguments)]
pub fn fire_overload_with_damage(
    commands: &mut Commands,
    _spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    let mut charge = OverloadCharge::new(damage);
    charge.blast_damage = damage;

    let charge_pos = spawn_position + Vec3::new(0.0, 0.3, 0.0);

    if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
        commands.spawn((
            Mesh3d(meshes.explosion.clone()),
            MeshMaterial3d(materials.thunder_strike.clone()),
            Transform::from_translation(charge_pos).with_scale(Vec3::splat(0.5)),
            charge,
            OverloadActive,
        ));
    } else {
        // Fallback for tests without mesh resources
        commands.spawn((
            Transform::from_translation(charge_pos),
            charge,
            OverloadActive,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod overload_charge_component_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_overload_charge_new_initializes_with_zero_charge() {
            let charge = OverloadCharge::new(50.0);
            assert_eq!(charge.current_charge, 0.0);
        }

        #[test]
        fn test_overload_charge_new_sets_max_charge() {
            let charge = OverloadCharge::new(50.0);
            assert_eq!(charge.max_charge, OVERLOAD_MAX_CHARGE);
        }

        #[test]
        fn test_overload_charge_new_sets_charge_per_hit() {
            let charge = OverloadCharge::new(50.0);
            assert_eq!(charge.charge_per_hit, OVERLOAD_CHARGE_PER_HIT);
        }

        #[test]
        fn test_overload_charge_new_sets_blast_damage() {
            let charge = OverloadCharge::new(75.0);
            assert_eq!(charge.blast_damage, 75.0);
        }

        #[test]
        fn test_overload_charge_new_sets_blast_radius() {
            let charge = OverloadCharge::new(50.0);
            assert_eq!(charge.blast_radius, OVERLOAD_BLAST_RADIUS);
        }

        #[test]
        fn test_overload_charge_from_spell() {
            let spell = Spell::new(SpellType::Overcharge);
            let charge = OverloadCharge::from_spell(&spell);

            // Spell damage * multiplier
            let expected_damage = spell.damage() * OVERLOAD_BLAST_DAMAGE_MULTIPLIER;
            assert_eq!(charge.blast_damage, expected_damage);
        }

        #[test]
        fn test_add_charge_increases_current_charge() {
            let mut charge = OverloadCharge::new(50.0);
            charge.add_charge(25.0);
            assert_eq!(charge.current_charge, 25.0);
        }

        #[test]
        fn test_add_charge_caps_at_max() {
            let mut charge = OverloadCharge::new(50.0);
            charge.add_charge(150.0); // More than max
            assert_eq!(charge.current_charge, OVERLOAD_MAX_CHARGE);
        }

        #[test]
        fn test_add_charge_returns_true_when_full() {
            let mut charge = OverloadCharge::new(50.0);
            let is_full = charge.add_charge(100.0);
            assert!(is_full);
        }

        #[test]
        fn test_add_charge_returns_false_when_not_full() {
            let mut charge = OverloadCharge::new(50.0);
            let is_full = charge.add_charge(50.0);
            assert!(!is_full);
        }

        #[test]
        fn test_is_full_returns_false_when_below_max() {
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 50.0;
            assert!(!charge.is_full());
        }

        #[test]
        fn test_is_full_returns_true_at_max() {
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 100.0;
            assert!(charge.is_full());
        }

        #[test]
        fn test_reset_sets_charge_to_zero() {
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 100.0;
            charge.reset();
            assert_eq!(charge.current_charge, 0.0);
        }

        #[test]
        fn test_charge_percent_returns_zero_initially() {
            let charge = OverloadCharge::new(50.0);
            assert_eq!(charge.charge_percent(), 0.0);
        }

        #[test]
        fn test_charge_percent_returns_correct_value() {
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 50.0;
            assert_eq!(charge.charge_percent(), 0.5);
        }

        #[test]
        fn test_charge_percent_returns_one_at_max() {
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 100.0;
            assert_eq!(charge.charge_percent(), 1.0);
        }

        #[test]
        fn test_overload_uses_lightning_element_color() {
            let color = overload_color();
            assert_eq!(color, Element::Lightning.color());
            assert_eq!(color, Color::srgb_u8(255, 255, 0)); // Yellow
        }
    }

    mod overload_blast_component_tests {
        use super::*;

        #[test]
        fn test_overload_blast_new_sets_origin() {
            let origin = Vec3::new(5.0, 1.0, 10.0);
            let blast = OverloadBlast::new(origin, 15.0, 100.0);
            assert_eq!(blast.origin, origin);
        }

        #[test]
        fn test_overload_blast_new_sets_radius() {
            let blast = OverloadBlast::new(Vec3::ZERO, 20.0, 100.0);
            assert_eq!(blast.radius, 20.0);
        }

        #[test]
        fn test_overload_blast_new_sets_damage() {
            let blast = OverloadBlast::new(Vec3::ZERO, 15.0, 150.0);
            assert_eq!(blast.damage, 150.0);
        }

        #[test]
        fn test_overload_blast_new_starts_unexpanded() {
            let blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            assert_eq!(blast.expansion, 0.0);
        }

        #[test]
        fn test_overload_blast_new_damage_not_applied() {
            let blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            assert!(!blast.damage_applied);
        }

        #[test]
        fn test_is_complete_returns_false_when_unexpanded() {
            let blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            assert!(!blast.is_complete());
        }

        #[test]
        fn test_is_complete_returns_true_at_full_expansion() {
            let mut blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            blast.expansion = 1.0;
            assert!(blast.is_complete());
        }

        #[test]
        fn test_current_radius_returns_zero_initially() {
            let blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            assert_eq!(blast.current_radius(), 0.0);
        }

        #[test]
        fn test_current_radius_scales_with_expansion() {
            let mut blast = OverloadBlast::new(Vec3::ZERO, 20.0, 100.0);
            blast.expansion = 0.5;
            assert_eq!(blast.current_radius(), 10.0);
        }

        #[test]
        fn test_current_radius_at_full_expansion() {
            let mut blast = OverloadBlast::new(Vec3::ZERO, 20.0, 100.0);
            blast.expansion = 1.0;
            assert_eq!(blast.current_radius(), 20.0);
        }
    }

    mod overload_charge_accumulate_system_tests {
        use super::*;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<LightningDamageEvent>();
            app
        }

        #[test]
        fn test_charge_increases_on_lightning_damage() {
            let mut app = setup_test_app();

            let charge_entity = app.world_mut().spawn(OverloadCharge::new(50.0)).id();

            // Send lightning damage event
            app.world_mut().write_message(LightningDamageEvent { damage: 10.0 });

            let _ = app.world_mut().run_system_once(overload_charge_accumulate_system);

            let charge = app.world().get::<OverloadCharge>(charge_entity).unwrap();
            // 10.0 damage / 10.0 * 10.0 charge_per_hit = 10.0
            assert_eq!(charge.current_charge, 10.0);
        }

        #[test]
        fn test_charge_accumulates_from_multiple_events() {
            let mut app = setup_test_app();

            let charge_entity = app.world_mut().spawn(OverloadCharge::new(50.0)).id();

            // Send multiple lightning damage events
            app.world_mut().write_message(LightningDamageEvent { damage: 10.0 });
            app.world_mut().write_message(LightningDamageEvent { damage: 20.0 });

            let _ = app.world_mut().run_system_once(overload_charge_accumulate_system);

            let charge = app.world().get::<OverloadCharge>(charge_entity).unwrap();
            // (10.0 + 20.0) / 10.0 * 10.0 = 30.0
            assert_eq!(charge.current_charge, 30.0);
        }

        #[test]
        fn test_no_charge_without_damage_events() {
            let mut app = setup_test_app();

            let charge_entity = app.world_mut().spawn(OverloadCharge::new(50.0)).id();

            let _ = app.world_mut().run_system_once(overload_charge_accumulate_system);

            let charge = app.world().get::<OverloadCharge>(charge_entity).unwrap();
            assert_eq!(charge.current_charge, 0.0);
        }

        #[test]
        fn test_charge_caps_at_max() {
            let mut app = setup_test_app();

            let charge_entity = app.world_mut().spawn(OverloadCharge::new(50.0)).id();

            // Send large damage to exceed max charge
            app.world_mut().write_message(LightningDamageEvent { damage: 1000.0 });

            let _ = app.world_mut().run_system_once(overload_charge_accumulate_system);

            let charge = app.world().get::<OverloadCharge>(charge_entity).unwrap();
            assert_eq!(charge.current_charge, OVERLOAD_MAX_CHARGE);
        }
    }

    mod overload_check_release_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_blast_spawns_when_charge_full() {
            let mut app = setup_test_app();

            // Create player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
            ));

            // Create full charge
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 100.0;
            app.world_mut().spawn((
                Transform::default(),
                charge,
            ));

            app.add_systems(Update, overload_check_release_system);
            app.update();

            // Blast should be spawned
            let mut blast_query = app.world_mut().query::<&OverloadBlast>();
            let blast_count = blast_query.iter(app.world()).count();
            assert_eq!(blast_count, 1, "Blast should spawn when charge is full");
        }

        #[test]
        fn test_charge_resets_after_blast() {
            let mut app = setup_test_app();

            // Create player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
            ));

            // Create full charge
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 100.0;
            let charge_entity = app.world_mut().spawn((
                Transform::default(),
                charge,
            )).id();

            app.add_systems(Update, overload_check_release_system);
            app.update();

            // Charge should be reset
            let charge = app.world().get::<OverloadCharge>(charge_entity).unwrap();
            assert_eq!(charge.current_charge, 0.0, "Charge should reset after blast");
        }

        #[test]
        fn test_no_blast_when_charge_not_full() {
            let mut app = setup_test_app();

            // Create player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(10.0, 0.5, 20.0)),
            ));

            // Create partial charge
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 50.0; // Only half full
            app.world_mut().spawn((
                Transform::default(),
                charge,
            ));

            app.add_systems(Update, overload_check_release_system);
            app.update();

            // No blast should spawn
            let mut blast_query = app.world_mut().query::<&OverloadBlast>();
            let blast_count = blast_query.iter(app.world()).count();
            assert_eq!(blast_count, 0, "No blast when charge not full");
        }

        #[test]
        fn test_blast_spawns_at_player_position() {
            let mut app = setup_test_app();

            let player_pos = Vec3::new(15.0, 0.5, 25.0);
            // Create player
            app.world_mut().spawn((
                Player {
                    speed: 200.0,
                    regen_rate: 1.0,
                    pickup_radius: 50.0,
                    last_movement_direction: Vec3::ZERO,
                },
                Transform::from_translation(player_pos),
            ));

            // Create full charge
            let mut charge = OverloadCharge::new(50.0);
            charge.current_charge = 100.0;
            app.world_mut().spawn((
                Transform::default(),
                charge,
            ));

            app.add_systems(Update, overload_check_release_system);
            app.update();

            // Blast should be at player position
            let mut blast_query = app.world_mut().query::<&OverloadBlast>();
            let blasts: Vec<_> = blast_query.iter(app.world()).collect();
            assert_eq!(blasts.len(), 1);
            assert_eq!(blasts[0].origin, player_pos);
        }
    }

    mod overload_blast_system_tests {
        use super::*;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.init_resource::<Time>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_blast_expands_over_time() {
            let mut app = setup_test_app();
            app.add_systems(Update, overload_blast_system);

            let blast_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                OverloadBlast::new(Vec3::ZERO, 15.0, 100.0),
            )).id();

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            let blast = app.world().get::<OverloadBlast>(blast_entity).unwrap();
            assert!(blast.expansion > 0.0, "Blast should expand over time");
        }

        #[test]
        fn test_blast_damages_enemies_in_radius() {
            let mut app = setup_test_app();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

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
            app.add_systems(Update, (overload_blast_system, count_damage_events).chain());

            // Create blast at origin
            let mut blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            blast.expansion = 0.4; // Just before damage threshold
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                blast,
            ));

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            // Advance time to trigger damage
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Enemy in radius should be damaged");
        }

        #[test]
        fn test_blast_does_not_damage_enemies_outside_radius() {
            let mut app = setup_test_app();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

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
            app.add_systems(Update, (overload_blast_system, count_damage_events).chain());

            // Create blast at origin with radius 15
            let mut blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            blast.expansion = 0.4;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                blast,
            ));

            // Create enemy outside range (distance = 50)
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(50.0, 0.375, 0.0)),
            ));

            // Advance time
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.1));
            }

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0, "Enemy outside radius should not be damaged");
        }

        #[test]
        fn test_blast_damage_only_applied_once() {
            let mut app = setup_test_app();

            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

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
            app.add_systems(Update, (overload_blast_system, count_damage_events).chain());

            // Create blast that's already past damage threshold
            let mut blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            blast.expansion = 0.4;
            app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                blast,
            ));

            // Create enemy within range
            app.world_mut().spawn((
                Enemy { speed: 50.0, strength: 10.0 },
                Transform::from_translation(Vec3::new(5.0, 0.375, 0.0)),
            ));

            // Run multiple updates
            for _ in 0..3 {
                {
                    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                    time.advance_by(Duration::from_secs_f32(0.1));
                }
                app.update();
            }

            assert_eq!(counter.0.load(Ordering::SeqCst), 1, "Damage should only apply once");
        }

        #[test]
        fn test_blast_despawns_when_complete() {
            let mut app = setup_test_app();
            app.add_systems(Update, overload_blast_system);

            // Create blast near completion
            let mut blast = OverloadBlast::new(Vec3::ZERO, 15.0, 100.0);
            blast.expansion = 0.99;
            let blast_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::ZERO),
                blast,
            )).id();

            // Advance enough time to complete
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(0.5));
            }

            app.update();

            // Blast should be despawned
            assert!(app.world().get_entity(blast_entity).is_err(), "Blast should despawn when complete");
        }
    }

    mod fire_overload_tests {
        use super::*;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_overload_spawns_charge() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Overcharge);
            let spawn_pos = Vec3::new(10.0, 0.5, 20.0);

            {
                let mut commands = app.world_mut().commands();
                fire_overload(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&OverloadCharge>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1, "Should spawn 1 overload charge");
        }

        #[test]
        fn test_fire_overload_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Overcharge);
            let expected_damage = spell.damage() * OVERLOAD_BLAST_DAMAGE_MULTIPLIER;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_overload(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&OverloadCharge>();
            for charge in query.iter(app.world()) {
                assert_eq!(charge.blast_damage, expected_damage);
            }
        }

        #[test]
        fn test_fire_overload_with_damage_uses_explicit_damage() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Overcharge);
            let explicit_damage = 200.0;
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_overload_with_damage(
                    &mut commands,
                    &spell,
                    explicit_damage,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&OverloadCharge>();
            for charge in query.iter(app.world()) {
                assert_eq!(charge.blast_damage, explicit_damage);
            }
        }

        #[test]
        fn test_fire_overload_has_active_marker() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Overcharge);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_overload(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<(&OverloadCharge, &OverloadActive)>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1, "Should have OverloadActive marker");
        }
    }
}
