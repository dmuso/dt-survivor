use bevy::prelude::*;

/// Marker component for arena wall entities
#[derive(Component)]
pub struct ArenaWall;

/// Marker component for entities that should be constrained to arena bounds
#[derive(Component)]
pub struct ArenaConstrained;

/// Component for torch lights that flicker randomly
#[derive(Component)]
pub struct TorchLight {
    /// Minimum intensity in lumens
    pub min_intensity: f32,
    /// Maximum intensity in lumens
    pub max_intensity: f32,
    /// Current target intensity (for smooth interpolation)
    pub target_intensity: f32,
    /// Time until next target change
    pub change_timer: Timer,
}

impl TorchLight {
    pub fn new(min_intensity: f32, max_intensity: f32) -> Self {
        Self {
            min_intensity,
            max_intensity,
            target_intensity: (min_intensity + max_intensity) / 2.0,
            change_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }
}

impl Default for TorchLight {
    fn default() -> Self {
        Self::new(10_000.0, 50_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_wall_component_can_be_created() {
        let _wall = ArenaWall;
    }

    #[test]
    fn arena_constrained_component_can_be_created() {
        let _constrained = ArenaConstrained;
    }

    #[test]
    fn torch_light_new_sets_correct_range() {
        let torch = TorchLight::new(1000.0, 2000.0);
        assert_eq!(torch.min_intensity, 1000.0);
        assert_eq!(torch.max_intensity, 2000.0);
        assert_eq!(torch.target_intensity, 1500.0);
    }
}
