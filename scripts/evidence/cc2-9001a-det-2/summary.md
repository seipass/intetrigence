# S2 match summary

- games: 1
- build id: `e24cca552047dc43`; binary: `C:\Users\Hasilan\.hasilan-code\agent\web-harness\agent-worktrees\1cf19499-c382-48a1-89f9-aad965f1bf06\target\release\arena.exe (1888768 bytes, mtime 1794815300)`
- intetrigence wins / losses / unresolved / no-contest: 1 / 0 / 0 / 0
- win rate: 100.0% (Wilson 95% CI 20.6%..100.0%)
- replay verified: 1/1 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 171 | yes | 72.2 / 100.1 | 80.7 / 100.2 | 1.00 / 1.00 | 1.00 / 0.98 | 27 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 171 | 0 / 171 | 0 / 171 | 0.848 | 61.0 | 6.7 / 16.0 | 0 |
| 1 | Cold Clear 2 | 40 | 171 | 4 / 171 | 0 / 171 | 0.772 | 71.0 | 9.7 / 20.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.
