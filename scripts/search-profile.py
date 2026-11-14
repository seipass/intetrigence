#!/usr/bin/env python3
"""Print the per-game search profile (depth / width / nodes / fallback).

Usage:
    python scripts/search-profile.py scripts/evidence/tuning-cc2

Reads `audit/*.json` and prints, for each game, the intetrigence search
statistics (protocol v1.31 §8.4.1) and a compact depth progression in
50-turn buckets. TBP opponents have no search statistics, so only slot 0
(intetrigence) is shown.
"""

from __future__ import annotations

import glob
import json
import statistics
import sys


def percentile(values: list[float], p: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = round((len(ordered) - 1) * p / 100.0)
    return ordered[index]


def fmt(value: float | None, digits: int = 1) -> str:
    return "n/a" if value is None else f"{value:.{digits}f}"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    directory = sys.argv[1]
    files = sorted(glob.glob(f"{directory}/audit/*.json"))
    if not files:
        print(f"no audit files in {directory}", file=sys.stderr)
        return 1
    for path in files:
        with open(path, encoding="utf-8") as handle:
            audit = json.load(handle)
        search = audit.get("search") or [[], []]
        turns = search[0] if search else []
        depths = [t["depth"] for t in turns if t.get("depth") is not None]
        widths = [t["width"] for t in turns if t.get("width") is not None]
        nodes = [t["nodes"] for t in turns if t.get("nodes") is not None]
        fallbacks = sum(1 for t in turns if t.get("fallback"))
        print(
            f"{audit.get('seed')}-{audit.get('sheet')} "
            f"{audit.get('reason')} turns={audit.get('turns')} "
            f"search_turns={len(turns)} fallback={fallbacks} "
            f"depth(med/p95/max)={fmt(percentile(depths, 50))}/"
            f"{fmt(percentile(depths, 95))}/{fmt(max(depths) if depths else None, 0)} "
            f"width(med/p95/max)={fmt(percentile(widths, 50))}/"
            f"{fmt(percentile(widths, 95))}/{fmt(max(widths) if widths else None, 0)} "
            f"nodes(med/max)={fmt(percentile(nodes, 50), 0)}/"
            f"{fmt(max(nodes) if nodes else None, 0)}"
        )
        # Depth progression: median depth per 50-turn bucket.
        buckets: dict[int, list[float]] = {}
        for turn in turns:
            depth = turn.get("depth")
            if depth is None:
                continue
            bucket = int(turn.get("turn", 0)) // 50 * 50
            buckets.setdefault(bucket, []).append(float(depth))
        if buckets:
            progression = " ".join(
                f"{bucket}-{bucket + 49}:{fmt(statistics.median(values), 0)}"
                for bucket, values in sorted(buckets.items())
            )
            print(f"  depth by turn bucket: {progression}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
