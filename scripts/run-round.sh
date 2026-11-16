#!/usr/bin/env bash
# Staged round runner wrapper (protocol v1.13 §1.4/§1.5).
#
#   ROUND=1 SEEDS_FILE=docs/seeds-round-1.md bash scripts/run-round.sh
#
# Environment:
#   ROUND          round number (required)
#   SEEDS_FILE     seeds file (default: docs/seeds-round-$ROUND.md)
#   SEEDS_OVERRIDE comma separated seeds (dry runs only; skips the seeds file)
#   OUT            evidence root (default: <repo>/scripts/evidence)
#   ATTAX_DIR      attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   ATTAX_COMMIT   attax commit (default: 7b38f51...)
#   ATTAX_PATCH_NOTE / ATTAX_PATCH_SHA256  compatibility patch disclosure
#   ATTAX_VISIBLE_ROWS  default 0 = auto-detect from movegen.ts
#   ATTAX_ENV      comma separated KEY=VALUE passed to the attax process
#   CC2_PROGRAM    CC2 launcher (empty = stage 2 not configured)
#   CC2_ARGS       comma separated CC2 arguments
#   CC2_COMMIT     CC2 commit
#   CC2_VISIBLE_ROWS  default 40
#   CC2_SUPPORTS_180 / CLYS_SUPPORTS_180  default false (none of the three
#                                          opponents has 180-degree rotation)
#   CLYS_PROGRAM / CLYS_ARGS / CLYS_COMMIT / CLYS_VISIBLE_ROWS
#   GATE_CC2 / GATE_CLYS / GATE_ATTAX   default 50 / 50 / 48
#   DEPTH / WIDTH  deterministic schedule. d6 w96 is the interim; once the
#                  bot's speedup lands, run the same round with DEPTH=8 WIDTH=96
#                  (the official target: depth 8, max under 100 ms).
#   Stage order is CC2 -> clys -> attax (new staged goal).
#   MS / THREADS / TURNS                default 100 / 8 / 1000
#   MAX_STAGES                          default 3
#   DRY_RUN=1                           run stage 1 only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Only one measurement may use the machine at a time.
source "$ROOT/scripts/measurement-lock.sh"
acquire_measurement "round-$ROUND"
ROUND="${ROUND:?set ROUND (e.g. ROUND=1)}"
SEEDS_FILE="${SEEDS_FILE:-$ROOT/docs/seeds-round-$ROUND.md}"
OUT="${OUT:-$ROOT/scripts/evidence}"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
ATTAX_COMMIT="${ATTAX_COMMIT:-7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b}"
ATTAX_PATCH_NOTE="${ATTAX_PATCH_NOTE:-attax-vision-40.patch (68 lines): ROWS 40, Y_SPAN 48, grid 40, board.rows 40, attaxCellsToTbp 19->39, posY = 18 - maxY (S2 spawn); no search/evaluation changes}"
ATTAX_PATCH_SHA256="${ATTAX_PATCH_SHA256:-e22aab7b5a2f9a95efb562e24b3f33da351bb942d6daf6e205475b5871ed3b70}"
ATTAX_VISIBLE_ROWS="${ATTAX_VISIBLE_ROWS:-0}"
ATTAX_ENV="${ATTAX_ENV:-}"
CC2_DIR="${CC2_DIR:-C:/Users/Hasilan/Projects/cold-clear-2}"
CC2_PATCH_NOTE="${CC2_PATCH_NOTE:-cc2-tbp-compat.patch: spawn position and the empty-hold representation (hold candidate enumeration + eval cutout); no weight changes}"
CC2_PATCH_SHA256="${CC2_PATCH_SHA256:-0202959a0af3dd77abca4caf4c8cfad6e21a38e70d0b3218ddf14bc3e42e95d5}"
CLYS_DIR="${CLYS_DIR:-C:/Users/Hasilan/Projects/clys}"
CLYS_PATCH_NOTE="${CLYS_PATCH_NOTE:-clys-tbp-spawn.patch: spawn row 19 -> 21 in core/move.cpp plus the TBP adapter; no search/evaluation changes}"
CLYS_PATCH_SHA256="${CLYS_PATCH_SHA256:-f25c138110e2a6a571bf62267477a79f714a6ae16dc2b4c0218df3e72962ddaa}"
ATTAX_ENV="${ATTAX_ENV:-}"
GATE_CC2="${GATE_CC2:-50}"
GATE_CLYS="${GATE_CLYS:-50}"
GATE_ATTAX="${GATE_ATTAX:-48}"
CC2_SUPPORTS_180="${CC2_SUPPORTS_180:-false}"
CLYS_SUPPORTS_180="${CLYS_SUPPORTS_180:-false}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
TURNS="${TURNS:-1000}"
MAX_STAGES="${MAX_STAGES:-3}"
mkdir -p "$OUT"

ARGS=(run --round "$ROUND" --out "$OUT"
  --attax-dir "$ATTAX_DIR" --attax-commit "$ATTAX_COMMIT"
  --attax-patch-note "$ATTAX_PATCH_NOTE" --attax-patch-sha256 "$ATTAX_PATCH_SHA256"
  --attax-visible-rows "$ATTAX_VISIBLE_ROWS"
  --gate-attax "$GATE_ATTAX" --gate-cc2 "$GATE_CC2" --gate-clys "$GATE_CLYS"
  --deterministic --depth "${DEPTH:-6}" --width "${WIDTH:-96}"
  --ms "$MS" --threads "$THREADS" --turns "$TURNS" --max-stages "$MAX_STAGES")

if [ -n "${SEEDS_OVERRIDE:-}" ]; then
  ARGS+=(--seeds "$SEEDS_OVERRIDE")
else
  ARGS+=(--seeds-file "$SEEDS_FILE")
fi
if [ -n "$ATTAX_ENV" ]; then
  IFS=',' read -ra _attax_envs <<< "$ATTAX_ENV"
  for _env in "${_attax_envs[@]}"; do
    ARGS+=(--attax-env "$_env")
  done
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
