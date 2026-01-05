# Beads Task Tracking Workflow

Use the `bd` CLI with hooks for tracking work tasks.

## How It Works

1. **SessionStart hook** runs `bd prime` automatically when Claude Code starts
2. `bd prime` injects a compact workflow reference
3. You use `bd` CLI commands directly
4. Git hooks auto-sync the database with JSONL

## CLI Quick Reference

### Finding Work

```bash
bd ready --json                # Unblocked issues
bd stale --days 30 --json      # Forgotten issues
bd list --status open --json   # All open issues
bd show <id> --json            # Detailed issue view
```

### Creating Issues

```bash
# Always include meaningful descriptions!
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json

# Issue discovered during work (inherits source_repo)
bd create "Found bug" --description="Details" -p 1 --deps discovered-from:<parent-id> --json
```

### Managing Issues

```bash
bd update <id> --status in_progress --json  # Claim work
bd close <id> --reason "Done" --json        # Complete work
bd close <id1> <id2> ... --json             # Close multiple at once
```

### Dependencies

```bash
bd dep add <issue> <depends-on>    # Add dependency (issue depends on depends-on)
bd blocked                          # Show all blocked issues
bd dep tree <id>                    # View dependency tree
```

### Sync (Critical!)

```bash
bd sync           # Force immediate export/commit/push
bd sync --status  # Check sync status without syncing
```

## Workflow

1. **Check for ready work**: `bd ready`
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work**: Create issues for bugs/TODOs found
5. **Complete**: `bd close <id> --reason "Implemented"`
6. **Sync at end of session**: `bd sync`

## Issue Descriptions

**Issues without descriptions lack context for future work.** Always include:

- **Why** the issue exists (problem statement or need)
- **What** needs to be done (scope and approach)
- **How** you discovered it (if applicable)

### Good Examples

```bash
# Bug discovered during work
bd create "Fix auth bug in login handler" \
  --description="Login fails with 500 error when password contains special characters. Found while testing GH#123. Stack trace shows unescaped SQL in auth/login.go:45." \
  -t bug -p 1 --deps discovered-from:bd-abc --json

# Feature request
bd create "Add password reset flow" \
  --description="Users need ability to reset forgotten passwords via email. Should follow OAuth best practices and include rate limiting." \
  -t feature -p 2 --json
```

### Bad Examples (Missing Context)

```bash
bd create "Fix auth bug" -t bug -p 1 --json       # What bug? Where?
bd create "Add feature" -t feature --json          # What feature?
bd create "Refactor code" -t task --json           # What code? Why?
```

## Issue Types

| Type | Description |
|------|-------------|
| `bug` | Something broken that needs fixing |
| `feature` | New functionality |
| `task` | Work item (tests, docs, refactoring) |
| `epic` | Large feature with multiple child issues |
| `chore` | Maintenance work (dependencies, tooling) |

## Priorities

| Priority | Description |
|----------|-------------|
| `0` | Critical (security, data loss, broken builds) |
| `1` | High (major features, important bugs) |
| `2` | Medium (nice-to-have features, minor bugs) |
| `3` | Low (polish, optimization) |
| `4` | Backlog (future ideas) |

## Dependency Types

| Type | Description | Affects Ready Queue? |
|------|-------------|---------------------|
| `blocks` | Hard dependency (X blocks Y) | Yes |
| `related` | Soft relationship | No |
| `parent-child` | Epic/subtask relationship | No |
| `discovered-from` | Track discovered work | No |

## Planning Work with Dependencies

Use **requirement language**, not temporal language (phases/steps).

### Cognitive Trap: Temporal Language Inverts Dependencies

Words like "Phase 1", "Step 1", "first", "before" trigger temporal reasoning that **flips dependency direction**.

```bash
# WRONG - temporal thinking
bd dep add phase1 phase2  # Says phase1 depends on phase2!

# RIGHT - requirement thinking
bd dep add msg-rendering buffer-layout  # msg-rendering NEEDS buffer-layout
```

**Verification**: Run `bd blocked` - tasks should be blocked by their prerequisites.

### Example Breakdown

```bash
# Create tasks named by what they do
bd create "Implement conversation region" -t task -p 1
bd create "Add header-line status display" -t task -p 1
bd create "Render tool calls inline" -t task -p 2

# Dependencies: X depends on Y means "X needs Y first"
bd dep add header-line conversation-region
bd dep add tool-calls conversation-region

# Verify
bd blocked
```

## Duplicate Detection & Merging

Proactively detect and merge duplicate issues:

```bash
# Find duplicates
bd duplicates
bd duplicates --auto-merge
bd duplicates --dry-run

# Manual merge
bd show bd-41 bd-42 bd-43 --json           # Compare
bd merge bd-42 bd-43 --into bd-41 --dry-run  # Preview
bd merge bd-42 bd-43 --into bd-41 --json     # Execute
```

**What gets merged:**
- All dependencies from source to target
- Text references updated across all issues
- Source issues closed with "Merged into bd-X" reason
- Source content NOT copied (copy manually before merging if needed)

## Deletion Tracking

Deleted issues are recorded in `.beads/deletions.jsonl`:

```bash
bd delete bd-42               # Delete single issue
bd cleanup -f                 # Delete all closed issues
bd deleted                    # Show recent deletions
bd deleted --since=30d        # Deletions in last 30 days
bd deleted --json             # Machine-readable output
```

## Hierarchical Children (Epics)

Epics can have child issues with dotted IDs:
- Parent: `bd-a3f8e9`
- Children: `bd-a3f8e9.1`, `bd-a3f8e9.2`, etc.

Up to 3 levels of nesting supported. Children auto-numbered sequentially.
