# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 18 / 2 / 0 / 18
- win rate: 90.0% (Wilson 95% CI 69.9%..97.2%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 18, forfeit-other 0; losses: TopOut 2, forfeit 0; turn-cap games 0
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | vision | 200 | yes | 97.2 / 37.8 | 98.9 / 47.5 | 1.00 / 1.00 | 1.00 / 0.99 | 34 |
| 9001 | B | intetrigence | Forfeit | vision | 808 | yes | 97.2 / 36.1 | 99.1 / 46.6 | 1.00 / 1.00 | 1.00 / 0.98 | 120 |
| 9002 | A | intetrigence | Forfeit | vision | 273 | yes | 96.9 / 36.9 | 98.9 / 46.4 | 1.00 / 1.00 | 1.00 / 0.98 | 33 |
| 9002 | B | intetrigence | Forfeit | vision | 113 | yes | 97.2 / 38.7 | 98.7 / 49.2 | 1.00 / 1.00 | 1.00 / 0.98 | 14 |
| 9003 | A | intetrigence | Forfeit | vision | 325 | yes | 96.9 / 36.7 | 98.9 / 45.3 | 1.00 / 1.00 | 1.00 / 0.99 | 51 |
| 9003 | B | intetrigence | Forfeit | vision | 270 | yes | 96.9 / 38.6 | 98.7 / 48.5 | 1.00 / 1.00 | 1.00 / 0.99 | 31 |
| 9004 | A | intetrigence | Forfeit | vision | 247 | yes | 97.1 / 36.5 | 99.0 / 44.5 | 1.00 / 1.00 | 1.00 / 0.99 | 31 |
| 9004 | B | intetrigence | Forfeit | vision | 120 | yes | 97.4 / 36.9 | 98.9 / 46.8 | 1.00 / 1.00 | 1.00 / 0.98 | 18 |
| 9005 | A | intetrigence | Forfeit | vision | 103 | yes | 97.2 / 36.1 | 99.1 / 44.3 | 1.00 / 1.00 | 1.00 / 0.99 | 14 |
| 9005 | B | intetrigence | Forfeit | vision | 265 | yes | 97.2 / 36.8 | 98.9 / 46.0 | 1.00 / 1.00 | 1.00 / 0.99 | 36 |
| 9006 | A | attax | TopOut | - | 111 | yes | 97.3 / 35.3 | 99.1 / 48.0 | 1.00 / 1.00 | 1.00 / 0.98 | 18 |
| 9006 | B | intetrigence | Forfeit | vision | 510 | yes | 97.0 / 37.8 | 98.9 / 46.7 | 1.00 / 1.00 | 1.00 / 0.98 | 75 |
| 9007 | A | intetrigence | Forfeit | vision | 213 | yes | 97.1 / 38.4 | 98.7 / 49.2 | 1.00 / 1.00 | 1.00 / 0.99 | 27 |
| 9007 | B | intetrigence | Forfeit | vision | 195 | yes | 97.1 / 37.9 | 98.9 / 47.9 | 1.00 / 1.00 | 1.00 / 0.99 | 29 |
| 9008 | A | intetrigence | Forfeit | vision | 243 | yes | 97.0 / 36.5 | 98.8 / 46.3 | 1.00 / 1.00 | 1.00 / 0.99 | 34 |
| 9008 | B | intetrigence | Forfeit | vision | 577 | yes | 97.0 / 38.2 | 98.9 / 48.0 | 1.00 / 1.00 | 1.00 / 0.98 | 75 |
| 9009 | A | intetrigence | Forfeit | vision | 198 | yes | 97.4 / 36.5 | 99.0 / 46.8 | 1.00 / 1.00 | 1.00 / 1.00 | 22 |
| 9009 | B | intetrigence | Forfeit | vision | 443 | yes | 97.0 / 35.6 | 98.7 / 44.7 | 1.00 / 1.00 | 1.00 / 0.98 | 56 |
| 9010 | A | attax | TopOut | - | 188 | yes | 97.0 / 36.6 | 99.0 / 45.6 | 1.00 / 1.00 | 1.00 / 0.98 | 21 |
| 9010 | B | intetrigence | Forfeit | vision | 142 | yes | 97.0 / 35.8 | 99.0 / 45.3 | 1.00 / 1.00 | 1.00 / 0.98 | 14 |

## Per-bot audit

| slot | bot | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 5534 | 0 / 5534 | 0 / 5534 | 1.000 | 72.0 | 0 |
| 1 | Attax | 5524 | 78 / 5524 | 0 / 5524 | 0.876 | 105.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
