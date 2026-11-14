# M4 reserve validation v2 (defense v2, seeds 10001-10020)

- evidence: `scripts/evidence/m4-reserve-v2`
- profile: **official** (empty start board; the only profile usable for final claims)
- games: 40 (reserve seeds 10001-10020 x sheets A/B)
- final-match seeds 20001+ were **not** used (M4 reserve v2 only)
- intetrigence: real search (`--ms 100 --threads 8`), commit `247ccf7`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+247ccf7` / `docs/s2-referee-protocol.md@v1.9+247ccf7`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **37 / 3 / 0**
- forfeits: attax **32** (20-row view), intetrigence **0**
- win rate: **92.5%** (Wilson 95% CI 80.1%..97.4%)
- win composition: TopOut 0, forfeit-vision 32, forfeit-other 0; losses: TopOut 3, forfeit 0; turn-cap games 5
- **non-forfeit games (40-row TopOut / TurnCap, n=8)**: 5 wins / 3 losses = **62.5%** (Wilson 95% CI 30.6%..86.3%)
- replay verified: **40/40**
- per sheet: A 18/20, B 19/20
- seat note: losses by sheet ['A', 'B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- This report does **not** claim a win rate against a full-vision opponent: all vision forfeit wins are reported separately and the non-forfeit (40-row) record is stated above.
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 97.0 ms, p95 98.7 ms, p99 99.5 ms, max 123.0 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 30.8 ms, p95 39.0 ms, p99 45.6 ms, max 174.3 ms
- placements over the 200 ms budget: intetrigence 0, attax 0
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00

## Attack and garbage

- attack/piece: intetrigence **0.966**, attax **0.872**
- received garbage per game: intetrigence **97.0**, attax **129.2**
- intetrigence search: 13,085,524 nodes total, 8 fallback placements
- `start` resends (attax): 1965; `cellsAboveRow20`: True

## Board height and survival

- mean turns 360.6; turn-cap games 5; TopOut games 3
- slot 0: max height 18.0, mean height 5.8, final 6.9, lines cleared/game 226.4
- slot 1: max height 19.6, mean height 6.0, final 16.9, lines cleared/game 246.2

## Scoring audit

- intetrigence: spin mismatches 0 / 14407, same-spin attack mismatches 0 / 14407
- attax: spin mismatches 200 / 14390, same-spin attack mismatches 0 / 14390

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 3

| seed | sheet | reason | turns | intetrigence recv | attax recv | intetrigence attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 10003 | A | TopOut | 267 | 90 | 81 | 0.861 | downstack (cleared but could not keep up with garbage) |
| 10005 | B | TopOut | 551 | 174 | 185 | 0.926 | downstack (cleared but could not keep up with garbage) |
| 10018 | A | TopOut | 821 | 245 | 287 | 0.977 | downstack (cleared but could not keep up with garbage) |

## Comparison with pre-v2 (same seeds, same protocol)

| metric | pre-v2 (m4-reserve) | v2 (m4-reserve-v2) |
|---|---|---|
| wins / losses / draws | 38 / 2 / 0 | 37 / 3 / 0 |
| win rate | 95.0% (Wilson 83.5..98.6) | 92.5% (Wilson 80.1..97.4) |
| forfeit-vision / turn-cap | 34 / 4 | 32 / 5 |
| TopOut losses | 2 | 3 |
| attack/piece | 0.977 | 0.966 |
| received garbage/game | 113.05 | 97.0 |
| think median / p95 | 97.1 / 98.7 ms | 97.0 / 98.7 ms |
| mean / max height | 5.71 / 17.18 | 5.81 / 17.98 |
| spin mismatches | 0 / 16486 | 0 / 14407 |
| same-spin attack mismatches | 0 / 16486 | 0 / 14407 |

- The v2 **reduces received garbage** (113.05 -> 97.0, -14%) but **does not
  improve the reserve win rate** (38-2 -> 37-3). The Wilson intervals overlap
  heavily; the point estimate is 2.5 points lower.
- Game-level flips: 10001-B **loss -> win** (v2 target), 10013-A **turn-cap ->
  forfeit** (a win either way); 10003-A **win -> loss**, 10018-A **win ->
  loss**; 10005-B **loss -> loss** (survived 389 -> 551 turns).

## Circularity note (pre-registered target seeds)

- 10001-B and 10005-B are exactly the two seeds the v2 change was designed
  against, so their outcome is **not independent evidence**: 10001-B flipping
  to a win is expected, and 10005-B still losing is a failed target.
- The main validation is the **other 38 games** (18 seeds x A/B minus the 2
  targets): pre-v2 38-0, v2 **36-2** (10003-A, 10018-A). The two new losses are
  on seeds the change was **not** aimed at, so they are the honest signal: v2
  trades some stack-management robustness for garbage cancellation.
- The turn-cap games (4 pre-v2, 5 v2) are wins in both runs; the change did not
  turn a turn-cap win into a loss.

## Loss analysis (v2)

| seed | sheet | turns | intetrigence attack | attax attack | recv (int/attax) | final height (int/attax) |
|---|---|---|---|---|---|---|
| 10003 | A | 267 | 230 | 240 | 90 / 81 | 24 / 4 |
| 10005 | B | 551 | 510 | 500 | 174 / 185 | 22 / 3 |
| 10018 | A | 821 | 802 | 762 | 245 / 287 | 23 / 5 |

- In all three losses intetrigence **out-attacked attax** (10003-A was close)
  and in two of three **received less garbage**, yet finished at height 22-24
  while attax finished at 3-5. The failure mode is **stack management /
  downstack speed**, not an attack deficit or a garbage-volume problem.
- 10005-B is the v2 target and still loses: the cancellation propagation helped
  it survive longer (389 -> 551 turns) but did not fix the underlying
  stack-growth pattern.

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- final-match seeds 20001+ were **not** used (M4 reserve v2 only).

## Reproduction

- `scripts/m4-reserve-v2.sh`
- `arena verify scripts/evidence/m4-reserve-v2`
- `python scripts/analyze-m2.py`
