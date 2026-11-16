# S2 match summary

- games: 20
- intetrigence wins / losses / unresolved / no-contest: 17 / 3 / 0 / 0
- win rate: 85.0% (Wilson 95% CI 64.0%..94.8%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 17, forfeit-vision 0, forfeit-other 0; losses: TopOut 3, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 123 | yes | 72.9 / 100.1 | 81.8 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 10 |
| 9001 | B | intetrigence | TopOut | - | 95 | yes | 70.1 / 100.1 | 76.9 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 9 |
| 9002 | A | intetrigence | TopOut | - | 289 | yes | 72.0 / 100.1 | 83.6 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 37 |
| 9002 | B | intetrigence | TopOut | - | 82 | yes | 73.1 / 100.1 | 86.3 / 100.1 | 1.00 / 1.00 | 1.00 / 0.96 | 8 |
| 9003 | A | attax | TopOut | - | 443 | yes | 73.8 / 100.1 | 96.0 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 45 |
| 9003 | B | intetrigence | TopOut | - | 112 | yes | 74.6 / 100.1 | 92.0 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 17 |
| 9004 | A | attax | TopOut | - | 97 | yes | 72.0 / 100.1 | 78.6 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 7 |
| 9004 | B | intetrigence | TopOut | - | 425 | yes | 71.7 / 100.1 | 79.5 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 44 |
| 9005 | A | intetrigence | TopOut | - | 357 | yes | 73.1 / 100.1 | 96.0 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 38 |
| 9005 | B | intetrigence | TopOut | - | 307 | yes | 84.3 / 100.1 | 98.0 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 33 |
| 9006 | A | intetrigence | TopOut | - | 35 | yes | 72.5 / 100.1 | 85.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 5 |
| 9006 | B | intetrigence | TopOut | - | 741 | yes | 75.3 / 100.1 | 93.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 73 |
| 9007 | A | intetrigence | TopOut | - | 128 | yes | 71.6 / 100.1 | 78.0 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 11 |
| 9007 | B | intetrigence | TopOut | - | 59 | yes | 70.9 / 100.1 | 77.4 / 100.1 | 1.00 / 1.00 | 1.00 / 0.95 | 8 |
| 9008 | A | intetrigence | TopOut | - | 248 | yes | 72.2 / 100.1 | 80.3 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 20 |
| 9008 | B | intetrigence | TopOut | - | 54 | yes | 70.4 / 100.1 | 78.2 / 100.1 | 1.00 / 1.00 | 1.00 / 0.96 | 10 |
| 9009 | A | intetrigence | TopOut | - | 82 | yes | 70.3 / 100.1 | 79.0 / 100.1 | 1.00 / 1.00 | 1.00 / 0.95 | 5 |
| 9009 | B | intetrigence | TopOut | - | 31 | yes | 66.6 / 100.1 | 78.6 / 100.1 | 1.00 / 1.00 | 1.00 / 0.97 | 1 |
| 9010 | A | intetrigence | TopOut | - | 115 | yes | 71.2 / 100.1 | 76.5 / 100.1 | 1.00 / 1.00 | 1.00 / 0.98 | 17 |
| 9010 | B | attax | TopOut | - | 243 | yes | 71.6 / 100.1 | 78.3 / 100.1 | 1.00 / 1.00 | 1.00 / 0.97 | 28 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 4057 | 0 / 4057 | 0 / 4057 | 0.708 | 61.6 | 5.2 / 15.8 | 0 |
| 1 | Cold Clear 2 | 40 | 4064 | 84 / 4064 | 0 / 4064 | 0.750 | 52.9 | 9.8 / 21.3 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- CC2 d8 w96, height risk floor 0.0
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
