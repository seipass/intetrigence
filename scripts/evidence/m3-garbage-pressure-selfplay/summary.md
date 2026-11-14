# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 13 / 7 / 0 / 0
- win rate: 65.0% (Wilson 95% CI 43.3%..81.9%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 8, forfeit-vision 0, forfeit-other 0; losses: TopOut 3, forfeit 0; turn-cap games 9

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.7 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | intetrigence | TopOut | - | 60 | yes | 96.8 / 96.5 | 97.7 / 98.0 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | intetrigence | TopOut | - | 84 | yes | 96.5 / 96.7 | 97.7 / 97.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TopOut | - | 284 | yes | 96.6 / 96.6 | 97.8 / 97.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | A | intetrigence | TurnCap | - | 300 | yes | 96.7 / 96.6 | 97.7 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | B | attax | TurnCap | - | 300 | yes | 96.6 / 96.5 | 97.9 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | A | intetrigence | TopOut | - | 109 | yes | 96.7 / 96.4 | 97.8 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | B | intetrigence | TurnCap | - | 300 | yes | 96.5 / 96.5 | 97.6 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9005 | A | attax | TopOut | - | 140 | yes | 96.4 / 96.6 | 97.5 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9005 | B | attax | TopOut | - | 244 | yes | 96.5 / 96.5 | 97.7 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9006 | A | intetrigence | TopOut | - | 240 | yes | 96.5 / 96.5 | 97.8 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9006 | B | intetrigence | TopOut | - | 92 | yes | 96.4 / 96.5 | 97.7 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9007 | A | attax | TurnCap | - | 300 | yes | 96.5 / 96.6 | 97.7 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9007 | B | intetrigence | TopOut | - | 265 | yes | 96.5 / 96.4 | 97.6 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9008 | A | intetrigence | TurnCap | - | 300 | yes | 96.6 / 96.5 | 97.7 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9008 | B | intetrigence | TurnCap | - | 300 | yes | 96.4 / 96.5 | 97.7 / 97.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9009 | A | attax | TurnCap | - | 300 | yes | 96.5 / 96.5 | 97.8 / 97.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9009 | B | intetrigence | TopOut | - | 118 | yes | 96.5 / 96.5 | 97.6 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9010 | A | intetrigence | TopOut | - | 72 | yes | 96.6 / 96.4 | 97.7 / 97.8 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9010 | B | attax | TurnCap | - | 300 | yes | 96.6 / 96.6 | 97.9 / 97.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 4404 | 0 / 4404 | 0 / 4404 | 1.087 | 93.0 | 0 |
| 1 | intetrigence | 40 | 4407 | 0 / 4407 | 0 / 4407 | 1.053 | 99.2 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 garbage-pressure self-play diagnostic (8 turns / 4 rows, --ms 100, 4 threads each). Not usable for final claims.
