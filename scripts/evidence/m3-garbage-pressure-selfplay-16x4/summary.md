# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 5 / 5 / 0 / 0
- win rate: 50.0% (Wilson 95% CI 23.7%..76.3%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 9

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.8 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | attax | TurnCap | - | 300 | yes | 96.5 / 96.6 | 97.8 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | intetrigence | TurnCap | - | 300 | yes | 96.4 / 96.6 | 97.7 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | intetrigence | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.7 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | A | intetrigence | TurnCap | - | 300 | yes | 96.6 / 96.4 | 97.5 / 97.4 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | B | attax | TurnCap | - | 300 | yes | 96.5 / 96.5 | 97.7 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | A | attax | TurnCap | - | 300 | yes | 96.6 / 96.4 | 97.8 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | B | attax | TurnCap | - | 300 | yes | 96.5 / 96.6 | 97.7 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9005 | A | intetrigence | TurnCap | - | 300 | yes | 96.5 / 96.5 | 97.7 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9005 | B | intetrigence | TopOut | - | 100 | yes | 96.4 / 96.5 | 97.7 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 2799 | 0 / 2799 | 0 / 2799 | 0.979 | 104.2 | 0 |
| 1 | intetrigence | 40 | 2800 | 0 / 2800 | 0 / 2800 | 1.001 | 98.3 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 garbage-pressure self-play diagnostic (16 turns / 4 rows, --ms 100, 4 threads each). Not usable for final claims.
