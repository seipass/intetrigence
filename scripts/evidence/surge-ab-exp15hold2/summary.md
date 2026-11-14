# S2 match summary

- games: 8
- intetrigence wins / losses / draws / forfeits: 5 / 3 / 0 / 0
- win rate: 62.5% (Wilson 95% CI 30.6%..86.3%)
- replay verified: 8/8 (failures: 0)
- win breakdown: TopOut 5, forfeit-vision 0, forfeit-other 0; losses: TopOut 3, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | attax | TopOut | - | 98 | yes | 97.4 / 97.3 | 99.5 / 101.2 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9001 | B | attax | TopOut | - | 115 | yes | 97.3 / 97.2 | 100.5 / 99.6 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | A | attax | TopOut | - | 141 | yes | 97.6 / 97.4 | 100.7 / 100.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9002 | B | intetrigence | TopOut | - | 186 | yes | 97.3 / 97.0 | 99.2 / 98.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | A | intetrigence | TopOut | - | 64 | yes | 97.1 / 96.7 | 98.7 / 99.1 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9003 | B | intetrigence | TopOut | - | 59 | yes | 97.1 / 97.4 | 100.4 / 99.7 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | A | intetrigence | TopOut | - | 75 | yes | 97.4 / 97.3 | 99.9 / 99.9 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |
| 9004 | B | intetrigence | TopOut | - | 106 | yes | 97.1 / 97.4 | 99.0 / 100.5 | 1.00 / 1.00 | 1.00 / 1.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 841 | 0 / 841 | 0 / 841 | 0.888 | 44.1 | 7.4 / 19.5 | 0 |
| 1 | intetrigence | 40 | 842 | 0 / 842 | 0 / 842 | 0.855 | 47.9 | 8.5 / 19.8 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- surge A/B exp15hold2 (surge_exponent=1.5,surge_hold_penalty=-60)
