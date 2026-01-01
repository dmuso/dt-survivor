use bevy::prelude::*;

/// The half-size of the arena in world units (arena extends from -ARENA_HALF_SIZE to +ARENA_HALF_SIZE)
/// Based on the ground plane which uses Plane3d with half-extents of 100x100 (so -100 to +100)
pub const ARENA_HALF_SIZE: f32 = 100.0;

/// The thickness of the arena walls
pub const WALL_THICKNESS: f32 = 1.3;

/// The height of the arena walls
pub const WALL_HEIGHT: f32 = 3.0;

/// Resource defining the playable arena boundaries
#[derive(Resource, Debug, Clone, Copy)]
pub struct ArenaBounds {
    /// Minimum X coordinate of the playable area
    pub min_x: f32,
    /// Maximum X coordinate of the playable area
    pub max_x: f32,
    /// Minimum Z coordinate of the playable area
    pub min_z: f32,
    /// Maximum Z coordinate of the playable area
    pub max_z: f32,
}

impl Default for ArenaBounds {
    fn default() -> Self {
        Self {
            min_x: -ARENA_HALF_SIZE,
            max_x: ARENA_HALF_SIZE,
            min_z: -ARENA_HALF_SIZE,
            max_z: ARENA_HALF_SIZE,
        }
    }
}

impl ArenaBounds {
    /// Creates a new ArenaBounds with the specified half-size
    pub fn new(half_size: f32) -> Self {
        Self {
            min_x: -half_size,
            max_x: half_size,
            min_z: -half_size,
            max_z: half_size,
        }
    }

    /// Returns the width of the arena (X axis)
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    /// Returns the depth of the arena (Z axis)
    pub fn depth(&self) -> f32 {
        self.max_z - self.min_z
    }

    /// Checks if a position (on XZ plane) is within the arena bounds
    pub fn contains(&self, position: Vec2) -> bool {
        position.x >= self.min_x
            && position.x <= self.max_x
            && position.y >= self.min_z
            && position.y <= self.max_z
    }

    /// Clamps a position to be within the arena bounds
    pub fn clamp(&self, position: Vec2) -> Vec2 {
        Vec2::new(
            position.x.clamp(self.min_x, self.max_x),
            position.y.clamp(self.min_z, self.max_z),
        )
    }

    /// Returns the inner bounds (accounting for entity radius) for spawning
    pub fn inner_bounds(&self, margin: f32) -> ArenaBounds {
        ArenaBounds {
            min_x: self.min_x + margin,
            max_x: self.max_x - margin,
            min_z: self.min_z + margin,
            max_z: self.max_z - margin,
        }
    }
}

/// Handle to the loaded wall model scene
#[derive(Resource)]
pub struct WallModelHandle(pub Handle<Scene>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_bounds_default_uses_constants() {
        let bounds = ArenaBounds::default();
        assert_eq!(bounds.min_x, -ARENA_HALF_SIZE);
        assert_eq!(bounds.max_x, ARENA_HALF_SIZE);
        assert_eq!(bounds.min_z, -ARENA_HALF_SIZE);
        assert_eq!(bounds.max_z, ARENA_HALF_SIZE);
    }

    #[test]
    fn arena_bounds_new_creates_symmetric_bounds() {
        let bounds = ArenaBounds::new(25.0);
        assert_eq!(bounds.min_x, -25.0);
        assert_eq!(bounds.max_x, 25.0);
        assert_eq!(bounds.min_z, -25.0);
        assert_eq!(bounds.max_z, 25.0);
    }

    #[test]
    fn arena_bounds_width_returns_correct_value() {
        let bounds = ArenaBounds::default();
        assert_eq!(bounds.width(), 200.0); // -100 to +100 = 200
    }

    #[test]
    fn arena_bounds_depth_returns_correct_value() {
        let bounds = ArenaBounds::default();
        assert_eq!(bounds.depth(), 200.0); // -100 to +100 = 200
    }

    #[test]
    fn arena_bounds_contains_returns_true_for_inside_position() {
        let bounds = ArenaBounds::new(10.0);
        assert!(bounds.contains(Vec2::new(0.0, 0.0)));
        assert!(bounds.contains(Vec2::new(5.0, 5.0)));
        assert!(bounds.contains(Vec2::new(-5.0, -5.0)));
        assert!(bounds.contains(Vec2::new(10.0, 10.0)));
        assert!(bounds.contains(Vec2::new(-10.0, -10.0)));
    }

    #[test]
    fn arena_bounds_contains_returns_false_for_outside_position() {
        let bounds = ArenaBounds::new(10.0);
        assert!(!bounds.contains(Vec2::new(15.0, 0.0)));
        assert!(!bounds.contains(Vec2::new(0.0, 15.0)));
        assert!(!bounds.contains(Vec2::new(-15.0, 0.0)));
        assert!(!bounds.contains(Vec2::new(0.0, -15.0)));
        assert!(!bounds.contains(Vec2::new(11.0, 11.0)));
    }

    #[test]
    fn arena_bounds_clamp_keeps_inside_position_unchanged() {
        let bounds = ArenaBounds::new(10.0);
        let pos = Vec2::new(5.0, -3.0);
        assert_eq!(bounds.clamp(pos), pos);
    }

    #[test]
    fn arena_bounds_clamp_constrains_outside_position() {
        let bounds = ArenaBounds::new(10.0);
        assert_eq!(bounds.clamp(Vec2::new(15.0, 0.0)), Vec2::new(10.0, 0.0));
        assert_eq!(bounds.clamp(Vec2::new(0.0, 15.0)), Vec2::new(0.0, 10.0));
        assert_eq!(bounds.clamp(Vec2::new(-15.0, -15.0)), Vec2::new(-10.0, -10.0));
        assert_eq!(bounds.clamp(Vec2::new(20.0, 20.0)), Vec2::new(10.0, 10.0));
    }

    #[test]
    fn arena_bounds_inner_bounds_shrinks_by_margin() {
        let bounds = ArenaBounds::new(10.0);
        let inner = bounds.inner_bounds(2.0);
        assert_eq!(inner.min_x, -8.0);
        assert_eq!(inner.max_x, 8.0);
        assert_eq!(inner.min_z, -8.0);
        assert_eq!(inner.max_z, 8.0);
    }

    #[test]
    fn arena_half_size_constant_matches_ground_plane() {
        // Ground plane uses Plane3d with half-extents 100x100, so arena extends from -100 to +100
        assert_eq!(ARENA_HALF_SIZE, 100.0);
    }
}
