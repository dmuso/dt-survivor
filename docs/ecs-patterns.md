# ECS Patterns Guide

This document covers the established ECS patterns that must be followed when adding new features.

## SystemSet Ordering with GameSet

All gameplay systems must be assigned to a `GameSet` to ensure deterministic execution order. The sets are defined in `src/game/sets.rs` and chained in order:

```rust
use crate::game::sets::GameSet;

// GameSet ordering: Input -> Movement -> Combat -> Spawning -> Effects -> Cleanup

// In your plugin:
app.add_systems(
    Update,
    my_movement_system
        .in_set(GameSet::Movement)
        .run_if(in_state(GameState::InGame)),
);
```

**Set Responsibilities:**
- `GameSet::Input` - Keyboard, mouse, and controller input handling
- `GameSet::Movement` - Player, enemy, projectile, and camera movement
- `GameSet::Combat` - Damage calculation, collision detection, death checking
- `GameSet::Spawning` - Enemy spawning, projectile creation, loot drops
- `GameSet::Effects` - Visual effects, screen tints, audio triggers, regeneration
- `GameSet::Cleanup` - Entity despawning, timer expiration, garbage collection

## Event-Driven Architecture

Use events (Bevy Messages) for decoupled communication between systems. Events are centralized in module `events.rs` files.

```rust
// Define events in events.rs
#[derive(Message)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
}

// Register in plugin.rs
app.add_message::<DamageEvent>();

// Write events
fn deal_damage(mut messages: MessageWriter<DamageEvent>) {
    messages.write(DamageEvent { target: entity, amount: 25.0 });
}

// Read events
fn apply_damage(mut messages: MessageReader<DamageEvent>) {
    for event in messages.read() {
        // Handle damage
    }
}
```

**Centralized Event Registration:**
- `combat/plugin.rs`: DamageEvent, DeathEvent, EnemyDeathEvent
- `game/plugin.rs`: PlayerEnemyCollisionEvent, BulletEnemyCollisionEvent, GameOverEvent
- `loot/plugin.rs`: LootDropEvent, PickupEvent, ItemEffectEvent

**Do not** register the same event in multiple plugins.

### Event Ownership Convention

Each event type must be registered by exactly **one** plugin. This prevents duplicate registration bugs and makes event ownership clear.

```rust
// combat/plugin.rs - OWNS these events
pub fn plugin(app: &mut App) {
    app.add_message::<DamageEvent>()
        .add_message::<DeathEvent>()
        .add_message::<EnemyDeathEvent>();
    // ...
}

// enemy_death/plugin.rs - USES EnemyDeathEvent but does NOT register it
pub fn plugin(app: &mut App) {
    // Note: EnemyDeathEvent is registered by combat_plugin
    app.add_systems(Update, handle_enemy_death.in_set(GameSet::Effects));
}
```

**Rules:**
1. Events are registered where they are **produced**, not consumed
2. Add comments in consuming plugins noting which plugin owns the event
3. If multiple plugins produce an event, choose one canonical owner
4. Never call `add_message::<T>()` for the same type in multiple plugins

## Composable Component Design

Use small, focused components that can be combined rather than monolithic entity-specific components.

### Health Component (combat module)
```rust
// Use the shared Health component instead of embedding health in Player/Enemy
use crate::combat::Health;

// Spawn player with separate Health component
commands.spawn((
    Player { speed: 200.0 },
    Health::new(100.0),
    Transform::default(),
));
```

### Movement Components (movement module)
```rust
use crate::movement::{Speed, Velocity, Knockback};

// Entities use composable movement components
commands.spawn((
    Enemy { enemy_type: EnemyType::Basic },
    Speed(150.0),
    Velocity::from_direction_and_speed(direction, 150.0),
    Transform::default(),
));

// Apply temporary knockback
commands.entity(entity).insert(Knockback::from_direction(hit_direction));
```

### Combat Components (combat module)
```rust
use crate::combat::{Health, Damage, Hitbox, Invincibility, CheckDeath};

// Enemy with full combat components
commands.spawn((
    Enemy { enemy_type: EnemyType::Tank },
    Health::new(200.0),
    Damage(15.0),
    Hitbox(20.0),
    CheckDeath,  // Marker for death checking system
    Transform::default(),
));

// Grant temporary invincibility
commands.entity(player).insert(Invincibility::new(2.0));
```

## Type-Safe Enums Over Strings

Always use enums instead of strings for type identification to leverage compile-time checking.

```rust
// WRONG - string comparison is error-prone
pub struct WeaponIcon {
    pub weapon_type: String,  // "pistol", "laser", etc.
}

// CORRECT - use WeaponType enum
use crate::weapon::WeaponType;

pub struct WeaponIcon {
    pub weapon_type: WeaponType,
}

// Pattern matching is compile-time checked
match weapon_type {
    WeaponType::Pistol { bullet_count, spread_angle } => { /* ... */ }
    WeaponType::Laser => { /* ... */ }
    WeaponType::RocketLauncher => { /* ... */ }
}
```

## Generic Cleanup Components

Use a single generic cleanup component with an enum discriminator instead of creating separate cleanup components.

```rust
use crate::audio::components::{CleanupTimer, CleanupType};

// WRONG - creating separate timer types
#[derive(Component)]
pub struct AudioCleanupTimer(Timer);
#[derive(Component)]
pub struct LootCleanupTimer(Timer);

// CORRECT - single generic component with type enum
commands.spawn((
    AudioSource { /* ... */ },
    CleanupTimer::from_secs(2.0, CleanupType::Audio),
));

commands.spawn((
    LootPickupSound,
    CleanupTimer::from_secs(1.5, CleanupType::Loot),
));
```

## Prelude Re-exports

Commonly used types should be re-exported via `src/prelude.rs` for convenient access across the codebase.

```rust
// In prelude.rs
pub use crate::combat::{Damage, DamageEvent, DeathEvent, EntityType, Health, Hitbox, Invincibility};
pub use crate::movement::{Knockback, Speed, Velocity};
pub use crate::weapon::WeaponType;

// Usage in other modules
use crate::prelude::*;

fn my_system(query: Query<(&Health, &Speed, &Velocity)>) {
    // Types are available without explicit imports
}
```

## Marker Components for System Filtering

Use marker components to opt entities into specific system behaviors rather than checking entity types.

```rust
use crate::combat::CheckDeath;

// Entities must have CheckDeath marker to be processed by death system
fn check_death_system(
    query: Query<(Entity, &Health, &Transform, &CheckDeath)>,
    mut messages: MessageWriter<DeathEvent>,
) {
    for (entity, health, transform, _) in query.iter() {
        if health.is_dead() {
            messages.write(DeathEvent::new(entity, transform.translation, EntityType::Enemy));
        }
    }
}

// Only add CheckDeath to entities that should trigger death events
commands.spawn((Enemy::default(), Health::new(50.0), CheckDeath));
commands.spawn((Bullet::default(), Health::new(1.0))); // No CheckDeath - bullets don't fire death events
```

## Sub-Plugin Composition

Complex features should be broken into sub-plugins that the main game plugin composes.

```rust
// In game/plugin.rs
use crate::combat::plugin as combat_plugin;
use crate::movement::plugin as movement_plugin;
use crate::weapon::plugin as weapon_plugin;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        combat_plugin,
        movement_plugin,
        weapon_plugin,
        // ... other sub-plugins
    ))
    // Game-specific systems
    .add_systems(Update, game_specific_system.in_set(GameSet::Combat));
}
```

## Import Strategy

- Use `prelude.rs` for common Bevy imports and local types
- Import specific items rather than glob imports when possible
- Keep imports organized and minimal

## Testing Strategy

- Use visual tests and confirm output image files, do not run the game itself
- Capture multiple frames to test animated visuals
- Tests should be co-located with the code they test
- Test components, systems, and integration scenarios
- Maintain 90% code coverage across all modules
- Use descriptive test names that explain what they're testing
