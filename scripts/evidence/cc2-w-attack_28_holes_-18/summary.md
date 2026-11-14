# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 6 / 4 / 0 / 0
- win rate: 60.0% (Wilson 95% CI 31.3%..83.2%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 6, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 46 | yes | 106.1 / 100.1 | 112.0 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 6 |
| 9001 | B | intetrigence | TopOut | - | 65 | yes | 107.3 / 100.1 | 111.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.94 | 11 |
| 9002 | A | intetrigence | TopOut | - | 129 | yes | 106.0 / 100.1 | 112.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 24 |
| 9002 | B | intetrigence | TopOut | - | 52 | yes | 104.2 / 100.1 | 111.7 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9003 | A | attax | TopOut | - | 61 | yes | 104.0 / 100.1 | 111.2 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 3 |
| 9003 | B | intetrigence | TopOut | - | 268 | yes | 101.8 / 100.1 | 110.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 33 |
| 9004 | A | attax | TopOut | - | 67 | yes | 106.1 / 100.1 | 111.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 6 |
| 9004 | B | attax | TopOut | - | 255 | yes | 103.4 / 100.1 | 111.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 24 |
| 9005 | A | attax | TopOut | - | 94 | yes | 105.6 / 100.1 | 111.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 7 |
| 9005 | B | intetrigence | TopOut | - | 52 | yes | 106.4 / 100.1 | 112.3 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 9 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1085 | 0 / 1085 | 0 / 1085 | 0.772 | 32.9 | 6.4 / 14.9 | 0 |
| 1 | Cold Clear 2 | 40 | 1086 | 18 / 1086 | 0 / 1086 | 0.773 | 32.6 | 9.7 / 17.9 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- CC2 weights attack=28,holes=-18
