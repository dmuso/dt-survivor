use std::collections::HashSet;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::events::FireballEnemyCollisionEvent;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::movement::components::from_xz;
use crate::spell::components::Spell;

/// Default configuration for Fireball spell
pub const FIREBALL_SPREAD_ANGLE: f32 = 15.0;
pub const FIREBALL_SPEED: f32 = 20.0;
pub const FIREBALL_LIFETIME: f32 = 5.0;
pub const FIREBALL_SIZE: Vec2 = Vec2::new(0.3, 0.3);

/// Burn effect configuration
pub const BURN_TICK_INTERVAL: f32 = 0.5;
pub const BURN_TOTAL_DURATION: f32 = 3.0;
pub const BURN_DAMAGE_RATIO: f32 = 0.25; // 25% of direct damage per tick

/// Marker component for fireball projectiles
#[derive(Component, Debug, Clone)]
pub struct FireballProjectile {
    /// Direction of travel on XZ plane
    pub direction: Vec2,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Direct hit damage
    pub damage: f32,
    /// Burn damage per tick when applied
    pub burn_tick_damage: f32,
}

impl FireballProjectile {
    pub fn new(direction: Vec2, speed: f32, lifetime_secs: f32, damage: f32) -> Self {
        let burn_tick_damage = damage * BURN_DAMAGE_RATIO;
        Self {
            direction,
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            burn_tick_damage,
        }
    }

    pub fn from_spell(direction: Vec2, spell: &Spell) -> Self {
        Self::new(direction, FIREBALL_SPEED, FIREBALL_LIFETIME, spell.damage())
    }
}

/// Burn damage-over-time effect applied to enemies hit by Fireball
#[derive(Component, Debug, Clone)]
pub struct BurnEffect {
    /// Timer between damage ticks
    pub tick_timer: Timer,
    /// Number of ticks remaining
    pub remaining_ticks: u32,
    /// Damage per tick
    pub tick_damage: f32,
}

impl BurnEffect {
    pub fn new(tick_damage: f32) -> Self {
        let total_ticks = (BURN_TOTAL_DURATION / BURN_TICK_INTERVAL) as u32;
        Self {
            tick_timer: Timer::from_seconds(BURN_TICK_INTERVAL, TimerMode::Repeating),
            remaining_ticks: total_ticks,
            tick_damage,
        }
    }

    /// Tick the burn effect and return true if damage should be applied
    pub fn tick(&mut self, delta: std::time::Duration) -> bool {
        self.tick_timer.tick(delta);
        if self.tick_timer.just_finished() && self.remaining_ticks > 0 {
            self.remaining_ticks -= 1;
            true
        } else {
            false
        }
    }

    /// Check if the burn effect has expired (no more ticks)
    pub fn is_expired(&self) -> bool {
        self.remaining_ticks == 0
    }
}

impl Default for BurnEffect {
    fn default() -> Self {
        Self::new(5.0)
    }
}

/// System that applies burn damage over time
pub fn burn_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut burn_query: Query<(Entity, &mut BurnEffect)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (entity, mut burn) in burn_query.iter_mut() {
        if burn.tick(time.delta()) {
            damage_events.write(DamageEvent::new(entity, burn.tick_damage));
        }

        if burn.is_expired() {
            commands.entity(entity).remove::<BurnEffect>();
        }
    }
}

/// Get the fire element color for visual effects
pub fn fireball_color() -> Color {
    Element::Fire.color()
}

/// Cast fireball spell - spawns projectiles with fire element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
#[allow(clippy::too_many_arguments)]
pub fn fire_fireball(
    commands: &mut Commands,
    spell: &Spell,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    fire_fireball_with_damage(
        commands,
        spell,
        spell.damage(),
        spawn_position,
        target_pos,
        asset_server,
        weapon_channel,
        sound_limiter,
        game_meshes,
        game_materials,
    );
}

/// Cast fireball spell with explicit damage - spawns projectiles with fire element visuals
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
#[allow(clippy::too_many_arguments)]
pub fn fire_fireball_with_damage(
    commands: &mut Commands,
    spell: &Spell,
    damage: f32,
    spawn_position: Vec3,
    target_pos: Vec2,
    asset_server: Option<&Res<AssetServer>>,
    weapon_channel: Option<&mut ResMut<AudioChannel<WeaponSoundChannel>>>,
    sound_limiter: Option<&mut ResMut<SoundLimiter>>,
    game_meshes: Option<&GameMeshes>,
    game_materials: Option<&GameMaterials>,
) {
    // Extract XZ position from spawn_position for direction calculation
    let spawn_xz = from_xz(spawn_position);
    let base_direction = (target_pos - spawn_xz).normalize();

    // Get projectile count based on spell level (1 at level 1-4, 2 at 5-9, 3 at 10)
    let projectile_count = spell.projectile_count();
    let spread_angle_rad = FIREBALL_SPREAD_ANGLE.to_radians();

    // Create projectiles in a spread pattern centered around the target direction
    for i in 0..projectile_count {
        let angle_offset = if projectile_count == 1 {
            0.0
        } else {
            let half_spread = (projectile_count - 1) as f32 / 2.0;
            (i as f32 - half_spread) * spread_angle_rad
        };

        let cos_offset = angle_offset.cos();
        let sin_offset = angle_offset.sin();
        let direction = Vec2::new(
            base_direction.x * cos_offset - base_direction.y * sin_offset,
            base_direction.x * sin_offset + base_direction.y * cos_offset,
        );

        let fireball = FireballProjectile::new(direction, FIREBALL_SPEED, FIREBALL_LIFETIME, damage);

        // Spawn fireball at Whisper's full 3D position
        if let (Some(meshes), Some(materials)) = (game_meshes, game_materials) {
            commands.spawn((
                Mesh3d(meshes.bullet.clone()),
                MeshMaterial3d(materials.fireball.clone()),
                Transform::from_translation(spawn_position),
                fireball,
            ));
        } else {
            // Fallback for tests without mesh resources
            commands.spawn((
                Transform::from_translation(spawn_position),
                fireball,
            ));
        }
    }

    // Play spell sound effect
    if let (Some(asset_server), Some(weapon_channel), Some(sound_limiter)) =
        (asset_server, weapon_channel, sound_limiter)
    {
        play_limited_sound(
            weapon_channel,
            asset_server,
            "sounds/143610__dwoboyle__weapons-synth-blast-02.wav",
            sound_limiter,
        );
    }
}

/// System that moves fireball projectiles
pub fn fireball_movement_system(
    mut fireball_query: Query<(&mut Transform, &FireballProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, fireball) in fireball_query.iter_mut() {
        let movement = fireball.direction * fireball.speed * time.delta_secs();
        // Movement on XZ plane: direction.x -> X axis, direction.y -> Z axis
        transform.translation += Vec3::new(movement.x, 0.0, movement.y);
    }
}

/// System that handles fireball lifetime
pub fn fireball_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut fireball_query: Query<(Entity, &mut FireballProjectile)>,
) {
    for (entity, mut fireball) in fireball_query.iter_mut() {
        fireball.lifetime.tick(time.delta());

        if fireball.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Collision radius for fireball-enemy detection (scaled for 3D world units)
pub const FIREBALL_COLLISION_RADIUS: f32 = 1.0;

/// System that detects fireball-enemy collisions and fires events
pub fn fireball_collision_detection(
    fireball_query: Query<(Entity, &Transform), With<FireballProjectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut collision_events: MessageWriter<FireballEnemyCollisionEvent>,
) {
    for (fireball_entity, fireball_transform) in fireball_query.iter() {
        let fireball_xz = Vec2::new(
            fireball_transform.translation.x,
            fireball_transform.translation.z,
        );

        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let enemy_xz = Vec2::new(
                enemy_transform.translation.x,
                enemy_transform.translation.z,
            );
            let distance = fireball_xz.distance(enemy_xz);

            if distance < FIREBALL_COLLISION_RADIUS {
                collision_events.write(FireballEnemyCollisionEvent {
                    fireball_entity,
                    enemy_entity,
                });
                break; // Only hit one enemy per fireball
            }
        }
    }
}

/// System that applies effects when fireballs collide with enemies
/// Sends DamageEvent and applies BurnEffect to enemies
pub fn fireball_collision_effects(
    mut commands: Commands,
    mut collision_events: MessageReader<FireballEnemyCollisionEvent>,
    fireball_query: Query<&FireballProjectile>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let mut fireballs_to_despawn = HashSet::new();
    let mut effects_to_apply: Vec<(Entity, f32, f32)> = Vec::new();

    for event in collision_events.read() {
        fireballs_to_despawn.insert(event.fireball_entity);

        // Get fireball damage values
        if let Ok(fireball) = fireball_query.get(event.fireball_entity) {
            effects_to_apply.push((event.enemy_entity, fireball.damage, fireball.burn_tick_damage));
        }
    }

    // Despawn fireballs
    for fireball_entity in fireballs_to_despawn {
        commands.entity(fireball_entity).try_despawn();
    }

    // Apply damage and burn effects
    for (enemy_entity, damage, burn_tick_damage) in effects_to_apply {
        // Direct damage
        damage_events.write(DamageEvent::new(enemy_entity, damage));

        // Apply burn effect (if enemy doesn't already have one, it gets added)
        commands.entity(enemy_entity).try_insert(BurnEffect::new(burn_tick_damage));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod fireball_projectile_tests {
        use super::*;
        use crate::spell::SpellType;

        #[test]
        fn test_fireball_projectile_new() {
            let direction = Vec2::new(1.0, 0.0);
            let fireball = FireballProjectile::new(direction, 20.0, 5.0, 15.0);

            assert_eq!(fireball.direction, direction);
            assert_eq!(fireball.speed, 20.0);
            assert_eq!(fireball.damage, 15.0);
            assert_eq!(fireball.burn_tick_damage, 15.0 * BURN_DAMAGE_RATIO);
        }

        #[test]
        fn test_fireball_from_spell() {
            let spell = Spell::new(SpellType::Fireball);
            let direction = Vec2::new(0.0, 1.0);
            let fireball = FireballProjectile::from_spell(direction, &spell);

            assert_eq!(fireball.direction, direction);
            assert_eq!(fireball.speed, FIREBALL_SPEED);
            assert_eq!(fireball.damage, spell.damage());
        }

        #[test]
        fn test_fireball_lifetime_timer() {
            let fireball = FireballProjectile::new(Vec2::X, 20.0, 5.0, 15.0);
            assert_eq!(fireball.lifetime.duration(), Duration::from_secs_f32(5.0));
            assert!(!fireball.lifetime.is_finished());
        }

        #[test]
        fn test_fireball_uses_fire_element_color() {
            let color = fireball_color();
            assert_eq!(color, Element::Fire.color());
            assert_eq!(color, Color::srgb_u8(255, 128, 0));
        }
    }

    mod burn_effect_tests {
        use super::*;

        #[test]
        fn test_burn_effect_new() {
            let burn = BurnEffect::new(5.0);
            assert_eq!(burn.tick_damage, 5.0);
            assert!(!burn.is_expired());
        }

        #[test]
        fn test_burn_effect_default() {
            let burn = BurnEffect::default();
            assert_eq!(burn.tick_damage, 5.0);
        }

        #[test]
        fn test_burn_effect_calculates_correct_ticks() {
            let burn = BurnEffect::new(5.0);
            let expected_ticks = (BURN_TOTAL_DURATION / BURN_TICK_INTERVAL) as u32;
            assert_eq!(burn.remaining_ticks, expected_ticks);
        }

        #[test]
        fn test_burn_effect_tick_applies_damage() {
            let mut burn = BurnEffect::new(5.0);
            let initial_ticks = burn.remaining_ticks;

            // First half-tick: no damage yet
            let should_damage = burn.tick(Duration::from_secs_f32(BURN_TICK_INTERVAL / 2.0));
            assert!(!should_damage);
            assert_eq!(burn.remaining_ticks, initial_ticks);

            // Complete the tick: damage should apply
            let should_damage = burn.tick(Duration::from_secs_f32(BURN_TICK_INTERVAL));
            assert!(should_damage);
            assert_eq!(burn.remaining_ticks, initial_ticks - 1);
        }

        #[test]
        fn test_burn_effect_expires_after_all_ticks() {
            let mut burn = BurnEffect::new(5.0);
            let total_ticks = burn.remaining_ticks;

            // Consume all ticks
            for _ in 0..total_ticks {
                burn.tick(Duration::from_secs_f32(BURN_TICK_INTERVAL));
            }

            assert!(burn.is_expired());
        }

        #[test]
        fn test_burn_effect_no_damage_after_expired() {
            let mut burn = BurnEffect::new(5.0);
            let total_ticks = burn.remaining_ticks;

            // Consume all ticks
            for _ in 0..total_ticks {
                burn.tick(Duration::from_secs_f32(BURN_TICK_INTERVAL));
            }

            // Extra tick should not produce damage
            let should_damage = burn.tick(Duration::from_secs_f32(BURN_TICK_INTERVAL));
            assert!(!should_damage);
        }
    }

    mod burn_damage_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_burn_damage_system_decrements_ticks() {
            let mut app = setup_test_app();

            // Spawn entity with burn effect
            let entity = app.world_mut().spawn(BurnEffect::new(5.0)).id();

            let initial_ticks = app.world().get::<BurnEffect>(entity).unwrap().remaining_ticks;

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BURN_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(burn_damage_system);

            // Remaining ticks should have decreased
            let burn = app.world().get::<BurnEffect>(entity).unwrap();
            assert_eq!(burn.remaining_ticks, initial_ticks - 1);
        }

        #[test]
        fn test_burn_damage_system_writes_damage_event() {
            let mut app = setup_test_app();

            // Spawn entity with burn effect
            let entity = app.world_mut().spawn(BurnEffect::new(5.0)).id();

            // Advance time past tick interval
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BURN_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(burn_damage_system);

            // Verify a damage event was written by checking through message queue
            // The event should be written but we can't easily read it in the same frame
            // Verify indirectly by checking tick was consumed
            let burn = app.world().get::<BurnEffect>(entity).unwrap();
            let expected_ticks = (BURN_TOTAL_DURATION / BURN_TICK_INTERVAL) as u32 - 1;
            assert_eq!(burn.remaining_ticks, expected_ticks);
        }

        #[test]
        fn test_burn_damage_system_removes_expired_burn() {
            let mut app = setup_test_app();

            // Create burn effect with only 1 tick remaining
            let mut burn = BurnEffect::new(5.0);
            burn.remaining_ticks = 1;

            let entity = app.world_mut().spawn(burn).id();

            // Advance time to trigger the final tick
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs_f32(BURN_TICK_INTERVAL + 0.01));
            }

            let _ = app.world_mut().run_system_once(burn_damage_system);

            // BurnEffect should be removed
            assert!(app.world().get::<BurnEffect>(entity).is_none());
        }
    }

    mod fire_fireball_tests {
        use super::*;
        use bevy::app::App;
        use crate::spell::SpellType;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app
        }

        #[test]
        fn test_fire_fireball_spawns_projectile() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fireball(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            // Should have spawned 1 fireball (level 1)
            let mut query = app.world_mut().query::<&FireballProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 1);
        }

        #[test]
        fn test_fire_fireball_spawns_multiple_at_higher_levels() {
            let mut app = setup_test_app();

            let mut spell = Spell::new(SpellType::Fireball);
            spell.level = 5; // Should spawn 2 projectiles
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fireball(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireballProjectile>();
            let count = query.iter(app.world()).count();
            assert_eq!(count, 2);
        }

        #[test]
        fn test_fire_fireball_direction_toward_target() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0); // Target in +X direction

            {
                let mut commands = app.world_mut().commands();
                fire_fireball(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireballProjectile>();
            for fireball in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(fireball.direction.x > 0.9, "Fireball should move toward target");
            }
        }

        #[test]
        fn test_fire_fireball_damage_from_spell() {
            let mut app = setup_test_app();

            let spell = Spell::new(SpellType::Fireball);
            let expected_damage = spell.damage();
            let spawn_pos = Vec3::new(0.0, 0.5, 0.0);
            let target_pos = Vec2::new(10.0, 0.0);

            {
                let mut commands = app.world_mut().commands();
                fire_fireball(
                    &mut commands,
                    &spell,
                    spawn_pos,
                    target_pos,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&FireballProjectile>();
            for fireball in query.iter(app.world()) {
                assert_eq!(fireball.damage, expected_damage);
                assert_eq!(fireball.burn_tick_damage, expected_damage * BURN_DAMAGE_RATIO);
            }
        }
    }

    mod fireball_movement_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_fireball_movement_on_xz_plane() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create fireball moving in +X direction
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::new(1.0, 0.0), 100.0, 5.0, 15.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(fireball_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 100.0); // Speed * 1 sec
            assert_eq!(transform.translation.y, 0.5);   // Y unchanged
            assert_eq!(transform.translation.z, 0.0);
        }

        #[test]
        fn test_fireball_movement_z_direction() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            // Create fireball moving in +Z direction (direction.y maps to Z)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::new(0.0, 1.0), 50.0, 5.0, 15.0),
            )).id();

            // Advance time 1 second
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(1));
            }

            let _ = app.world_mut().run_system_once(fireball_movement_system);

            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(transform.translation.x, 0.0);
            assert_eq!(transform.translation.y, 0.5);
            assert_eq!(transform.translation.z, 50.0); // Moved in +Z
        }
    }

    mod fireball_lifetime_system_tests {
        use super::*;
        use bevy::app::App;
        use bevy::ecs::system::RunSystemOnce;

        #[test]
        fn test_fireball_despawns_after_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::X, 100.0, 5.0, 15.0),
            )).id();

            // Advance time past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(6));
            }

            let _ = app.world_mut().run_system_once(fireball_lifetime_system);

            assert!(!app.world().entities().contains(entity));
        }

        #[test]
        fn test_fireball_survives_before_lifetime() {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());

            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::X, 100.0, 5.0, 15.0),
            )).id();

            // Advance time but not past lifetime
            {
                let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
                time.advance_by(Duration::from_secs(3));
            }

            let _ = app.world_mut().run_system_once(fireball_lifetime_system);

            assert!(app.world().entities().contains(entity));
        }
    }

    mod fireball_collision_tests {
        use super::*;
        use bevy::app::App;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::time::TimePlugin::default());
            app.add_message::<FireballEnemyCollisionEvent>();
            app.add_message::<DamageEvent>();
            app
        }

        #[test]
        fn test_collision_detection_fires_event() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<FireballEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (fireball_collision_detection, count_collisions).chain());

            // Spawn fireball at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
            ));

            // Spawn enemy within collision radius
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn test_collision_detection_no_event_when_far() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;

            let mut app = setup_test_app();

            #[derive(Resource, Clone)]
            struct CollisionCounter(Arc<AtomicUsize>);

            fn count_collisions(
                mut events: MessageReader<FireballEnemyCollisionEvent>,
                counter: Res<CollisionCounter>,
            ) {
                for _ in events.read() {
                    counter.0.fetch_add(1, Ordering::SeqCst);
                }
            }

            let counter = CollisionCounter(Arc::new(AtomicUsize::new(0)));
            app.insert_resource(counter.clone());

            app.add_systems(Update, (fireball_collision_detection, count_collisions).chain());

            // Spawn fireball at origin
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
            ));

            // Spawn enemy far away (beyond collision radius)
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(10.0, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            ));

            app.update();

            assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn test_collision_effects_despawns_fireball() {
            let mut app = setup_test_app();

            // Chain detection and effects so events are processed
            app.add_systems(
                Update,
                (fireball_collision_detection, fireball_collision_effects).chain(),
            );

            let fireball_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
            )).id();

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Fireball should be despawned
            assert!(!app.world().entities().contains(fireball_entity));
            // Enemy should still exist
            assert!(app.world().entities().contains(enemy_entity));
        }

        #[test]
        fn test_collision_effects_applies_burn() {
            let mut app = setup_test_app();

            // Chain detection and effects so events are processed
            app.add_systems(
                Update,
                (fireball_collision_detection, fireball_collision_effects).chain(),
            );

            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec2::X, 20.0, 5.0, 15.0),
            ));

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)),
                Enemy { speed: 50.0, strength: 10.0 },
            )).id();

            app.update();

            // Enemy should have BurnEffect component
            let burn = app.world().get::<BurnEffect>(enemy_entity);
            assert!(burn.is_some(), "Enemy should have BurnEffect after fireball hit");
            assert_eq!(burn.unwrap().tick_damage, 15.0 * BURN_DAMAGE_RATIO);
        }
    }
}
