# M4 iter2 defense-stress (16 start rows, cancel propagation)

- evidence: `scripts/evidence/m4-iter2-defense-16`
- profile: **defense-stress:16** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- held-out: seeds 10001+ were **not** used (tuning seeds only)
- intetrigence: real search (`--ms 100 --threads 8`), commit `9a5321f`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+9a5321f` / `docs/s2-referee-protocol.md@v1.9+9a5321f`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **19 / 1 / 0**
- forfeits: attax **17** (20-row view), intetrigence **0**
- win rate: **95.0%** (Wilson 95% CI 76.4%..99.1%)
- win composition: TopOut 0, forfeit-vision 17, forfeit-other 0; losses: TopOut 1, forfeit 0; turn-cap games 2
- **non-forfeit games (40-row TopOut / TurnCap, n=3)**: 2 wins / 1 losses = **66.7%** (Wilson 95% CI 20.8%..93.9%)
- replay verified: **20/20**
- per sheet: A 10/10, B 9/10
- seat note: losses by sheet ['B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- **Diagnostic profile**: these numbers are not usable for final claims (protocol v1.11 §1.3).
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 96.8 ms, p95 98.2 ms, p99 98.7 ms, max 100.1 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 27.1 ms, p95 33.7 ms, p99 38.2 ms, max 50.2 ms
- placements over the 200 ms budget: intetrigence 0, attax 0
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00

## Attack and garbage

- attack/piece: intetrigence **0.972**, attax **0.876**
- received garbage per game: intetrigence **111.0**, attax **144.4**
- intetrigence search: 6,809,195 nodes total, 1 fallback placements
- `start` resends (attax): 1008; `cellsAboveRow20`: True

## Board height and survival

- mean turns 371.4; turn-cap games 2; TopOut games 1
- slot 0: max height 19.0, mean height 7.3, final 5.5, lines cleared/game 244.8
- slot 1: max height 20.5, mean height 7.9, final 17.9, lines cleared/game 263.4

## Scoring audit

- intetrigence: spin mismatches 0 / 7422, same-spin attack mismatches 0 / 7422
- attax: spin mismatches 108 / 7412, same-spin attack mismatches 0 / 7412

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 1

| seed | sheet | reason | turns | intetrigence recv | attax recv | intetrigence attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9009 | B | TopOut | 48 | 37 | 22 | 0.646 | downstack (cleared but could not keep up with garbage) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.

## Reproduction

- `scripts/m3-defense.sh`
- `arena verify scripts/evidence/m2-tuning`
- `python scripts/analyze-m2.py`
