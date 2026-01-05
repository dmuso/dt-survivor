use std::collections::HashSet;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use crate::audio::plugin::*;
use crate::combat::DamageEvent;
use crate::element::Element;
use crate::enemies::components::Enemy;
use crate::game::events::FireballEnemyCollisionEvent;
use crate::game::resources::{GameMaterials, GameMeshes};
use crate::spell::components::Spell;
use super::fireball_effects::FireballEffects;

/// Default configuration for Fireball spell
pub const FIREBALL_SPREAD_ANGLE: f32 = 15.0;
pub const FIREBALL_SPEED: f32 = 20.0;
pub const FIREBALL_LIFETIME: f32 = 5.0;
pub const FIREBALL_SIZE: Vec2 = Vec2::new(0.3, 0.3);

/// Burn effect configuration
pub const BURN_TICK_INTERVAL: f32 = 0.5;
pub const BURN_TOTAL_DURATION: f32 = 3.0;
pub const BURN_DAMAGE_RATIO: f32 = 0.25; // 25% of direct damage per tick

/// Ground level for collision detection
pub const GROUND_LEVEL: f32 = 0.0;

/// Maximum trail length in shader units (before 4.0x multiplier in shader)
pub const FIREBALL_MAX_TRAIL_LENGTH: f32 = 0.75;
/// Trail grows to max length over this distance traveled (in world units)
pub const FIREBALL_TRAIL_GROW_DISTANCE: f32 = 6.0;

/// Marker component for fireball projectiles
#[derive(Component, Debug, Clone)]
pub struct FireballProjectile {
    /// Direction of travel in 3D space (normalized)
    pub direction: Vec3,
    /// Speed in units per second
    pub speed: f32,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Direct hit damage
    pub damage: f32,
    /// Burn damage per tick when applied
    pub burn_tick_damage: f32,
    /// Position where the fireball transitioned to flight phase
    pub spawn_position: Vec3,
}

impl FireballProjectile {
    /// Create a new fireball projectile with default spawn position at origin.
    pub fn new(direction: Vec3, speed: f32, lifetime_secs: f32, damage: f32) -> Self {
        Self::new_with_spawn(direction, speed, lifetime_secs, damage, Vec3::ZERO)
    }

    /// Create a new fireball projectile with explicit spawn position.
    /// The spawn position is used to calculate travel distance for trail growth.
    pub fn new_with_spawn(
        direction: Vec3,
        speed: f32,
        lifetime_secs: f32,
        damage: f32,
        spawn_position: Vec3,
    ) -> Self {
        let burn_tick_damage = damage * BURN_DAMAGE_RATIO;
        Self {
            direction: direction.normalize(),
            speed,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            damage,
            burn_tick_damage,
            spawn_position,
        }
    }

    pub fn from_spell(direction: Vec3, spell: &Spell) -> Self {
        Self::new(direction, FIREBALL_SPEED, FIREBALL_LIFETIME, spell.damage())
    }

    /// Calculate how far the fireball has traveled from its spawn position.
    pub fn travel_distance(&self, current_position: Vec3) -> f32 {
        current_position.distance(self.spawn_position)
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

/// Charge phase duration in seconds
pub const FIREBALL_CHARGE_DURATION: f32 = 0.5;
/// Height offset above player during charge phase
pub const FIREBALL_CHARGE_HEIGHT: f32 = 1.5;
/// Default enemy center height (half of enemy mesh height 1.5)
pub const ENEMY_CENTER_HEIGHT: f32 = 0.75;

/// Fireball in charge phase - spawns above player, grows with particles
#[derive(Component, Debug, Clone)]
pub struct ChargingFireball {
    /// Timer for the charge duration
    pub charge_timer: Timer,
    /// Target direction in 3D space (toward enemy center)
    pub target_direction: Vec3,
    /// Final damage to deal on hit
    pub damage: f32,
    /// Burn tick damage
    pub burn_tick_damage: f32,
    /// Starting scale (small)
    pub start_scale: f32,
    /// Final scale (full size)
    pub end_scale: f32,
}

impl ChargingFireball {
    pub fn new(target_direction: Vec3, damage: f32) -> Self {
        Self {
            charge_timer: Timer::from_seconds(FIREBALL_CHARGE_DURATION, TimerMode::Once),
            target_direction: target_direction.normalize(),
            damage,
            burn_tick_damage: damage * BURN_DAMAGE_RATIO,
            start_scale: 0.1,
            end_scale: 1.0,
        }
    }

    /// Get current scale based on charge progress (0.0 to 1.0)
    /// Uses ease-out cubic for satisfying growth
    pub fn current_scale(&self) -> f32 {
        let t = self.charge_timer.fraction();
        let eased = 1.0 - (1.0 - t).powi(3);
        self.start_scale + (self.end_scale - self.start_scale) * eased
    }

    /// Check if charge is complete
    pub fn is_finished(&self) -> bool {
        self.charge_timer.is_finished()
    }
}

/// Marker for the charge particle effect entity (child of charging fireball)
#[derive(Component, Debug)]
pub struct FireballChargeParticles;

/// Marker for the trail particle effect entity (child of active fireball)
#[derive(Component, Debug)]
pub struct FireballTrailParticles;

/// Marker for the spark particle effect entity (child of active fireball)
#[derive(Component, Debug)]
pub struct FireballSparkParticles;

/// Component for the shader-based charge effect entity
/// Stores the material handle so we can update charge progress
#[derive(Component, Debug)]
pub struct FireballChargeEffect {
    /// Handle to the charge material for this entity
    pub material_handle: Handle<super::materials::FireballChargeMaterial>,
}

/// Component for the shader-based trail effect entity (comet tail)
/// Stores the material handle so we can update velocity direction
#[derive(Component, Debug)]
pub struct FireballTrailEffect {
    /// Handle to the trail material for this entity
    pub material_handle: Handle<super::materials::FireballTrailMaterial>,
}

/// Component for the shader-based fireball core effect entity
/// Stores the material handle for the volumetric fire sphere
#[derive(Component, Debug)]
pub struct FireballCoreEffect {
    /// Handle to the core material for this entity
    pub material_handle: Handle<super::materials::FireballCoreMaterial>,
}

/// Component for the shader-based explosion core flash effect
/// Stores the material handle and lifetime timer for progress updates
#[derive(Component, Debug)]
pub struct ExplosionCoreEffect {
    /// Handle to the explosion core material for this entity
    pub material_handle: Handle<super::materials::ExplosionCoreMaterial>,
    /// Lifetime timer (0.25s for flash)
    pub lifetime: Timer,
}

impl ExplosionCoreEffect {
    pub fn new(material_handle: Handle<super::materials::ExplosionCoreMaterial>) -> Self {
        Self {
            material_handle,
            lifetime: Timer::from_seconds(0.25, TimerMode::Once),
        }
    }

    /// Get progress (0.0 to 1.0) through the lifetime
    pub fn progress(&self) -> f32 {
        self.lifetime.fraction()
    }

    /// Check if the effect is finished
    pub fn is_finished(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// Component for the shader-based explosion fire blast effect
/// Stores the material handle and lifetime timer for progress updates
#[derive(Component, Debug)]
pub struct ExplosionFireEffect {
    /// Handle to the explosion fire material for this entity
    pub material_handle: Handle<super::materials::ExplosionFireMaterial>,
    /// Lifetime timer (0.6s for fire blast)
    pub lifetime: Timer,
}

impl ExplosionFireEffect {
    pub fn new(material_handle: Handle<super::materials::ExplosionFireMaterial>) -> Self {
        Self {
            material_handle,
            lifetime: Timer::from_seconds(0.6, TimerMode::Once),
        }
    }

    /// Get progress (0.0 to 1.0) through the lifetime
    pub fn progress(&self) -> f32 {
        self.lifetime.fraction()
    }

    /// Check if the effect is finished
    pub fn is_finished(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// Component for the shader-based explosion embers effect (flying debris)
/// Stores the material handle and lifetime timer for progress updates
#[derive(Component, Debug)]
pub struct ExplosionEmbersEffect {
    /// Handle to the explosion embers material for this entity
    pub material_handle: Handle<super::materials::ExplosionEmbersMaterial>,
    /// Lifetime timer (0.8s for flying embers)
    pub lifetime: Timer,
}

impl ExplosionEmbersEffect {
    pub fn new(material_handle: Handle<super::materials::ExplosionEmbersMaterial>) -> Self {
        Self {
            material_handle,
            lifetime: Timer::from_seconds(0.8, TimerMode::Once),
        }
    }

    /// Get progress (0.0 to 1.0) through the lifetime
    pub fn progress(&self) -> f32 {
        self.lifetime.fraction()
    }

    /// Check if the effect is finished
    pub fn is_finished(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// Component for the shader-based explosion smoke effect (rising plume)
/// Stores the material handle and lifetime timer for progress updates
#[derive(Component, Debug)]
pub struct ExplosionSmokeEffect {
    /// Handle to the explosion smoke material for this entity
    pub material_handle: Handle<super::materials::ExplosionSmokeMaterial>,
    /// Lifetime timer (1.2s for rising smoke)
    pub lifetime: Timer,
}

impl ExplosionSmokeEffect {
    pub fn new(material_handle: Handle<super::materials::ExplosionSmokeMaterial>) -> Self {
        Self {
            material_handle,
            lifetime: Timer::from_seconds(1.2, TimerMode::Once),
        }
    }

    /// Get progress (0.0 to 1.0) through the lifetime
    pub fn progress(&self) -> f32 {
        self.lifetime.fraction()
    }

    /// Check if the effect is finished
    pub fn is_finished(&self) -> bool {
        self.lifetime.is_finished()
    }
}

/// Explosion particles (self-despawns after cleanup timer)
#[derive(Component, Debug)]
pub struct FireballExplosionParticles {
    pub cleanup_timer: Timer,
}

// ============================================================================
// Multi-Puff Smoke System - Replaces single-sphere smoke with billowing cloud
// ============================================================================

/// Configuration constants for smoke puff system
pub const SMOKE_PUFF_COUNT: u32 = 12;
pub const SMOKE_SPAWN_DURATION: f32 = 0.6;
pub const SMOKE_PUFF_LIFETIME_BASE: f32 = 1.8;
pub const SMOKE_PUFF_LIFETIME_VARIANCE: f32 = 0.4;
pub const SMOKE_PUFF_RISE_SPEED_BASE: f32 = 1.5;
pub const SMOKE_PUFF_RISE_SPEED_VARIANCE: f32 = 0.6;
pub const SMOKE_PUFF_INITIAL_SCALE: f32 = 0.4;   // Smaller puffs
pub const SMOKE_PUFF_MAX_SCALE_BASE: f32 = 1.0;  // Stay smaller
pub const SMOKE_PUFF_MAX_SCALE_VARIANCE: f32 = 0.3;
pub const SMOKE_PUFF_DRIFT_SPEED_BASE: f32 = 0.3;
pub const SMOKE_PUFF_DRIFT_SPEED_VARIANCE: f32 = 0.4;
pub const SMOKE_PUFF_SPAWN_SPREAD: f32 = 0.8;

/// Individual smoke puff - many of these create billowing smoke
#[derive(Component, Debug)]
pub struct SmokePuffEffect {
    /// Handle to the smoke material for this puff
    pub material_handle: Handle<super::materials::ExplosionSmokeMaterial>,
    /// Lifetime timer for this individual puff
    pub lifetime: Timer,
    /// Starting scale (small)
    pub initial_scale: f32,
    /// Final scale when fully expanded
    pub max_scale: f32,
    /// Vertical rise speed in units per second
    pub rise_speed: f32,
    /// Horizontal drift velocity (X, Z)
    pub drift_velocity: Vec2,
}

impl SmokePuffEffect {
    /// Get progress (0.0 to 1.0) through the lifetime
    pub fn progress(&self) -> f32 {
        self.lifetime.fraction()
    }

    /// Check if the puff is finished
    pub fn is_finished(&self) -> bool {
        self.lifetime.is_finished()
    }

    /// Calculate current scale based on progress with ease-out curve
    pub fn current_scale(&self) -> f32 {
        let t = self.progress();
        let eased = 1.0 - (1.0 - t).powi(2);
        self.initial_scale + (self.max_scale - self.initial_scale) * eased
    }
}

/// Spawns smoke puffs over time at explosion location
#[derive(Component, Debug)]
pub struct SmokePuffSpawner {
    /// Position to spawn puffs around
    pub position: Vec3,
    /// Timer between puff spawns
    pub spawn_timer: Timer,
    /// How many more puffs to spawn
    pub puffs_remaining: u32,
    /// Random number generator seed for deterministic randomness
    pub seed: u32,
}

impl SmokePuffSpawner {
    pub fn new(position: Vec3) -> Self {
        let spawn_interval = SMOKE_SPAWN_DURATION / SMOKE_PUFF_COUNT as f32;
        Self {
            position,
            spawn_timer: Timer::from_seconds(spawn_interval, TimerMode::Repeating),
            puffs_remaining: SMOKE_PUFF_COUNT,
            seed: position.x.to_bits() ^ position.z.to_bits(),
        }
    }

    /// Get next pseudo-random value (0.0 to 1.0) and advance seed
    pub fn next_random(&mut self) -> f32 {
        // Simple LCG for deterministic randomness
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        (self.seed >> 16) as f32 / 65535.0
    }
}

impl Default for FireballExplosionParticles {
    fn default() -> Self {
        Self {
            cleanup_timer: Timer::from_seconds(1.0, TimerMode::Once),
        }
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
    _game_materials: Option<&GameMaterials>,
    fireball_effects: Option<&FireballEffects>,
    core_materials: Option<&mut Assets<super::materials::FireballCoreMaterial>>,
    charge_materials: Option<&mut Assets<super::materials::FireballChargeMaterial>>,
    trail_materials: Option<&mut Assets<super::materials::FireballTrailMaterial>>,
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
        fireball_effects,
        core_materials,
        charge_materials,
        trail_materials,
    );
}

/// Cast fireball spell with explicit damage - spawns charging fireball with particle effects
/// `spawn_position` is Whisper's full 3D position, `target_pos` is enemy position on XZ plane
/// `damage` is the pre-calculated final damage (including attunement multiplier)
///
/// The fireball now goes through phases:
/// 1. Charge phase (0.5s): Spawns above player, grows with swirling particles
/// 2. Flight phase: Flies in 3D toward enemy center, can hit ground
/// 3. Explosion: Burst particles on enemy or ground hit
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
    _fireball_effects: Option<&FireballEffects>,
    core_materials: Option<&mut Assets<super::materials::FireballCoreMaterial>>,
    charge_materials: Option<&mut Assets<super::materials::FireballChargeMaterial>>,
    _trail_materials: Option<&mut Assets<super::materials::FireballTrailMaterial>>,
) {
    // Spawn position is above player (Whisper)
    let charge_position = spawn_position + Vec3::new(0.0, FIREBALL_CHARGE_HEIGHT, 0.0);

    // Target position in 3D: enemy XZ position + enemy center height
    let target_3d = Vec3::new(target_pos.x, ENEMY_CENTER_HEIGHT, target_pos.y);

    // Calculate 3D direction from charge position to enemy center
    let base_direction_3d = (target_3d - charge_position).normalize();

    // Get projectile count based on spell level (1 at level 1-4, 2 at 5-9, 3 at 10)
    let projectile_count = spell.projectile_count();
    let spread_angle_rad = FIREBALL_SPREAD_ANGLE.to_radians();

    // Pre-create core shader material handles for all projectiles (if available)
    let core_material_handles: Vec<_> = if let Some(core_mats) = core_materials {
        (0..projectile_count).map(|_| {
            let core_material = super::materials::FireballCoreMaterial::new();
            core_mats.add(core_material)
        }).collect()
    } else {
        Vec::new()
    };

    // Pre-create charge shader material handles for all projectiles (if available)
    let charge_material_handles: Vec<_> = if let Some(charge_mats) = charge_materials {
        (0..projectile_count).map(|_| {
            let mut charge_material = super::materials::FireballChargeMaterial::new();
            charge_material.set_outer_radius(1.0); // Same size as fireball
            charge_mats.add(charge_material)
        }).collect()
    } else {
        Vec::new()
    };

    // Create projectiles in a spread pattern centered around the target direction
    // Spread is applied as rotation around the Y axis (horizontal spread)
    for i in 0..projectile_count {
        let angle_offset = if projectile_count == 1 {
            0.0
        } else {
            let half_spread = (projectile_count - 1) as f32 / 2.0;
            (i as f32 - half_spread) * spread_angle_rad
        };

        // Rotate direction around Y axis for horizontal spread
        let cos_offset = angle_offset.cos();
        let sin_offset = angle_offset.sin();
        let direction = Vec3::new(
            base_direction_3d.x * cos_offset - base_direction_3d.z * sin_offset,
            base_direction_3d.y, // Keep Y component unchanged
            base_direction_3d.x * sin_offset + base_direction_3d.z * cos_offset,
        ).normalize();

        // Create ChargingFireball with 3D direction
        let charging = ChargingFireball::new(direction, damage);
        let initial_scale = charging.start_scale;

        // Spawn charging fireball above Whisper's position
        if let Some(meshes) = game_meshes {
            let fireball_mesh = meshes.fireball.clone();

            // Calculate rotation to face travel direction (for shader flame trailing)
            // NEG_Z is "forward" in Bevy convention, so Z column = -direction = trail direction
            let fireball_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);

            // Use shader material if available, otherwise spawn without material
            let mut entity_commands = if let Some(core_handle) = core_material_handles.get(i).cloned() {
                commands.spawn((
                    Mesh3d(meshes.fireball.clone()),
                    MeshMaterial3d(core_handle.clone()),
                    Transform::from_translation(charge_position)
                        .with_rotation(fireball_rotation)
                        .with_scale(Vec3::splat(initial_scale)),
                    charging,
                    FireballCoreEffect { material_handle: core_handle },
                ))
            } else {
                // Fallback: spawn without material (tests)
                commands.spawn((
                    Mesh3d(meshes.fireball.clone()),
                    Transform::from_translation(charge_position)
                        .with_rotation(fireball_rotation)
                        .with_scale(Vec3::splat(initial_scale)),
                    charging,
                ))
            };

            // Add charge shader effect as child (swirling energy gathering)
            // Note: Shader replaces particle effects for better visuals
            if let Some(material_handle) = charge_material_handles.get(i).cloned() {
                entity_commands.with_children(|parent| {
                    parent.spawn((
                        Mesh3d(fireball_mesh), // Reuse sphere mesh
                        MeshMaterial3d(material_handle.clone()),
                        Transform::from_scale(Vec3::splat(1.0)), // Same size as core
                        FireballChargeEffect { material_handle },
                    ));
                });
            }
        } else {
            // Fallback for tests without mesh resources
            let fireball_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);
            commands.spawn((
                Transform::from_translation(charge_position)
                    .with_rotation(fireball_rotation)
                    .with_scale(Vec3::splat(initial_scale)),
                charging,
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

/// System that moves fireball projectiles in 3D space
pub fn fireball_movement_system(
    mut fireball_query: Query<(&mut Transform, &FireballProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, fireball) in fireball_query.iter_mut() {
        // Full 3D movement toward target
        let movement = fireball.direction * fireball.speed * time.delta_secs();
        transform.translation += movement;
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

// ============================================================================
// Enhanced Fireball Systems (Charge Phase, Particles, Explosion)
// ============================================================================

/// System that adds FireballCoreMaterial to newly spawned charging fireballs
/// This is run separately to avoid parameter count limits in the spell casting system
pub fn fireball_add_core_material_system(
    mut commands: Commands,
    query: Query<Entity, (With<ChargingFireball>, Without<FireballCoreEffect>)>,
    game_meshes: Option<Res<GameMeshes>>,
    mut core_materials: Option<ResMut<Assets<super::materials::FireballCoreMaterial>>>,
) {
    let Some(core_mats) = core_materials.as_mut() else {
        return; // No material assets available (e.g., in tests)
    };
    let Some(_meshes) = game_meshes.as_ref() else {
        return; // No meshes available
    };

    for entity in query.iter() {
        // Create and add the core material
        let core_material = super::materials::FireballCoreMaterial::new();
        let core_handle = core_mats.add(core_material);

        commands.entity(entity).insert((
            MeshMaterial3d(core_handle.clone()),
            FireballCoreEffect { material_handle: core_handle },
        ));
    }
}

/// System that updates charging fireballs - scales up and ticks timer
pub fn fireball_charge_update_system(
    time: Res<Time>,
    mut query: Query<(&mut ChargingFireball, &mut Transform)>,
) {
    for (mut charging, mut transform) in query.iter_mut() {
        charging.charge_timer.tick(time.delta());

        // Update scale based on charge progress
        let scale = charging.current_scale();
        transform.scale = Vec3::splat(scale);
    }
}

/// System that updates charge effect shader materials with current charge progress
pub fn fireball_charge_effect_update_system(
    parent_query: Query<&ChargingFireball>,
    child_query: Query<(&ChildOf, &FireballChargeEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::FireballChargeMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    for (child_of, charge_effect) in child_query.iter() {
        // Get the parent's charge progress
        if let Ok(charging) = parent_query.get(child_of.parent()) {
            let progress = charging.charge_timer.fraction();
            // Update the material's charge progress
            if let Some(material) = materials.get_mut(&charge_effect.material_handle) {
                material.set_charge_progress(progress);
            }
        }
    }
}

/// System that updates trail effect shader materials with velocity direction and trail length.
/// Trail length grows dynamically based on how far the fireball has traveled from spawn.
pub fn fireball_trail_effect_update_system(
    parent_query: Query<(&Transform, &FireballProjectile)>,
    child_query: Query<(&ChildOf, &FireballTrailEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::FireballTrailMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    for (child_of, trail_effect) in child_query.iter() {
        // Get the parent's transform and fireball data
        if let Ok((transform, fireball)) = parent_query.get(child_of.parent()) {
            if let Some(material) = materials.get_mut(&trail_effect.material_handle) {
                // Update velocity direction
                material.set_velocity_direction(fireball.direction);

                // Update trail length based on travel distance
                let travel_distance = fireball.travel_distance(transform.translation);
                let trail_progress = (travel_distance / FIREBALL_TRAIL_GROW_DISTANCE).clamp(0.0, 1.0);
                let trail_length = trail_progress * FIREBALL_MAX_TRAIL_LENGTH;
                material.set_trail_length(trail_length);
            }
        }
    }
}

/// System that updates core effect shader materials with current velocity direction
/// This makes flames trail behind the fireball based on its travel direction
pub fn fireball_core_effect_update_system(
    fireball_query: Query<(&FireballProjectile, &FireballCoreEffect)>,
    charging_query: Query<(&ChargingFireball, &FireballCoreEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::FireballCoreMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    // Update active fireballs
    for (fireball, core_effect) in fireball_query.iter() {
        if let Some(material) = materials.get_mut(&core_effect.material_handle) {
            material.set_velocity_direction(fireball.direction);
        }
    }

    // Update charging fireballs (use target direction)
    for (charging, core_effect) in charging_query.iter() {
        if let Some(material) = materials.get_mut(&core_effect.material_handle) {
            material.set_velocity_direction(charging.target_direction);
        }
    }
}

/// System that transitions charging fireballs to active flight phase
pub fn fireball_charge_to_flight_system(
    mut commands: Commands,
    query: Query<(Entity, &ChargingFireball, &Transform, Option<&Children>)>,
    _fireball_effects: Option<Res<FireballEffects>>,
    charge_particles_query: Query<Entity, With<FireballChargeParticles>>,
    charge_effect_query: Query<Entity, With<FireballChargeEffect>>,
    game_meshes: Option<Res<GameMeshes>>,
    mut trail_materials: Option<ResMut<Assets<super::materials::FireballTrailMaterial>>>,
) {
    for (entity, charging, transform, children) in query.iter() {
        if charging.is_finished() {
            // Remove charge particles and charge effect shader
            if let Some(children) = children {
                for child in children.iter() {
                    if charge_particles_query.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                    if charge_effect_query.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                }
            }

            // Create FireballProjectile from ChargingFireball
            // Store the current position as spawn_position for trail length calculation
            let fireball = FireballProjectile {
                direction: charging.target_direction,
                speed: FIREBALL_SPEED,
                lifetime: Timer::from_seconds(FIREBALL_LIFETIME, TimerMode::Once),
                damage: charging.damage,
                burn_tick_damage: charging.burn_tick_damage,
                spawn_position: transform.translation,
            };

            // Update the entity: remove ChargingFireball, add FireballProjectile
            commands.entity(entity)
                .remove::<ChargingFireball>()
                .insert(fireball);

            // Add shader-based trail effect (comet tail)
            // Trail starts at zero length and grows as fireball travels
            if let (Some(meshes), Some(ref mut trail_mats)) = (&game_meshes, &mut trail_materials) {
                let mut trail_material = super::materials::FireballTrailMaterial::new();
                trail_material.set_velocity_direction(charging.target_direction);
                trail_material.set_trail_length(0.0); // Start at zero; grows via update system
                let trail_handle = trail_mats.add(trail_material);

                commands.entity(entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(meshes.fireball.clone()),
                        MeshMaterial3d(trail_handle.clone()),
                        Transform::default(), // Same scale as core; length from shader
                        FireballTrailEffect { material_handle: trail_handle },
                    ));
                });
            }

            // Note: Sparks are now handled by the trail shader effect
            // The FireballSparksMaterial could be used for additional spark entities if desired
        }
    }
}

/// System that spawns explosion shader effects at collision point
/// Spawns all four explosion layers for maximum impact using shader materials
#[allow(clippy::too_many_arguments)]
pub fn fireball_explosion_spawn_system(
    mut commands: Commands,
    mut collision_events: MessageReader<FireballEnemyCollisionEvent>,
    fireball_query: Query<&Transform, With<FireballProjectile>>,
    game_meshes: Option<Res<GameMeshes>>,
    mut explosion_core_materials: Option<ResMut<Assets<super::materials::ExplosionCoreMaterial>>>,
    mut explosion_fire_materials: Option<ResMut<Assets<super::materials::ExplosionFireMaterial>>>,
    mut explosion_embers_materials: Option<ResMut<Assets<super::materials::ExplosionEmbersMaterial>>>,
) {
    let _event_count = collision_events.len();
    for event in collision_events.read() {
        let Ok(transform) = fireball_query.get(event.fireball_entity) else {
            warn!("Could not find fireball transform for entity {:?}", event.fireball_entity);
            continue;
        };
        let pos = transform.translation;

        // Spawn shader-based explosion core flash effect (white-hot burst)
        if let (Some(meshes), Some(ref mut core_mats)) = (&game_meshes, &mut explosion_core_materials) {
            let core_material = super::materials::ExplosionCoreMaterial::new();
            let core_handle = core_mats.add(core_material);

            commands.spawn((
                Mesh3d(meshes.fireball.clone()),
                MeshMaterial3d(core_handle.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(2.5)),
                ExplosionCoreEffect::new(core_handle),
            ));
        }

        // Spawn shader-based explosion fire blast effect (main orange-red blast)
        if let (Some(meshes), Some(ref mut fire_mats)) = (&game_meshes, &mut explosion_fire_materials) {
            let fire_material = super::materials::ExplosionFireMaterial::new();
            let fire_handle = fire_mats.add(fire_material);

            commands.spawn((
                Mesh3d(meshes.fireball.clone()),
                MeshMaterial3d(fire_handle.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(3.5)),
                ExplosionFireEffect::new(fire_handle),
            ));
        }

        // Spawn shader-based explosion embers effect (flying debris)
        if let (Some(meshes), Some(ref mut embers_mats)) = (&game_meshes, &mut explosion_embers_materials) {
            let embers_material = super::materials::ExplosionEmbersMaterial::new();
            let embers_handle = embers_mats.add(embers_material);

            commands.spawn((
                Mesh3d(meshes.fireball.clone()),
                MeshMaterial3d(embers_handle.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(4.0)),
                ExplosionEmbersEffect::new(embers_handle),
            ));
        }

        // Spawn smoke puff spawner (creates multiple puffs over time)
        commands.spawn(SmokePuffSpawner::new(pos));
    }
}

/// System that checks for ground collision and spawns explosion
/// Fireballs that hit the ground explode without dealing damage
#[allow(clippy::too_many_arguments)]
pub fn fireball_ground_collision_system(
    mut commands: Commands,
    fireball_query: Query<(Entity, &Transform), With<FireballProjectile>>,
    game_meshes: Option<Res<GameMeshes>>,
    mut explosion_core_materials: Option<ResMut<Assets<super::materials::ExplosionCoreMaterial>>>,
    mut explosion_fire_materials: Option<ResMut<Assets<super::materials::ExplosionFireMaterial>>>,
    mut explosion_embers_materials: Option<ResMut<Assets<super::materials::ExplosionEmbersMaterial>>>,
) {
    for (entity, transform) in fireball_query.iter() {
        // Check if fireball has hit the ground (accounting for fireball radius)
        if transform.translation.y <= GROUND_LEVEL + 0.1 {
            let pos = Vec3::new(
                transform.translation.x,
                GROUND_LEVEL + 0.1, // Slightly above ground
                transform.translation.z,
            );

            // Spawn shader-based explosion core flash effect (white-hot burst)
            if let (Some(meshes), Some(ref mut core_mats)) = (&game_meshes, &mut explosion_core_materials) {
                let core_material = super::materials::ExplosionCoreMaterial::new();
                let core_handle = core_mats.add(core_material);

                commands.spawn((
                    Mesh3d(meshes.fireball.clone()),
                    MeshMaterial3d(core_handle.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::splat(2.5)),
                    ExplosionCoreEffect::new(core_handle),
                ));
            }

            // Spawn shader-based explosion fire blast effect (main orange-red blast)
            if let (Some(meshes), Some(ref mut fire_mats)) = (&game_meshes, &mut explosion_fire_materials) {
                let fire_material = super::materials::ExplosionFireMaterial::new();
                let fire_handle = fire_mats.add(fire_material);

                commands.spawn((
                    Mesh3d(meshes.fireball.clone()),
                    MeshMaterial3d(fire_handle.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::splat(3.5)),
                    ExplosionFireEffect::new(fire_handle),
                ));
            }

            // Spawn shader-based explosion embers effect (flying debris)
            if let (Some(meshes), Some(ref mut embers_mats)) = (&game_meshes, &mut explosion_embers_materials) {
                let embers_material = super::materials::ExplosionEmbersMaterial::new();
                let embers_handle = embers_mats.add(embers_material);

                commands.spawn((
                    Mesh3d(meshes.fireball.clone()),
                    MeshMaterial3d(embers_handle.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::splat(4.0)),
                    ExplosionEmbersEffect::new(embers_handle),
                ));
            }

            // Spawn smoke puff spawner (creates multiple puffs over time)
            commands.spawn(SmokePuffSpawner::new(pos));

            // Despawn the fireball
            commands.entity(entity).despawn();
        }
    }
}

/// System that cleans up explosion particles after their timer expires
pub fn fireball_explosion_cleanup_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FireballExplosionParticles)>,
) {
    for (entity, mut explosion) in query.iter_mut() {
        explosion.cleanup_timer.tick(time.delta());
        if explosion.cleanup_timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates explosion core shader effects (progress and cleanup)
pub fn explosion_core_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ExplosionCoreEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::ExplosionCoreMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    for (entity, mut effect) in query.iter_mut() {
        effect.lifetime.tick(time.delta());
        let progress = effect.progress();

        // Update the material's progress
        if let Some(material) = materials.get_mut(&effect.material_handle) {
            material.set_progress(progress);
        }

        // Despawn when finished
        if effect.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates explosion fire shader effects (progress and cleanup)
pub fn explosion_fire_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ExplosionFireEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::ExplosionFireMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    for (entity, mut effect) in query.iter_mut() {
        effect.lifetime.tick(time.delta());
        let progress = effect.progress();

        // Update the material's progress
        if let Some(material) = materials.get_mut(&effect.material_handle) {
            material.set_progress(progress);
        }

        // Despawn when finished
        if effect.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates explosion embers shader effects (progress and cleanup)
pub fn explosion_embers_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ExplosionEmbersEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::ExplosionEmbersMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    for (entity, mut effect) in query.iter_mut() {
        effect.lifetime.tick(time.delta());
        let progress = effect.progress();

        // Update the material's progress
        if let Some(material) = materials.get_mut(&effect.material_handle) {
            material.set_progress(progress);
        }

        // Despawn when finished
        if effect.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates explosion smoke shader effects (progress and cleanup)
pub fn explosion_smoke_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ExplosionSmokeEffect)>,
    mut materials: Option<ResMut<Assets<super::materials::ExplosionSmokeMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return; // No material assets available (e.g., in tests without MaterialPlugin)
    };

    for (entity, mut effect) in query.iter_mut() {
        effect.lifetime.tick(time.delta());
        let progress = effect.progress();

        // Update the material's progress
        if let Some(material) = materials.get_mut(&effect.material_handle) {
            material.set_progress(progress);
        }

        // Despawn when finished
        if effect.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// Multi-Puff Smoke Systems
// ============================================================================

/// System that spawns smoke puffs over time from SmokePuffSpawner entities
#[allow(clippy::too_many_arguments)]
pub fn smoke_puff_spawner_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SmokePuffSpawner)>,
    game_meshes: Option<Res<crate::game::resources::GameMeshes>>,
    mut smoke_materials: Option<ResMut<Assets<super::materials::ExplosionSmokeMaterial>>>,
) {
    let Some(meshes) = game_meshes.as_ref() else {
        return;
    };
    let Some(materials) = smoke_materials.as_mut() else {
        return;
    };

    for (entity, mut spawner) in query.iter_mut() {
        spawner.spawn_timer.tick(time.delta());

        // Spawn first puff immediately, then on timer
        let should_spawn = spawner.puffs_remaining == SMOKE_PUFF_COUNT
            || spawner.spawn_timer.just_finished();

        if should_spawn && spawner.puffs_remaining > 0 {
            // Random offset from center
            let offset_x = (spawner.next_random() - 0.5) * SMOKE_PUFF_SPAWN_SPREAD;
            let offset_z = (spawner.next_random() - 0.5) * SMOKE_PUFF_SPAWN_SPREAD;
            let offset = Vec3::new(offset_x, 0.0, offset_z);

            // Random drift direction
            let drift_angle = spawner.next_random() * std::f32::consts::TAU;
            let drift_speed = SMOKE_PUFF_DRIFT_SPEED_BASE
                + spawner.next_random() * SMOKE_PUFF_DRIFT_SPEED_VARIANCE;
            let drift_velocity = Vec2::new(
                drift_angle.cos() * drift_speed,
                drift_angle.sin() * drift_speed,
            );

            // Random lifetime and rise speed
            let lifetime = SMOKE_PUFF_LIFETIME_BASE
                + spawner.next_random() * SMOKE_PUFF_LIFETIME_VARIANCE;
            let rise_speed = SMOKE_PUFF_RISE_SPEED_BASE
                + spawner.next_random() * SMOKE_PUFF_RISE_SPEED_VARIANCE;
            let max_scale = SMOKE_PUFF_MAX_SCALE_BASE
                + spawner.next_random() * SMOKE_PUFF_MAX_SCALE_VARIANCE;

            // Create material for this puff
            let material = super::materials::ExplosionSmokeMaterial::new();
            let handle = materials.add(material);

            // Spawn puff just above explosion fire sphere (fire has scale 1.5 centered at y=1)
            // so puffs start at y ≈ 2.5 and rise from there
            let puff_pos = spawner.position + offset + Vec3::Y * 1.5;

            commands.spawn((
                Mesh3d(meshes.explosion.clone()),
                MeshMaterial3d(handle.clone()),
                Transform::from_translation(puff_pos)
                    .with_scale(Vec3::splat(SMOKE_PUFF_INITIAL_SCALE)),
                SmokePuffEffect {
                    material_handle: handle,
                    lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
                    initial_scale: SMOKE_PUFF_INITIAL_SCALE,
                    max_scale,
                    rise_speed,
                    drift_velocity,
                },
            ));

            spawner.puffs_remaining -= 1;
        }

        // Cleanup spawner when done
        if spawner.puffs_remaining == 0 {
            commands.entity(entity).despawn();
        }
    }
}

/// System that updates smoke puff effects (scale, position, material progress, cleanup)
pub fn smoke_puff_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SmokePuffEffect, &mut Transform)>,
    mut materials: Option<ResMut<Assets<super::materials::ExplosionSmokeMaterial>>>,
) {
    let Some(materials) = materials.as_mut() else {
        return;
    };

    for (entity, mut puff, mut transform) in query.iter_mut() {
        puff.lifetime.tick(time.delta());
        let progress = puff.progress();

        // Update material progress for fade
        if let Some(mat) = materials.get_mut(&puff.material_handle) {
            mat.set_progress(progress);
        }

        // Expand: small -> large with ease-out curve
        transform.scale = Vec3::splat(puff.current_scale());

        // Rise upward + horizontal drift
        let dt = time.delta_secs();
        transform.translation.y += puff.rise_speed * dt;
        transform.translation.x += puff.drift_velocity.x * dt;
        transform.translation.z += puff.drift_velocity.y * dt;

        // Despawn when finished
        if puff.is_finished() {
            commands.entity(entity).despawn();
        }
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
            let direction = Vec3::new(1.0, 0.0, 0.0);
            let fireball = FireballProjectile::new(direction, 20.0, 5.0, 15.0);

            assert_eq!(fireball.direction, direction.normalize());
            assert_eq!(fireball.speed, 20.0);
            assert_eq!(fireball.damage, 15.0);
            assert_eq!(fireball.burn_tick_damage, 15.0 * BURN_DAMAGE_RATIO);
        }

        #[test]
        fn test_fireball_from_spell() {
            let spell = Spell::new(SpellType::Fireball);
            let direction = Vec3::new(0.0, 0.0, 1.0);
            let fireball = FireballProjectile::from_spell(direction, &spell);

            assert_eq!(fireball.direction, direction.normalize());
            assert_eq!(fireball.speed, FIREBALL_SPEED);
            assert_eq!(fireball.damage, spell.damage());
        }

        #[test]
        fn test_fireball_lifetime_timer() {
            let fireball = FireballProjectile::new(Vec3::X, 20.0, 5.0, 15.0);
            assert_eq!(fireball.lifetime.duration(), Duration::from_secs_f32(5.0));
            assert!(!fireball.lifetime.is_finished());
        }

        #[test]
        fn test_fireball_uses_fire_element_color() {
            let color = fireball_color();
            assert_eq!(color, Element::Fire.color());
            assert_eq!(color, Color::srgb_u8(255, 128, 0));
        }

        #[test]
        fn test_fireball_projectile_tracks_spawn_position() {
            let spawn_pos = Vec3::new(5.0, 2.0, 3.0);
            let fireball = FireballProjectile::new_with_spawn(
                Vec3::X, 20.0, 5.0, 15.0, spawn_pos
            );
            assert_eq!(fireball.spawn_position, spawn_pos);
        }

        #[test]
        fn test_fireball_travel_distance_calculation() {
            let spawn_pos = Vec3::new(0.0, 1.0, 0.0);
            let current_pos = Vec3::new(3.0, 1.0, 4.0);  // 5.0 units away
            let fireball = FireballProjectile::new_with_spawn(
                Vec3::X, 20.0, 5.0, 15.0, spawn_pos
            );
            assert!((fireball.travel_distance(current_pos) - 5.0).abs() < 0.001);
        }

        #[test]
        fn test_fireball_travel_distance_is_zero_at_spawn() {
            let spawn_pos = Vec3::new(3.0, 2.0, 1.0);
            let fireball = FireballProjectile::new_with_spawn(
                Vec3::X, 20.0, 5.0, 15.0, spawn_pos
            );
            assert!((fireball.travel_distance(spawn_pos) - 0.0).abs() < 0.001);
        }

        #[test]
        fn test_fireball_new_defaults_spawn_position_to_zero() {
            let fireball = FireballProjectile::new(Vec3::X, 20.0, 5.0, 15.0);
            assert_eq!(fireball.spawn_position, Vec3::ZERO);
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
        fn test_fire_fireball_spawns_charging_fireball() {
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
                    None, // No particle effects in test
                    None, // No core materials in test
                    None, // No charge materials in test
                    None, // No trail materials in test
                );
            }
            app.update();

            // Should have spawned 1 charging fireball (level 1)
            let mut query = app.world_mut().query::<&ChargingFireball>();
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
                    None, // No particle effects in test
                    None, // No core materials in test
                    None, // No charge materials in test
                    None, // No trail materials in test
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChargingFireball>();
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
                    None, // No particle effects in test
                    None, // No core materials in test
                    None, // No charge materials in test
                    None, // No trail materials in test
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChargingFireball>();
            for charging in query.iter(app.world()) {
                // Direction should point toward +X
                assert!(charging.target_direction.x > 0.9, "Fireball should target +X direction");
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
                    None, // No particle effects in test
                    None, // No core materials in test
                    None, // No charge materials in test
                    None, // No trail materials in test
                );
            }
            app.update();

            let mut query = app.world_mut().query::<&ChargingFireball>();
            for charging in query.iter(app.world()) {
                assert_eq!(charging.damage, expected_damage);
                assert_eq!(charging.burn_tick_damage, expected_damage * BURN_DAMAGE_RATIO);
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

            // Create fireball moving in +X direction (3D direction)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec3::new(1.0, 0.0, 0.0), 100.0, 5.0, 15.0),
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

            // Create fireball moving in +Z direction (3D direction)
            let entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                FireballProjectile::new(Vec3::new(0.0, 0.0, 1.0), 50.0, 5.0, 15.0),
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
                FireballProjectile::new(Vec3::X, 100.0, 5.0, 15.0),
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
                FireballProjectile::new(Vec3::X, 100.0, 5.0, 15.0),
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
                FireballProjectile::new(Vec3::X, 20.0, 5.0, 15.0),
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
                FireballProjectile::new(Vec3::X, 20.0, 5.0, 15.0),
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
                FireballProjectile::new(Vec3::X, 20.0, 5.0, 15.0),
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
                FireballProjectile::new(Vec3::X, 20.0, 5.0, 15.0),
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

    mod explosion_effect_tests {
        use super::*;

        #[test]
        fn test_explosion_core_effect_new() {
            let handle = Handle::default();
            let effect = ExplosionCoreEffect::new(handle);
            assert_eq!(effect.lifetime.duration(), Duration::from_secs_f32(0.25));
            assert!(!effect.is_finished());
            assert_eq!(effect.progress(), 0.0);
        }

        #[test]
        fn test_explosion_core_effect_progress() {
            let handle = Handle::default();
            let mut effect = ExplosionCoreEffect::new(handle);

            // Tick halfway through lifetime
            effect.lifetime.tick(Duration::from_secs_f32(0.125));
            assert!((effect.progress() - 0.5).abs() < 0.01);
            assert!(!effect.is_finished());
        }

        #[test]
        fn test_explosion_core_effect_finished() {
            let handle = Handle::default();
            let mut effect = ExplosionCoreEffect::new(handle);

            // Tick past lifetime
            effect.lifetime.tick(Duration::from_secs_f32(0.3));
            assert!(effect.is_finished());
            assert_eq!(effect.progress(), 1.0);
        }

        #[test]
        fn test_explosion_fire_effect_new() {
            let handle = Handle::default();
            let effect = ExplosionFireEffect::new(handle);
            assert_eq!(effect.lifetime.duration(), Duration::from_secs_f32(0.6));
            assert!(!effect.is_finished());
            assert_eq!(effect.progress(), 0.0);
        }

        #[test]
        fn test_explosion_fire_effect_progress() {
            let handle = Handle::default();
            let mut effect = ExplosionFireEffect::new(handle);

            // Tick halfway through lifetime
            effect.lifetime.tick(Duration::from_secs_f32(0.3));
            assert!((effect.progress() - 0.5).abs() < 0.01);
            assert!(!effect.is_finished());
        }

        #[test]
        fn test_explosion_fire_effect_finished() {
            let handle = Handle::default();
            let mut effect = ExplosionFireEffect::new(handle);

            // Tick past lifetime
            effect.lifetime.tick(Duration::from_secs_f32(0.7));
            assert!(effect.is_finished());
            assert_eq!(effect.progress(), 1.0);
        }
    }

    mod fireball_core_effect_tests {
        use super::*;

        #[test]
        fn test_fireball_core_effect_component_exists() {
            // Verify the component can be created
            let handle = Handle::default();
            let effect = FireballCoreEffect { material_handle: handle };
            // No panic = success
            let _ = effect.material_handle;
        }
    }

    mod smoke_puff_tests {
        use super::*;

        #[test]
        fn test_smoke_puff_spawner_new() {
            let position = Vec3::new(1.0, 2.0, 3.0);
            let spawner = SmokePuffSpawner::new(position);

            assert_eq!(spawner.position, position);
            assert_eq!(spawner.puffs_remaining, SMOKE_PUFF_COUNT);
            // Seed is derived from position
            assert_eq!(spawner.seed, position.x.to_bits() ^ position.z.to_bits());
        }

        #[test]
        fn test_smoke_puff_spawner_random_sequence() {
            let mut spawner = SmokePuffSpawner::new(Vec3::new(10.0, 0.0, 20.0));

            // Generate several random values
            let val1 = spawner.next_random();
            let val2 = spawner.next_random();
            let val3 = spawner.next_random();

            // Values should be in range [0, 1]
            assert!(val1 >= 0.0 && val1 <= 1.0);
            assert!(val2 >= 0.0 && val2 <= 1.0);
            assert!(val3 >= 0.0 && val3 <= 1.0);

            // Values should be different
            assert_ne!(val1, val2);
            assert_ne!(val2, val3);
        }

        #[test]
        fn test_smoke_puff_spawner_deterministic() {
            let position = Vec3::new(5.0, 0.0, 10.0);
            let mut spawner1 = SmokePuffSpawner::new(position);
            let mut spawner2 = SmokePuffSpawner::new(position);

            // Same position should produce same random sequence
            assert_eq!(spawner1.next_random(), spawner2.next_random());
            assert_eq!(spawner1.next_random(), spawner2.next_random());
            assert_eq!(spawner1.next_random(), spawner2.next_random());
        }

        #[test]
        fn test_smoke_puff_effect_progress() {
            let handle = Handle::default();
            let mut effect = SmokePuffEffect {
                material_handle: handle,
                lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                initial_scale: 0.3,
                max_scale: 2.0,
                rise_speed: 2.0,
                drift_velocity: Vec2::ZERO,
            };

            // Initial progress
            assert_eq!(effect.progress(), 0.0);
            assert!(!effect.is_finished());

            // Halfway through
            effect.lifetime.tick(Duration::from_secs_f32(0.5));
            assert!((effect.progress() - 0.5).abs() < 0.01);

            // Finished
            effect.lifetime.tick(Duration::from_secs_f32(0.6));
            assert!(effect.is_finished());
            assert_eq!(effect.progress(), 1.0);
        }

        #[test]
        fn test_smoke_puff_effect_scale_calculation() {
            let handle = Handle::default();
            let mut effect = SmokePuffEffect {
                material_handle: handle,
                lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                initial_scale: 0.5,
                max_scale: 2.0,
                rise_speed: 2.0,
                drift_velocity: Vec2::ZERO,
            };

            // At start: scale should be initial_scale
            let start_scale = effect.current_scale();
            assert!((start_scale - 0.5).abs() < 0.01);

            // At end: scale should be max_scale
            effect.lifetime.tick(Duration::from_secs_f32(1.0));
            let end_scale = effect.current_scale();
            assert!((end_scale - 2.0).abs() < 0.01);
        }

        #[test]
        fn test_smoke_puff_config_constants() {
            // Verify constants are reasonable
            assert!(SMOKE_PUFF_COUNT > 0);
            assert!(SMOKE_SPAWN_DURATION > 0.0);
            assert!(SMOKE_PUFF_LIFETIME_BASE > 0.0);
            assert!(SMOKE_PUFF_INITIAL_SCALE > 0.0);
            assert!(SMOKE_PUFF_MAX_SCALE_BASE > SMOKE_PUFF_INITIAL_SCALE);
        }
    }
}
