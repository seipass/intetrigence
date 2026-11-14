# M3 garbage-pressure self-play (4 turns / 4 rows)

- evidence: `scripts/evidence/m3-garbage-pressure-selfplay-4x4`
- profile: **garbage-pressure:4:4** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 10 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search (`--ms 100 --threads 4`), commit `47df03e`
- opponent: intetrigence (self-play; both slots run the same search)
- rules: `docs/s2-spec.md@v1.3.11+47df03e` / `docs/s2-referee-protocol.md@v1.9+47df03e`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- command: `intetrigence --ms 100 --threads 4` (both slots)

## Result

- wins / losses / draws: **4 / 6 / 0**
- forfeits: none (self-play; both slots are full-vision)
- win rate: **40.0%** (Wilson 95% CI 16.8%..68.7%)
- win composition: TopOut 3, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 3
- **non-forfeit games (40-row TopOut / TurnCap, n=10)**: 4 wins / 6 losses = **40.0%** (Wilson 95% CI 16.8%..68.7%)
- replay verified: **10/10**
- per sheet: A 2/5, B 2/5
- seat note: losses by sheet ['A', 'B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- slot 0: median 96.6 ms, p95 97.8 ms, p99 98.2 ms, max 98.9 ms (median/p95/p99 are means of per-game values; max is the match worst)
- slot 1: median 96.6 ms, p95 97.7 ms, p99 98.1 ms, max 98.7 ms
- placements over the 200 ms budget: slot 0 0, slot 1 0
- first-choice acceptance (mean over games): slot 0 1.00, slot 1 1.00

## Attack and garbage

- attack/piece: slot 0 **1.134**, slot 1 **1.135**
- received garbage per game: slot 0 **94.7**, slot 1 **93.3**
- slot 0 search: 1,136,990 nodes total, 12 fallback placements

## Board height and survival

- mean turns 165.5; turn-cap games 3; TopOut games 7
- slot 0: max height 21.1, mean height 8.7, final 14.7, lines cleared/game 139.7
- slot 1: max height 21.1, mean height 8.1, final 15.1, lines cleared/game 137.7

## Scoring audit

- slot 0: spin mismatches 0 / 1653, same-spin attack mismatches 0 / 1653
- slot 1: spin mismatches 0 / 1652, same-spin attack mismatches 0 / 1652

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 3
- attrition (turn cap, received more garbage): 2
- opener (died within 40 turns): 1

| seed | sheet | reason | turns | slot 0 recv | slot 1 recv | slot 0 attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9002 | B | TurnCap | 300 | 181 | 169 | 1.177 | attrition (turn cap, received more garbage) |
| 9003 | A | TopOut | 181 | 85 | 99 | 1.116 | downstack (cleared but could not keep up with garbage) |
| 9004 | A | TopOut | 218 | 136 | 119 | 1.064 | downstack (cleared but could not keep up with garbage) |
| 9004 | B | TopOut | 81 | 62 | 47 | 1.025 | downstack (cleared but could not keep up with garbage) |
| 9005 | A | TopOut | 33 | 33 | 14 | 0.818 | opener (died within 40 turns) |
| 9005 | B | TurnCap | 300 | 173 | 171 | 1.210 | attrition (turn cap, received more garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m3-garbage-pressure-selfplay.sh`
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
