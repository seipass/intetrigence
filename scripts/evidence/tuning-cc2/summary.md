# S2 match summary

- games: 10
- intetrigence wins / losses / unresolved / no-contest: 10 / 0 / 0 / 0
- win rate: 100.0% (Wilson 95% CI 72.2%..100.0%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 10, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 446 | yes | 97.4 / 100.1 | 99.8 / 100.3 | 1.00 / 1.00 | 1.00 / 0.99 | 57 |
| 9002 | A | intetrigence | TopOut | - | 299 | yes | 97.6 / 100.1 | 100.4 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 46 |
| 9003 | A | intetrigence | TopOut | - | 244 | yes | 97.5 / 100.1 | 99.4 / 100.2 | 1.00 / 1.00 | 1.00 / 0.99 | 38 |
| 9004 | A | intetrigence | TopOut | - | 143 | yes | 97.3 / 100.1 | 99.2 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 22 |
| 9005 | A | intetrigence | TopOut | - | 94 | yes | 97.7 / 100.1 | 100.3 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 16 |
| 9006 | A | intetrigence | TopOut | - | 250 | yes | 97.3 / 100.1 | 99.8 / 100.2 | 1.00 / 1.00 | 1.00 / 0.96 | 31 |
| 9007 | A | intetrigence | TopOut | - | 66 | yes | 97.8 / 100.1 | 99.9 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 9 |
| 9008 | A | intetrigence | TopOut | - | 54 | yes | 97.9 / 100.1 | 103.0 / 100.2 | 1.00 / 1.00 | 1.00 / 1.00 | 7 |
| 9009 | A | intetrigence | TopOut | - | 70 | yes | 97.5 / 100.1 | 99.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.97 | 7 |
| 9010 | A | intetrigence | TopOut | - | 70 | yes | 96.7 / 100.1 | 98.7 / 100.3 | 1.00 / 1.00 | 1.00 / 0.96 | 11 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1736 | 0 / 1736 | 0 / 1736 | 0.886 | 49.3 | 6.5 / 17.9 | 0 |
| 1 | Cold Clear 2 | 40 | 1736 | 32 / 1736 | 0 / 1736 | 0.789 | 64.0 | 11.1 / 21.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- Tuning smoke vs CC2 (tuning seeds 9001-9010, sheet A, 1000 turns; evaluation seeds untouched).
