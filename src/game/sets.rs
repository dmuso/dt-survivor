use bevy::prelude::*;

/// System sets for explicit ordering of game systems.
/// These sets allow systems to be grouped and ordered relative to each other.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Input handling - keyboard, mouse, etc.
    Input,
    /// Movement systems - player, enemy, projectile movement
    Movement,
    /// Combat systems - damage, health, collisions
    Combat,
    /// Spawning systems - enemies, loot, projectiles
    Spawning,
    /// Effect systems - visual effects, audio, particles
    Effects,
    /// Cleanup systems - despawning, garbage collection
    Cleanup,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;

    #[test]
    fn test_game_set_derives_required_traits() {
        // Test that GameSet can be used as a SystemSet
        let input = GameSet::Input;
        let input_clone = input.clone();

        // Test PartialEq and Eq
        assert_eq!(input, input_clone);
        assert_eq!(GameSet::Movement, GameSet::Movement);
        assert_ne!(GameSet::Input, GameSet::Combat);

        // Test Debug
        let debug_str = format!("{:?}", GameSet::Input);
        assert!(debug_str.contains("Input"));

        // Test Hash (implicit through PartialEq + Eq + Hash)
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(GameSet::Input);
        set.insert(GameSet::Movement);
        set.insert(GameSet::Combat);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_game_set_variants_exist() {
        // Verify all expected variants exist
        let _input = GameSet::Input;
        let _movement = GameSet::Movement;
        let _combat = GameSet::Combat;
        let _spawning = GameSet::Spawning;
        let _effects = GameSet::Effects;
        let _cleanup = GameSet::Cleanup;
    }

    #[test]
    fn test_game_set_can_be_used_in_app() {
        // Test that GameSet can be used to configure system ordering in a Bevy App
        let mut app = App::new();

        // Configure sets with explicit ordering
        app.configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Movement,
                GameSet::Combat,
                GameSet::Spawning,
                GameSet::Effects,
                GameSet::Cleanup,
            )
                .chain(),
        );

        // If this compiles and runs, the GameSet works with Bevy's system set API
        app.update();
    }

    #[test]
    fn test_game_set_ordering_chain() {
        // Test that sets can be chained in the expected order
        let mut app = App::new();

        // Add a simple system to each set to verify ordering works
        fn input_system() {}
        fn movement_system() {}
        fn combat_system() {}

        app.configure_sets(
            Update,
            (GameSet::Input, GameSet::Movement, GameSet::Combat).chain(),
        );

        app.add_systems(Update, input_system.in_set(GameSet::Input));
        app.add_systems(Update, movement_system.in_set(GameSet::Movement));
        app.add_systems(Update, combat_system.in_set(GameSet::Combat));

        // Run the app to ensure no scheduling conflicts
        app.update();
    }
}
