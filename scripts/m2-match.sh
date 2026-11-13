#!/usr/bin/env bash
# M2 gate reproduction: tuning seeds 9001-9010 x both sheets against attax,
# with the real intetrigence search (--ms 100, 8 threads) and replay
# verification.
#
# Held-out seeds (10001+) must NOT be used here; this run is for tuning and the
# M2 gate, not the final match.
#
# Prerequisites:
#   - bun 1.4.0 on PATH
#   - attax at 7b38f51 (override with ATTAX_DIR)
#
# Environment:
#   ATTAX_DIR  attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   OUT_DIR    evidence directory (default: <repo>/scripts/evidence/m2-tuning)
#   SEEDS      comma separated tuning seeds (default: 9001..9010)
#   TURNS      turn cap (default: 1000)
#   MS         think budget per move (default: 100)
#   THREADS    search threads (default: 8)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m2-tuning}"
SEEDS="${SEEDS:-9001,9002,9003,9004,9005,9006,9007,9008,9009,9010}"
TURNS="${TURNS:-1000}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"

if [ ! -f "$ATTAX_DIR/tbp/attax-tbp.ts" ]; then
  echo "attax entry point not found: $ATTAX_DIR/tbp/attax-tbp.ts" >&2
  exit 1
fi

cd "$ROOT"
rm -rf "$OUT_DIR"

cargo run --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A,B --turns "$TURNS" \
  --attax "$ATTAX_DIR" --bot search --ms "$MS" --threads "$THREADS" \
  --out "$OUT_DIR"

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"
