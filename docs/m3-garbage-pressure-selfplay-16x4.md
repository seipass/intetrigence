# M3 garbage-pressure self-play (16 turns / 4 rows)

- evidence: `scripts/evidence/m3-garbage-pressure-selfplay-16x4`
- profile: **garbage-pressure:16:4** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 10 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search (`--ms 100 --threads 4`), commit `47df03e`
- opponent: intetrigence (self-play; both slots run the same search)
- rules: `docs/s2-spec.md@v1.3.11+47df03e` / `docs/s2-referee-protocol.md@v1.9+47df03e`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- command: `intetrigence --ms 100 --threads 4` (both slots)

## Result

- wins / losses / draws: **5 / 5 / 0**
- forfeits: none (self-play; both slots are full-vision)
- win rate: **50.0%** (Wilson 95% CI 23.7%..76.3%)
- win composition: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 9
- **non-forfeit games (40-row TopOut / TurnCap, n=10)**: 5 wins / 5 losses = **50.0%** (Wilson 95% CI 23.7%..76.3%)
- replay verified: **10/10**
- per sheet: A 3/5, B 2/5
- seat note: losses by sheet ['A', 'B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- slot 0: median 96.5 ms, p95 97.7 ms, p99 98.1 ms, max 99.1 ms (median/p95/p99 are means of per-game values; max is the match worst)
- slot 1: median 96.5 ms, p95 97.7 ms, p99 98.1 ms, max 99.4 ms
- placements over the 200 ms budget: slot 0 0, slot 1 0
- first-choice acceptance (mean over games): slot 0 1.00, slot 1 1.00

## Attack and garbage

- attack/piece: slot 0 **0.979**, slot 1 **1.001**
- received garbage per game: slot 0 **104.2**, slot 1 **98.3**
- slot 0 search: 1,827,901 nodes total, 1 fallback placements

## Board height and survival

- mean turns 280.0; turn-cap games 9; TopOut games 1
- slot 0: max height 18.6, mean height 6.5, final 7.3, lines cleared/game 199.5
- slot 1: max height 19.8, mean height 6.7, final 7.2, lines cleared/game 195.3

## Scoring audit

- slot 0: spin mismatches 0 / 2799, same-spin attack mismatches 0 / 2799
- slot 1: spin mismatches 0 / 2800, same-spin attack mismatches 0 / 2800

## Loss classification (coarse)

- attrition (turn cap, received more garbage): 5

| seed | sheet | reason | turns | slot 0 recv | slot 1 recv | slot 0 attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9001 | A | TurnCap | 300 | 112 | 102 | 0.970 | attrition (turn cap, received more garbage) |
| 9001 | B | TurnCap | 300 | 129 | 104 | 0.990 | attrition (turn cap, received more garbage) |
| 9003 | B | TurnCap | 300 | 122 | 116 | 1.013 | attrition (turn cap, received more garbage) |
| 9004 | A | TurnCap | 300 | 111 | 93 | 0.880 | attrition (turn cap, received more garbage) |
| 9004 | B | TurnCap | 300 | 122 | 101 | 0.933 | attrition (turn cap, received more garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m3-garbage-pressure-selfplay.sh`
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
