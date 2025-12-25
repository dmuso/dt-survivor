pub use bevy::prelude::*;
pub use crate::states::*;

// Re-export components
pub use crate::enemies::components::*;
pub use crate::game::components::*;
pub use crate::player::components::*;
pub use crate::ui::components::*;

// Re-export systems
pub use crate::enemies::systems::*;
pub use crate::game::systems::*;
pub use crate::player::systems::*;
pub use crate::ui::systems::*;

// Re-export combat module types
pub use crate::combat::{
    Damage, DamageEvent, DeathEvent, EntityType, Health, Hitbox, Invincibility,
};

// Re-export movement module types
pub use crate::movement::{Knockback, Speed, Velocity};

// Re-export weapon types
pub use crate::weapon::WeaponType;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_exports_combat_types() {
        // Health component
        let health = Health::new(100.0);
        assert_eq!(health.current, 100.0);
        assert_eq!(health.max, 100.0);

        // Damage component
        let damage = Damage(10.0);
        assert_eq!(damage.0, 10.0);

        // Hitbox component
        let hitbox = Hitbox(5.0);
        assert_eq!(hitbox.0, 5.0);
    }

    #[test]
    fn test_prelude_exports_movement_types() {
        // Velocity component
        let velocity = Velocity(Vec2::new(1.0, 2.0));
        assert_eq!(velocity.0, Vec2::new(1.0, 2.0));

        // Speed component
        let speed = Speed(100.0);
        assert_eq!(speed.0, 100.0);
    }

    #[test]
    fn test_prelude_exports_weapon_type() {
        // WeaponType enum
        let weapon = WeaponType::Laser;
        assert_eq!(weapon.id(), "laser");

        let pistol = WeaponType::Pistol {
            bullet_count: 1,
            spread_angle: 0.0,
        };
        assert_eq!(pistol.id(), "pistol");
    }
}