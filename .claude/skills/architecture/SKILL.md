---
name: architecture
description: Project architecture, file structure, and module organization patterns for the dt-survivor game. Use when adding new features, creating new modules, understanding code organization, or asking "where should this code go", "how is the project structured", "module pattern".
---

# Architecture Skill

This skill provides guidance on the project's domain-driven, plugin-based architecture.

## Core Principles

- **Domain-driven organization**: Group code by business domain, not technical type
- **Plugin-based architecture**: Each feature area exposes a plugin
- **Clear separation**: Components, systems, and resources logically separated
- **Scalable structure**: Easy to add features without disruption

## Key Modules

| Module | Purpose |
|--------|---------|
| `game/` | Core logic, GameSet ordering, resources |
| `spell/` | Spell system coordination |
| `spells/` | Individual spells by element (fire/, frost/, etc.) |
| `combat/` | Health, damage, death mechanics |
| `movement/` | Speed, velocity, knockback components |
| `player/` | Player entity and controls |
| `enemies/` | Enemy entities and AI |
| `loot/` | Item spawning and pickup |
| `ui/` | User interface systems |

## Module Structure Pattern

Each feature module follows this pattern:

```
my_feature/
├── mod.rs          # Re-exports public API
├── components.rs   # ECS components
├── systems.rs      # Game logic systems
├── resources.rs    # Global state (if needed)
├── events.rs       # Events (if needed)
└── plugin.rs       # Plugin composition
```

### mod.rs Template

```rust
pub mod components;
pub mod systems;
pub mod resources;      // if needed
pub mod plugin;

pub use components::*;
pub use systems::*;
pub use resources::*;   // if needed
pub use plugin::*;
```

### plugin.rs Template

```rust
use bevy::prelude::*;
use crate::states::*;
use crate::game::sets::GameSet;

pub fn plugin(app: &mut App) {
    app
        .add_event::<MyEvent>()
        .init_resource::<MyResource>()
        .add_systems(
            Update,
            my_system
                .in_set(GameSet::Movement)
                .run_if(in_state(GameState::InGame)),
        );
}
```

## Spell Module Pattern

Spells organized by element in `src/spells/`:

```
spells/fire/
├── mod.rs
├── fireball.rs         # Components, constants, fire/update systems
├── fireball_effects.rs # Particle effect resources
├── materials.rs        # Shader materials
└── inferno.rs          # Another spell
```

Each spell implementation has:
1. **Components** - Projectile markers, effect trackers
2. **Constants** - Speed, damage, durations
3. **Fire function** - `fire_<spell>()` to spawn
4. **Systems** - Movement, collision, effects
5. **Tests** - Inline tests for all behavior

## Advanced Patterns

### Option-Based Graceful Degradation

Systems that depend on assets should accept `Option<Res<T>>` to run in tests:

```rust
pub fn setup_animations(
    asset_server: Option<Res<AssetServer>>,
    graphs: Option<ResMut<Assets<AnimationGraph>>>,
) {
    let (Some(asset_server), Some(mut graphs)) = (asset_server, graphs) else {
        return;  // Skip in tests without assets
    };
    // ...
}
```

### Enum as Configuration Database

Use enums with computed properties for static configuration:

```rust
impl SpellType {
    pub fn element(&self) -> Element { ... }
    pub fn name(&self) -> &'static str { ... }
    pub fn base_damage(&self) -> f32 { ... }
    pub fn by_element(element: Element) -> &'static [SpellType] { ... }
}
```

**Benefits:** Type-safe, centralized, queryable, zero runtime cost.

## Development Workflow

1. Plan the feature: Identify which module(s) it belongs in
2. Create/update components
3. Implement systems
4. Create/update plugins
5. Add tests (90% coverage required)
6. Update documentation

## Full Documentation

For complete details including full module tree, see [docs/architecture.md](../../../docs/architecture.md).
