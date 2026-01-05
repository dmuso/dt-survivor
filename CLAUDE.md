# Donny Tango: Survivor

A Vampire Survivors / Brotato-style game built with Rust and Bevy ECS.

## Development Commands

```bash
nix-shell --run "make <target>"
```

| Target | Description |
|--------|-------------|
| `check` | Type checking |
| `lint` | Clippy linting |
| `test` | Run tests (quiet mode) |
| `build` | Debug build |
| `build-release` | Release build |
| `run` | Run the game |
| `clean` | Clean build artifacts |

## Development Rules

- **TDD Required**: Write failing test first, then implement code
- **90% Coverage**: Maintain via automated tests
- **Run after every change**: `make lint && make test`
- **Fix all warnings**: No skipped tests, no test deletions
- **Write tests inline**: Co-locate tests with code
- **Don't run the game to test**: Use the visual test mechanism

See [docs/testing-patterns.md](docs/testing-patterns.md) for testing conventions and examples.

## Visual Testing

When working on shaders or visual effects:

1. Create test scene in `src/visual_test/scenes.rs`
2. Capture: `nix-shell --run "cargo run -- --screenshot <scene-name>"`
3. Inspect: Read `tmp/screenshots/<scene-name>.png`
4. Iterate until correct

See [docs/visual-testing.md](docs/visual-testing.md) for detailed workflow.

## Project Architecture

**Domain-driven, plugin-based architecture** with modules organized by feature:

| Module | Purpose |
|--------|---------|
| `game/` | Core logic, GameSet ordering, resources |
| `spell/` | Spell system coordination |
| `spells/` | Individual spell implementations by element |
| `combat/` | Health, damage, death mechanics |
| `movement/` | Speed, velocity, knockback components |
| `player/` | Player entity and controls |
| `enemies/` | Enemy entities and AI |
| `loot/` | Item spawning and pickup |
| `ui/` | User interface systems |

See [docs/architecture.md](docs/architecture.md) for full structure and patterns.

## ECS Patterns (Quick Reference)

**SystemSet Ordering** (defined in `src/game/sets.rs`):
```
Input -> Movement -> Combat -> Spawning -> Effects -> Cleanup
```

**Key Patterns**:
- Use `GameSet` for all gameplay systems
- Events for decoupled communication
- Composable components (Health, Speed, Velocity)
- Type-safe enums over strings
- Marker components for system filtering

See [docs/ecs-patterns.md](docs/ecs-patterns.md) for detailed patterns.

## Shader Materials (Quick Reference)

Shaders in `assets/shaders/`, materials in Rust with `AsBindGroup`.

**Key Rules**:
- Use `Vec4` for uniforms (16-byte alignment for WebGL2)
- Use `#{MATERIAL_BIND_GROUP}` in WGSL (resolves to `@group(3)`)
- Register with `MaterialPlugin::<MyMaterial>::default()`

See [docs/shader-materials.md](docs/shader-materials.md) for full guide.
See [docs/bevy-017-material-bindings.md](docs/bevy-017-material-bindings.md) for Bevy 0.17 specifics.

## Task Tracking with Beads

Track work using `bd` CLI. Git hooks auto-sync.

**Essential Commands**:
```bash
bd ready                    # Find unblocked work
bd update <id> --status in_progress   # Claim task
bd close <id> --reason "Done"         # Complete task
bd sync                     # Sync at session end (CRITICAL!)
```

**Always include descriptions** when creating issues:
```bash
bd create "Fix bug X" --description="Details about what, why, how discovered" -t bug -p 1 --json
```

See [docs/beads-workflow.md](docs/beads-workflow.md) for full workflow.

## Documentation Index

| Document | Purpose |
|----------|---------|
| [docs/architecture.md](docs/architecture.md) | File structure, module patterns |
| [docs/ecs-patterns.md](docs/ecs-patterns.md) | ECS patterns and conventions |
| [docs/testing-patterns.md](docs/testing-patterns.md) | Testing patterns and TDD guide |
| [docs/shader-materials.md](docs/shader-materials.md) | Custom shader material guide |
| [docs/bevy-017-material-bindings.md](docs/bevy-017-material-bindings.md) | Bevy 0.17 shader bindings |
| [docs/visual-testing.md](docs/visual-testing.md) | Visual testing workflow |
| [docs/beads-workflow.md](docs/beads-workflow.md) | Task tracking with beads |
| [docs/explosion-shader.md](docs/explosion-shader.md) | Explosion visual design spec |
