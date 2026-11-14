# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 18 / 2 / 0 / 18
- win rate: 90.0% (Wilson 95% CI 69.9%..97.2%)
- replay verified: 20/20 (failures: 0)
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | 200 | yes | 96.9 / 27.7 | 98.5 / 36.5 | 1.00 / 1.00 | 1.00 / 0.99 | 34 |
| 9001 | B | intetrigence | Forfeit | 808 | yes | 96.9 / 29.4 | 98.4 / 37.1 | 1.00 / 1.00 | 1.00 / 0.98 | 120 |
| 9002 | A | intetrigence | Forfeit | 273 | yes | 97.0 / 27.9 | 98.4 / 33.8 | 1.00 / 1.00 | 1.00 / 0.98 | 33 |
| 9002 | B | intetrigence | Forfeit | 113 | yes | 97.1 / 28.2 | 98.5 / 36.0 | 1.00 / 1.00 | 1.00 / 0.98 | 14 |
| 9003 | A | intetrigence | Forfeit | 40 | yes | 97.2 / 30.6 | 98.7 / 36.5 | 1.00 / 1.00 | 1.00 / 1.00 | 3 |
| 9003 | B | intetrigence | Forfeit | 270 | yes | 97.2 / 29.7 | 99.5 / 38.7 | 1.00 / 1.00 | 1.00 / 0.99 | 31 |
| 9004 | A | intetrigence | Forfeit | 247 | yes | 97.2 / 29.2 | 98.8 / 35.0 | 1.00 / 1.00 | 1.00 / 0.99 | 31 |
| 9004 | B | intetrigence | Forfeit | 120 | yes | 97.2 / 28.5 | 98.5 / 34.4 | 1.00 / 1.00 | 1.00 / 0.98 | 18 |
| 9005 | A | intetrigence | Forfeit | 103 | yes | 97.5 / 29.1 | 98.9 / 35.2 | 1.00 / 1.00 | 1.00 / 0.99 | 14 |
| 9005 | B | intetrigence | Forfeit | 606 | yes | 97.2 / 28.2 | 98.7 / 33.9 | 1.00 / 1.00 | 1.00 / 0.98 | 80 |
| 9006 | A | attax | TopOut | 111 | yes | 97.1 / 28.4 | 98.7 / 36.1 | 1.00 / 1.00 | 1.00 / 0.98 | 18 |
| 9006 | B | intetrigence | Forfeit | 510 | yes | 96.9 / 28.2 | 98.6 / 35.0 | 1.00 / 1.00 | 1.00 / 0.98 | 75 |
| 9007 | A | intetrigence | Forfeit | 213 | yes | 97.0 / 30.0 | 98.5 / 36.9 | 1.00 / 1.00 | 1.00 / 0.99 | 27 |
| 9007 | B | intetrigence | Forfeit | 195 | yes | 96.9 / 29.8 | 98.6 / 36.4 | 1.00 / 1.00 | 1.00 / 0.99 | 29 |
| 9008 | A | intetrigence | Forfeit | 243 | yes | 96.9 / 29.8 | 98.6 / 36.8 | 1.00 / 1.00 | 1.00 / 0.99 | 34 |
| 9008 | B | intetrigence | Forfeit | 921 | yes | 97.2 / 28.2 | 98.7 / 34.2 | 1.00 / 1.00 | 1.00 / 0.98 | 123 |
| 9009 | A | intetrigence | Forfeit | 198 | yes | 97.1 / 30.2 | 98.7 / 36.9 | 1.00 / 1.00 | 1.00 / 1.00 | 22 |
| 9009 | B | intetrigence | Forfeit | 443 | yes | 97.3 / 28.8 | 98.7 / 35.4 | 1.00 / 1.00 | 1.00 / 0.98 | 56 |
| 9010 | A | attax | TopOut | 188 | yes | 97.3 / 29.4 | 98.7 / 36.2 | 1.00 / 1.00 | 1.00 / 0.98 | 21 |
| 9010 | B | intetrigence | Forfeit | 142 | yes | 97.3 / 31.7 | 98.7 / 37.0 | 1.00 / 1.00 | 1.00 / 0.98 | 14 |

## Per-bot audit

| slot | bot | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 5934 | 0 / 5934 | 0 / 5934 | 0.986 | 75.7 | 0 |
| 1 | Attax | 5924 | 88 / 5924 | 0 / 5924 | 0.866 | 110.0 | 0 |

## Notes

- M0 gate: plumbing / audit / replay proof. The intetrigence side is the in-process M0 baseline stand-in, not the real bot (crates/bot is still a skeleton).
- attax at this commit is guideline-scored and has a 20-row vision limit, so the win/loss numbers are reference only.
