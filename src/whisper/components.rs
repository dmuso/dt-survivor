use bevy::prelude::*;

/// Marker component for Whisper when it's a dropped collectible (before pickup)
#[derive(Component)]
pub struct WhisperDrop {
    pub pickup_radius: f32,
}

impl Default for WhisperDrop {
    fn default() -> Self {
        Self { pickup_radius: 25.0 }
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
            follow_offset: Vec3::new(0.0, 30.0, 0.5),
            bob_phase: 0.0,
            bob_amplitude: 5.0,
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
            speed: 200.0,
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
        assert_eq!(companion.follow_offset, Vec3::new(0.0, 30.0, 0.5));
        assert_eq!(companion.bob_phase, 0.0);
        assert_eq!(companion.bob_amplitude, 5.0);
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
        assert_eq!(bolt.speed, 200.0);
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
}
