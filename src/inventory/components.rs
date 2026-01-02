use bevy::prelude::*;
use crate::spell::SpellType;

#[derive(Component)]
pub struct EquippedSpell {
    pub spell_type: SpellType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipped_spell_uses_spell_type_enum() {
        let equipped = EquippedSpell {
            spell_type: SpellType::Fireball,
        };
        assert_eq!(equipped.spell_type.id(), 0);
    }

    #[test]
    fn equipped_spell_with_radiant_beam() {
        let equipped = EquippedSpell {
            spell_type: SpellType::RadiantBeam,
        };
        assert_eq!(equipped.spell_type.id(), 33);
    }
}
