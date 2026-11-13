# S2 match summary (M0 gate)

- games: 10
- intetrigence wins / losses / draws / forfeits: 0 / 10 / 0 / 0
- win rate: 0.0% (Wilson 95% CI 0.0%..27.8%)
- replay verified: 10/10 (failures: 0)
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|
| 1001 | A | attax | TopOut | 51 | yes | 1.0 / 28.3 | 1.2 / 35.4 | 1.00 / 1.00 | 1.00 / 0.96 | 1 |
| 1001 | B | attax | TopOut | 51 | yes | 0.9 / 29.0 | 1.1 / 36.6 | 1.00 / 1.00 | 1.00 / 0.96 | 1 |
| 1002 | A | attax | TopOut | 58 | yes | 0.8 / 27.2 | 1.2 / 35.3 | 1.00 / 1.00 | 1.00 / 1.00 | 3 |
| 1002 | B | attax | TopOut | 58 | yes | 0.8 / 28.7 | 1.2 / 34.5 | 1.00 / 1.00 | 1.00 / 1.00 | 3 |
| 1003 | A | attax | TopOut | 48 | yes | 0.7 / 29.8 | 1.2 / 33.7 | 1.00 / 1.00 | 1.00 / 0.98 | 2 |
| 1003 | B | attax | TopOut | 48 | yes | 0.8 / 31.0 | 1.4 / 38.9 | 1.00 / 1.00 | 1.00 / 0.98 | 2 |
| 1004 | A | attax | TopOut | 104 | yes | 0.8 / 27.7 | 1.2 / 38.8 | 1.00 / 1.00 | 1.00 / 0.97 | 9 |
| 1004 | B | attax | TopOut | 94 | yes | 0.9 / 28.6 | 1.3 / 40.3 | 1.00 / 1.00 | 1.00 / 0.99 | 7 |
| 1005 | A | attax | TopOut | 131 | yes | 0.4 / 29.3 | 1.1 / 39.5 | 1.00 / 1.00 | 1.00 / 0.98 | 7 |
| 1005 | B | attax | TopOut | 92 | yes | 0.6 / 27.9 | 1.3 / 35.4 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |

## Per-bot audit

| slot | bot | placements | spin mismatch | attack mismatch (same spin) | think > 200ms |
|---|---|---|---|---|---|
| 0 | intetrigence-baseline | 735 | 0 / 735 | 0 / 735 | 0 |
| 1 | Attax | 730 | 12 / 730 | 0 / 730 | 0 |

## Notes

- M0 gate: plumbing / audit / replay proof. The intetrigence side is the in-process M0 baseline stand-in, not the real bot (crates/bot is still a skeleton).
- attax at this commit is guideline-scored and has a 20-row vision limit, so the win/loss numbers are reference only.
