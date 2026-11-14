# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 4 / 6 / 0 / 0
- win rate: 40.0% (Wilson 95% CI 16.8%..68.7%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 3, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 3

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 130 | yes | 96.7 / 96.7 | 98.0 / 98.0 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | intetrigence | TopOut | - | 32 | yes | 96.6 / 96.5 | 97.9 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | intetrigence | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.7 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.9 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | A | attax | TopOut | - | 181 | yes | 96.5 / 96.6 | 97.5 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | B | intetrigence | TopOut | - | 80 | yes | 96.6 / 96.5 | 97.6 / 97.4 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | A | attax | TopOut | - | 218 | yes | 96.5 / 96.5 | 97.9 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | B | attax | TopOut | - | 81 | yes | 96.5 / 96.6 | 97.8 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9005 | A | attax | TopOut | - | 33 | yes | 96.5 / 96.5 | 97.8 / 97.2 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9005 | B | attax | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.8 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1653 | 0 / 1653 | 0 / 1653 | 1.134 | 94.7 | 0 |
| 1 | intetrigence | 40 | 1652 | 0 / 1652 | 0 / 1652 | 1.135 | 93.3 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 garbage-pressure self-play diagnostic (4 turns / 4 rows, --ms 100, 4 threads each). Not usable for final claims.
