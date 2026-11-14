# S2 match summary

- games: 10
- intetrigence wins / losses / unresolved / no-contest: 2 / 8 / 0 / 0
- win rate: 20.0% (Wilson 95% CI 5.7%..51.0%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 2, forfeit-vision 0, forfeit-other 0; losses: TopOut 8, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 108 | yes | 97.0 / 114.5 | 99.2 / 130.8 | 1.00 / 1.00 | 1.00 / 0.07 | 12 |
| 9002 | A | attax | TopOut | - | 137 | yes | 97.1 / 109.8 | 99.2 / 129.5 | 1.00 / 1.00 | 1.00 / 0.10 | 20 |
| 9003 | A | attax | TopOut | - | 74 | yes | 97.0 / 112.3 | 98.5 / 131.5 | 1.00 / 1.00 | 1.00 / 0.07 | 5 |
| 9004 | A | attax | TopOut | - | 73 | yes | 97.2 / 108.6 | 98.5 / 125.0 | 1.00 / 1.00 | 1.00 / 0.03 | 5 |
| 9005 | A | attax | TopOut | - | 100 | yes | 96.9 / 110.7 | 98.8 / 128.7 | 1.00 / 1.00 | 1.00 / 0.11 | 6 |
| 9006 | A | intetrigence | TopOut | - | 44 | yes | 97.1 / 114.2 | 99.3 / 124.1 | 1.00 / 1.00 | 1.00 / 0.09 | 6 |
| 9007 | A | attax | TopOut | - | 69 | yes | 97.2 / 111.2 | 99.1 / 125.8 | 1.00 / 1.00 | 1.00 / 0.09 | 5 |
| 9008 | A | attax | TopOut | - | 105 | yes | 97.1 / 114.0 | 99.2 / 130.3 | 1.00 / 1.00 | 1.00 / 0.08 | 7 |
| 9009 | A | attax | TopOut | - | 210 | yes | 97.3 / 111.7 | 99.3 / 130.9 | 1.00 / 1.00 | 1.00 / 0.08 | 26 |
| 9010 | A | attax | TopOut | - | 81 | yes | 97.1 / 109.5 | 98.7 / 131.5 | 1.00 / 1.00 | 1.00 / 0.06 | 6 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1001 | 0 / 1001 | 0 / 1001 | 0.819 | 46.9 | 8.2 / 18.5 | 0 |
| 1 | clys | 40 | 993 | 915 / 993 | 0 / 993 | 1.045 | 24.5 | 7.2 / 17.2 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- Tuning smoke vs clys (tuning seeds 9001-9010, sheet A, 1000 turns; evaluation seeds untouched).
