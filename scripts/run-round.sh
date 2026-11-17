#!/usr/bin/env bash
# Run the CC2 and clys evaluation stages. For Attax use adapters/s2lab/README.md.
#
#   ROUND=1 SEEDS_FILE=docs/seeds-round-1.md bash scripts/run-round.sh
#
# Environment:
#   ROUND          round number (required)
#   SEEDS_FILE     seeds file (default: docs/seeds-round-$ROUND.md)
#   SEEDS_OVERRIDE comma separated seeds (dry runs only; skips the seeds file)
#   OUT            evidence root (default: <repo>/scripts/evidence)
#   CC2_PROGRAM    CC2 launcher
#   CC2_ARGS       comma separated CC2 arguments
#   CC2_COMMIT     CC2 commit
#   CC2_VISIBLE_ROWS  default 40
#   CC2_SUPPORTS_180 / CLYS_SUPPORTS_180  default false
#   CLYS_PROGRAM / CLYS_ARGS / CLYS_COMMIT / CLYS_VISIBLE_ROWS
#   GATE_CC2 / GATE_CLYS                default 50 / 50
#   DEPTH / WIDTH                      default 8 / 88
#   MS / THREADS / TURNS                default 100 / 8 / 1000
#   MAX_STAGES                          default 2 (1 for dry runs)
#   DRY_RUN=1                           run without registering a seed allocation
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ROUND="${ROUND:?set ROUND (e.g. ROUND=1)}"
# Only one measurement may use the machine at a time.
source "$ROOT/scripts/measurement-lock.sh"
acquire_measurement "round-$ROUND"
SEEDS_FILE="${SEEDS_FILE:-$ROOT/docs/seeds-round-$ROUND.md}"
OUT="${OUT:-$ROOT/scripts/evidence}"
CC2_DIR="${CC2_DIR:-$ROOT/../cold-clear-2}"
CC2_PATCH_NOTE="${CC2_PATCH_NOTE:-cc2-tbp-compat.patch: spawn, initial empty-hold transition and on-request search window; no weight changes}"
CC2_PATCH_SHA256="${CC2_PATCH_SHA256:-$(sha256sum < "$ROOT/adapters/cc2-tbp-compat.patch" | cut -d' ' -f1)}"
CLYS_DIR="${CLYS_DIR:-$ROOT/../clys}"
CLYS_PATCH_NOTE="${CLYS_PATCH_NOTE:-clys-tbp-spawn.patch: spawn row 19 -> 21 and on-request search window; no weight changes}"
CLYS_PATCH_SHA256="${CLYS_PATCH_SHA256:-$(sha256sum < "$ROOT/adapters/clys-tbp-spawn.patch" | cut -d' ' -f1)}"
GATE_CC2="${GATE_CC2:-50}"
GATE_CLYS="${GATE_CLYS:-50}"
CC2_SUPPORTS_180="${CC2_SUPPORTS_180:-false}"
CLYS_SUPPORTS_180="${CLYS_SUPPORTS_180:-false}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
TURNS="${TURNS:-1000}"
mkdir -p "$OUT"

ARGS=(run --round "$ROUND" --out "$OUT"
  --gate-cc2 "$GATE_CC2" --gate-clys "$GATE_CLYS"
  --deterministic --depth "${DEPTH:-8}" --width "${WIDTH:-88}"
  --ms "$MS" --threads "$THREADS" --turns "$TURNS")

if [ -n "${MAX_STAGES:-}" ]; then
  ARGS+=(--max-stages "$MAX_STAGES")
fi

if [ -n "${SEEDS_OVERRIDE:-}" ]; then
  ARGS+=(--seeds "$SEEDS_OVERRIDE")
else
  ARGS+=(--seeds-file "$SEEDS_FILE")
fi
if [ -n "${CC2_PROGRAM:-}" ]; then
  ARGS+=(--cc2-program "$CC2_PROGRAM" --cc2-args "${CC2_ARGS:-}" --cc2-commit "${CC2_COMMIT:-}" --cc2-cwd "$CC2_DIR" --cc2-visible-rows "${CC2_VISIBLE_ROWS:-40}" --cc2-supports-180 "$CC2_SUPPORTS_180" --cc2-patch-note "$CC2_PATCH_NOTE" --cc2-patch-sha256 "$CC2_PATCH_SHA256")
fi
if [ -n "${CLYS_PROGRAM:-}" ]; then
  ARGS+=(--clys-program "$CLYS_PROGRAM" --clys-args "${CLYS_ARGS:-}" --clys-commit "${CLYS_COMMIT:-}" --clys-cwd "$CLYS_DIR" --clys-visible-rows "${CLYS_VISIBLE_ROWS:-40}" --clys-supports-180 "$CLYS_SUPPORTS_180" --clys-retry-empty-ms "${CLYS_RETRY_EMPTY_MS:-2000}" --clys-patch-note "$CLYS_PATCH_NOTE" --clys-patch-sha256 "$CLYS_PATCH_SHA256" --clys-env "CLYS_TBP_LOG=$(cd "$OUT" && pwd)/round-$ROUND-clys-adapter.jsonl")
fi
if [ "${DRY_RUN:-0}" = "1" ]; then
  ARGS+=(--dry-run)
fi

cd "$ROOT"
# Rebuild both binaries explicitly: a stale `arena` binary was once mistaken
# for a source gap (the `--tbp-*` flags were present but the binary was old).
echo "building arena + round (release)..."
cargo build --offline --release -p intetrigence-arena --bin round --bin arena
cargo run --offline --release -p intetrigence-arena --bin round -- "${ARGS[@]}"
