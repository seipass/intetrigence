# M2 tuning match report (intetrigence vs attax)

- evidence: `scripts/evidence/m2-tuning`
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- intetrigence: real search, `--ms 100`, 8 threads, commit `f1c2e79`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.9+f1c2e79` / `docs/s2-referee-protocol.md@v1.7+f1c2e79`

## Result

- wins / losses / draws: **18 / 2 / 0**
- forfeits: attax **18** (20-row view), intetrigence **0**
- win rate: **90.0%** (Wilson 95% CI 69.9%..97.2%)
- win composition: TopOut 0, TurnCap 0, attax forfeit 18 (all: attax's 20-row view cannot place the piece; 9 of them also had cells above row 19)
- **40-row top-out / turn-cap games only** (n=2): 0/2 = **0.0%** (Wilson 95% CI 0.0%..65.8%)
- replay verified: **20/20**
- per sheet: A 8/10, B 10/10
- seat note: the 2 losses are both sheet A (intetrigence in seat 1); sheet B is 10/10. The sample is too small to separate a seat effect from the opponent's forfeits.

## Think time

- intetrigence: median 97.1 ms, p95 (mean over games) 98.7 ms
- attax: median 29.1 ms, p95 (mean over games) 35.9 ms
- placements over the 200 ms budget: intetrigence 0, attax 0

## Attack and garbage

- attack/piece: intetrigence **0.986**, attax **0.866**
- received garbage per game: intetrigence **75.7**, attax **110.0**
- intetrigence search: 5,424,346 nodes total, 3 fallback placements
- `start` resends (attax): 797; `cellsAboveRow20`: True

## Scoring audit

- intetrigence: spin mismatches 0 / 5934, same-spin attack mismatches 0 / 5934
- attax: spin mismatches 88 / 5924, same-spin attack mismatches 0 / 5924

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 1
- buried (no clears in the last 8 turns, garbage >= 15): 1

| seed | sheet | reason | turns | bot recv | attax recv | bot attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 9006 | A | TopOut | 111 | 46 | 50 | 0.955 | downstack (cleared but could not keep up with garbage) |
| 9010 | A | TopOut | 188 | 73 | 73 | 1.011 | buried (no clears in the last 8 turns, garbage >= 15) |

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- Tuning seeds only. Held-out seeds (10001+) were not used.
