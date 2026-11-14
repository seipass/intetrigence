# M3 defense-stress baseline (16 start garbage rows)

- evidence: `scripts/evidence/m3-defense-16`
- profile: **defense-stress:16** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search, `--ms 100`, 8 threads, commit `c1535af`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+c1535af` / `docs/s2-referee-protocol.md@v1.9+c1535af`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **18 / 2 / 0**
- forfeits: attax **18** (20-row view), intetrigence **0**
- win rate: **90.0%** (Wilson 95% CI 69.9%..97.2%)
- win composition: TopOut 0, forfeit-vision 18, forfeit-other 0; losses: TopOut 2, forfeit 0; turn-cap games 0
- **non-forfeit games (40-row TopOut / TurnCap, n=2)**: 0 wins / 2 losses = **0.0%** (Wilson 95% CI 0.0%..65.8%)
- replay verified: **20/20**
- per sheet: A 10/10, B 8/10
- seat note: losses by sheet ['B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 96.9 ms, p95 98.4 ms, p99 99.0 ms, max 106.8 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 29.7 ms, p95 37.2 ms, p99 41.0 ms, max 70.1 ms
- placements over the 200 ms budget: intetrigence 0, attax 0
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00

## Attack and garbage

- attack/piece: intetrigence **1.003**, attax **0.895**
- received garbage per game: intetrigence **118.2**, attax **160.8**
- intetrigence search: 7,348,786 nodes total, 2 fallback placements
- `start` resends (attax): 1056; `cellsAboveRow20`: True

## Scoring audit

- intetrigence: spin mismatches 0 / 8011, same-spin attack mismatches 0 / 8011
- attax: spin mismatches 117 / 8001, same-spin attack mismatches 0 / 8001

## Loss classification (coarse)

- buried (no clears in the last 8 turns, garbage >= 15): 1
- downstack (cleared but could not keep up with garbage): 1

| seed | sheet | reason | turns | bot recv | attax recv | bot attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9006 | B | TopOut | 892 | 235 | 338 | 1.022 | buried (no clears in the last 8 turns, garbage >= 15) |
| 9009 | B | TopOut | 569 | 166 | 173 | 0.921 | downstack (cleared but could not keep up with garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m2-match.sh` (tuning seeds 9001-9010, `--ms 100`, 8 threads)
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
