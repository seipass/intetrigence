# M3 defense-stress baseline (16 start garbage rows, post-M3 bot)

- evidence: `scripts/evidence/m3-defense-16`
- profile: **defense-stress:16** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search, `--ms 100`, 8 threads, commit `343e36d`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+343e36d` / `docs/s2-referee-protocol.md@v1.9+343e36d`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **19 / 1 / 0**
- forfeits: attax **16** (20-row view), intetrigence **0**
- win rate: **95.0%** (Wilson 95% CI 76.4%..99.1%)
- win composition: TopOut 0, forfeit-vision 16, forfeit-other 0; losses: TopOut 1, forfeit 0; turn-cap games 3
- **non-forfeit games (40-row TopOut / TurnCap, n=4)**: 3 wins / 1 losses = **75.0%** (Wilson 95% CI 30.1%..95.4%)
- replay verified: **20/20**
- per sheet: A 10/10, B 9/10
- seat note: losses by sheet ['B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 97.1 ms, p95 99.0 ms, p99 99.6 ms, max 103.1 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 34.6 ms, p95 42.7 ms, p99 46.4 ms, max 56.3 ms
- placements over the 200 ms budget: intetrigence 0, attax 0
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00

## Attack and garbage

- attack/piece: intetrigence **0.972**, attax **0.873**
- received garbage per game: intetrigence **124.5**, attax **164.0**
- intetrigence search: 7,816,511 nodes total, 4 fallback placements
- `start` resends (attax): 1174; `cellsAboveRow20`: True

## Scoring audit

- intetrigence: spin mismatches 0 / 8565, same-spin attack mismatches 0 / 8565
- attax: spin mismatches 135 / 8557, same-spin attack mismatches 0 / 8557

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 1

| seed | sheet | reason | turns | bot recv | attax recv | bot attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9009 | B | TopOut | 48 | 41 | 22 | 0.479 | downstack (cleared but could not keep up with garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m2-match.sh` (tuning seeds 9001-9010, `--ms 100`, 8 threads)
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
