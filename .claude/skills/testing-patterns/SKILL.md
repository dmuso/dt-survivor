---
name: testing-patterns
description: Testing patterns and TDD conventions for this Bevy project. Use when writing tests, creating test modules, testing systems, testing components, or asking about "how to test", "TDD", "unit test", "integration test", "App test", "event testing", "mock".
---

# Testing Patterns Skill

This skill provides guidance on testing patterns and TDD conventions for this project.

## Core Rules

- **TDD Required**: Write failing test first, then implement
- **90% Coverage**: Maintain via automated tests
- **Inline Tests**: Co-locate tests with implementation code
- **Run after every change**: `make lint && make test`

## Test Organization

Organize tests in nested modules by feature:

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
        // ...
    }
}
```

## Component Trait Verification

Validate Bevy traits at compile time:

```rust
#[test]
fn my_component_is_a_component() {
    fn assert_component<T: Component>() {}
    assert_component::<MyComponent>();
}
```

## App-Based System Testing

Test ECS systems using Bevy's App harness:

```rust
#[test]
fn test_apply_damage_reduces_health() {
    let mut app = App::new();
    app.add_message::<DamageEvent>();
    app.add_systems(Update, apply_damage_system);

    let entity = app.world_mut().spawn(Health::new(100.0)).id();
    app.world_mut().write_message(DamageEvent::new(entity, 25.0));

    app.update();

    let health = app.world().get::<Health>(entity).unwrap();
    assert_eq!(health.current, 75.0);
}
```

**Key patterns:**
- Use `world_mut()` for setup, `world()` for assertions
- Each `app.update()` runs one frame
- Systems run in isolation

## Event Counting

Validate event firing with thread-safe counters:

```rust
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
fn test_death_event_fires() {
    let mut app = App::new();
    let counter = DeathEventCounter(Arc::new(AtomicUsize::new(0)));
    app.insert_resource(counter.clone());
    app.add_systems(Update, (check_death_system, count_death_events).chain());
    // ... setup and update
    assert_eq!(counter.0.load(Ordering::SeqCst), 1);
}
```

## Fake Entity IDs

Use high-value IDs for placeholder entities:

```rust
let fake_target = Entity::from_raw_u32(99999).unwrap();
let fake_cage = Entity::from_bits(9999);
```

## Time-Based Testing

Advance time manually for timer tests:

```rust
#[test]
fn test_invincibility_expires() {
    let mut app = App::new();
    app.init_resource::<Time>();

    let entity = app.world_mut().spawn(Invincibility::new(0.1)).id();

    {
        let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
        time.advance_by(Duration::from_secs_f32(0.2));
    }

    app.update();
    assert!(app.world().get::<Invincibility>(entity).is_none());
}
```

## Graceful Degradation

Systems that depend on assets should accept `Option<Res<T>>`:

```rust
pub fn setup_animations(
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;  // Skip in tests without assets
    };
    // ...
}
```

## Test Naming

Use descriptive names:

```rust
// Good
fn health_take_damage_clamps_to_zero() { }
fn spell_damage_scales_with_level() { }

// Bad
fn test_health() { }
fn damage_test() { }
```

## Running Tests

```bash
make test              # Run all tests (quiet)
cargo test             # Run with output
cargo test combat::    # Specific module
```

## Visual Testing

For shaders and visual effects, **do not run the game** - use the screenshot testing system instead:

1. Create test scene in `src/visual_test/scenes.rs`
2. Capture: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. Inspect: Read `tmp/screenshots/<scene-name>.png`
4. Iterate until correct

See the **visual-testing skill** or [docs/visual-testing.md](../../../docs/visual-testing.md) for the complete workflow.

## Full Documentation

For complete patterns, see [docs/testing-patterns.md](../../../docs/testing-patterns.md).
