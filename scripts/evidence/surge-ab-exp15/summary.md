# S2 match summary

- games: 4
- intetrigence wins / losses / draws / forfeits: 1 / 3 / 0 / 0
- win rate: 25.0% (Wilson 95% CI 4.6%..69.9%)
- replay verified: 4/4 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 1, forfeit 0; turn-cap games 2

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 99 | yes | 97.4 / 97.5 | 100.7 / 99.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | attax | TurnCap | - | 150 | yes | 97.3 / 97.3 | 100.2 / 99.3 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | intetrigence | TopOut | - | 105 | yes | 97.5 / 97.5 | 100.8 / 100.3 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TurnCap | - | 150 | yes | 97.2 / 97.2 | 99.2 / 99.3 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 504 | 0 / 504 | 0 / 504 | 0.857 | 56.5 | 6.8 / 20.0 | 0 |
| 1 | intetrigence | 40 | 503 | 0 / 503 | 0 / 503 | 0.938 | 46.5 | 7.5 / 18.8 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- surge A/B exp15 (surge_exponent=1.5)
