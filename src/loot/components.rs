use bevy::prelude::*;
use crate::weapon::components::Weapon;

/// Base rotation speed when pickup starts (radians per second)
pub const BASE_ROTATION_SPEED: f32 = 2.0;
/// Maximum rotation speed multiplier during attraction (10x base)
pub const MAX_ROTATION_MULTIPLIER: f32 = 10.0;

// Light constants for loot drops (3D PointLight)
/// Light intensity for XP orb drops (lumens)
pub const XP_ORB_LIGHT_INTENSITY: f32 = 2000.0;
/// Light radius for XP orb drops
pub const XP_ORB_LIGHT_RADIUS: f32 = 5.0;
/// Light color for XP orb drops (cyan-ish to match the experience feel)
pub const XP_ORB_LIGHT_COLOR: Color = Color::srgb(0.6, 0.9, 1.0);

/// Light intensity for health pack drops (lumens)
pub const HEALTH_PACK_LIGHT_INTENSITY: f32 = 3000.0;
/// Light radius for health pack drops
pub const HEALTH_PACK_LIGHT_RADIUS: f32 = 8.0;
/// Light color for health pack drops (green)
pub const HEALTH_PACK_LIGHT_COLOR: Color = Color::srgb(0.0, 1.0, 0.2);

/// Light intensity for weapon drops (lumens)
pub const WEAPON_LIGHT_INTENSITY: f32 = 4000.0;
/// Light radius for weapon drops
pub const WEAPON_LIGHT_RADIUS: f32 = 10.0;
/// Light color for pistol drops (yellow)
pub const WEAPON_PISTOL_LIGHT_COLOR: Color = Color::srgb(1.0, 1.0, 0.4);
/// Light color for laser drops (blue)
pub const WEAPON_LASER_LIGHT_COLOR: Color = Color::srgb(0.3, 0.5, 1.0);
/// Light color for rocket launcher drops (orange)
pub const WEAPON_ROCKET_LIGHT_COLOR: Color = Color::srgb(1.0, 0.6, 0.2);

/// Light intensity for powerup drops (lumens)
pub const POWERUP_LIGHT_INTENSITY: f32 = 5000.0;
/// Light radius for powerup drops
pub const POWERUP_LIGHT_RADIUS: f32 = 12.0;
/// Light color for powerup drops (magenta)
pub const POWERUP_LIGHT_COLOR: Color = Color::srgb(1.0, 0.4, 1.0);

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