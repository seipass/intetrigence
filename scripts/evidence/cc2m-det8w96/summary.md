# S2 match summary

- games: 20
- intetrigence wins / losses / unresolved / no-contest: 19 / 0 / 1 / 0
- win rate: 95.0% (Wilson 95% CI 76.4%..99.1%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 19, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 1

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 156 | yes | 70.8 / 100.1 | 81.1 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 20 |
| 9001 | B | intetrigence | TopOut | - | 44 | yes | 68.8 / 100.1 | 75.0 / 100.1 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9002 | A | intetrigence | TopOut | - | 612 | yes | 71.8 / 100.1 | 82.4 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 65 |
| 9002 | B | intetrigence | TopOut | - | 436 | yes | 73.6 / 100.1 | 94.4 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 49 |
| 9003 | A | intetrigence | TopOut | - | 527 | yes | 70.1 / 100.1 | 76.8 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 54 |
| 9003 | B | intetrigence | TopOut | - | 207 | yes | 72.6 / 100.1 | 84.1 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 21 |
| 9004 | A | intetrigence | TopOut | - | 67 | yes | 70.3 / 100.1 | 83.3 / 100.1 | 1.00 / 1.00 | 1.00 / 0.97 | 6 |
| 9004 | B | intetrigence | TopOut | - | 293 | yes | 70.1 / 100.1 | 76.8 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 31 |
| 9005 | A | intetrigence | TopOut | - | 246 | yes | 68.2 / 100.1 | 74.0 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 19 |
| 9005 | B | intetrigence | TopOut | - | 148 | yes | 67.7 / 100.1 | 74.8 / 100.1 | 1.00 / 1.00 | 1.00 / 0.97 | 16 |
| 9006 | A | intetrigence | TopOut | - | 412 | yes | 69.1 / 100.1 | 77.8 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 48 |
| 9006 | B | intetrigence | TopOut | - | 35 | yes | 63.7 / 100.1 | 73.0 / 100.1 | 1.00 / 1.00 | 1.00 / 0.97 | 8 |
| 9007 | A | intetrigence | TopOut | - | 218 | yes | 71.9 / 100.1 | 80.2 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 26 |
| 9007 | B | intetrigence | TopOut | - | 43 | yes | 70.5 / 100.1 | 76.0 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 3 |
| 9008 | A | intetrigence | TopOut | - | 105 | yes | 70.6 / 100.1 | 77.6 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 13 |
| 9008 | B | intetrigence | TopOut | - | 53 | yes | 69.3 / 100.1 | 76.4 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 9 |
| 9009 | A | intetrigence | TopOut | - | 210 | yes | 69.8 / 100.1 | 76.6 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 25 |
| 9009 | B | intetrigence | TopOut | - | 241 | yes | 67.3 / 100.1 | 75.0 / 100.1 | 1.00 / 1.00 | 1.00 / 0.97 | 23 |
| 9010 | A | intetrigence | TopOut | - | 532 | yes | 70.6 / 100.1 | 76.2 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 46 |
| 9010 | B | draw | TurnCap | - | 1000 | yes | 70.0 / 100.1 | 75.9 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 87 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 5576 | 0 / 5576 | 0 / 5576 | 0.761 | 86.2 | 5.5 / 16.6 | 0 |
| 1 | Cold Clear 2 | 40 | 5585 | 94 / 5585 | 0 / 5585 | 0.794 | 75.5 | 9.6 / 20.4 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- CC2 deterministic d8 w96 (clone-free slot scan)
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
