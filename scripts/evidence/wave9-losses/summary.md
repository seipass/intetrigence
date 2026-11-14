# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 3 / 7 / 0 / 0
- win rate: 30.0% (Wilson 95% CI 10.8%..60.3%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 6, forfeit 0; turn-cap games 3
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 242 | yes | 97.2 / 33.7 | 99.2 / 48.6 | 1.00 / 1.00 | 1.00 / 0.97 | 23 |
| 9001 | B | attax | TopOut | - | 60 | yes | 97.1 / 36.7 | 99.0 / 45.3 | 1.00 / 1.00 | 1.00 / 1.00 | 2 |
| 9002 | A | attax | TopOut | - | 163 | yes | 97.5 / 37.4 | 99.3 / 45.3 | 1.00 / 1.00 | 1.00 / 0.99 | 19 |
| 9002 | B | attax | TopOut | - | 404 | yes | 97.1 / 39.5 | 99.1 / 48.6 | 1.00 / 1.00 | 1.00 / 0.99 | 44 |
| 9003 | A | attax | TopOut | - | 295 | yes | 97.3 / 37.3 | 99.3 / 46.1 | 1.00 / 1.00 | 1.00 / 0.98 | 29 |
| 9003 | B | intetrigence | TopOut | - | 193 | yes | 97.0 / 33.0 | 99.1 / 42.3 | 1.00 / 1.00 | 1.00 / 0.98 | 21 |
| 9004 | A | attax | TurnCap | - | 600 | yes | 97.4 / 37.7 | 99.3 / 46.0 | 1.00 / 1.00 | 1.00 / 0.97 | 76 |
| 9004 | B | attax | TopOut | - | 54 | yes | 97.3 / 35.5 | 99.8 / 44.8 | 1.00 / 1.00 | 1.00 / 0.96 | 4 |
| 9005 | A | intetrigence | TurnCap | - | 600 | yes | 97.5 / 39.0 | 99.6 / 50.3 | 1.00 / 1.00 | 1.00 / 0.99 | 70 |
| 9005 | B | intetrigence | TurnCap | - | 600 | yes | 97.3 / 37.2 | 99.1 / 45.8 | 1.00 / 1.00 | 1.00 / 0.99 | 73 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 3210 | 0 / 3210 | 0 / 3210 | 0.820 | 101.6 | 7.4 / 20.4 | 0 |
| 1 | Attax | 40 | 3208 | 57 / 3208 | 0 / 3208 | 0.845 | 95.5 | 5.5 / 16.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M2 gate: real intetrigence search (--ms 100, 8 threads) vs attax 7b38f51. Forfeit wins are broken down by cause in the report.
