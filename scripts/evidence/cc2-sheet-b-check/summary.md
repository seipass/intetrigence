# S2 match summary

- games: 1
- intetrigence wins / losses / unresolved / no-contest: 1 / 0 / 0 / 0
- win rate: 100.0% (Wilson 95% CI 20.6%..100.0%)
- replay verified: 1/1 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | B | intetrigence | TopOut | - | 280 | yes | 70.8 / 100.1 | 78.8 / 100.1 | 1.00 / 1.00 | 1.00 / 0.99 | 32 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 279 | 0 / 279 | 0 / 279 | 0.792 | 95.0 | 5.8 / 15.0 | 0 |
| 1 | Cold Clear 2 | 40 | 280 | 3 / 280 | 0 / 280 | 0.807 | 88.0 | 11.4 / 22.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
