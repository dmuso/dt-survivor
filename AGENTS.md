# Donny Tango: Survivor

A game in the style of Vampire Survivors and Brotato, built with Rust and the Bevy ECS framework.

## Development Commands

All development tooling commands require Nix Shell to run.

- Type Checking: `nix-shell --run "cargo check"`
- Linting: `nix-shell --run "cargo clippy"`
- Testing: `nix-shell --run "cargo test"`
- Building: `nix-shell --run "cargo build"`
- Running: `nix-shell --run "cargo run"`

## Testing and Linting

- You should maintain 90% code coverage via automated tests
- Run linting and testing after every change
- Fix any errors or warnings that you get as feedback from linting and tests
- Write tests inline with code

## File Structure and Code Organization

This project follows a domain-driven, modular architecture to support the complex features planned for the survivor game (player classes, enemy AI, inventory, weapons, levels, etc.).

### Core Architecture Principles

- **Domain-driven organization**: Group code by business domain (game logic, UI, etc.) rather than technical type
- **Plugin-based architecture**: Each major feature area exposes a plugin for easy composition and testing
- **Clear separation of concerns**: Components, systems, and resources are logically separated
- **Scalable structure**: Easy to add new features without disrupting existing code

### Current Module Structure

```
src/
├── lib.rs              # Library exports and plugin composition
├── main.rs             # Minimal app entry point using plugins
├── prelude.rs          # Common imports used across modules
├── states.rs           # Game state management (GameState enum)
├── game/               # Core game logic
│   ├── mod.rs
│   ├── components.rs   # Game entity components (Player, Enemy, etc.)
│   ├── systems.rs      # Game systems (movement, combat, AI)
│   ├── resources.rs    # Game resources (score, settings)
│   └── plugin.rs       # Game plugin composition
├── ui/                 # User interface systems
│   ├── mod.rs
│   ├── components.rs   # UI components (buttons, menus, HUD)
│   ├── systems.rs      # UI interaction systems
│   └── plugin.rs       # UI plugin composition
└── [future modules]    # Additional feature modules as needed
```

### Module Organization Patterns

Each feature module should follow this pattern:

#### 1. Module Definition (`mod.rs`)
```rust
pub mod components;
pub mod systems;
pub mod resources;      // if needed
pub mod plugin;

// Re-export public API
pub use components::*;
pub use systems::*;
pub use resources::*;   // if needed
pub use plugin::*;
```

#### 2. Components (`components.rs`)
- Define ECS components for the domain
- Use descriptive names and derive necessary traits
- Group related components together

```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub health: f32,
    pub speed: f32,
}

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub damage: f32,
}
```

#### 3. Systems (`systems.rs`)
- Implement game logic systems
- Use clear, descriptive function names
- Group related systems and use system sets for ordering

```rust
use bevy::prelude::*;
use crate::game::components::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystems {
    Movement,
    Combat,
    AI,
}

pub fn player_movement_system(
    mut query: Query<(&mut Transform, &Player)>,
    time: Res<Time>,
) {
    // Movement logic
}

pub fn enemy_ai_system(
    mut query: Query<(&Transform, &mut Enemy)>,
    player_query: Query<&Transform, With<Player>>,
) {
    // AI logic
}
```

#### 4. Resources (`resources.rs`) - When Needed
- Define global game state
- Use for configuration and shared data

```rust
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    pub difficulty: Difficulty,
    pub sound_enabled: bool,
}

#[derive(Resource, Default)]
pub struct Score(pub u32);
```

#### 5. Plugin (`plugin.rs`)
- Compose systems into logical plugins
- Use run conditions and state management
- Register events and resources

```rust
use bevy::prelude::*;
use crate::states::*;
use crate::game::systems::*;

pub fn plugin(app: &mut App) {
    app
        .add_event::<PlayerDamaged>()
        .init_resource::<Score>()
        .add_systems(
            Update,
            (
                player_movement_system,
                enemy_ai_system,
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .in_set(GameSystems::Movement),
        )
        .add_systems(
            OnEnter(GameState::Playing),
            spawn_player,
        );
}
```

### Future Module Planning

As the game grows, plan for these additional modules:

- **player/**: Player-specific logic (classes, abilities, progression)
- **enemies/**: Enemy spawning, AI, and types
- **combat/**: Damage, health, and combat mechanics
- **inventory/**: Items, weapons, and equipment
- **levels/**: Level progression and world management
- **audio/**: Sound effects and music management
- **assets/**: Asset loading and management

### Import Strategy

- Use `prelude.rs` for common Bevy imports and local types
- Import specific items rather than glob imports when possible
- Keep imports organized and minimal

### Testing Strategy

- Tests should be co-located with the code they test
- Test components, systems, and integration scenarios
- Maintain 90% code coverage across all modules
- Use descriptive test names that explain what they're testing

### Development Workflow

1. **Plan the feature**: Identify which module(s) it belongs in
2. **Create/update components**: Add necessary ECS components
3. **Implement systems**: Write the game logic
4. **Create/update plugins**: Wire systems together
5. **Add tests**: Ensure functionality works correctly
6. **Update documentation**: Keep AGENTS.md current
7. **Run full test suite**: Verify no regressions
