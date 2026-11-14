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
#   TURNS      turn cap (default: 1000)
#   CC2_DIR    CC2 checkout (default: C:/Users/Hasilan/Projects/cold-clear-2)
#   CLYS_DIR   clys checkout (default: C:/Users/Hasilan/Projects/clys)
#   ADAPTER    clys adapter (default: C:/Users/Hasilan/Projects/intetrigence/adapters/clys/tbp/adapter.ts)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-$ROOT/scripts/evidence}"
SEEDS="${SEEDS:-9001,9002,9003,9004,9005,9006,9007,9008,9009,9010}"
TURNS="${TURNS:-1000}"
CC2_DIR="${CC2_DIR:-C:/Users/Hasilan/Projects/cold-clear-2}"
CLYS_DIR="${CLYS_DIR:-C:/Users/Hasilan/Projects/clys}"
ADAPTER="${ADAPTER:-C:/Users/Hasilan/Projects/intetrigence/adapters/clys/tbp/adapter.ts}"

cd "$ROOT"

echo "tuning smoke: CC2 ($SEEDS, sheet A, $TURNS turns)"
cargo run --offline --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A --turns "$TURNS" --bot search --ms 100 --threads 8 \
  --out "$OUT/tuning-cc2" \
  --tbp-program "$CC2_DIR/target/release/cold-clear-2.exe" \
  --tbp-cwd "$CC2_DIR" --tbp-visible-rows 40 \
  --tbp-suggest-delay-ms 100 --tbp-retry-empty-ms 2000 --tbp-name cc2 \
  --note "Tuning smoke vs CC2 (tuning seeds 9001-9010, sheet A, 1000 turns; evaluation seeds untouched)."

echo "tuning smoke: clys ($SEEDS, sheet A, $TURNS turns)"
cargo run --offline --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A --turns "$TURNS" --bot search --ms 100 --threads 8 \
  --out "$OUT/tuning-clys" \
  --tbp-program bun \
  --tbp-args "$ADAPTER,--clys-dir,$CLYS_DIR,--think-ms,100" \
  --tbp-cwd C:/Users/Hasilan/Projects/intetrigence --tbp-visible-rows 40 \
  --tbp-name clys \
  --note "Tuning smoke vs clys (tuning seeds 9001-9010, sheet A, 1000 turns; evaluation seeds untouched)."
