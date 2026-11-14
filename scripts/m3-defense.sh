#!/usr/bin/env bash
# M3 defense-stress diagnostic baseline (protocol v1.11 §1.3).
#
# Starts both players with deterministic garbage rows so the game reaches the
# 40-row top-out much more often than the official profile. This is a
# DIAGNOSTIC profile: its numbers are not usable for final claims.
#
# Held-out seeds (10001+) must NOT be used here.
#
# Environment:
#   ATTAX_DIR   attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   OUT_DIR     evidence directory (default: <repo>/scripts/evidence/m3-defense)
#   SEEDS       comma separated tuning seeds (default: 9001..9010)
#   TURNS       turn cap (default: 1000)
#   MS          think budget per move (default: 100)
#   THREADS     search threads (default: 8)
#   START_ROWS  start garbage rows (default: 12)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m3-defense}"
SEEDS="${SEEDS:-9001,9002,9003,9004,9005,9006,9007,9008,9009,9010}"
TURNS="${TURNS:-1000}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
START_ROWS="${START_ROWS:-12}"
REPORT="${REPORT:-$ROOT/docs/m3-defense-report.md}"
TITLE="${TITLE:-M3 defense-stress baseline ($START_ROWS start garbage rows)}"

if [ ! -f "$ATTAX_DIR/tbp/attax-tbp.ts" ]; then
  echo "attax entry point not found: $ATTAX_DIR/tbp/attax-tbp.ts" >&2
  exit 1
fi

cd "$ROOT"
rm -rf "$OUT_DIR"

cargo run --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A,B --turns "$TURNS" \
  --attax "$ATTAX_DIR" --bot search --ms "$MS" --threads "$THREADS" \
  --profile defense-stress --start-rows "$START_ROWS" \
  --out "$OUT_DIR" \
  --note "M3 defense-stress diagnostic baseline ($START_ROWS start garbage rows, --ms $MS, $THREADS threads). Not usable for final claims."

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"

EVIDENCE="$OUT_DIR" REPORT="$REPORT" TITLE="$TITLE" python "$ROOT/scripts/analyze-m2.py"
