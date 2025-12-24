use bevy::prelude::*;

#[derive(Resource)]
pub struct ExperienceRequirements {
    pub requirements: Vec<u32>,
}

impl Default for ExperienceRequirements {
    fn default() -> Self {
        // Experience required for each level (starting from level 2)
        // Level 1 requires 0 experience
        // Level 2 requires 10, Level 3 requires 25, etc.
        // Formula: base_exp * level^1.5
        let mut requirements = Vec::new();
        for level in 2..=100 { // Support up to level 100
            let base_exp = 10.0;
            let exp_required = (base_exp * (level as f32).powf(1.5)) as u32;
            requirements.push(exp_required);
        }
        Self { requirements }
    }
}

impl ExperienceRequirements {
    pub fn exp_required_for_level(&self, level: u32) -> u32 {
        if level <= 1 {
            0
        } else {
            self.requirements.get((level - 2) as usize).copied().unwrap_or(u32::MAX)
        }
    }

    pub fn total_exp_for_level(&self, level: u32) -> u32 {
        let mut total = 0;
        for l in 2..=level {
            total += self.exp_required_for_level(l);
        }
        total
    }
}