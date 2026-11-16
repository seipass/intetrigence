#!/usr/bin/env bash
# Ten-game tuning smoke against CC2 and clys (protocol v1.14; tuning seeds only).
#
# This is a *diagnostic* run: it uses the tuning seeds 9001-9010 (sheet A,
# 1000 turns) and never touches the evaluation seeds. The official round uses
# the pre-committed evaluation seeds and both sheets.
#
# Environment:
#   OUT        evidence root (default: <repo>/scripts/evidence)
#   SEEDS      comma separated seeds (default: 9001..9010)
#   CC2_SHEETS default A,B (20 games); CLYS_SHEETS default A (10 games)
#   PROFILE=1  print the per-game search profile after each match
#   TURNS      turn cap (default: 1000)
#   CC2_DIR    CC2 checkout (default: C:/Users/Hasilan/Projects/cold-clear-2)
#   CLYS_DIR   clys checkout (default: C:/Users/Hasilan/Projects/clys)
#   ADAPTER    clys adapter (default: C:/Users/Hasilan/Projects/intetrigence/adapters/clys/tbp/adapter.ts)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Only one measurement may use the machine at a time (the think times are
# contaminated by concurrent runs).
source "$ROOT/scripts/measurement-lock.sh"
acquire_measurement "tuning-smoke"
OUT="${OUT:-$ROOT/scripts/evidence}"
SEEDS="${SEEDS:-9001,9002,9003,9004,9005,9006,9007,9008,9009,9010}"
CC2_SHEETS="${CC2_SHEETS:-A,B}"
CLYS_SHEETS="${CLYS_SHEETS:-A}"
TURNS="${TURNS:-1000}"
CC2_DIR="${CC2_DIR:-C:/Users/Hasilan/Projects/cold-clear-2}"
CLYS_DIR="${CLYS_DIR:-C:/Users/Hasilan/Projects/clys}"
ADAPTER="${ADAPTER:-C:/Users/Hasilan/Projects/intetrigence/adapters/clys/tbp/adapter.ts}"

cd "$ROOT"

echo "tuning smoke: CC2 ($SEEDS, sheets $CC2_SHEETS, $TURNS turns)"
cargo run --offline --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets "$CC2_SHEETS" --turns "$TURNS" --bot search --ms 100 --threads 8 \
  --out "$OUT/tuning-cc2" \
  --tbp-program "$CC2_DIR/target/release/cold-clear-2.exe" \
  --tbp-cwd "$CC2_DIR" --tbp-visible-rows 40 \
  --tbp-suggest-delay-ms 100 --tbp-retry-empty-ms 2000 --tbp-supports-180 false --tbp-name cc2 \
  --note "Tuning smoke vs CC2 (tuning seeds 9001-9010, sheet A, 1000 turns; evaluation seeds untouched)."

echo "tuning smoke: clys ($SEEDS, sheets $CLYS_SHEETS, $TURNS turns)"
# Always enable the clys adapter log: a clys child exit is a no-contest, and
# the log records the exit code/signal (the stderr is often empty).
mkdir -p "$OUT"
CLYS_TBP_LOG="$(cd "$OUT" && pwd)/tuning-clys-adapter.jsonl" \
cargo run --offline --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets "$CLYS_SHEETS" --turns "$TURNS" --bot search --ms 100 --threads 8 \
  --out "$OUT/tuning-clys" \
  --tbp-program bun \
  --tbp-args "$ADAPTER,--clys-dir,$CLYS_DIR,--think-ms,100" \
  --tbp-cwd C:/Users/Hasilan/Projects/intetrigence --tbp-visible-rows 40 \
  --tbp-retry-empty-ms 2000 --tbp-supports-180 false \
  --tbp-name clys \
  --note "Tuning smoke vs clys (tuning seeds 9001-9010, sheet A, 1000 turns; evaluation seeds untouched)."

if [ "${PROFILE:-0}" = "1" ]; then
  echo "search profile: CC2"
  python scripts/search-profile.py "$OUT/tuning-cc2"
  echo "search profile: clys"
  python scripts/search-profile.py "$OUT/tuning-clys"
fi
