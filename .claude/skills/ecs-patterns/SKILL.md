---
name: ecs-patterns
description: ECS patterns and conventions for Bevy game development. Use when adding systems, components, events, or asking about GameSet ordering, system sets, composable components, plugin composition, marker components.
---

# ECS Patterns Skill

This skill provides guidance on established ECS patterns for this project.

## SystemSet Ordering (GameSet)

All gameplay systems must use `GameSet` for deterministic execution:

```
Input -> Movement -> Combat -> Spawning -> Effects -> Cleanup
```

```rust
use crate::game::sets::GameSet;

app.add_systems(
    Update,
    my_system
        .in_set(GameSet::Movement)
        .run_if(in_state(GameState::InGame)),
);
```

### Set Responsibilities

| Set | Purpose |
|-----|---------|
| `Input` | Keyboard, mouse, controller |
| `Movement` | Player, enemy, projectile, camera movement |
| `Combat` | Damage, collision, death checking |
| `Spawning` | Enemy spawning, projectile creation, loot |
| `Effects` | Visual effects, audio, regeneration |
| `Cleanup` | Despawning, timer expiration |

## Event-Driven Architecture

Use events for decoupled communication:

```rust
// Define in events.rs
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
}

// Register in plugin.rs
app.add_event::<DamageEvent>();

// Write events
fn deal_damage(mut events: EventWriter<DamageEvent>) {
    events.send(DamageEvent { target: entity, amount: 25.0 });
}

// Read events
fn apply_damage(mut events: EventReader<DamageEvent>) {
    for event in events.read() {
        // Handle damage
    }
}
```

**Event Registration Locations:**
- `combat/plugin.rs`: DamageEvent, DeathEvent, EnemyDeathEvent
- `game/plugin.rs`: CollisionEvents, GameOverEvent
- `loot/plugin.rs`: LootDropEvent, PickupEvent

### Event Ownership Convention

Each event type must be registered by exactly **one** plugin:

```rust
// combat/plugin.rs - OWNS these events
app.add_message::<DamageEvent>()
    .add_message::<DeathEvent>();

// enemy_death/plugin.rs - USES but doesn't register
// Note: EnemyDeathEvent owned by combat_plugin
app.add_systems(Update, handle_enemy_death);
```

**Rules:**
1. Register events where they are **produced**
2. Add comments in consuming plugins noting ownership
3. **Never** register same event in multiple plugins

## Composable Components

Use small, focused components that combine:

```rust
// Good: Separate components
commands.spawn((
    Player { speed: 200.0 },
    Health::new(100.0),
    Transform::default(),
));

// Bad: Monolithic component
commands.spawn(Player {
    speed: 200.0,
    health: 100.0,  // Don't embed health
    transform: ...,
});
```

### Key Composable Components

| Component | Module | Purpose |
|-----------|--------|---------|
| `Health` | combat | Health tracking |
| `Damage` | combat | Damage dealt |
| `Hitbox` | combat | Collision radius |
| `Speed` | movement | Movement speed |
| `Velocity` | movement | Direction + speed |
| `Knockback` | movement | Temporary pushback |
| `CheckDeath` | combat | Marker for death system |

## Marker Components

Use markers to opt entities into system behaviors:

```rust
// Only entities with CheckDeath trigger death events
fn check_death_system(
    query: Query<(Entity, &Health, &CheckDeath)>,
) {
    for (entity, health, _) in query.iter() {
        if health.is_dead() {
            // Handle death
        }
    }
}

// Enemies get CheckDeath, bullets don't
commands.spawn((Enemy::default(), Health::new(50.0), CheckDeath));
commands.spawn((Bullet::default(), Health::new(1.0)));  // No CheckDeath
```

## Type-Safe Enums

Always use enums over strings:

```rust
// Good
pub enum WeaponType { Pistol, Laser, Rocket }

// Bad
pub weapon_type: String  // "pistol", "laser"
```

## Plugin Composition

Break features into sub-plugins:

```rust
pub fn plugin(app: &mut App) {
    app.add_plugins((
        combat_plugin,
        movement_plugin,
        weapon_plugin,
    ));
}
```

## Full Documentation

For complete patterns, see [docs/ecs-patterns.md](../../../docs/ecs-patterns.md).
