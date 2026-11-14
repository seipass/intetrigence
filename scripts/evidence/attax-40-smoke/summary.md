# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 1 / 9 / 0 / 0
- win rate: 10.0% (Wilson 95% CI 1.8%..40.4%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 0, forfeit-other 0; losses: TopOut 9, forfeit 0; turn-cap games 1
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 11001 | A | attax | TopOut | - | 92 | yes | 97.4 / 30.8 | 99.1 / 37.6 | 1.00 / 1.00 | 1.00 / 0.96 | 11 |
| 11001 | B | attax | TopOut | - | 126 | yes | 97.2 / 30.5 | 98.9 / 39.7 | 1.00 / 1.00 | 1.00 / 0.98 | 11 |
| 11002 | A | attax | TopOut | - | 600 | yes | 97.2 / 30.9 | 98.9 / 38.7 | 1.00 / 1.00 | 1.00 / 0.99 | 66 |
| 11002 | B | attax | TopOut | - | 72 | yes | 97.1 / 29.8 | 98.9 / 37.9 | 1.00 / 1.00 | 1.00 / 1.00 | 9 |
| 11003 | A | attax | TopOut | - | 576 | yes | 97.2 / 30.4 | 98.9 / 41.7 | 1.00 / 1.00 | 1.00 / 0.99 | 60 |
| 11003 | B | attax | TopOut | - | 115 | yes | 97.0 / 28.1 | 98.8 / 34.0 | 1.00 / 1.00 | 1.00 / 0.97 | 10 |
| 11004 | A | attax | TopOut | - | 572 | yes | 97.4 / 28.9 | 98.9 / 36.7 | 1.00 / 1.00 | 1.00 / 0.97 | 68 |
| 11004 | B | intetrigence | TurnCap | - | 1000 | yes | 97.2 / 28.9 | 98.8 / 35.8 | 1.00 / 1.00 | 1.00 / 0.98 | 121 |
| 11005 | A | attax | TopOut | - | 562 | yes | 97.4 / 28.9 | 98.9 / 37.8 | 1.00 / 1.00 | 1.00 / 0.98 | 68 |
| 11005 | B | attax | TopOut | - | 160 | yes | 97.4 / 38.5 | 99.4 / 48.2 | 1.00 / 1.00 | 1.00 / 0.99 | 18 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 3875 | 0 / 3875 | 0 / 3875 | 0.855 | 118.6 | 8.2 / 21.2 | 0 |
| 1 | Attax | 40 | 3870 | 67 / 3870 | 0 / 3870 | 0.876 | 113.3 | 5.5 / 15.4 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- Patched attax (40 rows + posY, SHA-256 e22aab7b...); seeds 11001-11005 x A/B.
