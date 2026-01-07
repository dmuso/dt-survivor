# Visual Testing Guide

When working on shaders or visual effects, you MUST verify your work visually using the screenshot tool.

## Workflow

1. **Create a test scene** for the effect you're working on in `src/visual_test/scenes.rs`
2. **Run the screenshot capture**: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. **Read the screenshot**: Use Read tool on `tmp/screenshots/<scene-name>.png`
4. **Iterate**: Fix issues, re-capture, repeat until it looks right

## Types of Visual Tests

### Static Tests
For effects that don't animate or where you want to show different states side-by-side:
- Spawn entities directly in `setup_visual_test_scene` during Startup
- Use static materials with pre-set progress values
- Good for: color gradients, material property showcases, progress states

### Animated Tests
For effects that move, scale, change color, or have lifecycles:
- Use real game spawners (e.g., `BillowingFireSpawner`)
- Let actual game systems animate the effect
- Capture multiple frames across the effect's lifecycle
- **Must capture ALL phases** to catch regressions in any phase

## Animated Effect Implementation Pattern

Animated effects require special handling because:
1. Shader compilation causes 250ms+ frame spikes
2. Effect lifetimes get consumed during slow frames
3. Effects may despawn before screenshots are taken

### Required Components

#### 1. GameState::VisualTest
Effect systems must run in VisualTest state. Update `spell/plugin.rs`:
```rust
.run_if(in_state(GameState::InGame).or(in_state(GameState::VisualTest)))
```

#### 2. GameSet::Effects Enabled
In `game/plugin.rs`, GameSet::Effects runs in both states:
```rust
.configure_sets(
    Update,
    GameSet::Effects
        .after(GameSet::Spawning)
        .run_if(in_state(GameState::InGame).or(in_state(GameState::VisualTest))),
)
```

#### 3. Shader Warmup Sphere
Spawn an off-screen sphere in the scene setup to force shader compilation:
```rust
fn spawn_my_effect_test(commands: &mut Commands, meshes: &GameMeshes, materials: &mut Assets<MyMaterial>) {
    spawn_lighting(commands);

    // Force shader compile with off-screen sphere
    let warmup_handle = materials.add(MyMaterial::new());
    commands.spawn((
        Mesh3d(meshes.my_mesh.clone()),
        MeshMaterial3d(warmup_handle),
        Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)), // Off-screen
    ));
}
```

#### 4. Warmup System Spawning
Animated effects are spawned by `warmup_then_spawn` AFTER 30 frames of warmup:
```rust
// In visual_test/mod.rs
if warmup.0 == WARMUP_FRAMES {
    if state.scene == TestScene::MyAnimatedEffect {
        commands.spawn(MyEffectSpawner::new(position));
    }
}
```

#### 5. Fixed Delta Time
Effect update systems use fixed 16ms delta in VisualTest mode:
```rust
let delta = if *state.get() == GameState::VisualTest {
    std::time::Duration::from_millis(16)
} else {
    time.delta()
};
effect.lifetime.tick(delta);
```

### Timing Configuration

For an effect with 0.8s lifetime at 60fps (48 frames):

```rust
// frames_to_wait = warmup (30) + frames after spawn (5)
TestScene::MyEffect => 35,

// frames_between_captures - spread across lifetime
// 6 captures with 8-frame gaps = captures at 10%, 27%, 44%, 60%, 77%, 94%
TestScene::MyEffect => 8,

// total_frames - enough to cover all phases
TestScene::MyEffect => 6,
```

### Full Lifecycle Coverage

Visual tests MUST capture all phases of an effect. Example for billowing fire:

| Frame | Progress | Phase |
|-------|----------|-------|
| 000 | ~10% | Fire phase - bright orange |
| 001 | ~27% | Fire spreading |
| 002 | ~44% | Mid-transition |
| 003 | ~60% | Color shifting |
| 004 | ~77% | Smoke phase - gray color |
| 005 | ~94% | Dissipation |

If any phase is missing, the test won't catch regressions in that phase.

## Creating a Test Scene

Add to the `TestScene` enum in `src/visual_test/scenes.rs`:

```rust
pub enum TestScene {
    FireballTrailEast,
    MyNewEffect,  // Add your scene
}

impl TestScene {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MyNewEffect => "my-new-effect",
            // ...
        }
    }

    pub fn camera_position(&self) -> Vec3 {
        match self {
            // Wider camera for effects that spread out
            Self::MyNewEffect => Vec3::new(0.0, 5.0, 12.0),
            // ...
        }
    }

    pub fn frames_to_wait(&self) -> u32 {
        match self {
            // 30 warmup + 5 after spawn
            Self::MyNewEffect => 35,
            // ...
        }
    }

    pub fn total_frames(&self) -> u32 {
        match self {
            Self::MyNewEffect => 6,
            // ...
        }
    }

    pub fn frames_between_captures(&self) -> u32 {
        match self {
            Self::MyNewEffect => 8,
            // ...
        }
    }
}

impl FromStr for TestScene {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "my-new-effect" => Ok(TestScene::MyNewEffect),
            // ...
        }
    }
}
```

## Running Visual Tests

```bash
# List available test scenes
nix-shell --run "cargo run -- --screenshot list"

# Capture screenshot of a scene
nix-shell --run "cargo run -- --screenshot my-new-effect"

# Screenshots saved to tmp/screenshots/
```

## Requirements

- Every shader change must have a corresponding test scene
- Always inspect the screenshot before considering shader work complete
- If something looks wrong, fix it - don't commit broken visuals
- Animated effects must capture the FULL lifecycle to catch all regressions
- Use real game spawners for animated effects, not static material hacks

## Checklist for Animated Effect Tests

- [ ] Added `GameState::VisualTest` run condition to effect systems
- [ ] Shader warmup sphere spawned off-screen in scene setup
- [ ] Effect spawner added to `warmup_then_spawn` system
- [ ] Fixed delta time used in effect update system for VisualTest
- [ ] `frames_to_wait` accounts for 30-frame warmup
- [ ] `frames_between_captures` spreads captures across full lifetime
- [ ] Screenshots verified to show ALL phases (fire, transition, smoke, dissipation, etc.)
