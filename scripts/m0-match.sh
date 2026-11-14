#!/usr/bin/env bash
# M0 gate reproduction: 5 fixed seeds x both sheets against attax, with
# replay verification and audit logs (docs/arena.md §4).
#
# Prerequisites:
#   - bun 1.4.0 on PATH
#   - attax at 7b38f51 (override with ATTAX_DIR)
#
# Environment:
#   ATTAX_DIR  attax checkout (default: C:/Users/Hasilan/Projects/attax)
#   OUT_DIR    evidence directory (default: <repo>/scripts/evidence/m0)
#   SEEDS      comma separated seeds (default: 1001,1002,1003,1004,1005)
#   TURNS      turn cap (default: 1000)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATTAX_DIR="${ATTAX_DIR:-C:/Users/Hasilan/Projects/attax}"
OUT_DIR="${OUT_DIR:-$ROOT/scripts/evidence/m0}"
SEEDS="${SEEDS:-1001,1002,1003,1004,1005}"
TURNS="${TURNS:-1000}"

if [ ! -f "$ATTAX_DIR/tbp/attax-tbp.ts" ]; then
  echo "attax entry point not found: $ATTAX_DIR/tbp/attax-tbp.ts" >&2
  exit 1
fi

cd "$ROOT"
rm -rf "$OUT_DIR"

cargo run --release -p intetrigence-arena --bin arena -- match \
  --seeds "$SEEDS" --sheets A,B --turns "$TURNS" \
  --attax "$ATTAX_DIR" --out "$OUT_DIR"   --note "M0 gate: plumbing / audit / replay proof. The intetrigence side is the in-process M0 baseline stand-in, not the real bot."

cargo run --release -p intetrigence-arena --bin arena -- verify "$OUT_DIR"
