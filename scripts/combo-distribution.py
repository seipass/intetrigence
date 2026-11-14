#!/usr/bin/env python3
"""Combo band distribution from replay files.

Usage:
    python scripts/combo-distribution.py scripts/evidence/tuning-cc2

For each game and for the aggregate, counts the placements by the combo they
achieved (`combo_after`): 0 / 1-2 / 3-4 / 5-9 / 10+. A placement with
`combo_after == 0` did not clear a line, so it neither started nor continued a
combo. The report also prints the `combo_before == 0` share (the previous
placement did not clear), which is the number s2-spec-lead's 53.8% refers to.
"""

from __future__ import annotations

import glob
import json
import sys

BANDS = [(0, 0), (1, 2), (3, 4), (5, 9), (10, 10**9)]


def band(combo: int) -> str:
    for low, high in BANDS:
        if low <= combo <= high:
            return "0" if low == high == 0 else f"{low}-{high}" if high < 10**9 else f"{low}+"
    return "?"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    directory = sys.argv[1]
    files = sorted(glob.glob(f"{directory}/replays/*.jsonl"))
    if not files:
        print(f"no replays in {directory}", file=sys.stderr)
        return 1
    totals = [{} for _ in range(2)]
    before_zero = [0 for _ in range(2)]
    placements = [0 for _ in range(2)]
    for path in files:
        per = [{} for _ in range(2)]
        for line in open(path, encoding="utf-8"):
            record = json.loads(line)
            if record.get("type") != "turn":
                continue
            for placement in record.get("placements", []):
                slot = placement["slot"]
                if slot not in (0, 1):
                    continue
                combo = int(placement.get("combo_after", 0))
                key = band(combo)
                per[slot][key] = per[slot].get(key, 0) + 1
                totals[slot][key] = totals[slot].get(key, 0) + 1
                placements[slot] += 1
                if int(placement.get("combo_before", 0)) == 0:
                    before_zero[slot] += 1
        name = path.replace("\\", "/").split("/")[-1]
        parts = " | ".join(
            f"slot{slot}: "
            + " ".join(f"{key}={per[slot].get(key, 0)}" for key in ["0", "1-2", "3-4", "5-9", "10+"])
            for slot in (0, 1)
        )
        print(f"{name} {parts}")
    for slot in (0, 1):
        total = max(placements[slot], 1)
        shares = " ".join(
            f"{key}={totals[slot].get(key, 0)} ({100.0 * totals[slot].get(key, 0) / total:.1f}%)"
            for key in ["0", "1-2", "3-4", "5-9", "10+"]
        )
        print(
            f"slot {slot}: placements={placements[slot]} {shares} "
            f"combo_before==0={before_zero[slot]} ({100.0 * before_zero[slot] / total:.1f}%)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
