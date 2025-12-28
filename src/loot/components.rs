use bevy::prelude::*;
use crate::weapon::components::Weapon;

/// Base rotation speed when pickup starts (radians per second)
pub const BASE_ROTATION_SPEED: f32 = 2.0;
/// Maximum rotation speed multiplier during attraction (10x base)
pub const MAX_ROTATION_MULTIPLIER: f32 = 10.0;

#[derive(Component)]
pub struct DroppedItem {
    pub pickup_state: PickupState,
    pub item_data: ItemData,
    pub velocity: Vec3,
    /// Current rotation speed in radians per second (around Y axis)
    pub rotation_speed: f32,
    /// Rotation direction: 1.0 for clockwise, -1.0 for counter-clockwise (when viewed from above)
    pub rotation_direction: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PickupState {
    Idle,           // Waiting to be picked up
    PopUp,          // Flying up briefly before attraction
    BeingAttracted, // Moving toward player
    PickedUp,       // Just picked up, effects being applied
    Consumed,       // Effects applied, ready for cleanup
}

#[derive(Clone, Debug)]
pub enum ItemData {
    Weapon(Weapon),
    HealthPack { heal_amount: f32 },
    Experience { amount: u32 },
    Powerup(crate::powerup::components::PowerupType),
    Whisper,
}

/// Marker component for loot pickup sound effects
#[derive(Component)]
pub struct LootPickupSound;

/// Component that tracks the pop-up animation when an item is first picked up.
/// Items fly up quickly, hang briefly at the peak, then fly to player.
#[derive(Component, Clone, Debug)]
pub struct PopUpAnimation {
    /// Starting Y position when animation began
    pub start_y: f32,
    /// Target peak height above start position
    pub peak_height: f32,
    /// Current vertical velocity (positive = upward)
    pub vertical_velocity: f32,
    /// Whether the item is currently hanging at the peak
    pub hanging: bool,
    /// Remaining time to hang at peak (seconds)
    pub hang_time_remaining: f32,
}

/// Default hang time at peak of pop-up (seconds)
const DEFAULT_HANG_TIME: f32 = 0.15;
/// Default initial upward velocity calculated for ~2.0 unit peak height
/// Using h = v²/(2g) with g=120: v = sqrt(2 * 120 * 2.0) ≈ 22
const DEFAULT_INITIAL_VELOCITY: f32 = 22.0;

impl PopUpAnimation {
    /// Create a new pop-up animation with default settings
    pub fn new(start_y: f32) -> Self {
        Self {
            start_y,
            peak_height: 1.0, // Pop up 1 unit above ground
            vertical_velocity: DEFAULT_INITIAL_VELOCITY, // Fast initial launch
            hanging: false,
            hang_time_remaining: DEFAULT_HANG_TIME,
        }
    }

    /// Create a pop-up animation with custom peak height
    pub fn with_peak_height(start_y: f32, peak_height: f32) -> Self {
        Self {
            start_y,
            peak_height,
            vertical_velocity: DEFAULT_INITIAL_VELOCITY,
            hanging: false,
            hang_time_remaining: DEFAULT_HANG_TIME,
        }
    }
}