# S2 match summary

- games: 20
- intetrigence wins / losses / unresolved / no-contest: 19 / 1 / 0 / 0
- win rate: 95.0% (Wilson 95% CI 76.4%..99.1%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 19, forfeit-vision 0, forfeit-other 0; losses: TopOut 1, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 363 | yes | 78.0 / 100.1 | 89.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 48 |
| 9001 | B | intetrigence | TopOut | - | 158 | yes | 75.1 / 100.1 | 84.5 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 13 |
| 9002 | A | intetrigence | TopOut | - | 242 | yes | 76.6 / 100.1 | 97.3 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 24 |
| 9002 | B | intetrigence | TopOut | - | 73 | yes | 83.3 / 100.1 | 98.7 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 8 |
| 9003 | A | intetrigence | TopOut | - | 293 | yes | 78.3 / 100.1 | 97.1 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 37 |
| 9003 | B | attax | TopOut | - | 99 | yes | 74.5 / 100.1 | 82.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 8 |
| 9004 | A | intetrigence | TopOut | - | 237 | yes | 75.2 / 100.1 | 97.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 28 |
| 9004 | B | intetrigence | TopOut | - | 215 | yes | 82.7 / 100.1 | 98.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 32 |
| 9005 | A | intetrigence | TopOut | - | 132 | yes | 80.5 / 100.1 | 98.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 17 |
| 9005 | B | intetrigence | TopOut | - | 233 | yes | 83.5 / 100.1 | 98.4 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 28 |
| 9006 | A | intetrigence | TopOut | - | 184 | yes | 74.9 / 100.1 | 83.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 26 |
| 9006 | B | intetrigence | TopOut | - | 35 | yes | 68.7 / 100.1 | 80.1 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 7 |
| 9007 | A | intetrigence | TopOut | - | 292 | yes | 74.0 / 100.1 | 83.6 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 30 |
| 9007 | B | intetrigence | TopOut | - | 143 | yes | 73.0 / 100.1 | 78.9 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 15 |
| 9008 | A | intetrigence | TopOut | - | 159 | yes | 76.1 / 100.1 | 92.9 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 15 |
| 9008 | B | intetrigence | TopOut | - | 41 | yes | 79.5 / 100.1 | 89.3 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 3 |
| 9009 | A | intetrigence | TopOut | - | 241 | yes | 74.9 / 100.1 | 83.5 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 28 |
| 9009 | B | intetrigence | TopOut | - | 390 | yes | 74.8 / 100.1 | 82.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 38 |
| 9010 | A | intetrigence | TopOut | - | 65 | yes | 71.9 / 100.1 | 78.6 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 5 |
| 9010 | B | intetrigence | TopOut | - | 529 | yes | 77.8 / 100.1 | 94.4 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 54 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 4115 | 0 / 4115 | 0 / 4115 | 0.767 | 66.5 | 5.8 / 16.6 | 0 |
| 1 | Cold Clear 2 | 40 | 4124 | 70 / 4124 | 0 / 4124 | 0.789 | 61.0 | 9.8 / 20.2 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- CC2 mode anytime ()
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
