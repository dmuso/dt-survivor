use bevy::prelude::*;

/// Health component for entities that can take damage
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    /// Create a new Health component with full health
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    /// Apply damage to this entity
    pub fn take_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    /// Check if this entity is dead
    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    /// Get health as a percentage (0.0 to 1.0)
    pub fn percentage(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            self.current / self.max
        }
    }

    /// Heal this entity
    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new(100.0)
    }
}

/// Damage component for projectiles and damage sources
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Damage(pub f32);

impl Damage {
    pub fn new(amount: f32) -> Self {
        Self(amount)
    }

    pub fn amount(&self) -> f32 {
        self.0
    }
}

impl Default for Damage {
    fn default() -> Self {
        Self(10.0)
    }
}

/// Hitbox component for collision detection
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Hitbox(pub f32);

impl Hitbox {
    pub fn new(radius: f32) -> Self {
        Self(radius)
    }

    pub fn radius(&self) -> f32 {
        self.0
    }
}

impl Default for Hitbox {
    fn default() -> Self {
        Self(16.0)
    }
}

/// Invincibility component for damage immunity frames
#[derive(Component, Debug, Clone)]
pub struct Invincibility {
    pub timer: Timer,
}

impl Invincibility {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }

    /// Tick the invincibility timer
    pub fn tick(&mut self, delta: std::time::Duration) {
        self.timer.tick(delta);
    }

    /// Check if invincibility has expired
    pub fn is_expired(&self) -> bool {
        self.timer.is_finished()
    }
}

impl Default for Invincibility {
    fn default() -> Self {
        Self::new(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod health_tests {
        use super::*;

        #[test]
        fn test_health_new() {
            let health = Health::new(50.0);
            assert_eq!(health.current, 50.0);
            assert_eq!(health.max, 50.0);
        }

        #[test]
        fn test_health_default() {
            let health = Health::default();
            assert_eq!(health.current, 100.0);
            assert_eq!(health.max, 100.0);
        }

        #[test]
        fn test_health_take_damage() {
            let mut health = Health::new(100.0);
            health.take_damage(30.0);
            assert_eq!(health.current, 70.0);
        }

        #[test]
        fn test_health_take_damage_clamps_to_zero() {
            let mut health = Health::new(50.0);
            health.take_damage(100.0);
            assert_eq!(health.current, 0.0);
        }

        #[test]
        fn test_health_is_dead() {
            let mut health = Health::new(10.0);
            assert!(!health.is_dead());
            health.take_damage(10.0);
            assert!(health.is_dead());
        }

        #[test]
        fn test_health_percentage() {
            let health = Health::new(100.0);
            assert_eq!(health.percentage(), 1.0);

            let mut half_health = Health::new(100.0);
            half_health.take_damage(50.0);
            assert_eq!(half_health.percentage(), 0.5);
        }

        #[test]
        fn test_health_percentage_zero_max() {
            let health = Health { current: 0.0, max: 0.0 };
            assert_eq!(health.percentage(), 0.0);
        }

        #[test]
        fn test_health_heal() {
            let mut health = Health::new(100.0);
            health.take_damage(50.0);
            health.heal(30.0);
            assert_eq!(health.current, 80.0);
        }

        #[test]
        fn test_health_heal_clamps_to_max() {
            let mut health = Health::new(100.0);
            health.take_damage(10.0);
            health.heal(50.0);
            assert_eq!(health.current, 100.0);
        }
    }

    mod damage_tests {
        use super::*;

        #[test]
        fn test_damage_new() {
            let damage = Damage::new(25.0);
            assert_eq!(damage.amount(), 25.0);
        }

        #[test]
        fn test_damage_default() {
            let damage = Damage::default();
            assert_eq!(damage.amount(), 10.0);
        }
    }

    mod hitbox_tests {
        use super::*;

        #[test]
        fn test_hitbox_new() {
            let hitbox = Hitbox::new(32.0);
            assert_eq!(hitbox.radius(), 32.0);
        }

        #[test]
        fn test_hitbox_default() {
            let hitbox = Hitbox::default();
            assert_eq!(hitbox.radius(), 16.0);
        }
    }

    mod invincibility_tests {
        use super::*;

        #[test]
        fn test_invincibility_new() {
            let inv = Invincibility::new(2.0);
            assert!(!inv.is_expired());
        }

        #[test]
        fn test_invincibility_default() {
            let inv = Invincibility::default();
            assert!(!inv.is_expired());
        }

        #[test]
        fn test_invincibility_tick_and_expire() {
            let mut inv = Invincibility::new(1.0);
            assert!(!inv.is_expired());

            inv.tick(Duration::from_secs_f32(0.5));
            assert!(!inv.is_expired());

            inv.tick(Duration::from_secs_f32(0.6));
            assert!(inv.is_expired());
        }
    }
}
