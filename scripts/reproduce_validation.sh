#!/usr/bin/env bash
set -euo pipefail

root_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root_dir"

referee=target/release/intetrigence
strict_referee=target/strict/release/intetrigence
opponent=vendor/cold-clear-2/target/release/cold-clear-2
expected_referee=04241d00d60e80bbfab70318cc54d194e481f554891e1b8e7961eefb15910778
expected_opponent=344d3e4c1ca5f0564b59e17646a926dd2adeab24f3c22185bc6f08e74a8b061e
expected_config=9de091a9dfe69175ff59e3ea7f830230f0f12e419bf30d948e0dd7c7306fc561

# The pinned public source has no hidden worker-count or strength preset. Keep
# the strongest exposed profile explicit and fail if that upstream fact changes.
grep -Eq 'fn spawn_workers' vendor/cold-clear-2/src/lib.rs \
    && grep -Eq 'for _ in 0\.\.1' vendor/cold-clear-2/src/lib.rs || {
    echo "unexpected upstream worker configuration" >&2
    exit 1
}
[[ "$(find vendor/cold-clear-2 -path '*/target' -prune -o -type f -name '*.json' -print | wc -l)" -eq 1 ]] || {
    echo "unexpected upstream configuration files" >&2
    exit 1
}

[[ -x "$referee" ]] || { echo "missing $referee" >&2; exit 1; }
[[ -x "$strict_referee" ]] || { echo "missing $strict_referee" >&2; exit 1; }
[[ -x "$opponent" ]] || { echo "missing $opponent" >&2; exit 1; }

check_hash() {
    local path=$1 expected=$2 actual
    actual=$(sha256sum "$path" | awk '{print $1}')
    [[ "$actual" == "$expected" ]] || {
        echo "SHA-256 mismatch: $path" >&2
        echo "expected $expected, got      $actual" >&2
        exit 1
    }
}

check_hash "$referee" "$expected_referee"
check_hash "$strict_referee" "$expected_referee"
check_hash "$opponent" "$expected_opponent"
check_hash vendor/cold-clear-2/src/default.json "$expected_config"

for replay in \
    results/benchmarks/benchmark-10ms-40pps-100.jsonl \
    results/benchmarks/benchmark-100ms-5pps-100.jsonl \
    results/comparisons/cold-clear2/external-retained-10ms-40pps-100.jsonl \
    results/comparisons/cold-clear2/external-retained-100ms-5pps-100.jsonl
do
    "$referee" verify-replay --input "$replay"
done
"$strict_referee" verify-replay --input results/comparisons/cold-clear2/external-strict-100ms-5pps-100.jsonl
"$strict_referee" verify-replay --input results/comparisons/cold-clear2/external-guideline-strict-10ms-40pps-100.jsonl
"$strict_referee" verify-replay --input results/comparisons/cold-clear2/external-guideline-strict-10ms-40pps-100-v2.jsonl

if [[ "${RUN_BENCHMARKS:-0}" == 1 ]]; then
    "$referee" arena --games 100 --seed 5101 --ms 100 --pps 5 --turns 1000 \
        --output results/benchmarks/benchmark-100ms-5pps-100.json \
        --replay results/benchmarks/benchmark-100ms-5pps-100.jsonl
    "$strict_referee" external-arena --strict-external-time --opponent "$opponent" \
        --games 100 --seed 8301 --ms 100 --pps 5 --turns 1000 \
        --output results/comparisons/cold-clear2/external-strict-100ms-5pps-100.json \
        --replay results/comparisons/cold-clear2/external-strict-100ms-5pps-100.jsonl
fi
