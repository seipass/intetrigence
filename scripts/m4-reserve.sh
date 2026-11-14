#!/usr/bin/env bash
# M4 reserve validation: official profile, reserve seeds 10001-10020 x both
# sheets against attax. This is the overfitting gate before the final match.
#
# The final-match seeds (20001+) must NOT be used here.
#
# Environment:
#   ATTAX_DIR  attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   OUT_DIR    evidence directory (default: <repo>/scripts/evidence/m4-reserve)
#   SEEDS      comma separated reserve seeds (default: 10001..10020)
#   TURNS      turn cap (default: 1000)
#   MS         think budget per move (default: 100)
#   THREADS    search threads (default: 8)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m4-reserve}"
SEEDS="${SEEDS:-10001,10002,10003,10004,10005,10006,10007,10008,10009,10010,10011,10012,10013,10014,10015,10016,10017,10018,10019,10020}"
TURNS="${TURNS:-1000}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
REPORT="${REPORT:-$ROOT/docs/m4-reserve-report.md}"
TITLE="${TITLE:-M4 reserve validation (seeds 10001-10020)}"

if [ ! -f "$ATTAX_DIR/tbp/attax-tbp.ts" ]; then
  echo "attax entry point not found: $ATTAX_DIR/tbp/attax-tbp.ts" >&2
  exit 1
fi

cd "$ROOT"
rm -rf "$OUT_DIR"

cargo run --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A,B --turns "$TURNS" \
  --attax "$ATTAX_DIR" --bot search --ms "$MS" --threads "$THREADS" \
  --out "$OUT_DIR" \
  --note "M4 reserve validation (official profile, reserve seeds 10001-10020 x A/B). Final-match seeds 20001+ not used."

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"

EVIDENCE="$OUT_DIR" REPORT="$REPORT" TITLE="$TITLE" \
  SEED_RANGE="reserve seeds 10001-10020" \
  SEED_NOTE="final-match seeds 20001+ were **not** used (M4 reserve only)" \
  python "$ROOT/scripts/analyze-m2.py"
