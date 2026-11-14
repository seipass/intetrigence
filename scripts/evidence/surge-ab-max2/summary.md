# S2 match summary

- games: 8
- intetrigence wins / losses / draws / forfeits: 4 / 4 / 0 / 0
- win rate: 50.0% (Wilson 95% CI 21.5%..78.5%)
- replay verified: 8/8 (failures: 0)
- win breakdown: TopOut 4, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 98 | yes | 97.5 / 97.4 | 99.3 / 100.3 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | intetrigence | TopOut | - | 177 | yes | 97.2 / 97.3 | 99.8 / 98.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | attax | TopOut | - | 83 | yes | 97.4 / 97.5 | 98.8 / 99.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | attax | TopOut | - | 83 | yes | 96.9 / 97.0 | 99.2 / 99.3 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | A | intetrigence | TopOut | - | 58 | yes | 97.2 / 96.7 | 100.7 / 98.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | B | intetrigence | TopOut | - | 56 | yes | 97.1 / 96.8 | 98.6 / 98.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | A | attax | TopOut | - | 50 | yes | 96.9 / 97.0 | 98.5 / 98.4 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | B | intetrigence | TopOut | - | 115 | yes | 97.2 / 97.0 | 99.6 / 99.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 717 | 0 / 717 | 0 / 717 | 0.911 | 40.0 | 7.2 / 19.2 | 0 |
| 1 | intetrigence | 40 | 717 | 0 / 717 | 0 / 717 | 0.914 | 39.4 | 8.2 / 19.9 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- surge A/B max2 (surge_exponent=1.8,surge_hold_penalty=-100)
