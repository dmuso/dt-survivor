---
name: visual-testing
description: Visual testing workflow for shaders and visual effects. Use when working on shaders, visual effects, materials, or when you need to verify how something looks visually. Triggers on "test shader", "screenshot", "visual test", "how does it look", "verify visuals".
---

# Visual Testing Skill

This skill helps you verify shader and visual effect work using the screenshot testing system.

## Quick Workflow

1. **Create/identify test scene** in `src/visual_test/scenes.rs`
2. **Capture screenshot**: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. **Inspect result**: Read `tmp/screenshots/<scene-name>.png`
4. **Iterate**: Fix issues, re-capture, repeat

## Commands

```bash
# List available test scenes
nix-shell --run "cargo run -- --screenshot list"

# Capture a specific scene
nix-shell --run "cargo run -- --screenshot explosion-billowing-fire"
```

## Static vs Animated Tests

### Static Tests
- Spawn materials directly in `setup_visual_test_scene`
- Use pre-set progress values
- Good for: color showcases, material properties

### Animated Tests (IMPORTANT)
For effects with lifecycles (spawn → animate → despawn):

1. **Use real game spawners** - not static material hacks
2. **Capture ALL phases** - fire, transition, smoke, dissipation
3. **Handle shader warmup** - see pattern below

## Animated Effect Pattern

Animated effects require special handling due to shader compilation spikes.

### 5 Required Components

1. **GameState::VisualTest run condition** on effect systems:
   ```rust
   .run_if(in_state(GameState::InGame).or(in_state(GameState::VisualTest)))
   ```

2. **GameSet::Effects enabled for VisualTest** in `game/plugin.rs`

3. **Shader warmup sphere** - spawn off-screen in scene setup to force shader compile:
   ```rust
   commands.spawn((
       Mesh3d(meshes.explosion.clone()),
       MeshMaterial3d(materials.add(MyMaterial::new())),
       Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)), // Off-screen
   ));
   ```

4. **Warmup system spawning** - add to `warmup_then_spawn` in `visual_test/mod.rs`:
   ```rust
   if state.scene == TestScene::MyEffect {
       commands.spawn(MyEffectSpawner::new(position));
   }
   ```

5. **Fixed delta time** in effect update system for VisualTest:
   ```rust
   let delta = if *state.get() == GameState::VisualTest {
       std::time::Duration::from_millis(16)
   } else {
       time.delta()
   };
   ```

### Timing Configuration

For 0.8s lifetime effect:
- `frames_to_wait`: 35 (30 warmup + 5 after spawn)
- `frames_between_captures`: 8 (spreads 6 captures across lifecycle)
- `total_frames`: 6 (captures at ~10%, 27%, 44%, 60%, 77%, 94%)

### Full Lifecycle Coverage

Visual tests MUST capture ALL phases:

| Frame | Progress | Phase |
|-------|----------|-------|
| 000 | ~10% | Initial state |
| 001-002 | ~27-44% | Animation |
| 003-004 | ~60-77% | Transition |
| 005 | ~94% | Final state / dissipation |

**If a phase is missing, the test won't catch regressions in that phase.**

## Approach

* Visual tests must reflect in-game rendering - use real systems, animations, movement
* Capture the FULL effect lifecycle to catch regressions in any phase
* Use wider camera for effects that spread out
* Add comments explaining intended visual effect
* Never commit broken visuals

## Checklist for Animated Effects

- [ ] Effect systems have `GameState::VisualTest` run condition
- [ ] Shader warmup sphere spawned off-screen
- [ ] Effect spawner added to `warmup_then_spawn`
- [ ] Fixed delta time in effect update for VisualTest
- [ ] `frames_to_wait` accounts for 30-frame warmup
- [ ] Screenshots show ALL phases of effect lifecycle

## Full Documentation

See [docs/visual-testing.md](../../../docs/visual-testing.md) for complete details.
