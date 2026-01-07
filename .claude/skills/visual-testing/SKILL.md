---
name: visual-testing
description: Visual testing workflow for shaders and visual effects. Use when working on shaders, visual effects, materials, visual tests or when you need to verify how something looks visually. Triggers on "test shader", "screenshot", "visual test", "how does it look", "verify visuals".
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

## Approach

* A visual test should reflect the in game rendering as accurately as possible - updates, systems, animations, movement, velocity. Don't just create a static test if the in game effect involves animation.
* If the game has objects that are scaled or moved, then the visual tests should also have the same behaviour. 
* If scaling, movement, velocity or shader animation is involved, then the visual test should capture multiple frames of animation. The test doesn't have to capture every frame, but 5x frames captured over the lifetime of the animation is recommended.
* Ensure that the full effect is captured with visual tests. For larger objects or movement effects, this may require zooming the camera out in the scene.
* Ensure that visual tests always test the same visual effects as what is in game.
* Add comments to shader source code and related visual tests to explain the intended visual effect based on the requirements given. Use these comments when inspecting visual outputs to review work.
* If the inspected visual output from the test does not meet the requirements, then work to fix the shader or visual test.
* Visual effects are of paramount importance to a user's game experience. This is an area where we want to go the extra mile to delight users. It is critically important that shaders are implemented according to requirements.

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
