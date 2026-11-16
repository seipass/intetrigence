# S2 match summary

- games: 20
- intetrigence wins / losses / unresolved / no-contest: 8 / 2 / 0 / 10
- win rate: 40.0% (Wilson 95% CI 21.9%..61.3%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 8, forfeit-vision 0, forfeit-other 0; losses: TopOut 2, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 271 | yes | 69.9 / 0.1 | 76.2 / 0.1 | 1.00 / 1.00 | 1.00 / 0.98 | 26 |
| 9001 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9002 | A | intetrigence | TopOut | - | 828 | yes | 70.9 / 0.1 | 77.3 / 0.1 | 1.00 / 1.00 | 1.00 / 0.99 | 101 |
| 9002 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9003 | A | intetrigence | TopOut | - | 133 | yes | 68.5 / 0.1 | 76.5 / 0.1 | 1.00 / 1.00 | 1.00 / 0.98 | 11 |
| 9003 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.6 | 0.0 / 1.6 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9004 | A | intetrigence | TopOut | - | 69 | yes | 69.0 / 0.1 | 77.0 / 0.1 | 1.00 / 1.00 | 1.00 / 0.99 | 8 |
| 9004 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9005 | A | attax | TopOut | - | 58 | yes | 71.4 / 0.1 | 78.9 / 0.1 | 1.00 / 1.00 | 1.00 / 0.98 | 5 |
| 9005 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9006 | A | intetrigence | TopOut | - | 49 | yes | 69.7 / 0.1 | 77.1 / 0.1 | 1.00 / 1.00 | 1.00 / 0.90 | 7 |
| 9006 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9007 | A | intetrigence | TopOut | - | 154 | yes | 71.4 / 0.1 | 77.1 / 0.1 | 1.00 / 1.00 | 1.00 / 0.99 | 13 |
| 9007 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.3 | 0.0 / 1.3 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9008 | A | intetrigence | TopOut | - | 62 | yes | 70.6 / 0.1 | 75.2 / 0.1 | 1.00 / 1.00 | 1.00 / 0.97 | 7 |
| 9008 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.5 | 0.0 / 1.5 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9009 | A | attax | TopOut | - | 67 | yes | 66.0 / 0.1 | 75.8 / 0.1 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9009 | B | draw | Forfeit | other | 1 | yes | 0.0 / 1.4 | 0.0 / 1.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9010 | A | intetrigence | TopOut | - | 300 | yes | 70.9 / 0.1 | 79.0 / 0.1 | 1.00 / 1.00 | 1.00 / 0.99 | 36 |
| 9010 | B | draw | Forfeit | other | 1 | yes | 0.0 / 2.4 | 0.0 / 2.4 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1991 | 0 / 1991 | 0 / 1991 | 0.714 | 29.6 | 2.5 / 7.2 | 0 |
| 1 | Cold Clear 2 | 40 | 1989 | 33 / 1989 | 0 / 1989 | 0.732 | 27.0 | 4.9 / 9.8 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
