use bevy::prelude::*;
use std::f32::consts::TAU;

/// Marker component for Whisper when it's a dropped collectible (before pickup)
#[derive(Component)]
pub struct WhisperDrop {
    /// Pickup radius in 2D pixels (legacy)
    pub pickup_radius: f32,
}

impl Default for WhisperDrop {
    fn default() -> Self {
        Self { pickup_radius: 25.0 }
    }
}

impl WhisperDrop {
    /// Returns pickup radius in 3D world units
    pub fn pickup_radius_3d(&self) -> f32 {
        1.5 // 1.5 world units for 3D collision
    }
}

/// Marker component for Whisper when it's the active companion (after pickup)
#[derive(Component)]
pub struct WhisperCompanion {
    /// Offset above player where Whisper floats
    pub follow_offset: Vec3,
    /// Bobbing animation phase
    pub bob_phase: f32,
    /// Bobbing amplitude in pixels
    pub bob_amplitude: f32,
}

impl Default for WhisperCompanion {
    fn default() -> Self {
        Self {
            follow_offset: Vec3::new(0.0, 1.0, 0.0), // 3D world units above player
            bob_phase: 0.0,
            bob_amplitude: 0.15, // 3D world units (was 5 pixels)
        }
    }
}

/// Timer for spawning lightning arc bursts
#[derive(Component)]
pub struct ArcBurstTimer(pub Timer);

impl Default for ArcBurstTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.12, TimerMode::Repeating))
    }
}

/// Component for individual lightning arc sprites
#[derive(Component)]
pub struct WhisperArc {
    pub lifetime: Timer,
}

impl WhisperArc {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            lifetime: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }
}

/// Marker for the core glow sprite (inner bright part)
#[derive(Component)]
pub struct WhisperCoreGlow;

/// Marker for the outer glow sprite (larger, more transparent)
#[derive(Component)]
pub struct WhisperOuterGlow;

/// Lightning bolt effect that animates from center outward with jagged segments
#[derive(Component)]
pub struct LightningBolt {
    /// Angle of the lightning bolt in radians
    pub angle: f32,
    /// Current distance from center (leading edge)
    pub distance: f32,
    /// Maximum distance before despawning
    pub max_distance: f32,
    /// Speed of outward movement (pixels per second)
    pub speed: f32,
    /// Initial opacity (fades as distance increases)
    pub base_opacity: f32,
    /// Number of segments in the bolt
    pub segment_count: u8,
    /// Random seed for this bolt's jaggedness
    pub seed: u32,
    /// Center position where bolt originates
    pub center: Vec3,
    /// Pre-computed perpendicular offsets for each segment joint (for jagged look)
    pub jag_offsets: Vec<f32>,
    /// Pre-computed cumulative distances to each joint (for variable segment lengths)
    pub joint_distances: Vec<f32>,
    /// Base thickness at center
    pub base_thickness: f32,
    /// Minimum thickness at tip
    pub tip_thickness: f32,
}

impl LightningBolt {
    /// Base thickness at size 1.0
    const BASE_THICKNESS: f32 = 1.5;
    const BASE_TIP_THICKNESS: f32 = 0.5;
    /// Jag amount scales with bolt length
    const JAG_AMOUNT_RATIO: f32 = 0.15; // 15% of max_distance

    /// Creates a new lightning bolt with specified max distance.
    /// max_distance: absolute max distance in pixels (e.g., texture radius)
    /// size_multiplier: fraction of max_distance to use (0.2 to 1.0)
    pub fn new(angle: f32, seed: u32, center: Vec3, max_distance: f32, size_multiplier: f32) -> Self {
        let segment_count = 5u8;
        let actual_distance = max_distance * size_multiplier;
        let jag_amount = actual_distance * Self::JAG_AMOUNT_RATIO;

        // Generate random jagged offsets using the seed
        let jag_offsets = Self::generate_jag_offsets(seed, segment_count, jag_amount);
        // Generate variable segment lengths (longer near center, shorter at tips)
        let joint_distances = Self::generate_joint_distances(seed, segment_count, actual_distance);

        Self {
            angle,
            distance: 0.0,
            max_distance: actual_distance,
            speed: 7.0, // 3D world units/sec (was 200 pixels/sec)
            base_opacity: 1.0,
            segment_count,
            seed,
            center,
            jag_offsets,
            joint_distances,
            base_thickness: Self::BASE_THICKNESS * size_multiplier,
            tip_thickness: Self::BASE_TIP_THICKNESS * size_multiplier,
        }
    }

    /// Generate random perpendicular offsets for jagged appearance
    fn generate_jag_offsets(seed: u32, count: u8, jag_amount: f32) -> Vec<f32> {
        let mut offsets = Vec::with_capacity(count as usize + 1);
        offsets.push(0.0); // First point is always at center

        // Use simple hash-based pseudo-random for deterministic results
        for i in 1..=count {
            let hash = seed.wrapping_mul(1103515245).wrapping_add(i as u32 * 12345);
            let normalized = ((hash % 1000) as f32 / 1000.0) * 2.0 - 1.0; // -1 to 1
            offsets.push(normalized * jag_amount);
        }

        offsets
    }

    /// Generate cumulative distances to each joint with variable segment lengths
    /// Segments get shorter as they get further from center, with random variation
    fn generate_joint_distances(seed: u32, count: u8, max_distance: f32) -> Vec<f32> {
        let mut distances = Vec::with_capacity(count as usize + 1);
        distances.push(0.0); // First joint is at center

        // Calculate base lengths that taper (longer near center, shorter at tip)
        // Using a formula where first segment is ~1.5x average, last is ~0.5x average
        let mut base_lengths = Vec::with_capacity(count as usize);
        for i in 0..count {
            let progress = i as f32 / (count - 1).max(1) as f32;
            // Taper from 1.4 to 0.6 of average length
            let taper = 1.4 - progress * 0.8;
            base_lengths.push(taper);
        }

        // Add random variation to each segment length (±30%)
        let mut varied_lengths = Vec::with_capacity(count as usize);
        for i in 0..count {
            let hash = seed.wrapping_mul(7919).wrapping_add((i as u32 + 100) * 6529);
            let random_factor = 0.7 + ((hash % 1000) as f32 / 1000.0) * 0.6; // 0.7 to 1.3
            varied_lengths.push(base_lengths[i as usize] * random_factor);
        }

        // Normalize so total equals max_distance
        let total: f32 = varied_lengths.iter().sum();
        let scale = max_distance / total;

        let mut cumulative = 0.0;
        for length in varied_lengths {
            cumulative += length * scale;
            distances.push(cumulative);
        }

        distances
    }

    /// Calculate current opacity based on distance traveled
    pub fn current_opacity(&self) -> f32 {
        let progress = self.distance / self.max_distance;
        self.base_opacity * (1.0 - progress).max(0.0)
    }

    /// Check if the bolt has traveled its full distance
    pub fn is_expired(&self) -> bool {
        self.distance >= self.max_distance
    }

    /// Calculate thickness for a segment based on its index (tapers from center)
    /// Final segment is always exactly tip_thickness (1px)
    pub fn thickness_at_segment(&self, segment_index: u8) -> f32 {
        // Use (index + 1) so last segment reaches exactly tip_thickness
        let progress = (segment_index + 1) as f32 / self.segment_count as f32;
        self.base_thickness + (self.tip_thickness - self.base_thickness) * progress
    }

    /// Get the length of a specific segment
    pub fn segment_length(&self, segment_index: usize) -> f32 {
        let start = self.joint_distances.get(segment_index).copied().unwrap_or(0.0);
        let end = self.joint_distances.get(segment_index + 1).copied().unwrap_or(self.max_distance);
        end - start
    }

    /// Get the start distance of a specific segment from center
    pub fn segment_start_distance(&self, segment_index: usize) -> f32 {
        self.joint_distances.get(segment_index).copied().unwrap_or(0.0)
    }

    /// Get the end distance of a specific segment from center
    pub fn segment_end_distance(&self, segment_index: usize) -> f32 {
        self.joint_distances.get(segment_index + 1).copied().unwrap_or(self.max_distance)
    }

    /// Get the position of a joint point (accounting for jag offset)
    pub fn joint_position(&self, joint_index: usize) -> Vec2 {
        let dist_along = self.joint_distances.get(joint_index).copied().unwrap_or(0.0);

        // Direction along the bolt
        let dir = Vec2::new(self.angle.cos(), self.angle.sin());
        // Perpendicular direction for jag offset
        let perp = Vec2::new(-dir.y, dir.x);

        let jag = self.jag_offsets.get(joint_index).copied().unwrap_or(0.0);

        Vec2::new(self.center.x, self.center.y) + dir * dist_along + perp * jag
    }
}

/// Individual segment of a lightning bolt (child entity)
#[derive(Component)]
pub struct LightningSegment {
    /// Which segment this is (0 = closest to center)
    pub index: u8,
    /// Parent bolt entity
    pub bolt_entity: Entity,
}

/// Timer for spawning lightning bolt bursts with randomized intervals
#[derive(Component)]
pub struct LightningSpawnTimer {
    pub timer: Timer,
    /// Minimum interval between spawns (seconds)
    pub min_interval: f32,
    /// Maximum interval between spawns (seconds)
    pub max_interval: f32,
}

impl Default for LightningSpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.08, TimerMode::Once),
            min_interval: 0.04,
            max_interval: 0.15,
        }
    }
}

impl LightningSpawnTimer {
    /// Resets the timer with a new random duration
    pub fn reset_with_random_duration(&mut self, rng: &mut impl rand::Rng) {
        let duration = rng.gen_range(self.min_interval..=self.max_interval);
        self.timer = Timer::from_seconds(duration, TimerMode::Once);
    }
}

/// Particle that orbits Whisper in a tilted 3D plane projected to 2D.
/// Creates an electron-around-atom effect with depth via z-ordering.
#[derive(Component)]
pub struct OrbitalParticle {
    /// Orbit radius in pixels (25-45)
    pub radius: f32,
    /// Orbital period in seconds (1.5-3.0)
    pub period: f32,
    /// Current phase angle in radians (0 to TAU)
    pub phase: f32,
    /// Inclination of orbital plane in radians (0.3-1.2, ~17-69 degrees)
    pub inclination: f32,
    /// Longitude of ascending node - rotates the tilted orbital plane
    pub ascending_node: f32,
    /// Age of the particle in seconds (replaces lifetime timer)
    pub age: f32,
    /// Duration for initial fade-in effect in seconds
    pub fade_in_duration: f32,
    /// Duration for fade-out effect in seconds
    pub fade_out_duration: f32,
    /// Base particle size in pixels (3-5)
    pub size: f32,
    /// Base opacity (0.0 to 1.0)
    pub opacity: f32,
}

impl OrbitalParticle {
    /// Default fade-in duration (fast fade in)
    const DEFAULT_FADE_IN_DURATION: f32 = 0.08;
    /// Default fade-out duration (aggressive fade out)
    const DEFAULT_FADE_OUT_DURATION: f32 = 0.25;

    /// Create a new orbital particle with the given parameters
    pub fn new(
        radius: f32,
        period: f32,
        phase: f32,
        inclination: f32,
        ascending_node: f32,
        size: f32,
    ) -> Self {
        Self {
            radius,
            period,
            phase,
            inclination,
            ascending_node,
            age: 0.0,
            fade_in_duration: Self::DEFAULT_FADE_IN_DURATION,
            fade_out_duration: Self::DEFAULT_FADE_OUT_DURATION,
            size,
            opacity: 0.9,
        }
    }

    /// Calculate the head segment opacity based on age (fade in then fade out)
    pub fn head_opacity(&self) -> f32 {
        // Fade in phase
        if self.age < self.fade_in_duration {
            return self.age / self.fade_in_duration;
        }

        // Fade out phase
        let fade_out_progress = (self.age - self.fade_in_duration) / self.fade_out_duration;
        (1.0 - fade_out_progress).max(0.0)
    }

    /// Check if the particle has completed its full lifecycle (past fade-in + fade-out)
    pub fn is_fully_transparent(&self) -> bool {
        // Only despawn after completing the full fade-out, not during fade-in
        self.age >= self.fade_in_duration + self.fade_out_duration
    }

    /// Calculate segment opacity with progressive transparency along trail
    /// Each segment further from head is more transparent
    pub fn segment_opacity(&self, segment_index: usize, total_segments: usize) -> f32 {
        let head_opacity = self.head_opacity();

        // Progressive transparency: each segment is more transparent than the previous
        // Use cubic falloff for aggressive trail fade
        let progress = segment_index as f32 / (total_segments - 1).max(1) as f32;
        let segment_fade = (1.0 - progress).powf(3.0); // Cubic falloff - more aggressive

        head_opacity * segment_fade * self.opacity
    }

    /// Advance the particle's age
    pub fn advance_age(&mut self, delta_secs: f32) {
        self.age += delta_secs;
    }

    /// Calculate the 2D position and z-depth from 3D orbital mechanics.
    /// Returns (position_2d, z_depth) where z_depth indicates front (negative) to back (positive).
    pub fn calculate_position(&self) -> (Vec2, f32) {
        // Position in orbital plane (before inclination/rotation)
        let x_orbit = self.radius * self.phase.cos();
        let y_orbit = self.radius * self.phase.sin();

        // Apply inclination (rotate around X axis - tilts orbit)
        // y' = y * cos(i), z' = y * sin(i)
        let y_tilted = y_orbit * self.inclination.cos();
        let z_depth = y_orbit * self.inclination.sin();

        // Apply ascending node rotation (rotate around Z axis)
        let x_final =
            x_orbit * self.ascending_node.cos() - y_tilted * self.ascending_node.sin();
        let y_final =
            x_orbit * self.ascending_node.sin() + y_tilted * self.ascending_node.cos();

        (Vec2::new(x_final, y_final), z_depth)
    }

    /// Check if the particle is "behind" the core (positive z_depth)
    pub fn is_behind_core(&self) -> bool {
        let (_, z_depth) = self.calculate_position();
        z_depth > 0.0
    }

    /// Update the phase based on elapsed time
    pub fn advance_phase(&mut self, delta_secs: f32) {
        let angular_velocity = TAU / self.period;
        self.phase += angular_velocity * delta_secs;
        if self.phase >= TAU {
            self.phase -= TAU;
        }
    }

    /// Calculate the z-coordinate for rendering based on depth
    /// Maps z_depth to a range around 0.5 (Whisper core z)
    pub fn calculate_render_z(&self) -> f32 {
        let (_, z_depth) = self.calculate_position();
        let normalized_z = (z_depth / self.radius).clamp(-1.0, 1.0);
        0.5 + normalized_z * 0.08
    }
}

/// Position history for rendering comet-like trails.
/// Stores recent positions as a ring buffer.
#[derive(Component)]
pub struct ParticleTrail {
    /// Ring buffer of recent positions (newest first)
    pub positions: Vec<Vec2>,
    /// Corresponding z-depths for each position
    pub z_depths: Vec<f32>,
    /// Maximum number of positions to store
    pub max_positions: usize,
    /// Time between position samples in seconds
    pub sample_interval: f32,
    /// Time since last sample
    pub time_since_sample: f32,
}

impl ParticleTrail {
    /// Create a new trail with the given buffer size and sample interval
    pub fn new(max_positions: usize, sample_interval: f32) -> Self {
        Self {
            positions: Vec::with_capacity(max_positions),
            z_depths: Vec::with_capacity(max_positions),
            max_positions,
            sample_interval,
            time_since_sample: 0.0,
        }
    }

    /// Record a new position in the trail buffer
    pub fn record_position(&mut self, position: Vec2, z_depth: f32) {
        if self.positions.len() >= self.max_positions {
            self.positions.pop();
            self.z_depths.pop();
        }
        self.positions.insert(0, position);
        self.z_depths.insert(0, z_depth);
    }

    /// Check if it's time to sample a new position
    pub fn should_sample(&self) -> bool {
        self.time_since_sample >= self.sample_interval
    }

    /// Advance the sample timer
    pub fn tick(&mut self, delta_secs: f32) {
        self.time_since_sample += delta_secs;
    }

    /// Reset the sample timer after recording
    pub fn reset_sample_timer(&mut self) {
        self.time_since_sample = 0.0;
    }
}

impl Default for ParticleTrail {
    fn default() -> Self {
        Self::new(12, 0.03)
    }
}

/// Timer for spawning orbital particles at randomized intervals
#[derive(Component)]
pub struct OrbitalParticleSpawnTimer {
    pub timer: Timer,
    /// Minimum interval between spawns (seconds)
    pub min_interval: f32,
    /// Maximum interval between spawns (seconds)
    pub max_interval: f32,
}

impl Default for OrbitalParticleSpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            min_interval: 0.8,
            max_interval: 2.0,
        }
    }
}

impl OrbitalParticleSpawnTimer {
    /// Resets the timer with a new random duration
    pub fn reset_with_random_duration(&mut self, rng: &mut impl rand::Rng) {
        let duration = rng.gen_range(self.min_interval..=self.max_interval);
        self.timer = Timer::from_seconds(duration, TimerMode::Once);
    }
}

/// Marker for individual trail segment entities (mesh-based)
#[derive(Component)]
pub struct TrailSegment {
    /// Index in the trail (0 = closest to particle head)
    pub index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_drop_default() {
        let drop = WhisperDrop::default();
        assert_eq!(drop.pickup_radius, 25.0);
    }

    #[test]
    fn test_whisper_companion_default() {
        let companion = WhisperCompanion::default();
        assert_eq!(companion.follow_offset, Vec3::new(0.0, 1.0, 0.0)); // 3D world units
        assert_eq!(companion.bob_phase, 0.0);
        assert_eq!(companion.bob_amplitude, 0.15); // 3D world units
    }

    #[test]
    fn test_arc_burst_timer_default() {
        let timer = ArcBurstTimer::default();
        assert!(!timer.0.is_finished());
        assert_eq!(timer.0.duration().as_secs_f32(), 0.12);
    }

    #[test]
    fn test_whisper_arc_creation() {
        let arc = WhisperArc::new(0.06);
        assert!(!arc.lifetime.is_finished());
        assert_eq!(arc.lifetime.duration().as_secs_f32(), 0.06);
    }

    #[test]
    fn test_lightning_bolt_new_full_size() {
        // max_distance=32.0 (texture radius), size_multiplier=1.0 (full size)
        let bolt = LightningBolt::new(1.57, 42, Vec3::new(10.0, 20.0, 0.5), 32.0, 1.0);
        assert_eq!(bolt.angle, 1.57);
        assert_eq!(bolt.distance, 0.0);
        assert_eq!(bolt.max_distance, 32.0); // 32.0 * 1.0
        assert_eq!(bolt.speed, 7.0); // 3D world units/sec
        assert_eq!(bolt.base_opacity, 1.0);
        assert_eq!(bolt.segment_count, 5);
        assert_eq!(bolt.seed, 42);
        assert_eq!(bolt.center, Vec3::new(10.0, 20.0, 0.5));
        assert_eq!(bolt.jag_offsets.len(), 6); // segment_count + 1
        assert_eq!(bolt.joint_distances.len(), 6); // segment_count + 1
        assert_eq!(bolt.base_thickness, 1.5); // BASE_THICKNESS * 1.0
        assert_eq!(bolt.tip_thickness, 0.5); // BASE_TIP_THICKNESS * 1.0
    }

    #[test]
    fn test_lightning_bolt_size_multiplier() {
        let max_dist = 32.0; // texture radius

        // Full size bolt (100%)
        let full_bolt = LightningBolt::new(0.0, 42, Vec3::ZERO, max_dist, 1.0);
        assert_eq!(full_bolt.max_distance, 32.0);
        assert_eq!(full_bolt.base_thickness, 1.5);
        assert_eq!(full_bolt.tip_thickness, 0.5);

        // Half size bolt (50%)
        let half_bolt = LightningBolt::new(0.0, 42, Vec3::ZERO, max_dist, 0.5);
        assert_eq!(half_bolt.max_distance, 16.0); // 32.0 * 0.5
        assert_eq!(half_bolt.base_thickness, 0.75); // 1.5 * 0.5
        assert_eq!(half_bolt.tip_thickness, 0.25); // 0.5 * 0.5

        // Minimum size bolt (20%)
        let min_bolt = LightningBolt::new(0.0, 42, Vec3::ZERO, max_dist, 0.2);
        assert!((min_bolt.max_distance - 6.4).abs() < 0.001); // 32.0 * 0.2
        assert!((min_bolt.base_thickness - 0.3).abs() < 0.001); // 1.5 * 0.2
        assert!((min_bolt.tip_thickness - 0.1).abs() < 0.001); // 0.5 * 0.2
    }

    #[test]
    fn test_lightning_bolt_variable_segment_lengths() {
        let bolt = LightningBolt::new(0.0, 42, Vec3::ZERO, 32.0, 1.0);

        // First segment should be longer than last segment
        let first_length = bolt.segment_length(0);
        let last_length = bolt.segment_length(bolt.segment_count as usize - 1);

        assert!(
            first_length > last_length,
            "First segment ({}) should be longer than last ({})",
            first_length,
            last_length
        );
    }

    #[test]
    fn test_lightning_bolt_segment_lengths_sum_to_max() {
        let bolt = LightningBolt::new(0.0, 42, Vec3::ZERO, 32.0, 1.0);

        let total: f32 = (0..bolt.segment_count as usize)
            .map(|i| bolt.segment_length(i))
            .sum();

        assert!(
            (total - bolt.max_distance).abs() < 0.01,
            "Segment lengths should sum to max_distance: {} vs {}",
            total,
            bolt.max_distance
        );
    }

    #[test]
    fn test_lightning_bolt_joint_distances_deterministic() {
        let bolt1 = LightningBolt::new(0.0, 12345, Vec3::ZERO, 32.0, 1.0);
        let bolt2 = LightningBolt::new(0.0, 12345, Vec3::ZERO, 32.0, 1.0);

        // Same seed should produce same distances
        assert_eq!(bolt1.joint_distances, bolt2.joint_distances);

        // Different seed should produce different distances
        let bolt3 = LightningBolt::new(0.0, 54321, Vec3::ZERO, 32.0, 1.0);
        assert_ne!(bolt1.joint_distances, bolt3.joint_distances);
    }

    #[test]
    fn test_lightning_bolt_current_opacity_at_start() {
        let bolt = LightningBolt::new(0.0, 0, Vec3::ZERO, 32.0, 1.0);
        assert_eq!(bolt.current_opacity(), 1.0);
    }

    #[test]
    fn test_lightning_bolt_current_opacity_at_halfway() {
        let mut bolt = LightningBolt::new(0.0, 0, Vec3::ZERO, 32.0, 1.0);
        bolt.distance = bolt.max_distance / 2.0;
        assert!((bolt.current_opacity() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_lightning_bolt_current_opacity_at_end() {
        let mut bolt = LightningBolt::new(0.0, 0, Vec3::ZERO, 32.0, 1.0);
        bolt.distance = bolt.max_distance;
        assert_eq!(bolt.current_opacity(), 0.0);
    }

    #[test]
    fn test_lightning_bolt_is_expired() {
        let mut bolt = LightningBolt::new(0.0, 0, Vec3::ZERO, 32.0, 1.0);
        assert!(!bolt.is_expired());

        bolt.distance = bolt.max_distance - 0.1;
        assert!(!bolt.is_expired());

        bolt.distance = bolt.max_distance;
        assert!(bolt.is_expired());

        bolt.distance = bolt.max_distance + 10.0;
        assert!(bolt.is_expired());
    }

    #[test]
    fn test_lightning_bolt_thickness_tapers() {
        let bolt = LightningBolt::new(0.0, 0, Vec3::ZERO, 32.0, 1.0);

        // First segment should be thicker than last
        let first_thickness = bolt.thickness_at_segment(0);
        // Last segment (index = segment_count - 1) should be exactly tip_thickness
        let last_thickness = bolt.thickness_at_segment(bolt.segment_count - 1);

        assert!(
            first_thickness > last_thickness,
            "Thickness should taper: {} > {}",
            first_thickness,
            last_thickness
        );
        // Last segment should be exactly tip_thickness (0.5px at size 1.0)
        assert_eq!(last_thickness, bolt.tip_thickness);
        assert_eq!(last_thickness, 0.5);
    }

    #[test]
    fn test_lightning_bolt_jag_offsets_deterministic() {
        let bolt1 = LightningBolt::new(0.0, 12345, Vec3::ZERO, 32.0, 1.0);
        let bolt2 = LightningBolt::new(0.0, 12345, Vec3::ZERO, 32.0, 1.0);

        // Same seed should produce same offsets
        assert_eq!(bolt1.jag_offsets, bolt2.jag_offsets);

        // Different seed should produce different offsets
        let bolt3 = LightningBolt::new(0.0, 54321, Vec3::ZERO, 32.0, 1.0);
        assert_ne!(bolt1.jag_offsets, bolt3.jag_offsets);
    }

    #[test]
    fn test_lightning_bolt_joint_position_first_at_center() {
        let center = Vec3::new(50.0, 75.0, 0.5);
        let bolt = LightningBolt::new(0.0, 0, center, 32.0, 1.0);

        let first_joint = bolt.joint_position(0);
        assert!((first_joint.x - center.x).abs() < 0.001);
        assert!((first_joint.y - center.y).abs() < 0.001);
    }

    #[test]
    fn test_lightning_bolt_joint_positions_progress_outward() {
        let bolt = LightningBolt::new(0.0, 0, Vec3::ZERO, 32.0, 1.0); // angle 0 = positive X direction

        let joint0 = bolt.joint_position(0);
        let joint1 = bolt.joint_position(1);
        let joint2 = bolt.joint_position(2);

        // X should increase (moving in positive X direction)
        assert!(joint1.x > joint0.x);
        assert!(joint2.x > joint1.x);
    }

    #[test]
    fn test_lightning_spawn_timer_default() {
        let timer = LightningSpawnTimer::default();
        assert!(!timer.timer.is_finished());
        assert_eq!(timer.timer.duration().as_secs_f32(), 0.08);
        assert_eq!(timer.min_interval, 0.04);
        assert_eq!(timer.max_interval, 0.15);
    }

    #[test]
    fn test_lightning_spawn_timer_reset_with_random_duration() {
        use rand::SeedableRng;

        let mut timer = LightningSpawnTimer::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        timer.reset_with_random_duration(&mut rng);

        let duration = timer.timer.duration().as_secs_f32();
        assert!(
            duration >= timer.min_interval && duration <= timer.max_interval,
            "Duration {} should be between {} and {}",
            duration,
            timer.min_interval,
            timer.max_interval
        );
    }

    // ==================== OrbitalParticle Tests ====================

    #[test]
    fn test_orbital_particle_new() {
        let particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);
        assert_eq!(particle.radius, 30.0);
        assert_eq!(particle.period, 2.0);
        assert_eq!(particle.phase, 0.0);
        assert_eq!(particle.inclination, 0.5);
        assert_eq!(particle.ascending_node, 0.0);
        assert_eq!(particle.size, 4.0);
        assert_eq!(particle.opacity, 0.9);
        assert_eq!(particle.age, 0.0);
        assert!(particle.fade_in_duration > 0.0);
        assert!(particle.fade_out_duration > 0.0);
    }

    #[test]
    fn test_orbital_particle_head_opacity_fade_in() {
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);

        // At age 0, opacity should be 0 (start of fade in)
        assert_eq!(particle.head_opacity(), 0.0);

        // At half fade_in_duration, opacity should be ~0.5
        particle.age = particle.fade_in_duration / 2.0;
        assert!((particle.head_opacity() - 0.5).abs() < 0.01);

        // At end of fade_in, opacity should be 1.0
        particle.age = particle.fade_in_duration;
        assert!((particle.head_opacity() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_orbital_particle_head_opacity_fade_out() {
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);

        // After fade in, start fading out
        particle.age = particle.fade_in_duration + particle.fade_out_duration / 2.0;
        assert!((particle.head_opacity() - 0.5).abs() < 0.01);

        // At end of fade out, opacity should be ~0
        particle.age = particle.fade_in_duration + particle.fade_out_duration;
        assert!(particle.head_opacity() < 0.001, "Should be nearly 0");
    }

    #[test]
    fn test_orbital_particle_is_fully_transparent() {
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);

        // Not transparent at start (fading in) - even though opacity is 0, lifecycle not complete
        assert!(!particle.is_fully_transparent());

        // Not transparent during visible phase
        particle.age = particle.fade_in_duration;
        assert!(!particle.is_fully_transparent());

        // Not transparent during fade out
        particle.age = particle.fade_in_duration + particle.fade_out_duration * 0.5;
        assert!(!particle.is_fully_transparent());

        // Fully transparent after completing full lifecycle
        particle.age = particle.fade_in_duration + particle.fade_out_duration;
        assert!(particle.is_fully_transparent());
    }

    #[test]
    fn test_orbital_particle_segment_opacity() {
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);

        // At age 0, all segments should have 0 opacity (fade-in not started)
        assert_eq!(particle.segment_opacity(0, 10), 0.0);

        // After fade-in, head segment (index 0) should have full opacity
        particle.age = particle.fade_in_duration;
        let head_opacity = particle.segment_opacity(0, 10);
        assert!(head_opacity > 0.8, "Head should be near full opacity: {}", head_opacity);

        // Each subsequent segment should be less opaque
        let seg1 = particle.segment_opacity(1, 10);
        let seg5 = particle.segment_opacity(5, 10);
        let seg9 = particle.segment_opacity(9, 10);

        assert!(seg1 < head_opacity, "Segment 1 should be less opaque than head");
        assert!(seg5 < seg1, "Segment 5 should be less opaque than segment 1");
        assert!(seg9 < seg5, "Last segment should be least opaque");
        assert!(seg9 < 0.1, "Last segment should be nearly invisible: {}", seg9);
    }

    #[test]
    fn test_orbital_particle_advance_age() {
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.5, 0.0, 4.0);
        assert_eq!(particle.age, 0.0);

        particle.advance_age(0.1);
        assert!((particle.age - 0.1).abs() < 0.001);

        particle.advance_age(0.2);
        assert!((particle.age - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_orbital_particle_position_at_phase_zero_no_inclination() {
        // No inclination, no ascending node - particle should be at (radius, 0)
        let particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.0, 0.0, 4.0);
        let (pos, z_depth) = particle.calculate_position();

        assert!((pos.x - 30.0).abs() < 0.01, "x should be radius at phase 0");
        assert!(pos.y.abs() < 0.01, "y should be 0 at phase 0");
        assert!(z_depth.abs() < 0.01, "z_depth should be 0 with no inclination");
    }

    #[test]
    fn test_orbital_particle_position_at_phase_half_pi() {
        use std::f32::consts::FRAC_PI_2;

        // At phase PI/2, particle is at top of orbit (0, radius) before inclination
        let particle = OrbitalParticle::new(30.0, 2.0, FRAC_PI_2, 0.0, 0.0, 4.0);
        let (pos, z_depth) = particle.calculate_position();

        assert!(pos.x.abs() < 0.01, "x should be ~0 at phase PI/2");
        assert!((pos.y - 30.0).abs() < 0.01, "y should be radius at phase PI/2");
        assert!(z_depth.abs() < 0.01, "z_depth should be 0 with no inclination");
    }

    #[test]
    fn test_orbital_particle_behind_core_with_inclination() {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

        // At phase PI/2 with inclination PI/4, particle is tilted "backward"
        let particle = OrbitalParticle::new(30.0, 2.0, FRAC_PI_2, FRAC_PI_4, 0.0, 4.0);
        assert!(
            particle.is_behind_core(),
            "Particle at phase PI/2 with positive inclination should be behind core"
        );

        // At phase 3*PI/2 with inclination, particle is tilted "forward"
        let particle2 = OrbitalParticle::new(30.0, 2.0, 3.0 * FRAC_PI_2, FRAC_PI_4, 0.0, 4.0);
        assert!(
            !particle2.is_behind_core(),
            "Particle at phase 3*PI/2 with positive inclination should be in front"
        );
    }

    #[test]
    fn test_orbital_particle_z_depth_varies_with_phase() {
        use std::f32::consts::FRAC_PI_4;

        // With inclination, z_depth should vary as phase changes
        let particle1 = OrbitalParticle::new(30.0, 2.0, 0.0, FRAC_PI_4, 0.0, 4.0);
        let (_, z1) = particle1.calculate_position();

        let particle2 = OrbitalParticle::new(30.0, 2.0, std::f32::consts::FRAC_PI_2, FRAC_PI_4, 0.0, 4.0);
        let (_, z2) = particle2.calculate_position();

        assert!(
            (z2 - z1).abs() > 0.1,
            "z_depth should change between phase 0 and PI/2"
        );
    }

    #[test]
    fn test_orbital_particle_advance_phase() {
        let mut particle = OrbitalParticle::new(30.0, 2.0, 0.0, 0.0, 0.0, 4.0);

        // After 0.5 seconds with period 2.0, phase should be PI/2
        particle.advance_phase(0.5);
        assert!(
            (particle.phase - std::f32::consts::FRAC_PI_2).abs() < 0.01,
            "Phase should be ~PI/2 after 0.5s with 2s period"
        );

        // After full period, phase should wrap back near 0
        particle.advance_phase(1.5);
        assert!(
            particle.phase < 0.1,
            "Phase should wrap back near 0 after full period"
        );
    }

    #[test]
    fn test_orbital_particle_render_z_range() {
        use std::f32::consts::FRAC_PI_4;

        // Test that render Z stays within expected range
        let particle_front = OrbitalParticle::new(30.0, 2.0, 3.0 * std::f32::consts::FRAC_PI_2, FRAC_PI_4, 0.0, 4.0);
        let z_front = particle_front.calculate_render_z();
        assert!(z_front >= 0.42 && z_front <= 0.58, "Front render Z should be in valid range: {}", z_front);

        let particle_back = OrbitalParticle::new(30.0, 2.0, std::f32::consts::FRAC_PI_2, FRAC_PI_4, 0.0, 4.0);
        let z_back = particle_back.calculate_render_z();
        assert!(z_back >= 0.42 && z_back <= 0.58, "Back render Z should be in valid range: {}", z_back);

        // Back should have higher z than front
        assert!(z_back > z_front, "Behind particle should have higher z");
    }

    // ==================== ParticleTrail Tests ====================

    #[test]
    fn test_particle_trail_new() {
        let trail = ParticleTrail::new(10, 0.05);
        assert_eq!(trail.max_positions, 10);
        assert_eq!(trail.sample_interval, 0.05);
        assert!(trail.positions.is_empty());
        assert!(trail.z_depths.is_empty());
    }

    #[test]
    fn test_particle_trail_default() {
        let trail = ParticleTrail::default();
        assert_eq!(trail.max_positions, 12);
        assert_eq!(trail.sample_interval, 0.03);
    }

    #[test]
    fn test_particle_trail_record_position() {
        let mut trail = ParticleTrail::new(5, 0.03);

        trail.record_position(Vec2::new(10.0, 0.0), 0.1);
        trail.record_position(Vec2::new(20.0, 0.0), 0.2);
        trail.record_position(Vec2::new(30.0, 0.0), -0.1);

        assert_eq!(trail.positions.len(), 3);
        // Most recent should be first
        assert_eq!(trail.positions[0], Vec2::new(30.0, 0.0));
        assert_eq!(trail.z_depths[0], -0.1);
        // Oldest should be last
        assert_eq!(trail.positions[2], Vec2::new(10.0, 0.0));
    }

    #[test]
    fn test_particle_trail_ring_buffer() {
        let mut trail = ParticleTrail::new(3, 0.03);

        trail.record_position(Vec2::new(1.0, 0.0), 0.0);
        trail.record_position(Vec2::new(2.0, 0.0), 0.0);
        trail.record_position(Vec2::new(3.0, 0.0), 0.0);
        trail.record_position(Vec2::new(4.0, 0.0), 0.0);

        // Should have max 3 positions, oldest dropped
        assert_eq!(trail.positions.len(), 3);
        assert_eq!(trail.positions[0], Vec2::new(4.0, 0.0)); // Most recent
        assert_eq!(trail.positions[2], Vec2::new(2.0, 0.0)); // Oldest (1.0 dropped)
    }

    #[test]
    fn test_particle_trail_sampling() {
        let mut trail = ParticleTrail::new(10, 0.03);

        assert!(!trail.should_sample());

        trail.tick(0.02);
        assert!(!trail.should_sample());

        trail.tick(0.02);
        assert!(trail.should_sample());

        trail.reset_sample_timer();
        assert!(!trail.should_sample());
    }

    // ==================== OrbitalParticleSpawnTimer Tests ====================

    #[test]
    fn test_orbital_particle_spawn_timer_default() {
        let timer = OrbitalParticleSpawnTimer::default();
        assert!(!timer.timer.is_finished());
        assert_eq!(timer.min_interval, 0.8);
        assert_eq!(timer.max_interval, 2.0);
    }

    #[test]
    fn test_orbital_particle_spawn_timer_reset_with_random_duration() {
        use rand::SeedableRng;

        let mut timer = OrbitalParticleSpawnTimer::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        timer.reset_with_random_duration(&mut rng);

        let duration = timer.timer.duration().as_secs_f32();
        assert!(
            duration >= timer.min_interval && duration <= timer.max_interval,
            "Duration {} should be between {} and {}",
            duration,
            timer.min_interval,
            timer.max_interval
        );
    }

    // ==================== TrailSegment Tests ====================

    #[test]
    fn test_trail_segment() {
        let segment = TrailSegment { index: 5 };
        assert_eq!(segment.index, 5);
    }
}
