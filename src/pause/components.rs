use bevy::prelude::*;

/// Marker for the pause menu root node
#[derive(Component)]
pub struct PauseMenu;

/// Marker for the continue button
#[derive(Component)]
pub struct ContinueButton;

/// Marker for the new game button
#[derive(Component)]
pub struct NewGameButton;

/// Marker for the exit game button
#[derive(Component)]
pub struct ExitGameButton;

/// Marker for debug section container
#[derive(Component)]
pub struct DebugSection;

/// Marker for despawn enemies button
#[derive(Component)]
pub struct DespawnEnemiesButton;

/// Marker for despawn loot button
#[derive(Component)]
pub struct DespawnLootButton;

/// Marker for toggle wall lights button
#[derive(Component)]
pub struct ToggleWallLightsButton;

/// Marker for toggle spell cooldowns button
#[derive(Component)]
pub struct ToggleSpellCooldownsButton;

/// Resource to track if wall lights are currently visible
#[derive(Resource)]
pub struct WallLightsEnabled(pub bool);

impl Default for WallLightsEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Resource to track if spell cooldown overlays are visible
#[derive(Resource)]
pub struct SpellCooldownsVisible(pub bool);

impl Default for SpellCooldownsVisible {
    fn default() -> Self {
        Self(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_menu_component_can_be_created() {
        let _menu = PauseMenu;
    }

    #[test]
    fn continue_button_component_can_be_created() {
        let _button = ContinueButton;
    }

    #[test]
    fn new_game_button_component_can_be_created() {
        let _button = NewGameButton;
    }

    #[test]
    fn exit_game_button_component_can_be_created() {
        let _button = ExitGameButton;
    }

    #[test]
    fn debug_section_component_can_be_created() {
        let _section = DebugSection;
    }

    #[test]
    fn despawn_enemies_button_component_can_be_created() {
        let _button = DespawnEnemiesButton;
    }

    #[test]
    fn despawn_loot_button_component_can_be_created() {
        let _button = DespawnLootButton;
    }

    #[test]
    fn toggle_wall_lights_button_component_can_be_created() {
        let _button = ToggleWallLightsButton;
    }

    #[test]
    fn wall_lights_enabled_default_is_true() {
        let enabled = WallLightsEnabled::default();
        assert!(enabled.0);
    }

    #[test]
    fn toggle_spell_cooldowns_button_component_can_be_created() {
        let _button = ToggleSpellCooldownsButton;
    }

    #[test]
    fn spell_cooldowns_visible_default_is_true() {
        let visible = SpellCooldownsVisible::default();
        assert!(visible.0);
    }
}
