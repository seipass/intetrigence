#!/usr/bin/env bash
# Machine-wide measurement lock. Only one measurement may run at a time.
#
#   source scripts/measurement-lock.sh
#   acquire_measurement "tuning-smoke"
#   ... measurement ...
#   # released by the EXIT trap
#
# The lock is a directory (atomic mkdir). A measurement that runs concurrently
# with another one contaminates the think times (the same `d6 w128` schedule
# measured max 37.2 ms alone and 111.9 ms under load). If a run was killed,
# remove the lock directory by hand.
set -euo pipefail

MEASUREMENT_LOCK_DIR="${MEASUREMENT_LOCK_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.measurement-lock}"

acquire_measurement() {
  local name="${1:-measurement}"
  if ! mkdir "$MEASUREMENT_LOCK_DIR" 2>/dev/null; then
    local owner
    owner="$(cat "$MEASUREMENT_LOCK_DIR/owner" 2>/dev/null || echo unknown)"
    echo "error: another measurement is running ($owner)." >&2
    echo "       remove $MEASUREMENT_LOCK_DIR if it is stale." >&2
    return 1
  fi
  printf '%s pid=%s started=%s\n' "$name" "$$" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    > "$MEASUREMENT_LOCK_DIR/owner"
  trap 'release_measurement' EXIT INT TERM
}

release_measurement() {
  rm -rf "$MEASUREMENT_LOCK_DIR"
}
