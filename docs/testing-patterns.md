# Testing Patterns Guide

This project uses TDD with 90% code coverage target. All tests are co-located with implementation code in inline `#[cfg(test)]` modules.

## Test Organization

### Nested Module Structure

Organize tests in feature-specific nested modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod health_tests {
        use super::*;

        #[test]
        fn take_damage_reduces_current() { ... }

        #[test]
        fn take_damage_clamps_to_zero() { ... }
    }

    mod damage_tests {
        use super::*;

        #[test]
        fn damage_multiplier_applies() { ... }
    }
}
```

Examples: `combat/components.rs`, `experience/components.rs`

## Component Trait Verification

Validate that types correctly implement Bevy traits at compile time:

```rust
#[test]
fn my_component_is_a_component() {
    fn assert_component<T: Component>() {}
    assert_component::<MyComponent>();
}

#[test]
fn my_resource_is_a_resource() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<MyResource>();
}
```

This catches missing derives early. Found in: `ui/components.rs`, `pause/components.rs`

## App-Based System Integration Testing

Test ECS systems using Bevy's `App` test harness:

```rust
use bevy::prelude::*;

#[test]
fn test_apply_damage_reduces_health() {
    // 1. Create isolated app
    let mut app = App::new();

    // 2. Register required events and systems
    app.add_message::<DamageEvent>();
    app.add_systems(Update, apply_damage_system);

    // 3. Spawn test entities
    let entity = app.world_mut().spawn(Health::new(100.0)).id();

    // 4. Send test events
    app.world_mut().write_message(DamageEvent::new(entity, 25.0));

    // 5. Run one frame
    app.update();

    // 6. Assert results
    let health = app.world().get::<Health>(entity).unwrap();
    assert_eq!(health.current, 75.0);
}
```

Key patterns:
- Use `world_mut()` for setup, `world()` for assertions
- Each `app.update()` runs one frame
- Systems run in isolation without other game systems

## Event Counting

Validate that events fire correctly using thread-safe counters:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Resource, Clone)]
struct DeathEventCounter(Arc<AtomicUsize>);

fn count_death_events(
    mut events: MessageReader<DeathEvent>,
    counter: Res<DeathEventCounter>,
) {
    for _ in events.read() {
        counter.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_check_death_fires_event_when_dead() {
    let mut app = App::new();
    let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));

    app.add_message::<DeathEvent>();
    app.insert_resource(counter.clone());
    app.add_systems(Update, (check_death_system, count_death_events).chain());

    // Spawn entity with zero health
    app.world_mut().spawn((Health::new(0.0), CheckDeath));

    app.update();

    assert_eq!(counter.0.load(Ordering::SeqCst), 1);
}
```

## Fake Entity IDs

Create placeholder entities for component initialization tests:

```rust
#[test]
fn test_effect_component_stores_target() {
    // Use high-value IDs that won't conflict with real entities
    let fake_target = Entity::from_raw_u32(99999).unwrap();
    // Or alternatively:
    let fake_cage = Entity::from_bits(9999);

    let effect = MyEffect::new(fake_target);
    assert_eq!(effect.target, fake_target);
}
```

Found in: lightning, psychic, dark, and light spell tests

## Time-Based Testing

Test timer and duration-dependent logic by advancing time manually:

```rust
#[test]
fn test_invincibility_expires_after_duration() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_systems(Update, tick_invincibility_system);

    let entity = app.world_mut().spawn(Invincibility::new(0.1)).id();

    // Advance time past the duration
    {
        let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
        time.advance_by(Duration::from_secs_f32(0.2));
    }

    app.update();

    // Component should be removed
    assert!(app.world().get::<Invincibility>(entity).is_none());
}
```

## Graceful Degradation in Systems

Systems that depend on assets should accept `Option<Res<T>>` to run in test environments:

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

This allows systems to be added to test apps without providing full asset pipelines.

## Testing Patterns by Domain

### Component Tests
- Test calculated properties and methods
- Verify boundary conditions (zero, max, negative)
- Include `Debug` derive for readable assertion failures

### System Tests
- Use App-based integration tests
- Test one system behavior per test
- Mock dependencies via direct entity spawning

### Event Tests
- Use event counters to verify firing
- Test event data correctness
- Chain producer and consumer systems

## Test Naming Conventions

Use descriptive names that explain behavior:

```rust
// Good - describes what and expected outcome
#[test]
fn health_take_damage_clamps_to_zero() { ... }

#[test]
fn invincibility_prevents_damage_application() { ... }

#[test]
fn spell_damage_scales_with_level() { ... }

// Avoid - too vague
#[test]
fn test_health() { ... }

#[test]
fn damage_test() { ... }
```

## Running Tests

```bash
# Run all tests (quiet mode)
make test

# Run tests with output
cargo test

# Run specific module tests
cargo test combat::

# Run with coverage (if configured)
cargo tarpaulin --out Html
```

## Visual Testing

For shaders and visual effects, **do not run the game** to verify - use the screenshot testing system instead.

### Quick Workflow

1. Create test scene in `src/visual_test/scenes.rs`
2. Capture: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. Inspect: Read `tmp/screenshots/<scene-name>.png`
4. Iterate until correct

### Requirements

- Every shader change MUST have a corresponding test scene
- Always inspect the screenshot before considering work complete
- Capture multiple frames for animated effects

See [docs/visual-testing.md](visual-testing.md) for the complete visual testing workflow.
