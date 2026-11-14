# S2 match summary

- games: 4
- intetrigence wins / losses / draws / forfeits: 0 / 4 / 0 / 0
- win rate: 0.0% (Wilson 95% CI 0.0%..49.0%)
- replay verified: 4/4 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 99 | yes | 97.1 / 97.0 | 98.9 / 98.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | attax | TopOut | - | 107 | yes | 96.9 / 96.9 | 98.4 / 98.3 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | attax | TopOut | - | 51 | yes | 96.7 / 97.1 | 98.1 / 98.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TopOut | - | 116 | yes | 97.0 / 97.1 | 99.5 / 99.2 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 373 | 0 / 373 | 0 / 373 | 0.764 | 41.0 | 8.0 / 21.0 | 0 |
| 1 | intetrigence | 40 | 371 | 0 / 371 | 0 / 371 | 0.860 | 32.5 | 6.6 / 17.5 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 garbage-pressure self-play diagnostic (8 turns / 4 rows, --ms 100, 4 threads each). Not usable for final claims.
