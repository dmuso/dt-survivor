# Visual Testing Guide

When working on shaders or visual effects, you MUST verify your work visually using the screenshot tool.

## Workflow

1. **Create a test scene** for the effect you're working on in `src/visual_test/scenes.rs`
2. **Run the screenshot capture**: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. **Read the screenshot**: Use Read tool on `tmp/screenshots/<scene-name>.png`
4. **Inspect and iterate**: Fix issues, re-capture, repeat until it looks right

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
            // ...
            Self::MyNewEffect => "my-new-effect",
        }
    }

    fn fireball_config(&self) -> (Vec3, Vec3) {
        match self {
            // Return (spawn_position, direction) for your effect
            Self::MyNewEffect => (Vec3::new(0.0, 1.0, 0.0), Vec3::X),
            // ...
        }
    }
}

impl FromStr for TestScene {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // ...
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
nix-shell --run "cargo run -- --screenshot fireball-trail-east"

# Then read the screenshot file to inspect
# tmp/screenshots/fireball-trail-east.png
```

## Requirements

- Every shader change must have a corresponding test scene
- Always inspect the screenshot before considering shader work complete
- If something looks wrong, fix it - don't commit broken visuals
- Capture multiple frames to test animated visuals
