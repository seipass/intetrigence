# M3 garbage-pressure self-play baseline (8 turns / 4 rows)

- evidence: `scripts/evidence/m3-garbage-pressure-selfplay`
- profile: **garbage-pressure:8:4** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search (`--ms 100 --threads 4`), commit `47df03e`
- opponent: intetrigence (self-play; both slots run the same search)
- rules: `docs/s2-spec.md@v1.3.11+47df03e` / `docs/s2-referee-protocol.md@v1.9+47df03e`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- command: `intetrigence --ms 100 --threads 4` (both slots)

## Result

- wins / losses / draws: **13 / 7 / 0**
- forfeits: none (self-play; both slots are full-vision)
- win rate: **65.0%** (Wilson 95% CI 43.3%..81.9%)
- win composition: TopOut 8, forfeit-vision 0, forfeit-other 0; losses: TopOut 3, forfeit 0; turn-cap games 9
- **non-forfeit games (40-row TopOut / TurnCap, n=20)**: 13 wins / 7 losses = **65.0%** (Wilson 95% CI 43.3%..81.9%)
- replay verified: **20/20**
- per sheet: A 7/10, B 6/10
- seat note: losses by sheet ['A', 'B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- slot 0: median 96.5 ms, p95 97.7 ms, p99 98.2 ms, max 99.9 ms (median/p95/p99 are means of per-game values; max is the match worst)
- slot 1: median 96.5 ms, p95 97.8 ms, p99 98.2 ms, max 106.4 ms
- placements over the 200 ms budget: slot 0 0, slot 1 0
- first-choice acceptance (mean over games): slot 0 1.00, slot 1 1.00

## Attack and garbage

- attack/piece: slot 0 **1.087**, slot 1 **1.053**
- received garbage per game: slot 0 **93.0**, slot 1 **99.2**
- slot 0 search: 2,897,272 nodes total, 4 fallback placements

## Board height and survival

- mean turns 220.4; turn-cap games 9; TopOut games 11
- slot 0: max height 19.1, mean height 7.0, final 8.7, lines cleared/game 165.2
- slot 1: max height 22.1, mean height 7.9, final 12.7, lines cleared/game 166.6

## Scoring audit

- slot 0: spin mismatches 0 / 4404, same-spin attack mismatches 0 / 4404
- slot 1: spin mismatches 0 / 4407, same-spin attack mismatches 0 / 4407

## Loss classification (coarse)

- attrition (turn cap, received more garbage): 4
- downstack (cleared but could not keep up with garbage): 3

| seed | sheet | reason | turns | slot 0 recv | slot 1 recv | slot 0 attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9002 | B | TopOut | 284 | 122 | 123 | 0.989 | downstack (cleared but could not keep up with garbage) |
| 9003 | B | TurnCap | 300 | 118 | 113 | 1.063 | attrition (turn cap, received more garbage) |
| 9005 | A | TopOut | 140 | 66 | 47 | 0.871 | downstack (cleared but could not keep up with garbage) |
| 9005 | B | TopOut | 244 | 115 | 108 | 1.127 | downstack (cleared but could not keep up with garbage) |
| 9007 | A | TurnCap | 300 | 137 | 126 | 1.173 | attrition (turn cap, received more garbage) |
| 9009 | A | TurnCap | 300 | 138 | 123 | 1.040 | attrition (turn cap, received more garbage) |
| 9010 | B | TurnCap | 300 | 133 | 133 | 1.013 | attrition (turn cap, received more garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m3-garbage-pressure-selfplay.sh`
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
