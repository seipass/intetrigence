# S2 match summary

- games: 20
- intetrigence wins / losses / unresolved / no-contest: 10 / 0 / 0 / 10
- win rate: 50.0% (Wilson 95% CI 29.9%..70.1%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 10, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 47 | yes | 83.0 / 0.1 | 96.0 / 0.2 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9001 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9002 | A | intetrigence | TopOut | - | 104 | yes | 74.7 / 0.1 | 95.5 / 0.1 | 1.00 / 1.00 | 1.00 / 0.97 | 7 |
| 9002 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9003 | A | intetrigence | TopOut | - | 98 | yes | 76.0 / 0.1 | 90.5 / 0.1 | 1.00 / 1.00 | 1.00 / 0.98 | 14 |
| 9003 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9004 | A | intetrigence | TopOut | - | 277 | yes | 75.4 / 0.1 | 85.6 / 0.1 | 1.00 / 1.00 | 1.00 / 0.97 | 25 |
| 9004 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9005 | A | intetrigence | TopOut | - | 108 | yes | 73.6 / 0.1 | 86.1 / 0.1 | 1.00 / 1.00 | 1.00 / 0.99 | 16 |
| 9005 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9006 | A | intetrigence | TopOut | - | 295 | yes | 73.0 / 0.1 | 84.7 / 0.1 | 1.00 / 1.00 | 1.00 / 0.98 | 30 |
| 9006 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.7 | 0.0 / 1.7 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9007 | A | intetrigence | TopOut | - | 107 | yes | 74.0 / 0.1 | 86.5 / 0.1 | 1.00 / 1.00 | 1.00 / 1.00 | 15 |
| 9007 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9008 | A | intetrigence | TopOut | - | 84 | yes | 73.0 / 0.1 | 84.5 / 0.1 | 1.00 / 1.00 | 1.00 / 1.00 | 7 |
| 9008 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9009 | A | intetrigence | TopOut | - | 82 | yes | 71.3 / 0.1 | 80.0 / 0.1 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9009 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9010 | A | intetrigence | TopOut | - | 421 | yes | 77.1 / 0.1 | 93.9 / 0.1 | 1.00 / 1.00 | 1.00 / 0.98 | 47 |
| 9010 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1623 | 0 / 1623 | 0 / 1623 | 0.723 | 23.1 | 2.6 / 7.2 | 0 |
| 1 | Cold Clear 2 | 40 | 1623 | 29 / 1623 | 0 / 1623 | 0.717 | 23.4 | 5.2 / 10.7 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
