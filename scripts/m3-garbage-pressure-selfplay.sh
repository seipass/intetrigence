#!/usr/bin/env bash
# M3 garbage-pressure self-play baseline (protocol v1.11 §1.3).
#
# Both slots run the in-process search, so both players are full-vision and both
# receive the same periodic garbage injection. The mirror match makes the win
# rate ~50%; the diagnostic metrics are survival turns, board height and lines
# cleared, not the win rate.
#
# This is a DIAGNOSTIC profile: its numbers are not usable for final claims.
# Held-out seeds (20001+) must NOT be used here.
#
# Environment:
#   OUT_DIR        evidence directory (default: <repo>/scripts/evidence/m3-garbage-pressure-selfplay)
#   SEEDS          comma separated tuning seeds (default: 9001..9010)
#   TURNS          turn cap (default: 300; a lower bound on survival)
#   MS             think budget per move (default: 100)
#   THREADS        search threads per bot (default: 4; 8 total)
#   INTERVAL       turns between injections (default: 8)
#   PRESSURE_ROWS  rows per injection (default: 4)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m3-garbage-pressure-selfplay}"
SEEDS="${SEEDS:-9001,9002,9003,9004,9005,9006,9007,9008,9009,9010}"
TURNS="${TURNS:-300}"
MS="${MS:-100}"
THREADS="${THREADS:-4}"
INTERVAL="${INTERVAL:-8}"
PRESSURE_ROWS="${PRESSURE_ROWS:-4}"
REPORT="${REPORT:-$ROOT/docs/m3-garbage-pressure-selfplay.md}"
TITLE="${TITLE:-M3 garbage-pressure self-play baseline (${INTERVAL} turns / ${PRESSURE_ROWS} rows)}"

cd "$ROOT"
rm -rf "$OUT_DIR"

cargo run --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A,B --turns "$TURNS" \
  --profile garbage-pressure --interval "$INTERVAL" --pressure-rows "$PRESSURE_ROWS" \
  --self-play --bot search --ms "$MS" --threads "$THREADS" \
  --out "$OUT_DIR" \
  --note "M3 garbage-pressure self-play diagnostic ($INTERVAL turns / $PRESSURE_ROWS rows, --ms $MS, $THREADS threads each). Not usable for final claims."

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"

EVIDENCE="$OUT_DIR" REPORT="$REPORT" TITLE="$TITLE" python "$ROOT/scripts/analyze-m2.py"
