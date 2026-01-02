use bevy::prelude::*;
use crate::inventory::resources::*;
use crate::player::components::*;

/// Spell follow player system - spells follow the player
pub fn spell_follow_player_system(
    mut spell_query: Query<(&mut Transform, &crate::spell::components::Spell), Without<Player>>,
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(player_transform) = player_query.single() {
        for (mut spell_transform, _) in spell_query.iter_mut() {
            // Keep spell entities positioned at the player
            spell_transform.translation = player_transform.translation;
        }
    }
}

/// Initialize inventory resources when game starts
pub fn inventory_initialization_system(
    _spell_list: Res<SpellList>,
) {
    // SpellList is now initialized empty, spells are added when Whisper is collected
    // and player selects attunement
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::SpellType;

    #[test]
    fn spell_list_starts_empty() {
        let spell_list = SpellList::default();
        assert!(spell_list.find_empty_slot().is_some());
        assert_eq!(spell_list.iter_spells().count(), 0);
    }

    #[test]
    fn spell_list_can_equip_spell() {
        let mut spell_list = SpellList::default();
        let spell = crate::spell::Spell::new(SpellType::Fireball);
        let result = spell_list.equip(spell);
        assert_eq!(result, Some(0));
        assert_eq!(spell_list.iter_spells().count(), 1);
    }
}
