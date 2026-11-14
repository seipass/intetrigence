# S2 match summary

- games: 20
- intetrigence wins / losses / unresolved / no-contest: 2 / 16 / 0 / 2
- win rate: 10.0% (Wilson 95% CI 2.8%..30.1%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 2, forfeit-vision 0, forfeit-other 0; losses: TopOut 16, forfeit 0; turn-cap games 0
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | draw | Forfeit | other | 243 | yes | 97.5 / 34.0 | 99.3 / 45.7 | 1.00 / 1.00 | 1.00 / 0.98 | 26 |
| 9001 | B | attax | TopOut | - | 60 | yes | 97.5 / 33.8 | 98.9 / 45.9 | 1.00 / 1.00 | 1.00 / 1.00 | 2 |
| 9002 | A | attax | TopOut | - | 163 | yes | 97.4 / 33.7 | 99.0 / 48.1 | 1.00 / 1.00 | 1.00 / 0.99 | 19 |
| 9002 | B | attax | TopOut | - | 404 | yes | 97.2 / 32.9 | 99.1 / 46.1 | 1.00 / 1.00 | 1.00 / 0.99 | 44 |
| 9003 | A | attax | TopOut | - | 295 | yes | 97.3 / 32.2 | 99.3 / 45.5 | 1.00 / 1.00 | 1.00 / 0.98 | 29 |
| 9003 | B | intetrigence | TopOut | - | 193 | yes | 97.1 / 30.3 | 98.6 / 38.2 | 1.00 / 1.00 | 1.00 / 0.98 | 21 |
| 9004 | A | attax | TopOut | - | 269 | yes | 97.3 / 32.2 | 99.0 / 44.4 | 1.00 / 1.00 | 1.00 / 0.98 | 35 |
| 9004 | B | attax | TopOut | - | 133 | yes | 97.4 / 35.1 | 99.5 / 47.0 | 1.00 / 1.00 | 1.00 / 1.00 | 11 |
| 9005 | A | attax | TopOut | - | 401 | yes | 97.3 / 32.5 | 99.0 / 41.3 | 1.00 / 1.00 | 1.00 / 0.98 | 40 |
| 9005 | B | attax | TopOut | - | 246 | yes | 97.3 / 35.9 | 99.2 / 55.8 | 1.00 / 1.00 | 1.00 / 0.98 | 24 |
| 9006 | A | attax | TopOut | - | 403 | yes | 97.2 / 29.5 | 98.8 / 39.7 | 1.00 / 1.00 | 1.00 / 0.99 | 46 |
| 9006 | B | attax | TopOut | - | 531 | yes | 97.2 / 30.6 | 99.0 / 43.1 | 1.00 / 1.00 | 1.00 / 0.98 | 68 |
| 9007 | A | attax | TopOut | - | 800 | yes | 97.3 / 35.0 | 99.3 / 45.2 | 1.00 / 1.00 | 1.00 / 0.98 | 98 |
| 9007 | B | attax | TopOut | - | 217 | yes | 97.2 / 38.4 | 99.3 / 46.1 | 1.00 / 1.00 | 1.00 / 0.99 | 20 |
| 9008 | A | attax | TopOut | - | 143 | yes | 97.6 / 38.5 | 99.5 / 46.8 | 1.00 / 1.00 | 1.00 / 0.99 | 11 |
| 9008 | B | attax | TopOut | - | 53 | yes | 97.2 / 36.2 | 99.6 / 45.0 | 1.00 / 1.00 | 1.00 / 0.98 | 2 |
| 9009 | A | intetrigence | TopOut | - | 64 | yes | 97.5 / 37.3 | 99.7 / 47.6 | 1.00 / 1.00 | 1.00 / 0.97 | 7 |
| 9009 | B | attax | TopOut | - | 239 | yes | 97.4 / 34.4 | 99.0 / 45.5 | 1.00 / 1.00 | 1.00 / 0.96 | 28 |
| 9010 | A | attax | TopOut | - | 537 | yes | 97.2 / 30.0 | 98.9 / 38.9 | 1.00 / 1.00 | 1.00 / 0.99 | 63 |
| 9010 | B | draw | Forfeit | other | 459 | yes | 97.1 / 30.0 | 99.0 / 39.4 | 1.00 / 1.00 | 1.00 / 0.97 | 50 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 5851 | 0 / 5851 | 0 / 5851 | 0.814 | 91.2 | 7.0 / 20.2 | 0 |
| 1 | Attax | 40 | 5843 | 106 / 5843 | 0 / 5843 | 0.844 | 83.2 | 5.3 / 16.4 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- Patched attax (40 rows + posY, SHA-256 e22aab7b...); tuning seeds 9001-9010 x A/B.
