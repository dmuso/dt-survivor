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
nix-shell --run "cargo run -- --screenshot fireball-trail-east"
```

## Creating a New Test Scene

Add to `src/visual_test/scenes.rs`:

```rust
pub enum TestScene {
    // ... existing scenes
    MyNewEffect,  // Add your scene
}

impl TestScene {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MyNewEffect => "my-new-effect",
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

## Requirements

- Every shader change MUST have a corresponding test scene
- Always inspect the screenshot before considering work complete
- If something looks wrong, fix it - don't commit broken visuals

## Full Documentation

For complete details, see [docs/visual-testing.md](../../../docs/visual-testing.md).
