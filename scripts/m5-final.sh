#!/usr/bin/env bash
# M5 final match: 50 held-out final seeds x both sheets = 100 games.
#
# Seeds 20001-20050 are reserved for this run. They must NOT be used for
# tuning, diagnostics or any other run.
#
# Protocol: official profile (empty start board), 5 PPS (frame = turn * 12),
# 1000 turn cap, 5 preview + hold, --ms 100, 8 search threads.
#
# Environment:
#   ATTAX_DIR  attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   OUT_DIR    evidence directory (default: <repo>/scripts/evidence/m5-final)
#   SEEDS      comma separated seeds (default: 20001..20050)
#   TURNS      turn cap (default: 1000)
#   MS         think budget per move (default: 100)
#   THREADS    search threads (default: 8)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m5-final}"
SEEDS="${SEEDS:-$(seq -s, 20001 20050)}"
TURNS="${TURNS:-1000}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
REPORT="${REPORT:-$ROOT/docs/m5-final-report.md}"
TITLE="${TITLE:-M5 final match (held-out seeds 20001-20050, official profile)}"

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
  --note "M5 final match (official profile, held-out final seeds 20001-20050 x A/B). Seeds 20001-20050 were not used in either project before this run."

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"

EVIDENCE="$OUT_DIR" REPORT="$REPORT" TITLE="$TITLE" \
  SEED_RANGE="held-out final seeds 20001-20050" \
  SEED_NOTE="seeds 20001-20050 are reserved for this match and were **not** used in either project's tuning, diagnostics or prior matches" \
  ATTAX_NOTE="attax at this commit is guideline-scored and has a 20-row vision limit; the S2 version is not released upstream (7b38f51 vs upstream 57a5a6a: docs/page-generation only, no scoring/search/vision changes)." \
  HELDOUT="attax: tuning 2001-2060 (scripts/tune.ts), held-out benchmark 1001-1200, prior matches 5001-5050 and 7001-7050; intetrigence: M0 1001-1005, M2 9001-9010, M4 10001-10020. Seeds 20001-20050 appear in neither project's evidence." \
  REPRO="scripts/m5-final.sh" \
  python "$ROOT/scripts/analyze-m2.py"
