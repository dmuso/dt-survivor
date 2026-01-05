# Project Architecture

This project follows a domain-driven, modular architecture to support the complex features planned for the survivor game (player classes, enemy AI, inventory, weapons, levels, etc.).

## Core Architecture Principles

- **Domain-driven organization**: Group code by business domain (game logic, UI, etc.) rather than technical type
- **Plugin-based architecture**: Each major feature area exposes a plugin for easy composition and testing
- **Clear separation of concerns**: Components, systems, and resources are logically separated
- **Scalable structure**: Easy to add new features without disrupting existing code

## Current Module Structure

```
src/
├── lib.rs              # Library exports and plugin composition
├── main.rs             # Minimal app entry point using plugins
├── prelude.rs          # Common imports used across modules
├── states.rs           # Game state management (GameState enum)
│
├── game/               # Core game logic and resources
│   ├── mod.rs
│   ├── components.rs   # World components (Arena, etc.)
│   ├── events.rs       # Game events (CollisionEvent, GameOverEvent)
│   ├── sets.rs         # SystemSet definitions (GameSet enum)
│   ├── systems.rs      # Core game systems (spawning, physics)
│   ├── resources.rs    # GameMeshes, GameMaterials resources
│   └── plugin.rs       # Game plugin composition
│
├── spell/              # Spell system coordination
│   ├── mod.rs
│   ├── components.rs   # Spell base components
│   ├── resources.rs    # Spell resources
│   ├── systems.rs      # Spell casting system (iterates SpellList)
│   └── plugin.rs       # Registers all spell plugins
│
├── spells/             # Individual spell implementations by element
│   ├── mod.rs          # Re-exports all spell modules
│   ├── fire/           # Fire element spells
│   │   ├── mod.rs
│   │   ├── fireball.rs # Components, constants, fire/update systems
│   │   ├── fireball_effects.rs  # Particle effect resources
│   │   ├── materials.rs # FireballCoreMaterial, ExplosionFireMaterial, etc.
│   │   └── inferno.rs
│   ├── frost/          # Frost element spells
│   ├── lightning/      # Lightning element spells
│   ├── psychic/        # Psychic element spells
│   └── light/          # Light element spells
│
├── combat/             # Damage, health, and combat mechanics
│   ├── mod.rs
│   ├── components.rs   # Health, Damage, Hitbox, Invincibility, CheckDeath
│   ├── events.rs       # DamageEvent, DeathEvent
│   ├── systems.rs      # apply_damage, check_death, handle_enemy_death
│   └── plugin.rs       # Combat plugin composition
│
├── movement/           # Reusable movement components and systems
│   ├── mod.rs
│   ├── components.rs   # Speed, Velocity, Knockback, from_xz()
│   ├── systems.rs      # apply_velocity, player_movement, enemy_movement
│   └── plugin.rs       # Movement plugin composition
│
├── player/             # Player entity and controls
│   ├── mod.rs
│   ├── components.rs   # Player component
│   └── systems.rs      # Player systems
│
├── enemies/            # Enemy entities and AI
│   ├── mod.rs
│   ├── components.rs   # Enemy component, spawn patterns
│   └── systems.rs      # Enemy AI, spawning, movement toward player
│
├── enemy_death/        # Enemy death handling and effects
│   ├── mod.rs
│   ├── systems.rs      # Enemy death particles, sounds, loot drops
│   └── plugin.rs       # Enemy death plugin composition
│
├── inventory/          # Player inventory and spell management
│   ├── mod.rs
│   ├── components.rs   # Inventory components
│   ├── resources.rs    # SpellList (5 spell slots)
│   ├── systems.rs      # Inventory systems
│   └── plugin.rs       # Inventory plugin composition
│
├── loot/               # Loot spawning and pickup
│   ├── mod.rs
│   ├── components.rs   # Loot components (XP orbs, items)
│   ├── systems.rs      # Loot attraction, pickup, magnet range
│   └── plugin.rs       # Loot plugin composition
│
├── experience/         # Experience and leveling
│   ├── mod.rs
│   ├── components.rs   # Experience components
│   ├── resources.rs    # PlayerLevel, XP thresholds
│   ├── systems.rs      # Experience gain, level up
│   └── plugin.rs       # Experience plugin composition
│
├── powerup/            # Power-up selection on level up
│   ├── mod.rs
│   ├── components.rs   # PowerUp types
│   ├── systems.rs      # PowerUp UI, selection
│   └── plugin.rs       # PowerUp plugin composition
│
├── ui/                 # User interface systems
│   ├── mod.rs
│   ├── components.rs   # UI components (HUD, menus)
│   ├── materials.rs    # RadialCooldownMaterial (UiMaterial)
│   ├── systems.rs      # UI update systems
│   └── plugin.rs       # UI plugin composition
│
├── camera/             # Camera setup and control
│   ├── mod.rs
│   ├── systems.rs      # Camera spawn, follow player, HDR/Bloom config
│   └── plugin.rs       # Camera plugin composition
│
├── arena/              # Arena/level setup
│   ├── mod.rs
│   ├── components.rs   # Arena boundaries
│   ├── systems.rs      # Arena spawning
│   └── plugin.rs       # Arena plugin composition
│
├── pause/              # Pause menu
│   ├── mod.rs
│   ├── components.rs   # Pause state
│   ├── systems.rs      # Pause/resume toggle
│   └── plugin.rs       # Pause plugin composition
│
├── whisper/            # Whisper attunement system
│   ├── mod.rs
│   ├── components.rs   # Whisper components
│   ├── resources.rs    # WhisperAttunement, SpellOrigin
│   ├── systems.rs      # Whisper mechanics
│   └── plugin.rs       # Whisper plugin composition
│
├── element/            # Element types (Fire, Frost, etc.)
│   └── mod.rs          # Element enum definition
│
├── audio/              # Audio management
│   ├── mod.rs
│   ├── components.rs   # Audio components
│   ├── systems.rs      # Audio systems
│   └── plugin.rs       # Audio plugin composition
│
├── score/              # Score tracking
│   ├── mod.rs
│   ├── components.rs   # Score components
│   ├── resources.rs    # Score resource
│   └── systems.rs      # Score systems
│
└── visual_test/        # Visual testing utilities
    └── mod.rs          # Debug visualization helpers
```

## Module Organization Patterns

Each feature module should follow this pattern:

### 1. Module Definition (`mod.rs`)
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

### 2. Components (`components.rs`)
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

### 3. Systems (`systems.rs`)
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

### 4. Resources (`resources.rs`) - When Needed
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

### 5. Plugin (`plugin.rs`)
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

## Spells Module Structure

The `src/spells/` directory organizes spells by element with shared materials:

```
src/spells/
├── mod.rs              # Re-exports all spell modules
├── fire/               # Fire element spells
│   ├── mod.rs
│   ├── fireball.rs     # Fireball projectile, charging, collision, explosion
│   ├── fireball_effects.rs  # Particle effect resources
│   ├── materials.rs    # All fire shader materials (core, charge, trail, explosion)
│   └── inferno.rs      # Fire nova spell
├── frost/              # Frost element spells
│   ├── mod.rs
│   ├── ice_shard.rs
│   ├── glacial_pulse.rs
│   └── materials.rs    # Frost shader materials
├── lightning/          # Lightning element spells
│   ├── mod.rs
│   ├── thunder_strike.rs
│   └── materials.rs
├── psychic/            # Psychic element spells
│   ├── mod.rs
│   ├── echo_thought.rs
│   └── mind_cage.rs
└── light/              # Light element spells
    ├── mod.rs
    └── radiant_beam.rs
```

**Spell Module Pattern:**

Each spell implementation follows this structure:
1. **Components** - Projectile markers, effect trackers, timers
2. **Constants** - Speed, damage ratios, durations, collision radii
3. **Fire function** - `fire_<spell>()` to spawn the spell entity
4. **Systems** - Movement, lifetime, collision detection, effect updates
5. **Tests** - Inline tests for all behavior

## Advanced Patterns

### Option-Based Graceful Degradation

Systems that depend on assets should accept `Option<Res<T>>` to allow running in test environments without full asset pipelines:

```rust
pub fn setup_player_animations(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    graphs: Option<ResMut<Assets<AnimationGraph>>>,
) {
    // Early return if resources unavailable (common in tests)
    let (Some(asset_server), Some(mut graphs)) = (asset_server, graphs) else {
        return;
    };

    // Proceed with asset-dependent setup
    let animation = asset_server.load("animations/player.gltf#Animation0");
    // ...
}
```

**When to use:**
- Systems that load assets (textures, models, animations)
- Systems that depend on external resources
- Any system that should be testable without a full Bevy app

**Do not use for:**
- Core game logic that should always have its dependencies
- Systems where missing resources indicate a bug

### Enum as Configuration Database

Use enums with computed properties to store static configuration instead of config files or resources:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpellType {
    Fireball,
    IceShard,
    ThunderStrike,
    // ...
}

impl SpellType {
    /// Get the element for this spell type
    pub fn element(&self) -> Element {
        match self {
            Self::Fireball => Element::Fire,
            Self::IceShard => Element::Frost,
            Self::ThunderStrike => Element::Lightning,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fireball => "Fireball",
            Self::IceShard => "Ice Shard",
            Self::ThunderStrike => "Thunder Strike",
        }
    }

    /// Get base damage for this spell
    pub fn base_damage(&self) -> f32 {
        match self {
            Self::Fireball => 25.0,
            Self::IceShard => 15.0,
            Self::ThunderStrike => 40.0,
        }
    }

    /// Query spells by element
    pub fn by_element(element: Element) -> &'static [SpellType] {
        match element {
            Element::Fire => &[Self::Fireball, Self::Inferno],
            Element::Frost => &[Self::IceShard, Self::GlacialPulse],
            // ...
        }
    }

    /// Get all spell types
    pub fn all() -> &'static [SpellType] {
        &[Self::Fireball, Self::IceShard, Self::ThunderStrike, /* ... */]
    }
}
```

**Benefits:**
- Type-safe: compiler catches invalid spell types
- Centralized: all spell metadata in one place
- Queryable: can filter by properties
- Zero runtime cost: all values are compile-time constants

**Location:** `src/spell/components.rs`

## Future Module Planning

As the game grows, plan for these additional modules:

- **levels/**: Level progression and world management
- **assets/**: Asset loading and management

## Development Workflow

1. **Plan the feature**: Identify which module(s) it belongs in
2. **Create/update components**: Add necessary ECS components
3. **Implement systems**: Write the game logic
4. **Create/update plugins**: Wire systems together
5. **Add tests**: Ensure functionality works correctly
6. **Update documentation**: Keep docs current
7. **Run full test suite**: Verify no regressions
