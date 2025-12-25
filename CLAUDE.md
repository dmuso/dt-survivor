# Donny Tango: Survivor

A game in the style of Vampire Survivors and Brotato, built with Rust and the Bevy ECS framework.

## Development Commands

All development tooling commands require Nix Shell to run.

- Type Checking: `nix-shell --run "cargo check"`
- Linting: `nix-shell --run "cargo clippy"`
- Testing: `nix-shell --run "cargo test"`
- Building: `nix-shell --run "cargo build"`
- Running: `nix-shell --run "cargo run"`

## Testing and Linting

- Use TDD when making changes to code. Write a failing test, and then implement the code and confirm the test passes.
- You should maintain 90% code coverage via automated tests
- Run linting and testing after every change
- Fix any errors or warnings that you get as feedback from linting and tests
- Write tests inline with code

## File Structure and Code Organization

This project follows a domain-driven, modular architecture to support the complex features planned for the survivor game (player classes, enemy AI, inventory, weapons, levels, etc.).

### Core Architecture Principles

- **Domain-driven organization**: Group code by business domain (game logic, UI, etc.) rather than technical type
- **Plugin-based architecture**: Each major feature area exposes a plugin for easy composition and testing
- **Clear separation of concerns**: Components, systems, and resources are logically separated
- **Scalable structure**: Easy to add new features without disrupting existing code

### Current Module Structure

```
src/
├── lib.rs              # Library exports and plugin composition
├── main.rs             # Minimal app entry point using plugins
├── prelude.rs          # Common imports used across modules
├── states.rs           # Game state management (GameState enum)
├── game/               # Core game logic
│   ├── mod.rs
│   ├── components.rs   # Game entity components (Player, Enemy, etc.)
│   ├── systems.rs      # Game systems (movement, combat, AI)
│   ├── resources.rs    # Game resources (score, settings)
│   └── plugin.rs       # Game plugin composition
├── ui/                 # User interface systems
│   ├── mod.rs
│   ├── components.rs   # UI components (buttons, menus, HUD)
│   ├── systems.rs      # UI interaction systems
│   └── plugin.rs       # UI plugin composition
├── pistol/             # Pistol weapon behavior
│   ├── mod.rs
│   ├── components.rs   # Pistol-specific components and configuration
│   └── systems.rs      # Pistol firing logic and systems
├── weapon/             # Generic weapon management
│   ├── mod.rs
│   ├── components.rs   # Weapon components and types
│   ├── resources.rs    # Weapon resources
│   ├── systems.rs      # Weapon firing coordination
│   └── plugin.rs       # Weapon plugin composition
├── inventory/          # Player inventory management
│   ├── mod.rs
│   ├── components.rs   # Inventory components
│   ├── resources.rs    # Inventory storage
│   ├── systems.rs      # Inventory systems
│   └── plugin.rs       # Inventory plugin composition
├── bullets/            # Bullet entities and behavior
│   ├── mod.rs
│   ├── components.rs   # Bullet components
│   └── systems.rs      # Bullet movement and collision
├── laser/              # Laser weapon behavior
│   ├── mod.rs
│   ├── components.rs   # Laser components
│   ├── systems.rs      # Laser systems
│   └── plugin.rs       # Laser plugin composition
├── rocket_launcher/    # Rocket launcher weapon behavior
│   ├── mod.rs
│   ├── components.rs   # Rocket components
│   ├── systems.rs      # Rocket systems
│   └── plugin.rs       # Rocket launcher plugin composition
├── loot/               # Loot spawning and pickup
│   ├── mod.rs
│   ├── components.rs   # Loot components
│   ├── systems.rs      # Loot systems
│   └── plugin.rs       # Loot plugin composition
├── enemies/            # Enemy entities and AI
│   ├── mod.rs
│   ├── components.rs   # Enemy components
│   └── systems.rs      # Enemy AI and behavior
├── player/             # Player entity and controls
│   ├── mod.rs
│   ├── components.rs   # Player components
│   └── systems.rs      # Player systems
├── experience/         # Experience and leveling
│   ├── mod.rs
│   ├── components.rs   # Experience components
│   ├── resources.rs    # Experience resources
│   ├── systems.rs      # Experience systems
│   └── plugin.rs       # Experience plugin composition
├── score/              # Score tracking
│   ├── mod.rs
│   ├── components.rs   # Score components
│   ├── resources.rs    # Score resources
│   └── systems.rs      # Score systems
├── audio/              # Audio management
│   ├── mod.rs
│   ├── components.rs   # Audio components
│   ├── systems.rs      # Audio systems
│   └── plugin.rs       # Audio plugin composition
└── game/               # Core game logic and resources
    ├── mod.rs
    ├── components.rs   # Game components
    ├── resources.rs    # Game resources
    ├── systems.rs      # Game systems
    └── plugin.rs       # Game plugin composition
```

### Module Organization Patterns

Each feature module should follow this pattern:

#### 1. Module Definition (`mod.rs`)
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

#### 2. Components (`components.rs`)
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

#### 3. Systems (`systems.rs`)
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

#### 4. Resources (`resources.rs`) - When Needed
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

#### 5. Plugin (`plugin.rs`)
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

### Future Module Planning

As the game grows, plan for these additional modules:

- **player/**: Player-specific logic (classes, abilities, progression)
- **enemies/**: Enemy spawning, AI, and types
- **combat/**: Damage, health, and combat mechanics
- **inventory/**: Items, weapons, and equipment
- **levels/**: Level progression and world management
- **audio/**: Sound effects and music management
- **assets/**: Asset loading and management

### Import Strategy

- Use `prelude.rs` for common Bevy imports and local types
- Import specific items rather than glob imports when possible
- Keep imports organized and minimal

### Testing Strategy

- Tests should be co-located with the code they test
- Test components, systems, and integration scenarios
- Maintain 90% code coverage across all modules
- Use descriptive test names that explain what they're testing

### Development Workflow

1. **Plan the feature**: Identify which module(s) it belongs in
2. **Create/update components**: Add necessary ECS components
3. **Implement systems**: Write the game logic
4. **Create/update plugins**: Wire systems together
5. **Add tests**: Ensure functionality works correctly
6. **Update documentation**: Keep AGENTS.md current
7. **Run full test suite**: Verify no regressions

## Planning and Tracking work tasks using Beads

### CLI + Hooks

Use the `bd` CLI with hooks for the best experience.

**How it works:**

1. **SessionStart hook** runs `bd prime` automatically when Claude Code starts
2. `bd prime` injects a compact workflow reference
3. You use `bd` CLI commands directly
4. Git hooks auto-sync the database with JSONL

### CLI Quick Reference

**Essential commands for AI agents:**

```bash
# Find work
bd ready --json                                    # Unblocked issues
bd stale --days 30 --json                          # Forgotten issues

# Create and manage issues
bd create "Issue title" --description="Detailed context about the issue" -t bug|feature|task -p 0-4 --json
bd create "Found bug" --description="What the bug is and how it was discovered" -p 1 --deps discovered-from:<parent-id> --json
bd update <id> --status in_progress --json
bd close <id> --reason "Done" --json

# Search and filter
bd list --status open --priority 1 --json
bd list --label-any urgent,critical --json
bd show <id> --json

# Sync (CRITICAL at end of session!)
bd sync  # Force immediate export/commit/push
```

### Workflow

1. **Check for ready work**: Run `bd ready` to see what's unblocked (or `bd stale` to find forgotten issues)
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work**: If you find bugs or TODOs, create issues:
   - Old way (two commands): `bd create "Found bug in auth" --description="Details about the bug" -t bug -p 1 --json` then `bd dep add <new-id> <current-id> --type discovered-from`
   - New way (one command): `bd create "Found bug in auth" --description="Login fails with 500 when password has special chars" -t bug -p 1 --deps discovered-from:<current-id> --json`
5. **Complete**: `bd close <id> --reason "Implemented"`
6. **Sync at end of session**: `bd sync` (see "Agent Session Workflow" below)

### IMPORTANT: Always Include Issue Descriptions

**Issues without descriptions lack context for future work.** When creating issues, always include a meaningful description with:

- **Why** the issue exists (problem statement or need)
- **What** needs to be done (scope and approach)
- **How** you discovered it (if applicable during work)

**Good examples:**

```bash
# Bug discovered during work
bd create "Fix auth bug in login handler" \
  --description="Login fails with 500 error when password contains special characters like quotes. Found while testing GH#123 feature. Stack trace shows unescaped SQL in auth/login.go:45." \
  -t bug -p 1 --deps discovered-from:bd-abc --json

# Feature request
bd create "Add password reset flow" \
  --description="Users need ability to reset forgotten passwords via email. Should follow OAuth best practices and include rate limiting to prevent abuse." \
  -t feature -p 2 --json

# Technical debt
bd create "Refactor auth package for testability" \
  --description="Current auth code has tight DB coupling making unit tests difficult. Need to extract interfaces and add dependency injection. Blocks writing tests for bd-xyz." \
  -t task -p 3 --json
```

**Bad examples (missing context):**

```bash
bd create "Fix auth bug" -t bug -p 1 --json  # What bug? Where? Why?
bd create "Add feature" -t feature --json     # What feature? Why needed?
bd create "Refactor code" -t task --json      # What code? Why refactor?
```

### Deletion Tracking

When issues are deleted (via `bd delete` or `bd cleanup`), they are recorded in `.beads/deletions.jsonl`. This manifest:

- **Propagates deletions across clones**: When you pull, deleted issues from other clones are removed from your local database
- **Provides audit trail**: See what was deleted, when, and by whom with `bd deleted`
- **Auto-prunes**: Old records are automatically cleaned up during `bd sync` (configurable retention)

**Commands:**

```bash
bd delete bd-42                # Delete issue (records to manifest)
bd cleanup -f                  # Delete closed issues (records all to manifest)
bd deleted                     # Show recent deletions (last 7 days)
bd deleted --since=30d         # Show deletions in last 30 days
bd deleted bd-xxx              # Show deletion details for specific issue
bd deleted --json              # Machine-readable output
```

**How it works:**

1. `bd delete` or `bd cleanup` appends deletion records to `deletions.jsonl`
2. The file is committed and pushed via `bd sync`
3. On other clones, `bd sync` imports the deletions and removes those issues from local DB
4. Git history fallback handles edge cases (pruned records, shallow clones)

### Issue Types

- `bug` - Something broken that needs fixing
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature composed of multiple issues (supports hierarchical children)
- `chore` - Maintenance work (dependencies, tooling)

**Hierarchical children:** Epics can have child issues with dotted IDs (e.g., `bd-a3f8e9.1`, `bd-a3f8e9.2`). Children are auto-numbered sequentially. Up to 3 levels of nesting supported. The parent hash ensures unique namespace - no coordination needed between agents working on different epics.

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (nice-to-have features, minor bugs)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Dependency Types

- `blocks` - Hard dependency (issue X blocks issue Y)
- `related` - Soft relationship (issues are connected)
- `parent-child` - Epic/subtask relationship
- `discovered-from` - Track issues discovered during work (automatically inherits parent's `source_repo`)

Only `blocks` dependencies affect the ready work queue.

**Note:** When creating an issue with a `discovered-from` dependency, the new issue automatically inherits the parent's `source_repo` field. This ensures discovered work stays in the same repository as the parent task.

### Planning Work with Dependencies

When breaking down large features into tasks, use **beads dependencies** to sequence work - NOT phases or numbered steps.

**⚠️ COGNITIVE TRAP: Temporal Language Inverts Dependencies**

Words like "Phase 1", "Step 1", "first", "before" trigger temporal reasoning that **flips dependency direction**. Your brain thinks:
- "Phase 1 comes before Phase 2" → "Phase 1 blocks Phase 2" → `bd dep add phase1 phase2`

But that's **backwards**! The correct mental model:
- "Phase 2 **depends on** Phase 1" → `bd dep add phase2 phase1`

**Solution: Use requirement language, not temporal language**

Instead of phases, name tasks by what they ARE, and think about what they NEED:

```bash
# ❌ WRONG - temporal thinking leads to inverted deps
bd create "Phase 1: Create buffer layout" ...
bd create "Phase 2: Add message rendering" ...
bd dep add phase1 phase2  # WRONG! Says phase1 depends on phase2

# ✅ RIGHT - requirement thinking
bd create "Create buffer layout" ...
bd create "Add message rendering" ...
bd dep add msg-rendering buffer-layout  # msg-rendering NEEDS buffer-layout
```

**Verification**: After adding deps, run `bd blocked` - tasks should be blocked by their prerequisites, not their dependents.

**Example breakdown** (for a multi-part feature):
```bash
# Create tasks named by what they do, not what order they're in
bd create "Implement conversation region" -t task -p 1
bd create "Add header-line status display" -t task -p 1
bd create "Render tool calls inline" -t task -p 2
bd create "Add streaming content support" -t task -p 2

# Set up dependencies: X depends on Y means "X needs Y first"
bd dep add header-line conversation-region    # header needs region
bd dep add tool-calls conversation-region     # tools need region
bd dep add streaming tool-calls               # streaming needs tools

# Verify with bd blocked - should show sensible blocking
bd blocked
```

### Duplicate Detection & Merging

AI agents should proactively detect and merge duplicate issues to keep the database clean:

**Automated duplicate detection:**

```bash
# Find all content duplicates in the database
bd duplicates

# Automatically merge all duplicates
bd duplicates --auto-merge

# Preview what would be merged
bd duplicates --dry-run

# During import
bd import -i issues.jsonl --dedupe-after
```

**Detection strategies:**

1. **Before creating new issues**: Search for similar existing issues

   ```bash
   bd list --json | grep -i "authentication"
   bd show bd-41 bd-42 --json  # Compare candidates
   ```

2. **Periodic duplicate scans**: Review issues by type or priority

   ```bash
   bd list --status open --priority 1 --json  # High-priority issues
   bd list --issue-type bug --json             # All bugs
   ```

3. **During work discovery**: Check for duplicates when filing discovered-from issues
   ```bash
   # Before: bd create "Fix auth bug" --description="Details..." --deps discovered-from:bd-100
   # First: bd list --json | grep -i "auth bug"
   # Then decide: create new or link to existing
   ```

**Merge workflow:**

```bash
# Step 1: Identify duplicates (bd-42 and bd-43 duplicate bd-41)
bd show bd-41 bd-42 bd-43 --json

# Step 2: Preview merge to verify
bd merge bd-42 bd-43 --into bd-41 --dry-run

# Step 3: Execute merge
bd merge bd-42 bd-43 --into bd-41 --json

# Step 4: Verify result
bd dep tree bd-41  # Check unified dependency tree
bd show bd-41 --json  # Verify merged content
```

**What gets merged:**

- ✅ All dependencies from source → target
- ✅ Text references updated across ALL issues (descriptions, notes, design, acceptance criteria)
- ✅ Source issues closed with "Merged into bd-X" reason
- ❌ Source issue content NOT copied (target keeps its original content)

**Important notes:**

- Merge preserves target issue completely; only dependencies/references migrate
- If source issues have valuable content, manually copy it to target BEFORE merging
- Cannot merge in daemon mode yet (bd-190); use `--no-daemon` flag
- Operation cannot be undone (but git history preserves the original)

**Best practices:**

- Merge early to prevent dependency fragmentation
- Choose the oldest or most complete issue as merge target
- Add labels like `duplicate` to source issues before merging (for tracking)
- File a discovered-from issue if you found duplicates during work:
  ```bash
  bd create "Found duplicates during bd-X" \
    --description="Issues bd-A, bd-B, and bd-C are duplicates and need merging" \
    -p 2 --deps discovered-from:bd-X --json
  ```
