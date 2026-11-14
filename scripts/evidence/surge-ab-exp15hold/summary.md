# S2 match summary

- games: 4
- intetrigence wins / losses / draws / forfeits: 0 / 4 / 0 / 0
- win rate: 0.0% (Wilson 95% CI 0.0%..49.0%)
- replay verified: 4/4 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 0, forfeit-other 0; losses: TopOut 2, forfeit 0; turn-cap games 2

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 98 | yes | 97.4 / 97.3 | 99.4 / 99.5 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | attax | TopOut | - | 103 | yes | 97.4 / 97.4 | 101.1 / 100.0 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | attax | TurnCap | - | 150 | yes | 97.4 / 97.3 | 101.1 / 99.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TurnCap | - | 150 | yes | 97.2 / 97.2 | 100.0 / 99.0 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 501 | 0 / 501 | 0 / 501 | 0.880 | 53.8 | 6.7 / 19.5 | 0 |
| 1 | intetrigence | 40 | 500 | 0 / 500 | 0 / 500 | 0.942 | 46.0 | 7.2 / 18.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- surge A/B exp15hold (surge_exponent=1.5,surge_hold_penalty=-60)
