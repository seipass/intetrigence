# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 7 / 3 / 0 / 1
- win rate: 70.0% (Wilson 95% CI 39.7%..89.2%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 6, forfeit-vision 0, forfeit-other 1; losses: TopOut 3, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 47 | yes | 103.5 / 100.1 | 113.5 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 7 |
| 9001 | B | intetrigence | TopOut | - | 153 | yes | 103.9 / 100.1 | 111.0 / 100.2 | 1.00 / 1.00 | 1.00 / 0.96 | 24 |
| 9002 | A | intetrigence | TopOut | - | 130 | yes | 105.4 / 100.1 | 115.6 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 19 |
| 9002 | B | intetrigence | TopOut | - | 48 | yes | 107.2 / 100.1 | 112.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 8 |
| 9003 | A | intetrigence | TopOut | - | 93 | yes | 107.3 / 100.1 | 113.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 9 |
| 9003 | B | attax | TopOut | - | 64 | yes | 105.8 / 100.1 | 111.2 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9004 | A | intetrigence | Forfeit | other | 131 | yes | 102.9 / 100.1 | 110.9 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 13 |
| 9004 | B | attax | TopOut | - | 121 | yes | 105.3 / 100.1 | 113.5 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 10 |
| 9005 | A | intetrigence | TopOut | - | 47 | yes | 106.3 / 100.1 | 113.3 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 6 |
| 9005 | B | attax | TopOut | - | 353 | yes | 106.4 / 100.1 | 113.6 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 49 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1185 | 0 / 1185 | 0 / 1185 | 0.741 | 34.2 | 5.8 / 15.7 | 1 |
| 1 | Cold Clear 2 | 40 | 1186 | 20 / 1186 | 0 / 1186 | 0.745 | 33.1 | 10.1 / 19.4 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- CC2 weights attack=28
