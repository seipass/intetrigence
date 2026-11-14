#!/usr/bin/env bash
# M5 provisional match: 50 fresh seeds x both sheets = 100 games.
#
# This is a REFERENCE-VALUE run against the current attax (guideline scoring,
# 20-row vision). The official 90% claim uses the attax S2 version with the
# reserved final seeds 20001-20050; those seeds must NOT be used here.
#
# Seeds 11001-11050 are a fresh range unused by either project:
#   intetrigence: 1001-1005, 9001-9020, 10001-1040
#   attax:        1001-1200, 2001-2060, 5001-5050, 7001-7050
#
# Protocol: official profile (empty start board), 5 PPS (frame = turn * 12),
# 1000 turn cap, 5 preview + hold, --ms 100, 8 search threads.
#
# Environment:
#   ATTAX_DIR  attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   OUT_DIR    evidence directory (default: <repo>/scripts/evidence/m5-provisional)
#   SEEDS      comma separated seeds (default: 11001..11050)
#   TURNS      turn cap (default: 1000)
#   MS         think budget per move (default: 100)
#   THREADS    search threads (default: 8)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m5-provisional}"
SEEDS="${SEEDS:-$(seq -s, 11001 11050)}"
TURNS="${TURNS:-1000}"
MS="${MS:-100}"
THREADS="${THREADS:-8}"
REPORT="${REPORT:-$ROOT/docs/m5-provisional-report.md}"
TITLE="${TITLE:-M5 provisional match (100 games, seeds 11001-11050, current guideline attax)}"

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
  --note "M5 provisional match (official profile, fresh seeds 11001-11050 x A/B). Reference value only: attax is guideline-scored with a 20-row vision limit; the official claim will use the attax S2 version with the reserved seeds 20001-20050."

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"

EVIDENCE="$OUT_DIR" REPORT="$REPORT" TITLE="$TITLE" \
  SEED_RANGE="provisional seeds 11001-11050" \
  SEED_NOTE="seeds 11001-11050 are a fresh range unused by either project; the reserved final seeds 20001-20050 were **not** used" \
  ATTAX_NOTE="attax at this commit is guideline-scored and has a 20-row vision limit; the S2 version is not released upstream (7b38f51 vs upstream 57a5a6a: docs/page-generation only, no scoring/search/vision changes). This match is a **reference value**; the official 90% claim will be made against the attax S2 version with the reserved seeds 20001-20050." \
  HELDOUT="attax: tuning 2001-2060 (scripts/tune.ts), held-out benchmark 1001-1200, prior matches 5001-5050 and 7001-7050; intetrigence: M0 1001-1005, M2 9001-9010, M4 10001-10020. Seeds 11001-11050 appear in neither project's evidence." \
  REPRO="scripts/m5-provisional.sh" \
  python "$ROOT/scripts/analyze-m2.py"
