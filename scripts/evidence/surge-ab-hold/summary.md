# S2 match summary

- games: 4
- intetrigence wins / losses / draws / forfeits: 0 / 4 / 0 / 0
- win rate: 0.0% (Wilson 95% CI 0.0%..49.0%)
- replay verified: 4/4 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 0, forfeit-other 0; losses: TopOut 3, forfeit 0; turn-cap games 1

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 98 | yes | 97.2 / 97.4 | 99.0 / 100.0 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | attax | TurnCap | - | 150 | yes | 97.3 / 97.3 | 99.9 / 100.0 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | attax | TopOut | - | 111 | yes | 97.2 / 97.3 | 100.2 / 99.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TopOut | - | 40 | yes | 97.4 / 97.3 | 99.6 / 99.1 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 399 | 0 / 399 | 0 / 399 | 0.872 | 49.2 | 7.0 / 19.8 | 0 |
| 1 | intetrigence | 40 | 397 | 0 / 397 | 0 / 397 | 0.990 | 39.5 | 6.7 / 18.8 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- surge A/B hold (surge_hold_penalty=-60)
