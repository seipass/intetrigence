#!/usr/bin/env bash
# Shared measurement lock for this user and machine.
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

if [[ "$OSTYPE" == msys* || "$OSTYPE" == cygwin* ]]; then
  MEASUREMENT_LOCK_DIR="${MEASUREMENT_LOCK_DIR:-${LOCALAPPDATA:-${USERPROFILE:?set LOCALAPPDATA or USERPROFILE}/AppData/Local}/intetrigence/measurement.lock}"
  MEASUREMENT_LOCK_DIR="$(cygpath -am "$MEASUREMENT_LOCK_DIR")"
else
  MEASUREMENT_LOCK_DIR="${MEASUREMENT_LOCK_DIR:-${HOME:?set HOME}/.cache/intetrigence/measurement.lock}"
fi

acquire_measurement() {
  local name="${1:-measurement}"
  mkdir -p -- "$(dirname "$MEASUREMENT_LOCK_DIR")"
  if ! mkdir "$MEASUREMENT_LOCK_DIR" 2>/dev/null; then
    local owner
    owner="$(cat "$MEASUREMENT_LOCK_DIR/owner" 2>/dev/null || echo unknown)"
    echo "error: another measurement is running ($owner)." >&2
    echo "       remove $MEASUREMENT_LOCK_DIR if it is stale." >&2
    return 1
  fi
  INTETRIGENCE_MEASUREMENT_OWNER="$name pid=$$ started=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if ! printf '%s\n' "$INTETRIGENCE_MEASUREMENT_OWNER" > "$MEASUREMENT_LOCK_DIR/owner"; then
    rm -f -- "$MEASUREMENT_LOCK_DIR/owner"
    rmdir -- "$MEASUREMENT_LOCK_DIR"
    return 1
  fi
  export MEASUREMENT_LOCK_DIR INTETRIGENCE_MEASUREMENT_OWNER
  trap 'release_measurement' EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
}

release_measurement() {
  if [[ "$(cat "$MEASUREMENT_LOCK_DIR/owner" 2>/dev/null)" == "$INTETRIGENCE_MEASUREMENT_OWNER" ]]; then
    rm -f -- "$MEASUREMENT_LOCK_DIR/owner"
    rmdir -- "$MEASUREMENT_LOCK_DIR"
  fi
}
