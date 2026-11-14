# M3 defense-stress baseline (12 start garbage rows)

- evidence: `scripts/evidence/m3-defense`
- profile: **defense-stress:12** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search, `--ms 100`, 8 threads, commit `c1535af`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+c1535af` / `docs/s2-referee-protocol.md@v1.9+c1535af`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **20 / 0 / 0**
- forfeits: attax **20** (20-row view), intetrigence **0**
- win rate: **100.0%** (Wilson 95% CI 83.9%..100.0%)
- win composition: TopOut 0, forfeit-vision 20, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 0
- **non-forfeit games (40-row TopOut / TurnCap, n=0)**: 0 wins / 0 losses = **0.0%** (Wilson 95% CI 0.0%..100.0%)
- replay verified: **20/20**
- per sheet: A 10/10, B 10/10
- seat note: losses by sheet []; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 97.2 ms, p95 99.0 ms, p99 99.7 ms, max 103.2 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 35.8 ms, p95 44.4 ms, p99 48.1 ms, max 55.4 ms
- placements over the 200 ms budget: intetrigence 0, attax 0
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00

## Attack and garbage

- attack/piece: intetrigence **1.002**, attax **0.871**
- received garbage per game: intetrigence **76.2**, attax **108.7**
- intetrigence search: 4,717,697 nodes total, 0 fallback placements
- `start` resends (attax): 688; `cellsAboveRow20`: True

## Scoring audit

- intetrigence: spin mismatches 0 / 5177, same-spin attack mismatches 0 / 5177
- attax: spin mismatches 85 / 5167, same-spin attack mismatches 0 / 5167

## Loss classification (coarse)


| seed | sheet | reason | turns | bot recv | attax recv | bot attack/piece | classification |
|---|---|---|---|---|---|---|---|

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m2-match.sh` (tuning seeds 9001-9010, `--ms 100`, 8 threads)
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
