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

// Re-export weapon types (deprecated, use spell types instead)
pub use crate::weapon::WeaponType;

// Re-export spell types
pub use crate::spell::{Spell, SpellType};

// Re-export element types
pub use crate::element::Element;

// Re-export whisper module types
pub use crate::whisper::{SpellOrigin, WeaponOrigin, WhisperState};

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
        // WeaponType enum (deprecated)
        let weapon = WeaponType::Laser;
        assert_eq!(weapon.id(), "laser");

        let pistol = WeaponType::Pistol {
            bullet_count: 1,
            spread_angle: 0.0,
        };
        assert_eq!(pistol.id(), "pistol");
    }

    #[test]
    fn test_prelude_exports_spell_type() {
        // SpellType enum with 64 variants
        let spell_type = SpellType::RadiantBeam;
        assert_eq!(spell_type.id(), 33);
        assert_eq!(spell_type.element(), Element::Light);

        let fireball = SpellType::Fireball;
        assert_eq!(fireball.id(), 0);
        assert_eq!(fireball.element(), Element::Fire);

        // Test all 64 spells exist
        assert_eq!(SpellType::all().len(), 64);
    }

    #[test]
    fn test_prelude_exports_spell() {
        // Spell component
        let spell = Spell::new(SpellType::ThunderStrike);
        assert_eq!(spell.spell_type, SpellType::ThunderStrike);
        assert_eq!(spell.element, Element::Lightning);
        assert_eq!(spell.name, "Thunder Strike");
    }

    #[test]
    fn test_prelude_exports_element() {
        // Element enum
        let fire = Element::Fire;
        assert_eq!(fire.name(), "Fire");

        // Test color method is accessible
        let color = Element::Frost.color();
        assert_eq!(color, bevy::prelude::Color::srgb_u8(135, 206, 235));
    }

    #[test]
    fn test_prelude_exports_spell_origin() {
        // SpellOrigin resource
        let origin = SpellOrigin::default();
        assert!(origin.position.is_none());
        assert!(!origin.is_active());

        // WeaponOrigin is an alias for SpellOrigin
        let weapon_origin: WeaponOrigin = SpellOrigin::default();
        assert!(weapon_origin.position.is_none());
    }
}
