use bevy::prelude::*;
use std::collections::HashMap;

/// The different types of powerups available in the game
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PowerupType {
    /// Increases maximum health permanently
    MaxHealth,
    /// Increases health regeneration rate permanently
    HealthRegen,
    /// Doubles fire rate for all weapons temporarily
    WeaponFireRate,
    /// Increases loot pickup radius permanently
    PickupRadius,
    /// Increases movement speed temporarily
    MovementSpeed,
}

impl PowerupType {
    /// Get the display name for the powerup
    pub fn display_name(&self) -> &'static str {
        match self {
            PowerupType::MaxHealth => "Max Health +",
            PowerupType::HealthRegen => "Health Regen +",
            PowerupType::WeaponFireRate => "Weapon Speed",
            PowerupType::PickupRadius => "Pickup Range +",
            PowerupType::MovementSpeed => "Movement Speed",
        }
    }

    /// Get the color for visual representation
    pub fn color(&self) -> Color {
        match self {
            PowerupType::MaxHealth => Color::srgb(1.0, 0.0, 0.0), // Red
            PowerupType::HealthRegen => Color::srgb(0.0, 1.0, 0.0), // Green
            PowerupType::WeaponFireRate => Color::srgb(1.0, 1.0, 0.0), // Yellow
            PowerupType::PickupRadius => Color::srgb(0.0, 1.0, 1.0), // Cyan
            PowerupType::MovementSpeed => Color::srgb(1.0, 0.0, 1.0), // Magenta
        }
    }

    /// Check if this powerup is permanent (true) or temporary (false)
    pub fn is_permanent(&self) -> bool {
        match self {
            PowerupType::MaxHealth => true,
            PowerupType::HealthRegen => true,
            PowerupType::WeaponFireRate => false,
            PowerupType::PickupRadius => true,
            PowerupType::MovementSpeed => false,
        }
    }

    /// Get the duration for temporary powerups (in seconds)
    pub fn duration(&self) -> f32 {
        match self {
            PowerupType::WeaponFireRate => 20.0,
            PowerupType::MovementSpeed => 20.0,
            _ => 0.0, // Permanent powerups have no duration
        }
    }
}

/// Component for powerup entities that can be picked up
#[derive(Component)]
pub struct PowerupItem {
    pub powerup_type: PowerupType,
    pub velocity: Vec2,
}

/// Component for pulsing animation on powerup sprites
#[derive(Component)]
pub struct PowerupPulse {
    pub base_scale: Vec3,
    pub amplitude: f32,
    pub frequency: f32,
    pub time: f32,
}

/// Resource tracking all active powerups and their stacks/counts
#[derive(Resource, Default)]
pub struct ActivePowerups {
    /// Map of powerup type to stack count
    pub stacks: HashMap<PowerupType, u32>,
    /// Map of powerup type to remaining duration (for temporary powerups)
    pub timers: HashMap<PowerupType, f32>,
}

impl ActivePowerups {
    /// Add a powerup, either stacking it or starting its timer
    pub fn add_powerup(&mut self, powerup_type: PowerupType) {
        if powerup_type.is_permanent() {
            // Permanent powerups stack
            *self.stacks.entry(powerup_type).or_insert(0) += 1;
        } else {
            // Temporary powerups reset the timer
            self.timers.insert(powerup_type.clone(), powerup_type.duration());
            *self.stacks.entry(powerup_type).or_insert(0) += 1;
        }
    }

    /// Get the total stack count for a powerup type
    pub fn get_stack_count(&self, powerup_type: &PowerupType) -> u32 {
        self.stacks.get(powerup_type).copied().unwrap_or(0)
    }

    /// Get the remaining duration for a temporary powerup
    pub fn get_remaining_duration(&self, powerup_type: &PowerupType) -> Option<f32> {
        self.timers.get(powerup_type).copied()
    }

    /// Update timers and remove expired temporary powerups
    pub fn update_timers(&mut self, delta_time: f32) {
        let mut expired = Vec::new();

        for (powerup_type, remaining) in self.timers.iter_mut() {
            *remaining -= delta_time;
            if *remaining <= 0.0 {
                expired.push(powerup_type.clone());
            }
        }

        for expired_type in expired {
            self.timers.remove(&expired_type);
            if let Some(stack) = self.stacks.get_mut(&expired_type) {
                if *stack > 0 {
                    *stack -= 1;
                    if *stack == 0 {
                        self.stacks.remove(&expired_type);
                    }
                }
            }
        }
    }

    /// Get all active powerups (both permanent and temporary)
    pub fn get_active_powerups(&self) -> Vec<&PowerupType> {
        self.stacks.keys().collect()
    }
}

/// Component for powerup UI display table
#[derive(Component)]
pub struct PowerupDisplay;

/// Component for individual powerup display rows in the UI table
#[derive(Component)]
pub struct PowerupRow {
    pub powerup_type: PowerupType,
}