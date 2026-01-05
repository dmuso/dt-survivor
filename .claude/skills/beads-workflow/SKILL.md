---
name: beads-workflow
description: Task tracking with beads (bd CLI). Use when creating issues, tracking work, managing dependencies, planning features, or asking about "bd command", "task tracking", "issue", "ready work", "sync".
---

# Beads Workflow Skill

This skill provides guidance on tracking work with the `bd` CLI.

## Essential Commands

```bash
# Find work
bd ready                           # Unblocked issues
bd stale --days 30                 # Forgotten issues

# Create issue (ALWAYS include description!)
bd create "Title" --description="Details" -t bug|feature|task -p 0-4 --json

# Manage issues
bd update <id> --status in_progress   # Claim work
bd close <id> --reason "Done"         # Complete work
bd close <id1> <id2> ...              # Close multiple

# Dependencies
bd dep add <issue> <depends-on>       # issue NEEDS depends-on
bd blocked                            # Show blocked issues

# CRITICAL: Sync at session end!
bd sync
```

## Workflow

1. **Find work**: `bd ready`
2. **Claim task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover work**: Create issues for bugs/TODOs found
5. **Complete**: `bd close <id> --reason "Done"`
6. **Sync**: `bd sync` (CRITICAL!)

## Always Include Descriptions

**Bad:**
```bash
bd create "Fix bug" -t bug -p 1 --json  # What bug? Where?
```

**Good:**
```bash
bd create "Fix auth validation" \
  --description="Login fails with 500 when password has quotes. Found in auth/login.go:45" \
  -t bug -p 1 --json
```

## Issue Types & Priorities

| Type | Use |
|------|-----|
| `bug` | Something broken |
| `feature` | New functionality |
| `task` | Work item (tests, docs) |
| `epic` | Large feature with children |
| `chore` | Maintenance |

| Priority | Meaning |
|----------|---------|
| 0 | Critical (security, broken builds) |
| 1 | High (major features, important bugs) |
| 2 | Medium (nice-to-have) |
| 3 | Low (polish) |
| 4 | Backlog |

## Dependencies: Think "NEEDS", Not "Before"

**Cognitive trap**: Temporal language inverts dependencies!

```bash
# WRONG - "Phase 1 before Phase 2" thinking
bd dep add phase1 phase2  # Says phase1 depends on phase2!

# RIGHT - "X needs Y" thinking
bd dep add msg-rendering buffer-layout  # msg-rendering NEEDS buffer-layout
```

**Verify**: Run `bd blocked` - tasks should be blocked by prerequisites.

## Discovered Work

When you find bugs/TODOs during work:

```bash
bd create "Found bug in X" \
  --description="Details..." \
  -t bug -p 1 \
  --deps discovered-from:<current-id> \
  --json
```

The new issue inherits `source_repo` from parent.

## Duplicate Management

```bash
bd duplicates                    # Find duplicates
bd duplicates --auto-merge       # Auto-merge
bd merge bd-42 bd-43 --into bd-41  # Manual merge
```

## Full Documentation

For complete workflow, see [docs/beads-workflow.md](../../../docs/beads-workflow.md).
