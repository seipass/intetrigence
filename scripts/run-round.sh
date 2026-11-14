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
#   CC2_PROGRAM    CC2 launcher (empty = stage 2 not configured)
#   CC2_ARGS       comma separated CC2 arguments
#   CC2_COMMIT     CC2 commit
#   CC2_VISIBLE_ROWS  default 40
#   CLYS_PROGRAM / CLYS_ARGS / CLYS_COMMIT / CLYS_VISIBLE_ROWS
#   GATE_ATTAX / GATE_CC2 / GATE_CLYS   default 50 / 48 / 48
#   MS / THREADS / TURNS                default 100 / 8 / 1000
#   MAX_STAGES                          default 3
#   DRY_RUN=1                           run stage 1 only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ROUND="${ROUND:?set ROUND (e.g. ROUND=1)}"
SEEDS_FILE="${SEEDS_FILE:-$ROOT/docs/seeds-round-$ROUND.md}"
OUT="${OUT:-$ROOT/scripts/evidence}"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
ATTAX_COMMIT="${ATTAX_COMMIT:-7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b}"
ATTAX_PATCH_NOTE="${ATTAX_PATCH_NOTE:-attax-vision-40.patch (68 lines): ROWS 40, Y_SPAN 48, grid 40, board.rows 40, attaxCellsToTbp 19->39, posY = 18 - maxY (S2 spawn); no search/evaluation changes}"
ATTAX_PATCH_SHA256="${ATTAX_PATCH_SHA256:-e22aab7b5a2f9a95efb562e24b3f33da351bb942d6daf6e205475b5871ed3b70}"
ATTAX_VISIBLE_ROWS="${ATTAX_VISIBLE_ROWS:-0}"
GATE_ATTAX="${GATE_ATTAX:-50}"
GATE_CC2="${GATE_CC2:-48}"
GATE_CLYS="${GATE_CLYS:-48}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
TURNS="${TURNS:-1000}"
MAX_STAGES="${MAX_STAGES:-3}"

ARGS=(run --round "$ROUND" --out "$OUT"
  --attax-dir "$ATTAX_DIR" --attax-commit "$ATTAX_COMMIT"
  --attax-patch-note "$ATTAX_PATCH_NOTE" --attax-patch-sha256 "$ATTAX_PATCH_SHA256"
  --attax-visible-rows "$ATTAX_VISIBLE_ROWS"
  --gate-attax "$GATE_ATTAX" --gate-cc2 "$GATE_CC2" --gate-clys "$GATE_CLYS"
  --ms "$MS" --threads "$THREADS" --turns "$TURNS" --max-stages "$MAX_STAGES")

if [ -n "${SEEDS_OVERRIDE:-}" ]; then
  ARGS+=(--seeds "$SEEDS_OVERRIDE")
else
  ARGS+=(--seeds-file "$SEEDS_FILE")
fi
if [ -n "${CC2_PROGRAM:-}" ]; then
  ARGS+=(--cc2-program "$CC2_PROGRAM" --cc2-args "${CC2_ARGS:-}" --cc2-commit "${CC2_COMMIT:-}" --cc2-visible-rows "${CC2_VISIBLE_ROWS:-40}")
fi
if [ -n "${CLYS_PROGRAM:-}" ]; then
  ARGS+=(--clys-program "$CLYS_PROGRAM" --clys-args "${CLYS_ARGS:-}" --clys-commit "${CLYS_COMMIT:-}" --clys-visible-rows "${CLYS_VISIBLE_ROWS:-40}")
fi
if [ "${DRY_RUN:-0}" = "1" ]; then
  ARGS+=(--dry-run)
fi

cd "$ROOT"
cargo run --offline --release -p intetrigence-arena --bin round -- "${ARGS[@]}"
