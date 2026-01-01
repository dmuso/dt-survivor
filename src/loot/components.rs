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

/// Gravity for falling animation (units per second squared)
const FALLING_GRAVITY: f32 = 40.0;
/// Initial horizontal speed when ejecting (similar to attraction speed)
const FALLING_EJECT_SPEED: f32 = 8.0;
/// Initial upward velocity for slight arc
const FALLING_INITIAL_UP_VELOCITY: f32 = 4.0;
/// Ground level Y position for XP orbs
const FALLING_GROUND_Y: f32 = 0.2;
/// Bounce restitution (velocity multiplier on bounce)
const FALLING_BOUNCE_RESTITUTION: f32 = 0.3;
/// Minimum velocity to continue bouncing
const FALLING_MIN_BOUNCE_VELOCITY: f32 = 0.5;
/// Rotation speed during fall (radians per second)
const FALLING_ROTATION_SPEED: f32 = 8.0;

/// Component that tracks the falling animation when an XP orb spawns.
/// Orbs fly backward from spawn point while falling to ground, then bounce.
#[derive(Component, Clone, Debug)]
pub struct FallingAnimation {
    /// Horizontal velocity on XZ plane (units per second)
    pub horizontal_velocity: Vec2,
    /// Vertical velocity (positive = upward)
    pub vertical_velocity: f32,
    /// Rotation velocity for tumbling (radians per second for each axis)
    pub rotation_velocity: Vec3,
    /// Whether the orb has settled (stopped bouncing)
    pub settled: bool,
}

impl FallingAnimation {
    /// Create a new falling animation with direction away from player
    pub fn new(direction_away: Vec2) -> Self {
        let normalized = direction_away.normalize_or_zero();
        Self {
            horizontal_velocity: normalized * FALLING_EJECT_SPEED,
            vertical_velocity: FALLING_INITIAL_UP_VELOCITY,
            rotation_velocity: Vec3::new(
                FALLING_ROTATION_SPEED,
                FALLING_ROTATION_SPEED * 0.5,
                FALLING_ROTATION_SPEED * 0.3,
            ),
            settled: false,
        }
    }

    /// Create with random direction
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let direction = Vec2::new(angle.cos(), angle.sin());
        Self::new(direction)
    }

    /// Apply gravity and return true if still animating
    pub fn tick(&mut self, delta: f32, current_y: f32) -> bool {
        if self.settled {
            return false;
        }

        // Apply gravity
        self.vertical_velocity -= FALLING_GRAVITY * delta;

        // Check for ground collision
        if current_y <= FALLING_GROUND_Y && self.vertical_velocity < 0.0 {
            // Bounce
            self.vertical_velocity = -self.vertical_velocity * FALLING_BOUNCE_RESTITUTION;
            self.horizontal_velocity *= 0.7; // Reduce horizontal speed on bounce

            // Check if we should stop bouncing
            if self.vertical_velocity.abs() < FALLING_MIN_BOUNCE_VELOCITY {
                self.vertical_velocity = 0.0;
                self.horizontal_velocity = Vec2::ZERO;
                self.rotation_velocity = Vec3::ZERO;
                self.settled = true;
                return false;
            }
        }

        true
    }

    /// Get the ground Y level
    pub fn ground_y() -> f32 {
        FALLING_GROUND_Y
    }
}