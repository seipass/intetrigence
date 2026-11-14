# M4 reserve validation (seeds 10001-10020)

- evidence: `scripts/evidence/m4-reserve`
- profile: **official** (empty start board; the only profile usable for final claims)
- games: 40 (reserve seeds 10001-10020 x sheets A/B)
- final-match seeds 20001+ were **not** used (M4 reserve only)
- intetrigence: real search (`--ms 100 --threads 8`), commit `a7cab1e`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+a7cab1e` / `docs/s2-referee-protocol.md@v1.9+a7cab1e`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **38 / 2 / 0**
- forfeits: attax **34** (20-row view), intetrigence **0**
- win rate: **95.0%** (Wilson 95% CI 83.5%..98.6%)
- win composition: TopOut 0, forfeit-vision 34, forfeit-other 0; losses: TopOut 2, forfeit 0; turn-cap games 4
- **non-forfeit games (40-row TopOut / TurnCap, n=6)**: 4 wins / 2 losses = **66.7%** (Wilson 95% CI 30.0%..90.3%)
- replay verified: **40/40**
- per sheet: A 20/20, B 18/20
- seat note: losses by sheet ['B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- This report does **not** claim a win rate against a full-vision opponent: all vision forfeit wins are reported separately and the non-forfeit (40-row) record is stated above.
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 97.1 ms, p95 98.7 ms, p99 99.3 ms, max 109.5 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 32.9 ms, p95 41.8 ms, p99 45.7 ms, max 147.6 ms
- placements over the 200 ms budget: intetrigence 0, attax 0
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00

## Attack and garbage

- attack/piece: intetrigence **0.977**, attax **0.885**
- received garbage per game: intetrigence **113.0**, attax **149.0**
- intetrigence search: 15,015,976 nodes total, 3 fallback placements
- `start` resends (attax): 2233; `cellsAboveRow20`: True

## Board height and survival

- mean turns 412.6; turn-cap games 4; TopOut games 2
- slot 0: max height 17.2, mean height 5.7, final 5.8, lines cleared/game 262.7
- slot 1: max height 20.1, mean height 6.0, final 17.0, lines cleared/game 284.5

## Scoring audit

- intetrigence: spin mismatches 0 / 16486, same-spin attack mismatches 0 / 16486
- attax: spin mismatches 241 / 16468, same-spin attack mismatches 0 / 16468

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 2

| seed | sheet | reason | turns | intetrigence recv | attax recv | intetrigence attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 10001 | B | TopOut | 753 | 206 | 260 | 0.942 | downstack (cleared but could not keep up with garbage) |
| 10005 | B | TopOut | 389 | 136 | 138 | 0.974 | downstack (cleared but could not keep up with garbage) |

## Consistency with the M3 tuning run

| metric | M3 tuning (9001-9010) | M4 reserve (10001-10020) |
|---|---|---|
| games | 20 | 40 |
| wins / losses / draws | 20 / 0 / 0 | 38 / 2 / 0 |
| win rate | 100.0% (Wilson 83.9..100.0) | 95.0% (Wilson 83.5..98.6) |
| win breakdown | TopOut 0 / vision 19 / other 0 / turn-cap 1 | TopOut 0 / vision 34 / other 0 / turn-cap 4 |
| attack/piece | 0.962 | 0.977 |
| received garbage/game | 109.3 | 113.0 |
| think median / p95 | 97.1 / 99.0 ms | 97.1 / 98.7 ms |
| mean turns | 395.1 | 412.6 |
| spin mismatches | 0 / 7892 | 0 / 16486 |
| same-spin attack mismatches | 0 / 7892 | 0 / 16486 |

- The reserve metrics are **consistent with tuning**: attack/piece 0.977 vs
  0.962, received 113.0 vs 109.3, think 97.1/98.7 vs 97.1/99.0, mean turns
  412.6 vs 395.1, and the scoring audit is 0/0 on both.
- The reserve exposes **2 non-forfeit losses** (tuning had 0). Both are 40-row
  TopOuts with the M3 pattern: the bot kept clearing lines but the stack kept
  growing, and the final height was 23 / 26. **No new loss shape.**
- 10001-B is the clearest: the bot out-attacked attax (709 vs 657 total attack)
  but still topped out, so the loss is a defensive/downstack failure, not an
  attack deficit.
- The gate (≥85%) is passed at 95.0%; the non-forfeit record (4/6, Wilson
  30.0..90.3) is the small-sample caveat.

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- final-match seeds 20001+ were **not** used (M4 reserve only).

## Reproduction

- `scripts/m4-reserve.sh`
- `arena verify scripts/evidence/m4-reserve`
- `python scripts/analyze-m2.py`
