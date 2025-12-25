#!/bin/bash

# Handle Ctrl+C gracefully
trap 'echo -e "\n❌ Script interrupted by user"; exit 130' INT

# CODEME="ccr code"
EXEC="claude"

PROMPT="implement the next outstanding task from beads, ignoring in-progress tasks"

# Function to check if there are outstanding beads tasks
has_outstanding_tasks() {
    # Get count of ready (unblocked) tasks
    local ready_count
    ready_count=$(bd ready --json 2>/dev/null | jq -r 'length // 0' 2>/dev/null)

    # Get count of in-progress tasks
    local in_progress_count
    in_progress_count=$(bd list --status=in_progress --json 2>/dev/null | jq -r 'length // 0' 2>/dev/null)

    # Return true (0) if there are any outstanding tasks
    [ "${ready_count:-0}" -gt 0 ] || [ "${in_progress_count:-0}" -gt 0 ]
}

iteration=1
while true; do
    # Check for outstanding tasks
    ready_count=$(bd ready --json 2>/dev/null | jq -r 'length // 0' 2>/dev/null)
    in_progress_count=$(bd list --status=in_progress --json 2>/dev/null | jq -r 'length // 0' 2>/dev/null)
    total_remaining=$((${ready_count:-0} + ${in_progress_count:-0}))

    if [ "$total_remaining" -eq 0 ]; then
        echo "✅ All beads tasks complete! Exiting loop."
        exit 0
    fi

    echo "Running iteration $iteration... (${total_remaining} tasks remaining: ${ready_count:-0} ready, ${in_progress_count:-0} in progress)"
    $EXEC --verbose -p --output-format stream-json --dangerously-skip-permissions "$PROMPT" | ~/Documents/code/cc-streamer/zig-out/bin/ccstreamer
    iteration=$((iteration + 1))
done
