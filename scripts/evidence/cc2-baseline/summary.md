# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 16 / 4 / 0 / 0
- win rate: 80.0% (Wilson 95% CI 58.4%..91.9%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 16, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 145 | yes | 97.5 / 100.1 | 100.4 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 18 |
| 9001 | B | intetrigence | TopOut | - | 70 | yes | 97.1 / 100.1 | 99.1 / 100.3 | 1.00 / 1.00 | 1.00 / 0.99 | 11 |
| 9002 | A | intetrigence | TopOut | - | 138 | yes | 97.3 / 100.1 | 99.9 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 15 |
| 9002 | B | intetrigence | TopOut | - | 106 | yes | 97.2 / 100.1 | 99.3 / 100.3 | 1.00 / 1.00 | 1.00 / 0.97 | 16 |
| 9003 | A | intetrigence | TopOut | - | 297 | yes | 97.3 / 100.1 | 99.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 35 |
| 9003 | B | attax | TopOut | - | 146 | yes | 97.5 / 100.1 | 100.9 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 20 |
| 9004 | A | intetrigence | TopOut | - | 89 | yes | 97.9 / 100.1 | 101.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 10 |
| 9004 | B | attax | TopOut | - | 76 | yes | 97.1 / 100.1 | 98.5 / 100.4 | 1.00 / 1.00 | 1.00 / 1.00 | 7 |
| 9005 | A | intetrigence | TopOut | - | 97 | yes | 97.3 / 100.1 | 100.6 / 100.3 | 1.00 / 1.00 | 1.00 / 0.98 | 13 |
| 9005 | B | intetrigence | TopOut | - | 54 | yes | 97.5 / 100.2 | 100.1 / 100.3 | 1.00 / 1.00 | 1.00 / 0.98 | 8 |
| 9006 | A | intetrigence | TopOut | - | 187 | yes | 97.5 / 100.1 | 99.4 / 100.3 | 1.00 / 1.00 | 1.00 / 0.98 | 26 |
| 9006 | B | intetrigence | TopOut | - | 67 | yes | 97.3 / 100.1 | 99.4 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 9 |
| 9007 | A | attax | TopOut | - | 99 | yes | 97.5 / 100.1 | 99.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 9 |
| 9007 | B | intetrigence | TopOut | - | 35 | yes | 97.8 / 100.1 | 99.8 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9008 | A | intetrigence | TopOut | - | 486 | yes | 97.2 / 100.1 | 99.3 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 65 |
| 9008 | B | intetrigence | TopOut | - | 185 | yes | 97.0 / 100.1 | 98.8 / 100.3 | 1.00 / 1.00 | 1.00 / 0.99 | 19 |
| 9009 | A | attax | TopOut | - | 115 | yes | 97.2 / 100.1 | 99.6 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 10 |
| 9009 | B | intetrigence | TopOut | - | 208 | yes | 97.3 / 100.1 | 99.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 28 |
| 9010 | A | intetrigence | TopOut | - | 42 | yes | 96.8 / 100.1 | 99.0 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 5 |
| 9010 | B | intetrigence | TopOut | - | 19 | yes | 97.2 / 100.1 | 99.2 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 3 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 2653 | 0 / 2653 | 0 / 2653 | 0.830 | 35.3 | 5.7 / 15.0 | 2 |
| 1 | Cold Clear 2 | 40 | 2659 | 41 / 2659 | 0 / 2659 | 0.759 | 44.0 | 10.3 / 21.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- CC2 baseline, tuning seeds 9001-9010 x both sheets, 1000 turns, 100ms/8t
