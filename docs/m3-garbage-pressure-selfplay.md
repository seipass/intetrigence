# M3 garbage-pressure self-play baseline (8 turns / 4 rows)

- evidence: `scripts/evidence/surge-selfplay`
- profile: **garbage-pressure:8:4** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 4 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search (`--ms 100 --threads 4`), commit `7aa0c73`
- opponent: intetrigence (self-play; both slots run the same search)
- rules: `docs/s2-spec.md@v1.3.11+7aa0c73` / `docs/s2-referee-protocol.md@v1.9+7aa0c73`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- command: `intetrigence --ms 100 --threads 4` (both slots)

## Result

- wins / losses / draws: **0 / 4 / 0**
- forfeits: none (self-play; both slots are full-vision)
- win rate: **0.0%** (Wilson 95% CI 0.0%..49.0%)
- win composition: TopOut 0, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 0
- **non-forfeit games (40-row TopOut / TurnCap, n=4)**: 0 wins / 4 losses = **0.0%** (Wilson 95% CI 0.0%..49.0%)
- replay verified: **4/4**
- per sheet: A 0/2, B 0/2
- seat note: losses by sheet ['A', 'B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- slot 0: median 96.9 ms, p95 98.7 ms, p99 103.9 ms, max 128.5 ms (median/p95/p99 are means of per-game values; max is the match worst)
- slot 1: median 97.0 ms, p95 98.7 ms, p99 103.4 ms, max 122.8 ms
- placements over the 200 ms budget: slot 0 0, slot 1 0
- first-choice acceptance (mean over games): slot 0 1.00, slot 1 1.00

## Attack and garbage

- attack/piece: slot 0 **0.764**, slot 1 **0.860**
- received garbage per game: slot 0 **41.0**, slot 1 **32.5**
- slot 0 search: 246,101 nodes total, 0 fallback placements

## Board height and survival

- mean turns 93.2; turn-cap games 0; TopOut games 4
- slot 0: max height 21.0, mean height 8.0, final 18.0, lines cleared/game 54.0
- slot 1: max height 17.5, mean height 6.6, final 8.0, lines cleared/game 60.8

## Scoring audit

- slot 0: spin mismatches 0 / 373, same-spin attack mismatches 0 / 373
- slot 1: spin mismatches 0 / 371, same-spin attack mismatches 0 / 371

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 3
- buried (no clears in the last 8 turns, garbage >= 15): 1

| seed | sheet | reason | turns | slot 0 recv | slot 1 recv | slot 0 attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9001 | A | TopOut | 99 | 43 | 35 | 0.717 | downstack (cleared but could not keep up with garbage) |
| 9001 | B | TopOut | 107 | 44 | 29 | 0.682 | downstack (cleared but could not keep up with garbage) |
| 9002 | A | TopOut | 51 | 23 | 20 | 0.765 | buried (no clears in the last 8 turns, garbage >= 15) |
| 9002 | B | TopOut | 116 | 54 | 46 | 0.879 | downstack (cleared but could not keep up with garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- held-out: seeds 10001+ were **not** used (tuning seeds only).

## Reproduction

- `scripts/m3-garbage-pressure-selfplay.sh`
- `arena verify scripts/evidence/surge-selfplay`
- `python scripts/analyze-m2.py`
